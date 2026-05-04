# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Node assignment integration tests for the tokenserver."""

from integration_tests.tokenserver.helpers import (
    NODE_ID,
    add_node,
    build_oauth_headers,
    count_users,
    execute_sql,
    get_node,
    get_user,
)
from sqlalchemy.sql import text as sqltext


def test_user_creation(ts_ctx):
    """Test user creation."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]

    # Add a few more nodes
    add_node(db_conn, service_id, available=0, node="https://node1")
    add_node(db_conn, service_id, available=1, node="https://node2")
    add_node(db_conn, service_id, available=5, node="https://node3")

    # Send a request from an unseen user
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    # Ensure a single user was created
    assert count_users(db_conn) == 1
    # Ensure the user has the correct attributes
    user1 = get_user(db_conn, res.json["uid"])
    assert user1["generation"] == 1234
    assert user1["keys_changed_at"] == 1234
    assert user1["client_state"] == "aaaa"
    assert user1["nodeid"] == NODE_ID
    assert user1["service"] == service_id
    # Ensure the 'available' and 'current_load' counts on the node
    # assigned to the user have been decremented appropriately
    node = get_node(db_conn, NODE_ID)
    assert node["available"] == 99
    assert node["current_load"] == 1
    # Send a request from the same user
    app.get("/1.0/sync/1.5", headers=headers)
    # Ensure another user record was not created
    assert count_users(db_conn) == 1


def test_new_user_allocation(ts_ctx):
    """Test new user allocation."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]

    # Start with a clean database
    execute_sql(db_conn, sqltext("DELETE FROM nodes"), {}).close()

    add_node(
        db_conn,
        service_id,
        available=100,
        current_load=0,
        capacity=100,
        backoff=1,
        node="https://node1",
    )
    add_node(
        db_conn,
        service_id,
        available=100,
        current_load=0,
        capacity=100,
        downed=1,
        node="https://node2",
    )
    node_id = add_node(
        db_conn,
        service_id,
        available=99,
        current_load=1,
        capacity=100,
        node="https://node3",
    )
    add_node(
        db_conn,
        service_id,
        available=98,
        current_load=2,
        capacity=100,
        node="https://node4",
    )
    add_node(
        db_conn,
        service_id,
        available=97,
        current_load=3,
        capacity=100,
        node="https://node5",
    )

    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    # The user should have been allocated to the least-loaded node
    # (computed as current_load / capacity) that has backoff and downed
    # set to 0
    user = get_user(db_conn, res.json["uid"])
    assert user["nodeid"] == node_id
    # The selected node should have current_load incremented and available
    # decremented
    node = get_node(db_conn, node_id)
    assert node["current_load"] == 2
    assert node["available"] == 98


def test_successfully_releasing_node_capacity(ts_ctx):
    """Test successfully releasing node capacity."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]

    # Start with a clean database
    execute_sql(db_conn, sqltext("DELETE FROM nodes"), {}).close()

    node_id1 = add_node(
        db_conn,
        service_id,
        available=0,
        current_load=99,
        capacity=100,
        node="https://node1",
    )
    node_id2 = add_node(
        db_conn,
        service_id,
        available=0,
        current_load=90,
        capacity=100,
        node="https://node2",
    )
    node_id3 = add_node(
        db_conn,
        service_id,
        available=0,
        current_load=80,
        capacity=81,
        node="https://node3",
    )
    node_id4 = add_node(
        db_conn,
        service_id,
        available=0,
        current_load=70,
        capacity=71,
        backoff=1,
        node="https://node4",
    )
    node_id5 = add_node(
        db_conn,
        service_id,
        available=0,
        current_load=60,
        capacity=61,
        downed=1,
        node="https://node5",
    )

    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    # Since every node has no available spots, capacity is added to each
    # node according to the equation
    # min(capacity*capacity_release_rate, capacity - current_load). Since
    # capacity - current_load is 0 for every node, the node with the
    # greatest capacity is chosen
    user = get_user(db_conn, res.json["uid"])
    assert user["nodeid"] == node_id2
    # min(100 * 0.1, 100 - 99) = 1
    node1 = get_node(db_conn, node_id1)
    assert node1["available"] == 1
    # min(100 * 0.1, 100 - 90) = 10, and this is the node to which the
    # user was assigned, so the final available count is 9
    node2 = get_node(db_conn, node_id2)
    assert node2["available"] == 9
    # min(81 * 0.1, 81 - 80) = 1
    node3 = get_node(db_conn, node_id3)
    assert node3["available"] == 1
    # min(100 * 0.1, 71 - 70) = 1
    node4 = get_node(db_conn, node_id4)
    assert node4["available"] == 1
    # Nodes with downed set to 1 do not have their availability updated
    node5 = get_node(db_conn, node_id5)
    assert node5["available"] == 0
    # Suppress unused variable warnings — node IDs retained for readability
    _ = node_id5


def test_unsuccessfully_releasing_node_capacity(ts_ctx):
    """Test unsuccessfully releasing node capacity."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]

    # Start with a clean database
    execute_sql(db_conn, sqltext("DELETE FROM nodes"), {}).close()

    add_node(
        db_conn,
        service_id,
        available=0,
        current_load=100,
        capacity=100,
        node="https://node1",
    )
    add_node(
        db_conn,
        service_id,
        available=0,
        current_load=90,
        capacity=90,
        node="https://node2",
    )
    add_node(
        db_conn,
        service_id,
        available=0,
        current_load=80,
        capacity=80,
        node="https://node3",
    )

    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    # All of these nodes are completely full, and no capacity can be released
    res = app.get("/1.0/sync/1.5", headers=headers, status=503)
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
    assert res.json == expected_error_response
