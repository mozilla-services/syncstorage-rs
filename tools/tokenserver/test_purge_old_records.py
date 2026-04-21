# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Tests for the tokenserver purge_old_records script."""

import pytest

import hawkauthlib
import re
import threading
import tokenlib
import unittest
from wsgiref.simple_server import make_server

from database import Database
from purge_old_records import purge_old_records


class PurgeOldRecordsTestCase(unittest.TestCase):
    """Test case for the purge_old_records functionality."""

    @classmethod
    def setUpClass(cls):
        """Set up class-level test fixtures."""
        cls.service_requests = []
        cls.service = make_server("localhost", 0, cls._service_app)
        host, port = cls.service.server_address
        cls.service_node = f"http://{host}:{port}"
        cls.service_thread = threading.Thread(target=cls.service.serve_forever)
        # Note: If the following `start` causes the test thread to hang,
        # you may need to specify
        # `[app::pyramid.app] pyramid.worker_class = sync` in the test_*.ini
        # files
        cls.service_thread.start()
        # This silences nuisance on-by-default logging output.
        cls.service.RequestHandlerClass.log_request = lambda *a: None

    def setUp(self):
        """Set up test fixtures."""
        super().setUp()

        # Configure the node-assignment backend to talk to our test service.
        self.database = Database()
        self.database.add_service("sync-1.5", r"{node}/1.5/{uid}")
        self.database.add_node(self.service_node, 100)

    def tearDown(self):
        """Tear down test fixtures."""
        cursor = self.database._execute_sql("DELETE FROM users")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM nodes")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM services")
        cursor.close()

        del self.service_requests[:]

    @classmethod
    def tearDownClass(cls):
        """Tear down class-level test fixtures."""
        cls.service.shutdown()
        cls.service_thread.join()

    @classmethod
    def _service_app(cls, environ, start_response):
        cls.service_requests.append(environ)
        start_response("200 OK", [])
        return ""


class TestPurgeOldRecords(PurgeOldRecordsTestCase):
    """A testcase for proper functioning of the purge_old_records.py script.

    This is a tricky one, because we have to actually run the script and
    test that it does the right thing.  We also run a mock downstream service
    so we can test that data-deletion requests go through ok.
    """

    def test_purging_of_old_user_records(self):
        """Test purging of old user records."""
        # Make some old user records.
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa", generation=123)
        self.database.update_user(
            user, client_state="bb", generation=456, keys_changed_at=450
        )
        self.database.update_user(user, client_state="cc", generation=789)
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 3)
        user = self.database.get_user(email)
        self.assertEqual(user["client_state"], "cc")
        self.assertEqual(len(user["old_client_states"]), 2)

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
            self.assertEqual(environ["REQUEST_METHOD"], "DELETE")
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
        self.assertEqual(user["client_state"], "cc")
        self.assertEqual(len(user["old_client_states"]), 0)

    def test_purging_is_not_done_on_downed_nodes(self):
        """Test purging is not done on downed nodes."""
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
        """Test force."""
        # Make some old user records.
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.update_user(user, client_state="bb")
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)

        # With the node down, we should be able to purge any records.
        self.database.update_node(self.service_node, downed=1)

        self.assertTrue(purge_old_records(node_secret, grace_period=0, force=True))

        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(len(self.service_requests), 1)

    def test_dry_run(self):
        """Test dry run."""
        # Make some old user records.
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.update_user(user, client_state="bb")
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)

        self.database.update_node(self.service_node, downed=1)

        # Don't actually perform anything destructive.
        self.assertTrue(purge_old_records(node_secret, grace_period=0, dryrun=True))

        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 2)
        self.assertEqual(len(self.service_requests), 0)


