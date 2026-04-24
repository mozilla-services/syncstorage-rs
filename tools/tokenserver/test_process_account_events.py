# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Tests for the tokenserver process_account_events module."""

import json
import os
import unittest

import pytest

from pyramid import testing
from testfixtures import LogCapture

from database import Database
from process_account_events import process_account_event


PATTERN = "{node}/1.5/{uid}"
EMAIL = "test@example.com"
UID = "test"
ISS = "example.com"


def message_body(**kwds):
    """Build a JSON-encoded SNS message body from the given keyword arguments."""
    return json.dumps({"Message": json.dumps(kwds)})


class ProcessAccountEventsTestCase(unittest.TestCase):
    """Base test case for processing account events."""

    def get_ini(self):
        """Return the path to the test INI configuration file."""
        return os.path.join(os.path.dirname(__file__), "test_sql.ini")

    def setUp(self):
        """Set up test fixtures."""
        self.database = Database()
        self.database.add_service("sync-1.5", r"{node}/1.5/{uid}")
        self.database.add_node("https://phx12", 100)
        self.logs = LogCapture()

    def tearDown(self):
        """Tear down test fixtures."""
        self.logs.uninstall()
        testing.tearDown()

        cursor = self.database._execute_sql("DELETE FROM users")
        cursor.close()

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
        """Clear all captured log records."""
        del self.logs.records[:]

    def process_account_event(self, body):
        """Process the given account event body against the test database."""
        process_account_event(self.database, body)


class TestProcessAccountEvents(ProcessAccountEventsTestCase):
    """Tests for processing account events from SNS messages."""

    def test_delete_user(self):
        """Test delete user."""
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
        """Test delete user by legacy uid format."""
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
        """Test delete user who is not in the db."""
        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

        self.process_account_event(message_body(event="delete", uid=UID, iss=ISS))

        records = list(self.database.get_user_records(EMAIL))
        self.assertEqual(len(records), 0)

    def test_reset_user(self):
        """Test reset user."""
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
        """Test reset user by legacy uid format."""
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
        """Test reset user who is not in the db."""
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
        """Test password change."""
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
        """Test password change user not in db."""
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
        """Test malformed events."""
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
        """Test update with no keys changed at."""
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
        """Test update with no keys changed at2."""
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
    """Tests for processing account events with forced Spanner routing."""

    def setUp(self):
        """Set up test fixtures."""
        super().setUp()
        self.database.spanner_node_id = self.database.get_node_id("https://phx12")

    def test_delete_user_force_spanner(self):
        """Test delete user force spanner."""
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


# =============================================================================
# Standalone pytest equivalents (classes above retained until CI confirms)
# =============================================================================


@pytest.fixture(scope="function")
def logs():
    """Per-test log capture, installed before the test and uninstalled after."""
    lc = LogCapture()
    yield lc
    lc.uninstall()


@pytest.fixture(scope="function")
def db_spanner(db):
    """DB fixture with spanner_node_id set to the default node."""
    db.spanner_node_id = db.get_node_id("https://phx12")
    return db


def _assert_message_was_logged(lc, msg):
    for r in lc.records:
        if msg in r.getMessage():
            break
    else:
        assert False, "message %r was not logged" % (msg,)


def _clear_logs(lc):
    del lc.records[:]


def test_delete_user(db):
    """Test delete user."""
    db.allocate_user(EMAIL)
    user = db.get_user(EMAIL)
    db.update_user(user, client_state="abcdef")
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    assert records[0]["replaced_at"] is not None

    process_account_event(db, message_body(event="delete", uid=UID, iss=ISS))

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    for row in records:
        assert row["replaced_at"] is not None


def test_delete_user_by_legacy_uid_format(db):
    """Test delete user by legacy uid format."""
    db.allocate_user(EMAIL)
    user = db.get_user(EMAIL)
    db.update_user(user, client_state="abcdef")
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    assert records[0]["replaced_at"] is not None

    process_account_event(db, message_body(event="delete", uid=EMAIL))

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    for row in records:
        assert row["replaced_at"] is not None


