# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

import hawkauthlib
import re
import threading
import tokenlib
import unittest
from wsgiref.simple_server import make_server

from database import Database
from purge_old_records import purge_old_records


class TestPurgeOldRecords(unittest.TestCase):
    """A testcase for proper functioning of the purge_old_records.py script.

    This is a tricky one, because we have to actually run the script and
    test that it does the right thing.  We also run a mock downstream service
    so we can test that data-deletion requests go through ok.
    """

    @classmethod
    def setUpClass(cls):
        cls.service_requests = []
        cls.service_node = "http://localhost:8002"
        cls.service = make_server("localhost", 8002, cls._service_app)
        target = cls.service.serve_forever
        cls.service_thread = threading.Thread(target=target)
        # Note: If the following `start` causes the test thread to hang,
        # you may need to specify
        # `[app::pyramid.app] pyramid.worker_class = sync` in the test_*.ini
        # files
        cls.service_thread.start()
        # This silences nuisance on-by-default logging output.
        cls.service.RequestHandlerClass.log_request = lambda *a: None

    def setUp(self):
        super(TestPurgeOldRecords, self).setUp()

        # Configure the node-assignment backend to talk to our test service.
        self.database = Database()
        self.database.add_service('sync-1.5', r'{node}/1.5/{uid}')
        self.database.add_node(self.service_node, 100)

    def tearDown(self):
        cursor = self.database._execute_sql('DELETE FROM users')
        cursor.close()

        cursor = self.database._execute_sql('DELETE FROM nodes')
        cursor.close()

        cursor = self.database._execute_sql('DELETE FROM services')
        cursor.close()

        del self.service_requests[:]

    def test_settings(self, args:dict[str, any]=dict()):
        class Settings(object):
            pass

        settings = Settings()
        setattr(settings, "force", args.get("force", False))
        setattr(settings, "dryrun", args.get("dryrun", False))
        setattr(settings, "max_records", args.get("max_records", 20))

        return settings


    @classmethod
    def tearDownClass(cls):
        cls.service.shutdown()
        cls.service_thread.join()

    @classmethod
    def _service_app(cls, environ, start_response):
        cls.service_requests.append(environ)
        start_response("200 OK", [])
        return ""

    def test_purging_of_old_user_records(self):
        # Make some old user records.
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa",
                                           generation=123)
        self.database.update_user(user, client_state="bb",
                                  generation=456, keys_changed_at=450)
        self.database.update_user(user, client_state="cc",
                                  generation=789)
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 3)
        user = self.database.get_user(email)
        self.assertEquals(user["client_state"], "cc")
        self.assertEquals(len(user["old_client_states"]), 2)

        # The default grace-period should prevent any cleanup.
        node_secret = "SECRET"
        self.assertTrue(purge_old_records(node_secret))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 3)
        self.assertEqual(len(self.service_requests), 0)

        # With no grace period, we should cleanup two old records.
        self.assertTrue(purge_old_records(node_secret, grace_period=0))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(len(self.service_requests), 2)

        # Check that the proper delete requests were made to the service.
        expected_kids = ["0000000000450-uw", "0000000000123-qg"]
        for i, environ in enumerate(self.service_requests):
            # They must be to the correct path.
            self.assertEquals(environ["REQUEST_METHOD"], "DELETE")
            self.assertTrue(re.match("/1.5/[0-9]+", environ["PATH_INFO"]))
            # They must have a correct request signature.
            token = hawkauthlib.get_id(environ)
            secret = tokenlib.get_derived_secret(token, secret=node_secret)
            self.assertTrue(hawkauthlib.check_signature(environ, secret))
            userdata = tokenlib.parse_token(token, secret=node_secret)
            self.assertTrue("uid" in userdata)
            self.assertTrue("node" in userdata)
            self.assertEqual(userdata["fxa_uid"], "test")
            self.assertEqual(userdata["fxa_kid"], expected_kids[i])

        # Check that the user's current state is unaffected
        user = self.database.get_user(email)
        self.assertEquals(user["client_state"], "cc")
        self.assertEquals(len(user["old_client_states"]), 0)

    def test_purging_is_not_done_on_downed_nodes(self):
        # Make some old user records.
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.update_user(user, client_state="bb")
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)

        # With the node down, we should not purge any records.
        self.database.update_node(self.service_node, downed=1)
        self.assertTrue(purge_old_records(node_secret, grace_period=0))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)
        self.assertEqual(len(self.service_requests), 0)

        # With the node back up, we should purge correctly.
        self.database.update_node(self.service_node, downed=0)
        self.assertTrue(purge_old_records(node_secret, grace_period=0))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(len(self.service_requests), 1)

    def test_force(self):
        # Make some old user records.
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.update_user(user, client_state="bb")
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)

        # With the node down, we should be able to purge any records.
        self.database.update_node(self.service_node, downed=1)

        settings = self.test_settings({"force":True})
        self.assertTrue(purge_old_records(node_secret, grace_period=0, settings=settings))

        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(len(self.service_requests), 1)

    def test_dry_run(self):
        # Make some old user records.
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.update_user(user, client_state="bb")
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)

        self.database.update_node(self.service_node, downed=1)

        # Don't actually perform anything destructive.
        settings = self.test_settings({"dryrun": True})
        self.assertTrue(purge_old_records(node_secret, grace_period=0, settings=settings))

        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)
        self.assertEqual(len(self.service_requests), 0)