@pytest.mark.migration_records
class TestMigrationRecords(PurgeOldRecordsTestCase):
    """Test user records migrated from old MySQL syncstorage nodes to Spanner."""

    @classmethod
    def setUpClass(cls):
        """Set up class-level test fixtures."""
        super().setUpClass()
        cls.spanner_service = make_server("localhost", 0, cls._service_app)
        host, port = cls.spanner_service.server_address
        cls.spanner_node = f"http://{host}:{port}"
        cls.spanner_thread = threading.Thread(target=cls.spanner_service.serve_forever)
        cls.spanner_thread.start()
        cls.downed_node = f"http://{host}:9999"

    @classmethod
    def tearDownClass(cls):
        """Tear down class-level test fixtures."""
        super().tearDownClass()
        cls.spanner_service.shutdown()
        cls.spanner_thread.join()

    def setUp(self):
        """Set up test fixtures."""
        super().setUp()
        self.database.add_node(self.downed_node, 100, downed=True)
        self.database.add_node(self.spanner_node, 100)

    def test_purging_replaced_at(self):
        """Test purging replaced at."""
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.replace_user_record(user["uid"])

        self.assertTrue(purge_old_records(node_secret, grace_period=0))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 0)
        self.assertEqual(len(self.service_requests), 1)

    def test_purging_no_override(self):
        """Test purging no override."""
        node_secret = "SECRET"
        email = "test@mozilla.com"
        user = self.database.allocate_user(email, client_state="aa")
        self.database.replace_user_record(user["uid"])
        user = self.database.allocate_user(
            email, node=self.spanner_node, client_state="aa"
        )

        self.assertTrue(purge_old_records(node_secret, grace_period=0))
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(len(self.service_requests), 1)

    def test_purging_override_with_migrated(self):
        """Test purging override with migrated."""
        node_secret = "SECRET"
        email = "test@mozilla.com"

        # User previously on a node now downed
        user = self.database.allocate_user(
            email, node=self.downed_node, client_state="aa"
        )
        # Simulate the Spanner migration process (mark their original record as
        # replaced_at):
        # https://github.com/mozilla-services/cloudops-docs/blob/389e61f/Services/Durable%20Sync/SYNC-PY-MIGRATION.md#migration-steps

        # The process then copied their data to spanner_node with no change to
        # their generation/client_state
        self.database.replace_user_record(user["uid"])
        # Migration finished: the user's active record now points to Spanner
        user = self.database.allocate_user(
            email, node=self.spanner_node, client_state="aa"
        )

        self.assertTrue(
            purge_old_records(
                node_secret, grace_period=0, force=True, override_node=self.spanner_node
            )
        )
        user_records = list(self.database.get_user_records(email))
        # The user's old downed node record was purged
        self.assertEqual(len(user_records), 1)
        self.assertEqual(user_records[0].node, self.spanner_node)
        # But that old downed node record had an identical
        # generation/client_state to the active spanner_node's record: so a
        # simple forcing of a delete to the spanner node would delete their
        # current data. Ensure force/override_node includes logic to detect
        # this case and not issue such a delete
        self.assertEqual(len(self.service_requests), 0)

    def test_purging_override_with_migrated_password_change(self):
        """Test purging override with migrated password change."""
        node_secret = "SECRET"
        email = "test@mozilla.com"

        # A user migrated to spanner (like test_purging_override_with_migrated)
        user = self.database.allocate_user(
            email, node=self.downed_node, client_state="aa"
        )
        self.database.replace_user_record(user["uid"])
        user = self.database.allocate_user(
            email, node=self.spanner_node, client_state="aa"
        )
        # User changes their password
        self.database.update_user(user, client_state="ab")

        self.assertTrue(
            purge_old_records(
                node_secret, grace_period=0, force=True, override_node=self.spanner_node
            )
        )
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        # Both replaced_at records issued deletes as normal as neither point to
        # their active record
        self.assertEqual(len(self.service_requests), 2)

    def test_purging_override_null_keys_changed_at(self):
        """Test purging override null keys changed at."""
        # Same as test_purging_override_with_migrated but with a null
        # keys_changed_at
        node_secret = "SECRET"
        email = "test@mozilla.com"

        user = self.database.allocate_user(
            email,
            node=self.downed_node,
            client_state="aa",
            keys_changed_at=None,
        )
        self.database.replace_user_record(user["uid"])
        user = self.database.allocate_user(
            email,
            node=self.spanner_node,
            client_state="aa",
            keys_changed_at=None,
        )

        self.assertTrue(
            purge_old_records(
                node_secret, grace_period=0, force=True, override_node=self.spanner_node
            )
        )
        user_records = list(self.database.get_user_records(email))
        self.assertEqual(len(user_records), 1)
        self.assertEqual(user_records[0].node, self.spanner_node)
        self.assertEqual(len(self.service_requests), 0)


