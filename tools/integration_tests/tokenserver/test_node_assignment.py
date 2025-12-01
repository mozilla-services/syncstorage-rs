# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import pytest
import unittest

from integration_tests.tokenserver.test_support import TestCase
from sqlalchemy.sql import text as sqltext


@pytest.mark.usefixtures("setup_server_local_testing_with_oauth")
class TestNodeAssignment(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestNodeAssignment, self).setUp()

    def tearDown(self):
        super(TestNodeAssignment, self).tearDown()

    def test_user_creation(self):
        # Add a few more nodes
        self._add_node(available=0, node="https://node1")
        self._add_node(available=1, node="https://node2")
        self._add_node(available=5, node="https://node3")
        # Send a request from an unseen user
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # Ensure a single user was created
        self.assertEqual(self._count_users(), 1)
        # Ensure the user has the correct attributes
        user1 = self._get_user(res.json["uid"])
        self.assertEqual(user1["generation"], 1234)
        self.assertEqual(user1["keys_changed_at"], 1234)
        self.assertEqual(user1["client_state"], "aaaa")
        self.assertEqual(user1["nodeid"], self.NODE_ID)
        self.assertEqual(user1["service"], self.service_id)
        # Ensure the 'available' and 'current_load' counts on the node
        # assigned to the user have been decremented appropriately
        node = self._get_node(self.NODE_ID)
        self.assertEqual(node["available"], 99)
        self.assertEqual(node["current_load"], 1)
        # Send a request from the same user
        self.app.get("/1.0/sync/1.5", headers=headers)
        # Ensure another user record was not created
        self.assertEqual(self._count_users(), 1)

    def test_new_user_allocation(self):
        # Start with a clean database
        cursor = self._execute_sql(sqltext("DELETE FROM nodes"), {})
        cursor.close()

        self._add_node(
            available=100, current_load=0, capacity=100, backoff=1, node="https://node1"
        )
        self._add_node(
            available=100, current_load=0, capacity=100, downed=1, node="https://node2"
        )
        node_id = self._add_node(
            available=99, current_load=1, capacity=100, node="https://node3"
        )
        self._add_node(available=98, current_load=2, capacity=100, node="https://node4")
        self._add_node(available=97, current_load=3, capacity=100, node="https://node5")
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # The user should have been allocated to the least-loaded node
        # (computed as current_load / capacity) that has backoff and downed
        # set to 0
        user = self._get_user(res.json["uid"])
        self.assertEqual(user["nodeid"], node_id)
        # The selected node should have current_load incremented and available
        # decremented
        node = self._get_node(node_id)
        self.assertEqual(node["current_load"], 2)
        self.assertEqual(node["available"], 98)

    def test_successfully_releasing_node_capacity(self):
        # Start with a clean database
        cursor = self._execute_sql(sqltext("DELETE FROM nodes"), {})
        cursor.close()

        node_id1 = self._add_node(
            available=0, current_load=99, capacity=100, node="https://node1"
        )
        node_id2 = self._add_node(
            available=0, current_load=90, capacity=100, node="https://node2"
        )
        node_id3 = self._add_node(
            available=0, current_load=80, capacity=81, node="https://node3"
        )
        node_id4 = self._add_node(
            available=0, current_load=70, capacity=71, node="https://node4", backoff=1
        )
        node_id5 = self._add_node(
            available=0, current_load=60, capacity=61, node="https://node5", downed=1
        )
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        # Since every node has no available spots, capacity is added to each
        # node according to the equation
        # min(capacity*capacity_release_rate, capacity - current_load). Since
        # capacity - current_load is 0 for every node, the node with the
        # greatest capacity is chosen
        user = self._get_user(res.json["uid"])
        self.assertEqual(user["nodeid"], node_id2)
        # min(100 * 0.1, 100 - 99) = 1
        node1 = self._get_node(node_id1)
        self.assertEqual(node1["available"], 1)
        # min(100 * 0.1, 100 - 90) = 10, and this is the node to which the
        # user was assigned, so the final available count is 9
        node2 = self._get_node(node_id2)
        self.assertEqual(node2["available"], 9)
        # min(81 * 0.1, 81 - 80) = 1
        node3 = self._get_node(node_id3)
        self.assertEqual(node3["available"], 1)
        # min(100 * 0.1, 71 - 70) = 1
        node4 = self._get_node(node_id4)
        self.assertEqual(node4["available"], 1)
        # Nodes with downed set to 1 do not have their availability updated
        node5 = self._get_node(node_id5)
        self.assertEqual(node5["available"], 0)

    def test_unsuccessfully_releasing_node_capacity(self):
        # Start with a clean database
        cursor = self._execute_sql(sqltext("DELETE FROM nodes"), {})
        cursor.close()

        self._add_node(
            available=0, current_load=100, capacity=100, node="https://node1"
        )
        self._add_node(available=0, current_load=90, capacity=90, node="https://node2")
        self._add_node(available=0, current_load=80, capacity=80, node="https://node3")
        headers = self._build_auth_headers(
            generation=1234, keys_changed_at=1234, client_state="aaaa"
        )
        # All of these nodes are completely full, and no capacity can be
        # released
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=503)
        # The response has the expected body
        expected_error_response = {
            "errors": [
                {
                    "description": "Unexpected error: unable to get a node",
                    "location": "internal",
                    "name": "",
                }
            ],
            "status": "internal-error",
        }
        self.assertEqual(res.json, expected_error_response)