def test_delete_user_who_is_not_in_the_db(db):
    """Test delete user who is not in the db."""
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0

    process_account_event(db, message_body(event="delete", uid=UID, iss=ISS))

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0


def test_reset_user(db):
    """Test reset user."""
    db.allocate_user(EMAIL, generation=12)

    process_account_event(
        db, message_body(event="reset", uid=UID, iss=ISS, generation=43)
    )

    user = db.get_user(EMAIL)
    assert user["generation"] == 42


def test_reset_user_by_legacy_uid_format(db):
    """Test reset user by legacy uid format."""
    db.allocate_user(EMAIL, generation=12)

    process_account_event(db, message_body(event="reset", uid=EMAIL, generation=43))

    user = db.get_user(EMAIL)
    assert user["generation"] == 42


def test_reset_user_who_is_not_in_the_db(db):
    """Test reset user who is not in the db."""
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0

    process_account_event(
        db, message_body(event="reset", uid=UID, iss=ISS, generation=43)
    )

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0


def test_password_change(db):
    """Test password change."""
    db.allocate_user(EMAIL, generation=12)

    process_account_event(
        db, message_body(event="passwordChange", uid=UID, iss=ISS, generation=43)
    )

    user = db.get_user(EMAIL)
    assert user["generation"] == 42


def test_password_change_user_not_in_db(db):
    """Test password change user not in db."""
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0

    process_account_event(
        db, message_body(event="passwordChange", uid=UID, iss=ISS, generation=43)
    )

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 0


def test_malformed_events(db, logs):
    """Test malformed events."""
    # Unknown event type.
    process_account_event(
        db, message_body(event="party", uid=UID, iss=ISS, generation=43)
    )
    _assert_message_was_logged(logs, "Dropping unknown event type")
    _clear_logs(logs)

    # Missing event type.
    process_account_event(db, message_body(uid=UID, iss=ISS, generation=43))
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Missing uid.
    process_account_event(db, message_body(event="delete", iss=ISS))
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Missing generation for reset events.
    process_account_event(db, message_body(event="reset", uid=UID, iss=ISS))
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Missing generation for passwordChange events.
    process_account_event(db, message_body(event="passwordChange", uid=UID, iss=ISS))
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Missing issuer with non-email uid.
    process_account_event(db, message_body(event="delete", uid=UID))
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Non-JSON garbage.
    process_account_event(db, "wat")
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Non-JSON garbage in Message field.
    process_account_event(db, '{ "Message": "wat" }')
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)

    # Badly-typed JSON value in Message field.
    process_account_event(db, '{ "Message": "[1, 2, 3"] }')
    _assert_message_was_logged(logs, "Invalid account message")
    _clear_logs(logs)


def test_update_with_no_keys_changed_at(db):
    """Test update with no keys changed at."""
    user = db.allocate_user(EMAIL, generation=12, keys_changed_at=None)

    db.update_user(user, generation=13)
    db.update_user(user, generation=14, client_state="abcdef", keys_changed_at=13)

    process_account_event(
        db, message_body(event="reset", uid=UID, iss=ISS, generation=43)
    )

    user = db.get_user(EMAIL)
    assert user["generation"] == 42


def test_update_with_no_keys_changed_at2(db):
    """Test update with no keys changed at (replaced record variant)."""
    user = db.allocate_user(EMAIL, generation=12, keys_changed_at=None)
    db.replace_user_record(user["uid"])

    process_account_event(
        db, message_body(event="reset", uid=UID, iss=ISS, generation=43)
    )

    user = db.get_user(EMAIL)
    assert user["generation"] == 42


def test_delete_user_force_spanner(db_spanner):
    """Test delete user with spanner_node_id set (force-spanner routing)."""
    db = db_spanner
    db.allocate_user(EMAIL)
    user = db.get_user(EMAIL)
    db.update_user(user, client_state="abcdef")
    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    assert records[0]["replaced_at"] is not None

    process_account_event(db, message_body(event="delete", uid=UID, iss=ISS))

    records = list(db.get_user_records(EMAIL))
    assert len(records) == 2
    for row in records:
        assert row["replaced_at"] is not None