# ===========================================================================
# pytest-style equivalents — added alongside the classes above.
# Once confirmed passing in CI the unittest classes will be removed.
# Module-scoped WSGI server fixtures are defined here (file-local) because
# they are only needed by this module.
# ===========================================================================


def _make_service_app(service_requests):
    """Return a WSGI app that records each request into the given list."""

    def _service_app(environ, start_response):
        service_requests.append(environ)
        start_response("200 OK", [])
        return ""

    return _service_app


@pytest.fixture(scope="module")
def mock_service_server():
    """Module-scoped mock WSGI service server.

    Module scope is justified: the server is expensive to start (OS port
    allocation + thread) and is stateless between tests — the requests list
    is cleared in each per-test fixture's teardown.
    """
    service_requests = []
    server = make_server("localhost", 0, _make_service_app(service_requests))
    server.RequestHandlerClass.log_request = lambda *a: None
    host, port = server.server_address
    service_node = f"http://{host}:{port}"
    thread = threading.Thread(target=server.serve_forever)
    thread.daemon = True
    thread.start()
    yield {"node": service_node, "requests": service_requests}
    server.shutdown()
    thread.join()


@pytest.fixture(scope="module")
def mock_spanner_server(mock_service_server):
    """Module-scoped spanner WSGI server for migration tests.

    Module scope justified: same reason as mock_service_server. Shares the
    service_requests list with mock_service_server to replicate the original
    test class behaviour where both servers appended to the same list.
    """
    service_requests = mock_service_server["requests"]
    server = make_server("localhost", 0, _make_service_app(service_requests))
    server.RequestHandlerClass.log_request = lambda *a: None
    host, port = server.server_address
    spanner_node = f"http://{host}:{port}"
    downed_node = f"http://{host}:9999"
    thread = threading.Thread(target=server.serve_forever)
    thread.daemon = True
    thread.start()
    yield {"node": spanner_node, "downed_node": downed_node}
    server.shutdown()
    thread.join()


@pytest.fixture(scope="function")
def purge_db(mock_service_server):
    """Per-test Database seeded with the mock service node."""
    database = Database()
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    database.add_service("sync-1.5", r"{node}/1.5/{uid}")
    database.add_node(mock_service_server["node"], 100)
    yield database
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    del mock_service_server["requests"][:]
    database.close()


@pytest.fixture(scope="function")
def migration_db(mock_service_server, mock_spanner_server):
    """Per-test Database seeded with service, spanner, and downed nodes."""
    database = Database()
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    database.add_service("sync-1.5", r"{node}/1.5/{uid}")
    database.add_node(mock_service_server["node"], 100)
    database.add_node(mock_spanner_server["downed_node"], 100, downed=True)
    database.add_node(mock_spanner_server["node"], 100)
    yield database
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    del mock_service_server["requests"][:]
    database.close()


