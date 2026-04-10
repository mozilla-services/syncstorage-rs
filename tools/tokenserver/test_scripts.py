# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Tests for the tokenserver CLI management scripts."""

import json
import os
import tempfile
import pytest

from add_node import main as add_node_script
from allocate_user import main as allocate_user_script
from count_users import main as count_users_script
from database import Database
from remove_node import main as remove_node_script
from unassign_node import main as unassign_node_script
from update_node import main as update_node_script
from util import get_timestamp

NODE_ID = 800
NODE_URL = "https://node1"


@pytest.fixture(scope="function")
def db():
    """Per-test Database with service and base node, cleaned up after each test.

    Creates a fresh connection, seeds the service and base node, yields the
    Database instance for assertions, then tears everything down.
    """
    database = Database()
    database.add_service("sync-1.5", r"{node}/1.5/{uid}")
    # Ensure we have a node with enough capacity to run the tests.
    database.add_node(NODE_URL, 100, id=NODE_ID)
    yield database
    # Clean up at the end, for good measure.
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    database.close()


def test_add_node(db):
    """Test add node."""
    add_node_script(args=["--current-load", "9", "test_node", "100"])
    res = db.get_node("test_node")
    # The node should have the expected attributes
    assert res.capacity == 100
    assert res.available == 10
    assert res.current_load == 9
    assert res.downed == 0
    assert res.backoff == 0
    assert res.service == db.service_id


def test_add_node_with_explicit_available(db):
    """Test add node with explicit available."""
    add_node_script(
        args=["--current-load", "9", "--available", "5", "test_node", "100"]
    )
    res = db.get_node("test_node")
    # The node should have the expected attributes
    assert res.capacity == 100
    assert res.available == 5
    assert res.current_load == 9
    assert res.downed == 0
    assert res.backoff == 0
    assert res.service == db.service_id


def test_add_downed_node(db):
    """Test add downed node."""
    add_node_script(args=["--downed", "test_node", "100"])
    res = db.get_node("test_node")
    # The node should have the expected attributes
    assert res.capacity == 100
    assert res.available == 10
    assert res.current_load == 0
    assert res.downed == 1
    assert res.backoff == 0
    assert res.service == db.service_id


def test_add_backoff_node(db):
    """Test add backoff node."""
    add_node_script(args=["--backoff", "test_node", "100"])
    res = db.get_node("test_node")
    # The node should have the expected attributes
    assert res.capacity == 100
    assert res.available == 10
    assert res.current_load == 0
    assert res.downed == 0
    assert res.backoff == 1
    assert res.service == db.service_id


def test_allocate_user_user_already_exists(db):
    """Test allocate user when user already exists reassigns to given node."""
    email = "test@test.com"
    db.allocate_user(email)
    node = "https://node2"
    db.add_node(node, 100)
    allocate_user_script(args=[email, node])
    user = db.get_user(email)
    # The user should be assigned to the given node
    assert user["node"] == node
    # Another user should not have been created
    count = db.count_users()
    assert count == 1


def test_allocate_user_given_node(db):
    """Test allocate user assigns to the specified node."""
    email = "test@test.com"
    node = "https://node2"
    db.add_node(node, 100)
    allocate_user_script(args=[email, node])
    user = db.get_user(email)
    # A new user should be created and assigned to the given node
    assert user["node"] == node


def test_allocate_user_not_given_node(db):
    """Test allocate user without node picks the least-loaded node."""
    email = "test@test.com"
    db.add_node("https://node2", 100, current_load=10)
    db.add_node("https://node3", 100, current_load=20)
    db.add_node("https://node4", 100, current_load=30)
    allocate_user_script(args=[email])
    user = db.get_user(email)
    # The user should be assigned to the least-loaded node
    assert user["node"] == NODE_URL


def test_count_users(db):
    """Test count users reports correct totals at given timestamps."""
    db.allocate_user("test1@test.com")
    db.allocate_user("test2@test.com")
    db.allocate_user("test3@test.com")

    timestamp = get_timestamp()

    fd, filename = tempfile.mkstemp()
    os.close(fd)
    try:
        count_users_script(args=["--output", filename, "--timestamp", str(timestamp)])
        with open(filename) as f:
            info = json.loads(f.readline())
        assert info["total_users"] == 3
        assert info["op"] == "sync_count_users"
    finally:
        os.remove(filename)

    fd, filename = tempfile.mkstemp()
    os.close(fd)
    try:
        args = ["--output", filename, "--timestamp", str(timestamp - 10000)]
        count_users_script(args=args)
        with open(filename) as f:
            info = json.loads(f.readline())
        assert info["total_users"] == 0
        assert info["op"] == "sync_count_users"
    finally:
        os.remove(filename)


def test_remove_node(db):
    """Test remove node reassigns affected users to a remaining node."""
    db.add_node("https://node2", 100)
    db.allocate_user("test1@test.com", node="https://node2")
    db.allocate_user("test2@test.com", node=NODE_URL)
    db.allocate_user("test3@test.com", node=NODE_URL)

    remove_node_script(args=["https://node2"])

    # The node should have been removed from the database
    with pytest.raises(ValueError):
        db.get_node_id("https://node2")
    # The first user should have been assigned to a new node
    user = db.get_user("test1@test.com")
    assert user["node"] == NODE_URL
    # The second and third users should still be on the first node
    user = db.get_user("test2@test.com")
    assert user["node"] == NODE_URL
    user = db.get_user("test3@test.com")
    assert user["node"] == NODE_URL


def test_unassign_node(db):
    """Test unassign node moves all users off the given node."""
    db.add_node("https://node2", 100)
    db.allocate_user("test1@test.com", node="https://node2")
    db.allocate_user("test2@test.com", node="https://node2")
    db.allocate_user("test3@test.com", node=NODE_URL)

    unassign_node_script(args=["https://node2"])
    db.remove_node("https://node2")
    # All of the users should now be assigned to the first node
    user = db.get_user("test1@test.com")
    assert user["node"] == NODE_URL
    user = db.get_user("test2@test.com")
    assert user["node"] == NODE_URL
    user = db.get_user("test3@test.com")
    assert user["node"] == NODE_URL


def test_update_node(db):
    """Test update node modifies node attributes correctly."""
    db.add_node("https://node2", 100)
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
    node = db.get_node("https://node2")
    # Ensure the node has the expected attributes
    assert node["capacity"] == 150
    assert node["available"] == 125
    assert node["current_load"] == 25
    assert node["downed"] == 1
    assert node["backoff"] == 1
