# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

import json
import os
import unittest
import uuid

from add_node import main as add_node_script
from allocate_user import main as allocate_user_script
from count_users import main as count_users_script
from database import Database
from remove_node import main as remove_node_script
from unassign_node import main as unassign_node_script
from update_node import main as update_node_script
from util import get_timestamp


class TestScripts(unittest.TestCase):
    NODE_ID = 800
    NODE_URL = "https://node1"

    def setUp(self):
        self.database = Database()

        # Start each test with a blank slate.
        cursor = self.database._execute_sql("DELETE FROM users")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM nodes")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM services")
        cursor.close()

        # Add a service
        self.database.add_service("sync-1.5", r"{node}/1.5/{uid}")

        # Ensure we have a node with enough capacity to run the tests.
        self.database.add_node(self.NODE_URL, 100, id=self.NODE_ID)

    def tearDown(self):
        # And clean up at the end, for good measure.
        cursor = self.database._execute_sql("DELETE FROM users")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM nodes")
        cursor.close()

        cursor = self.database._execute_sql("DELETE FROM services")
        cursor.close()

        self.database.close()

    def test_add_node(self):
        add_node_script(args=["--current-load", "9", "test_node", "100"])
        res = self.database.get_node("test_node")
        # The node should have the expected attributes
        self.assertEqual(res.capacity, 100)
        self.assertEqual(res.available, 10)
        self.assertEqual(res.current_load, 9)
        self.assertEqual(res.downed, 0)
        self.assertEqual(res.backoff, 0)
        self.assertEqual(res.service, self.database.service_id)

    def test_add_node_with_explicit_available(self):
        args = ["--current-load", "9", "--available", "5", "test_node", "100"]
        add_node_script(args=args)
        res = self.database.get_node("test_node")
        # The node should have the expected attributes
        self.assertEqual(res.capacity, 100)
        self.assertEqual(res.available, 5)
        self.assertEqual(res.current_load, 9)
        self.assertEqual(res.downed, 0)
        self.assertEqual(res.backoff, 0)
        self.assertEqual(res.service, self.database.service_id)

    def test_add_downed_node(self):
        add_node_script(args=["--downed", "test_node", "100"])
        res = self.database.get_node("test_node")
        # The node should have the expected attributes
        self.assertEqual(res.capacity, 100)
        self.assertEqual(res.available, 10)
        self.assertEqual(res.current_load, 0)
        self.assertEqual(res.downed, 1)
        self.assertEqual(res.backoff, 0)
        self.assertEqual(res.service, self.database.service_id)

    def test_add_backoff_node(self):
        add_node_script(args=["--backoff", "test_node", "100"])
        res = self.database.get_node("test_node")
        # The node should have the expected attributes
        self.assertEqual(res.capacity, 100)
        self.assertEqual(res.available, 10)
        self.assertEqual(res.current_load, 0)
        self.assertEqual(res.downed, 0)
        self.assertEqual(res.backoff, 1)
        self.assertEqual(res.service, self.database.service_id)

    def test_allocate_user_user_already_exists(self):
        email = "test@test.com"
        self.database.allocate_user(email)
        node = "https://node2"
        self.database.add_node(node, 100)
        allocate_user_script(args=[email, node])
        user = self.database.get_user(email)
        # The user should be assigned to the given node
        self.assertEqual(user["node"], node)
        # Another user should not have been created
        count = self.database.count_users()
        self.assertEqual(count, 1)

    def test_allocate_user_given_node(self):
        email = "test@test.com"
        node = "https://node2"
        self.database.add_node(node, 100)
        allocate_user_script(args=[email, node])
        user = self.database.get_user(email)
        # A new user should be created and assigned to the given node
        self.assertEqual(user["node"], node)

    def test_allocate_user_not_given_node(self):
        email = "test@test.com"
        self.database.add_node("https://node2", 100, current_load=10)
        self.database.add_node("https://node3", 100, current_load=20)
        self.database.add_node("https://node4", 100, current_load=30)
        allocate_user_script(args=[email])
        user = self.database.get_user(email)
        # The user should be assigned to the least-loaded node
        self.assertEqual(user["node"], "https://node1")

    def test_count_users(self):
        self.database.allocate_user("test1@test.com")
        self.database.allocate_user("test2@test.com")
        self.database.allocate_user("test3@test.com")

        timestamp = get_timestamp()
        filename = "/tmp/" + str(uuid.uuid4())
        try:
            count_users_script(
                args=["--output", filename, "--timestamp", str(timestamp)]
            )

            with open(filename) as f:
                info = json.loads(f.readline())
                self.assertEqual(info["total_users"], 3)
                self.assertEqual(info["op"], "sync_count_users")
        finally:
            os.remove(filename)

        filename = "/tmp/" + str(uuid.uuid4())
        try:
            args = ["--output", filename, "--timestamp", str(timestamp - 10000)]
            count_users_script(args=args)

            with open(filename) as f:
                info = json.loads(f.readline())
                self.assertEqual(info["total_users"], 0)
                self.assertEqual(info["op"], "sync_count_users")
        finally:
            os.remove(filename)

    def test_remove_node(self):
        self.database.add_node("https://node2", 100)
        self.database.allocate_user("test1@test.com", node="https://node2")
        self.database.allocate_user("test2@test.com", node=self.NODE_URL)
        self.database.allocate_user("test3@test.com", node=self.NODE_URL)

        remove_node_script(args=["https://node2"])

        # The node should have been removed from the database
        args = ["https://node2"]
        self.assertRaises(ValueError, self.database.get_node_id, *args)
        # The first user should have been assigned to a new node
        user = self.database.get_user("test1@test.com")
        self.assertEqual(user["node"], self.NODE_URL)
        # The second and third users should still be on the first node
        user = self.database.get_user("test2@test.com")
        self.assertEqual(user["node"], self.NODE_URL)
        user = self.database.get_user("test3@test.com")
        self.assertEqual(user["node"], self.NODE_URL)

    def test_unassign_node(self):
        self.database.add_node("https://node2", 100)
        self.database.allocate_user("test1@test.com", node="https://node2")
        self.database.allocate_user("test2@test.com", node="https://node2")
        self.database.allocate_user("test3@test.com", node=self.NODE_URL)

        unassign_node_script(args=["https://node2"])
        self.database.remove_node("https://node2")
        # All of the users should now be assigned to the first node
        user = self.database.get_user("test1@test.com")
        self.assertEqual(user["node"], self.NODE_URL)
        user = self.database.get_user("test2@test.com")
        self.assertEqual(user["node"], self.NODE_URL)
        user = self.database.get_user("test3@test.com")
        self.assertEqual(user["node"], self.NODE_URL)

    def test_update_node(self):
        self.database.add_node("https://node2", 100)
        update_node_script(
            args=[
                "--capacity",
                "150",
                "--available",
                "125",
                "--current-load",
                "25",
                "--downed",
                "--backoff",
                "https://node2",
            ]
        )
        node = self.database.get_node("https://node2")
        # Ensure the node has the expected attributes
        self.assertEqual(node["capacity"], 150)
        self.assertEqual(node["available"], 125)
        self.assertEqual(node["current_load"], 25)
        self.assertEqual(node["downed"], 1)
        self.assertEqual(node["backoff"], 1)