def test_purging_of_old_user_records(purge_db, mock_service_server):
    """Test purging of old user records."""
    database = purge_db
    service_requests = mock_service_server["requests"]

    # Make some old user records.
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa", generation=123)
    database.update_user(user, client_state="bb", generation=456, keys_changed_at=450)
    database.update_user(user, client_state="cc", generation=789)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 3
    user = database.get_user(email)
    assert user["client_state"] == "cc"
    assert len(user["old_client_states"]) == 2

    # The default grace-period should prevent any cleanup.
    node_secret = "SECRET"
    assert purge_old_records(node_secret)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 3
    assert len(service_requests) == 0

    # With no grace period, we should cleanup two old records.
    assert purge_old_records(node_secret, grace_period=0)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    assert len(service_requests) == 2

    # Check that the proper delete requests were made to the service.
    expected_kids = ["0000000000450-uw", "0000000000123-qg"]
    for i, environ in enumerate(service_requests):
        # They must be to the correct path.
        assert environ["REQUEST_METHOD"] == "DELETE"
        assert re.match("/1.5/[0-9]+", environ["PATH_INFO"])
        # They must have a correct request signature.
        token = hawkauthlib.get_id(environ)
        secret = tokenlib.get_derived_secret(token, secret=node_secret)
        assert hawkauthlib.check_signature(environ, secret)
        userdata = tokenlib.parse_token(token, secret=node_secret)
        assert "uid" in userdata
        assert "node" in userdata
        assert userdata["fxa_uid"] == "test"
        assert userdata["fxa_kid"] == expected_kids[i]

    # Check that the user's current state is unaffected
    user = database.get_user(email)
    assert user["client_state"] == "cc"
    assert len(user["old_client_states"]) == 0


def test_purging_is_not_done_on_downed_nodes(purge_db, mock_service_server):
    """Test purging is not done on downed nodes."""
    database = purge_db
    service_node = mock_service_server["node"]
    service_requests = mock_service_server["requests"]
    node_secret = "SECRET"
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa")
    database.update_user(user, client_state="bb")
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 2

    # With the node down, we should not purge any records.
    database.update_node(service_node, downed=1)
    assert purge_old_records(node_secret, grace_period=0)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 2
    assert len(service_requests) == 0

    # With the node back up, we should purge correctly.
    database.update_node(service_node, downed=0)
    assert purge_old_records(node_secret, grace_period=0)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    assert len(service_requests) == 1


def test_force(purge_db, mock_service_server):
    """Test force."""
    database = purge_db
    service_node = mock_service_server["node"]
    service_requests = mock_service_server["requests"]
    node_secret = "SECRET"
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa")
    database.update_user(user, client_state="bb")
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 2

    # With the node down, we should be able to purge any records.
    database.update_node(service_node, downed=1)
    assert purge_old_records(node_secret, grace_period=0, force=True)

    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    assert len(service_requests) == 1


def test_dry_run(purge_db, mock_service_server):
    """Test dry run."""
    database = purge_db
    service_node = mock_service_server["node"]
    service_requests = mock_service_server["requests"]
    node_secret = "SECRET"
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa")
    database.update_user(user, client_state="bb")
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 2

    database.update_node(service_node, downed=1)

    # Don't actually perform anything destructive.
    assert purge_old_records(node_secret, grace_period=0, dryrun=True)

    user_records = list(database.get_user_records(email))
    assert len(user_records) == 2
    assert len(service_requests) == 0


@pytest.mark.migration_records
def test_purging_replaced_at(migration_db, mock_service_server, mock_spanner_server):
    """Test purging replaced at."""
    database = migration_db
    service_requests = mock_service_server["requests"]
    node_secret = "SECRET"
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa")
    database.replace_user_record(user["uid"])

    assert purge_old_records(node_secret, grace_period=0)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 0
    assert len(service_requests) == 1


