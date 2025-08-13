# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

import json
import os
import unittest

from pyramid import testing
from testfixtures import LogCapture

from database import Database
from process_account_events import process_account_event


PATTERN = "{node}/1.5/{uid}"
EMAIL = "test@example.com"
UID = "test"
ISS = "example.com"


def message_body(**kwds):
    return json.dumps({"Message": json.dumps(kwds)})


class ProcessAccountEventsTestCase(unittest.TestCase):
    def get_ini(self):
        return os.path.join(os.path.dirname(__file__), "test_sql.ini")

    def setUp(self):
        self.database = Database()
        self.database.add_service("sync-1.5", r"{node}/1.5/{uid}")
        self.database.add_node("https://phx12", 100)
        self.logs = LogCapture()

    def tearDown(self):
        self.logs.uninstall()
        testing.tearDown()

        cursor = self.database._execute_sql("DELETE FROM users")
        cursor.close

        cursor = self.database._execute_sql("DELETE FROM nodes")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM services")
        cursor.close()

    def assertMessageWasLogged(self, msg):
        """Check that a metric was logged during the request."""
        for r in self.logs.records:
            if msg in r.getMessage():
                break
        else:
            assert False, "message %r was not logged" % (msg,)

    def clearLogs(self):
        del self.logs.records[:]

    def process_account_event(self, body):
        process_account_event(self.database, body)


class TestProcessAccountEvents(ProcessAccountEventsTestCase):
    def test_delete_user(self):
        self.database.allocate_user(EMAIL)
        user = self.database.get_user(EMAIL)
        self.database.update_user(user, client_state="abcdef")
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        self.assertTrue(records[0]["replaced_at"] is not None)

        self.process_account_event(
            message_body(
                event="delete",
                uid=UID,
                iss=ISS,
            )
        )

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        for row in records:
            self.assertTrue(row["replaced_at"] is not None)

    def test_delete_user_by_legacy_uid_format(self):
        self.database.allocate_user(EMAIL)
        user = self.database.get_user(EMAIL)
        self.database.update_user(user, client_state="abcdef")
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        self.assertTrue(records[0]["replaced_at"] is not None)

        self.process_account_event(
            message_body(
                event="delete",
                uid=EMAIL,
            )
        )

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        for row in records:
            self.assertTrue(row["replaced_at"] is not None)

    def test_delete_user_who_is_not_in_the_db(self):
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

        self.process_account_event(message_body(event="delete", uid=UID, iss=ISS))

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

    def test_reset_user(self):
        self.database.allocate_user(EMAIL, generation=12)

        self.process_account_event(
            message_body(
                event="reset",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        user = self.database.get_user(EMAIL)
        self.assertEqual(user["generation"], 42)

    def test_reset_user_by_legacy_uid_format(self):
        self.database.allocate_user(EMAIL, generation=12)

        self.process_account_event(
            message_body(
                event="reset",
                uid=EMAIL,
                generation=43,
            )
        )

        user = self.database.get_user(EMAIL)
        self.assertEqual(user["generation"], 42)

    def test_reset_user_who_is_not_in_the_db(self):
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

        self.process_account_event(
            message_body(
                event="reset",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

    def test_password_change(self):
        self.database.allocate_user(EMAIL, generation=12)

        self.process_account_event(
            message_body(
                event="passwordChange",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        user = self.database.get_user(EMAIL)
        self.assertEqual(user["generation"], 42)

    def test_password_change_user_not_in_db(self):
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

        self.process_account_event(
            message_body(
                event="passwordChange",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

    def test_malformed_events(self):
        # Unknown event type.
        self.process_account_event(
            message_body(
                event="party",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )
        self.assertMessageWasLogged("Dropping unknown event type")
        self.clearLogs()

        # Missing event type.
        self.process_account_event(
            message_body(
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Missing uid.
        self.process_account_event(
            message_body(
                event="delete",
                iss=ISS,
            )
        )
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Missing generation for reset events.
        self.process_account_event(
            message_body(
                event="reset",
                uid=UID,
                iss=ISS,
            )
        )
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Missing generation for passwordChange events.
        self.process_account_event(
            message_body(
                event="passwordChange",
                uid=UID,
                iss=ISS,
            )
        )
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Missing issuer with nonemail uid
        self.process_account_event(
            message_body(
                event="delete",
                uid=UID,
            )
        )
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Non-JSON garbage.
        self.process_account_event("wat")
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Non-JSON garbage in Message field.
        self.process_account_event('{ "Message": "wat" }')
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

        # Badly-typed JSON value in Message field.
        self.process_account_event('{ "Message": "[1, 2, 3"] }')
        self.assertMessageWasLogged("Invalid account message")
        self.clearLogs()

    def test_update_with_no_keys_changed_at(self):
        user = self.database.allocate_user(EMAIL, generation=12, keys_changed_at=None)

        # These update_user calls previously failed (SYNC-3633)
        self.database.update_user(user, generation=13)
        self.database.update_user(
            user, generation=14, client_state="abcdef", keys_changed_at=13
        )

        self.process_account_event(
            message_body(
                event="reset",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        user = self.database.get_user(EMAIL)
        self.assertEqual(user["generation"], 42)

    def test_update_with_no_keys_changed_at2(self):
        user = self.database.allocate_user(EMAIL, generation=12, keys_changed_at=None)
        # Mark the current record as replaced. This can probably only occur
        # during a race condition in row creation
        self.database.replace_user_record(user["uid"])

        self.process_account_event(
            message_body(
                event="reset",
                uid=UID,
                iss=ISS,
                generation=43,
            )
        )

        user = self.database.get_user(EMAIL)
        self.assertEqual(user["generation"], 42)


class TestProcessAccountEventsForceSpanner(ProcessAccountEventsTestCase):
    def setUp(self):
        super().setUp()
        self.database.spanner_node_id = self.database.get_node_id("https://phx12")

    def test_delete_user_force_spanner(self):
        self.database.allocate_user(EMAIL)
        user = self.database.get_user(EMAIL)
        self.database.update_user(user, client_state="abcdef")
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        self.assertTrue(records[0]["replaced_at"] is not None)

        self.process_account_event(
            message_body(
                event="delete",
                uid=UID,
                iss=ISS,
            )
        )

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 2)
        for row in records:
            self.assertTrue(row["replaced_at"] is not None)
