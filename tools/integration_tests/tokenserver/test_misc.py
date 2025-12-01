# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import pytest
import unittest

from integration_tests.tokenserver.test_support import TestCase

MAX_GENERATION = 9223372036854775807


@pytest.mark.usefixtures("setup_server_local_testing_with_oauth")
class TestMisc(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestMisc, self).setUp()

    def tearDown(self):
        super(TestMisc, self).tearDown()

    def test_unknown_app(self):
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/xXx/token", headers=headers, status=404)
        expected_error_response = {
            "errors": [
                {
                    "description": "Unsupported application",
                    "location": "url",
                    "name": "application",
                }
            ],
            "status": "error",
        }
        self.assertEqual(res.json, expected_error_response)

    def test_unknown_version(self):
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.2", headers=headers, status=404)
        expected_error_response = {
            "errors": [
                {
                    "description": "Unsupported application version",
                    "location": "url",
                    "name": "1.2",
                }
            ],
            "status": "error",
        }
        self.assertEqual(res.json, expected_error_response)

    def test_valid_app(self):
        self._add_user()
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertIn("https://example.com/1.5", res.json["api_endpoint"])
        self.assertIn("duration", res.json)
        self.assertEqual(res.json["duration"], 3600)

    def test_current_user_is_the_most_up_to_date(self):
        # Add some users
        self._add_user(generation=1234, created_at=1234)
        self._add_user(generation=1235, created_at=1234)
        self._add_user(generation=1234, created_at=1235)
        uid = self._add_user(generation=1236, created_at=1233)
        # Users are sorted by (generation, created_at), so the fourth user
        # record is considered to be the current user
        headers = self._build_auth_headers(
            generation=1236, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res.json["uid"], uid)

    def test_user_creation_when_most_current_user_is_replaced(self):
        # Add some users
        uid1 = self._add_user(generation=1234, created_at=1234)
        uid2 = self._add_user(generation=1235, created_at=1235)
        uid3 = self._add_user(generation=1236, created_at=1236, replaced_at=1237)
        seen_uids = [uid1, uid2, uid3]
        # Because the current user (the one with uid3) has been replaced, a new
        # user record is created
        headers = self._build_auth_headers(
            generation=1237, keys_changed_at=1237, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertNotIn(res.json["uid"], seen_uids)

    def test_old_users_marked_as_replaced_in_race_recovery(self):
        # Add some users
        uid1 = self._add_user(generation=1234, created_at=1234)
        uid2 = self._add_user(generation=1235, created_at=1235)
        uid3 = self._add_user(generation=1236, created_at=1240)
        # Make a request
        headers = self._build_auth_headers(
            generation=1236, keys_changed_at=1236, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # uid3 is associated with the current user
        self.assertEqual(res.json["uid"], uid3)
        # The users associated with uid1 and uid2 have replaced_at set to be
        # equal to created_at on the current user record
        user1 = self._get_user(uid1)
        user2 = self._get_user(uid2)
        self.assertEqual(user1["replaced_at"], 1240)
        self.assertEqual(user2["replaced_at"], 1240)

    def test_user_updates_with_new_client_state(self):
        # Start with a single user in the database
        uid = self._add_user(generation=1234, keys_changed_at=1234, client_state="aaaa")
        # Send a request, updating the generation, keys_changed_at, and
        # client_state
        headers = self._build_auth_headers(
            generation=1235, keys_changed_at=1235, client_state="bbbb"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # A new user should have been created
        self.assertEqual(self._count_users(), 2)
        self.assertNotEqual(uid, res.json["uid"])
        # The new user record should have the updated generation,
        # keys_changed_at, and client_state
        user = self._get_user(res.json["uid"])
        self.assertEqual(user["generation"], 1235)
        self.assertEqual(user["keys_changed_at"], 1235)
        self.assertEqual(user["client_state"], "bbbb")
        # The old user record should not have the updated values
        user = self._get_user(uid)
        self.assertEqual(user["generation"], 1234)
        self.assertEqual(user["keys_changed_at"], 1234)
        self.assertEqual(user["client_state"], "aaaa")
        # Get all the replaced users
        email = f"test@{self.FXA_EMAIL_DOMAIN}"
        replaced_users = self._get_replaced_users(self.service_id, email)
        # Only one user should be replaced
        self.assertEqual(len(replaced_users), 1)
        # The replaced user record should have the old generation,
        # keys_changed_at, and client_state
        replaced_user = replaced_users[0]
        self.assertEqual(replaced_user["generation"], 1234)
        self.assertEqual(replaced_user["keys_changed_at"], 1234)
        self.assertEqual(replaced_user["client_state"], "aaaa")

    def test_user_updates_with_same_client_state(self):
        # Start with a single user in the database
        uid = self._add_user(generation=1234, keys_changed_at=1234)
        # Send a request, updating the generation and keys_changed_at but not
        # the client state
        headers = self._build_auth_headers(
            generation=1235, keys_changed_at=1235, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # A new user should not have been created
        self.assertEqual(self._count_users(), 1)
        self.assertEqual(uid, res.json["uid"])
        # The user record should have been updated
        user = self._get_user(uid)
        self.assertEqual(user["generation"], 1235)
        self.assertEqual(user["keys_changed_at"], 1235)

    def test_retired_users_can_make_requests(self):
        # Add a retired user to the database
        self._add_user(generation=MAX_GENERATION)
        headers = self._build_auth_headers(
            generation=1235, keys_changed_at=1234, client_state="aaaa"
        )
        # Retired users cannot make requests with a generation smaller than
        # the max generation
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            "status": "invalid-generation",
            "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
        }
        self.assertEqual(res.json, expected_error_response)
        # Retired users can make requests with a generation number equal to
        # the max generation
        headers = self._build_auth_headers(
            generation=MAX_GENERATION, keys_changed_at=1234, client_state="aaaa"
        )
        self.app.get("/1.0/sync/1.5", headers=headers)

    def test_replaced_users_can_make_requests(self):
        # Add a replaced user to the database
        self._add_user(generation=1234, created_at=1234, replaced_at=1234)
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        # Replaced users can make requests
        self.app.get("/1.0/sync/1.5", headers=headers)

    def test_retired_users_with_no_node_cannot_make_requests(self):
        # Add a retired user to the database
        invalid_node_id = self.NODE_ID + 1
        self._add_user(generation=MAX_GENERATION, nodeid=invalid_node_id)
        # Retired users without a node cannot make requests
        headers = self._build_auth_headers(
            generation=MAX_GENERATION, keys_changed_at=1234, client_state="aaaa"
        )
        self.app.get("/1.0/sync/1.5", headers=headers, status=500)

    def test_replaced_users_with_no_node_can_make_requests(self):
        # Add a replaced user to the database
        invalid_node_id = self.NODE_ID + 1
        self._add_user(created_at=1234, replaced_at=1234, nodeid=invalid_node_id)
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        # Replaced users without a node can make requests
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        user = self._get_user(res.json["uid"])
        # The user is assigned to a new node
        self.assertEqual(user["nodeid"], self.NODE_ID)

    def test_x_content_type_options(self):
        self._add_user(generation=1234, keys_changed_at=1234, client_state="aaaa")
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # Tokenserver responses should include the
        # `X-Content-Type-Options: nosniff` header
        self.assertEqual(res.headers["X-Content-Type-Options"], "nosniff")