@pytest.mark.migration_records
def test_purging_no_override(migration_db, mock_service_server, mock_spanner_server):
    """Test purging no override."""
    database = migration_db
    service_requests = mock_service_server["requests"]
    spanner_node = mock_spanner_server["node"]
    node_secret = "SECRET"
    email = "test@mozilla.com"
    user = database.allocate_user(email, client_state="aa")
    database.replace_user_record(user["uid"])
    user = database.allocate_user(email, node=spanner_node, client_state="aa")

    assert purge_old_records(node_secret, grace_period=0)
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    assert len(service_requests) == 1


@pytest.mark.migration_records
def test_purging_override_with_migrated(
    migration_db, mock_service_server, mock_spanner_server
):
    """Test purging override with migrated."""
    database = migration_db
    service_requests = mock_service_server["requests"]
    spanner_node = mock_spanner_server["node"]
    downed_node = mock_spanner_server["downed_node"]
    node_secret = "SECRET"
    email = "test@mozilla.com"

    # User previously on a node now downed
    user = database.allocate_user(email, node=downed_node, client_state="aa")
    # Simulate the Spanner migration process (mark their original record as
    # replaced_at):
    # https://github.com/mozilla-services/cloudops-docs/blob/389e61f/Services/Durable%20Sync/SYNC-PY-MIGRATION.md#migration-steps

    # The process then copied their data to spanner_node with no change to
    # their generation/client_state
    database.replace_user_record(user["uid"])
    # Migration finished: the user's active record now points to Spanner
    user = database.allocate_user(email, node=spanner_node, client_state="aa")

    assert purge_old_records(
        node_secret, grace_period=0, force=True, override_node=spanner_node
    )
    user_records = list(database.get_user_records(email))
    # The user's old downed node record was purged
    assert len(user_records) == 1
    assert user_records[0].node == spanner_node
    # But that old downed node record had an identical
    # generation/client_state to the active spanner_node's record: so a
    # simple forcing of a delete to the spanner node would delete their
    # current data. Ensure force/override_node includes logic to detect
    # this case and not issue such a delete
    assert len(service_requests) == 0


@pytest.mark.migration_records
def test_purging_override_with_migrated_password_change(
    migration_db, mock_service_server, mock_spanner_server
):
    """Test purging override with migrated password change."""
    database = migration_db
    service_requests = mock_service_server["requests"]
    spanner_node = mock_spanner_server["node"]
    downed_node = mock_spanner_server["downed_node"]
    node_secret = "SECRET"
    email = "test@mozilla.com"

    # A user migrated to spanner (like test_purging_override_with_migrated)
    user = database.allocate_user(email, node=downed_node, client_state="aa")
    database.replace_user_record(user["uid"])
    user = database.allocate_user(email, node=spanner_node, client_state="aa")
    # User changes their password
    database.update_user(user, client_state="ab")

    assert purge_old_records(
        node_secret, grace_period=0, force=True, override_node=spanner_node
    )
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    # Both replaced_at records issued deletes as normal as neither point to
    # their active record
    assert len(service_requests) == 2


@pytest.mark.migration_records
def test_purging_override_null_keys_changed_at(
    migration_db, mock_service_server, mock_spanner_server
):
    """Test purging override null keys changed at."""
    # Same as test_purging_override_with_migrated but with a null
    # keys_changed_at
    database = migration_db
    service_requests = mock_service_server["requests"]
    spanner_node = mock_spanner_server["node"]
    downed_node = mock_spanner_server["downed_node"]
    node_secret = "SECRET"
    email = "test@mozilla.com"

    user = database.allocate_user(
        email,
        node=downed_node,
        client_state="aa",
        keys_changed_at=None,
    )
    database.replace_user_record(user["uid"])
    user = database.allocate_user(
        email,
        node=spanner_node,
        client_state="aa",
        keys_changed_at=None,
    )

    assert purge_old_records(
        node_secret, grace_period=0, force=True, override_node=spanner_node
    )
    user_records = list(database.get_user_records(email))
    assert len(user_records) == 1
    assert user_records[0].node == spanner_node
    assert len(service_requests) == 0
