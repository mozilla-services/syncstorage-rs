# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Tests for the tokenserver database module."""

import time

import pytest

from collections import defaultdict
from database import MAX_GENERATION
from util import get_timestamp


def test_node_allocation(db):
    """Test node allocation."""
    user = db.get_user("test1@example.com")
    assert user is None

    user = db.allocate_user("test1@example.com")
    wanted = "https://phx12"
    assert user["node"] == wanted

    user = db.get_user("test1@example.com")
    assert user["node"] == wanted


def test_allocation_to_least_loaded_node(db):
    """Test allocation to least loaded node."""
    db.add_node("https://phx13", 100)
    user1 = db.allocate_user("test1@mozilla.com")
    user2 = db.allocate_user("test2@mozilla.com")
    assert user1["node"] != user2["node"]


def test_allocation_is_not_allowed_to_downed_nodes(db):
    """Test allocation is not allowed to downed nodes."""
    db.update_node("https://phx12", downed=True)
    with pytest.raises(Exception):
        db.allocate_user("test1@mozilla.com")


def test_allocation_is_not_allowed_to_backoff_nodes(db):
    """Test allocation is not allowed to backoff nodes."""
    db.update_node("https://phx12", backoff=True)
    with pytest.raises(Exception):
        db.allocate_user("test1@mozilla.com")


def test_update_generation_number(db):
    """Test update generation number."""
    user = db.allocate_user("test1@example.com")
    assert user["generation"] == 0
    assert user["client_state"] == ""
    orig_uid = user["uid"]
    orig_node = user["node"]

    # Changing generation should leave other properties unchanged.
    db.update_user(user, generation=42)
    assert user["uid"] == orig_uid
    assert user["node"] == orig_node
    assert user["generation"] == 42
    assert user["client_state"] == ""

    user = db.get_user("test1@example.com")
    assert user["uid"] == orig_uid
    assert user["node"] == orig_node
    assert user["generation"] == 42
    assert user["client_state"] == ""

    # It's not possible to move generation number backwards.
    db.update_user(user, generation=17)
    assert user["uid"] == orig_uid
    assert user["node"] == orig_node
    assert user["generation"] == 42
    assert user["client_state"] == ""

    user = db.get_user("test1@example.com")
    assert user["uid"] == orig_uid
    assert user["node"] == orig_node
    assert user["generation"] == 42
    assert user["client_state"] == ""


def test_update_client_state(db):
    """Test update client state."""
    user = db.allocate_user("test1@example.com")
    assert user["generation"] == 0
    assert user["client_state"] == ""
    assert set(user["old_client_states"]) == set(())
    seen_uids = set((user["uid"],))
    orig_node = user["node"]

    # Changing client-state allocates a new userid.
    db.update_user(user, client_state="aaaa")
    assert user["uid"] not in seen_uids
    assert user["node"] == orig_node
    assert user["generation"] == 0
    assert user["client_state"] == "aaaa"
    assert set(user["old_client_states"]) == set(("",))

    user = db.get_user("test1@example.com")
    assert user["uid"] not in seen_uids
    assert user["node"] == orig_node
    assert user["generation"] == 0
    assert user["client_state"] == "aaaa"
    assert set(user["old_client_states"]) == set(("",))

    seen_uids.add(user["uid"])

    # It's possible to change client-state and generation at once.
    db.update_user(user, client_state="bbbb", generation=12)
    assert user["uid"] not in seen_uids
    assert user["node"] == orig_node
    assert user["generation"] == 12
    assert user["client_state"] == "bbbb"
    assert set(user["old_client_states"]) == set(("", "aaaa"))

    user = db.get_user("test1@example.com")
    assert user["uid"] not in seen_uids
    assert user["node"] == orig_node
    assert user["generation"] == 12
    assert user["client_state"] == "bbbb"
    assert set(user["old_client_states"]) == set(("", "aaaa"))

    # You can't go back to an old client_state.
    orig_uid = user["uid"]
    with pytest.raises(Exception):
        db.update_user(user, client_state="aaaa")

    user = db.get_user("test1@example.com")
    assert user["uid"] == orig_uid
    assert user["node"] == orig_node
    assert user["generation"] == 12
    assert user["client_state"] == "bbbb"
    assert set(user["old_client_states"]) == set(("", "aaaa"))


def test_user_retirement(db):
    """Test user retirement."""
    db.allocate_user("test@mozilla.com")
    user1 = db.get_user("test@mozilla.com")
    db.retire_user("test@mozilla.com")
    user2 = db.get_user("test@mozilla.com")
    assert user2["generation"] > user1["generation"]


def test_cleanup_of_old_records(db):
    """Test cleanup of old records."""
    # Create 6 user records for the first user.
    # Do a sleep halfway through so we can test use of grace period.
    email1 = "test1@mozilla.com"
    user1 = db.allocate_user(email1)
    # We have to sleep between every user create/update operation: if two
    # users are created with the same timestamp, it can lead to a
    # situation where two active user records exist for a single email.
    time.sleep(0.1)
    db.update_user(user1, client_state="aaaa")
    time.sleep(0.1)
    db.update_user(user1, client_state="bbbb")
    time.sleep(0.1)
    db.update_user(user1, client_state="cccc")
    time.sleep(0.1)
    break_time = time.time()
    time.sleep(0.1)
    db.update_user(user1, client_state="dddd")
    time.sleep(0.1)
    db.update_user(user1, client_state="eeee")
    time.sleep(0.1)
    records = list(db.get_user_records(email1))
    assert len(records) == 6
    # Create 3 user records for the second user.
    email2 = "test2@mozilla.com"
    user2 = db.allocate_user(email2)
    time.sleep(0.1)
    db.update_user(user2, client_state="aaaa")
    time.sleep(0.1)
    db.update_user(user2, client_state="bbbb")
    time.sleep(0.1)
    records = list(db.get_user_records(email2))
    assert len(records) == 3
    # That should be a total of 7 old records.
    old_records = list(db.get_old_user_records(0))
    assert len(old_records) == 7
    # And with max_offset of 3, the first record should be id 4.
    old_records = list(db.get_old_user_records(0, 100, 3))
    # The 'limit' parameter should be respected.
    old_records = list(db.get_old_user_records(0, 2))
    assert len(old_records) == 2
    # The default grace period is too big to pick them up.
    old_records = list(db.get_old_user_records())
    assert len(old_records) == 0
    # The grace period can select a subset of the records.
    grace = time.time() - break_time
    old_records = list(db.get_old_user_records(grace))
    assert len(old_records) == 3
    # Old records can be successfully deleted.
    for record in old_records:
        db.delete_user_record(record.uid)
    old_records = list(db.get_old_user_records(0))
    assert len(old_records) == 4


def test_node_reassignment_when_records_are_replaced(db):
    """Test node reassignment when records are replaced."""
    db.allocate_user(
        "test@mozilla.com", generation=42, keys_changed_at=12, client_state="aaaa"
    )
    user1 = db.get_user("test@mozilla.com")
    db.replace_user_records("test@mozilla.com")
    user2 = db.get_user("test@mozilla.com")
    # They should have got a new uid.
    assert user2["uid"] != user1["uid"]
    # But their account metadata should have been preserved.
    assert user2["generation"] == user1["generation"]
    assert user2["keys_changed_at"] == user1["keys_changed_at"]
    assert user2["client_state"] == user1["client_state"]


def test_node_reassignment_not_done_for_retired_users(db):
    """Test node reassignment not done for retired users."""
    db.allocate_user("test@mozilla.com", generation=42, client_state="aaaa")
    user1 = db.get_user("test@mozilla.com")
    db.retire_user("test@mozilla.com")
    user2 = db.get_user("test@mozilla.com")
    assert user2["uid"] == user1["uid"]
    assert user2["generation"] == MAX_GENERATION
    assert user2["client_state"] == user2["client_state"]


def test_recovery_from_racy_record_creation(db):
    """Test recovery from racy record creation."""
    timestamp = get_timestamp()
    # Simulate race for forcing creation of two rows with same timestamp.
    user1 = db.allocate_user("test@mozilla.com", timestamp=timestamp)
    user2 = db.allocate_user("test@mozilla.com", timestamp=timestamp)
    assert user1["uid"] != user2["uid"]
    # Neither is marked replaced initially.
    old_records = list(db.get_old_user_records(0))
    assert len(old_records) == 0
    # Reading current details will detect the problem and fix it.
    db.get_user("test@mozilla.com")
    old_records = list(db.get_old_user_records(0))
    assert len(old_records) == 1


def test_that_race_recovery_respects_generation_number_monotonicity(db):
    """Test that race recovery respects generation number monotonicity."""
    timestamp = get_timestamp()
    # Simulate race between clients with different generation numbers,
    # in which the out-of-date client gets a higher timestamp.
    user1 = db.allocate_user("test@mozilla.com", generation=1, timestamp=timestamp)
    user2 = db.allocate_user("test@mozilla.com", generation=2, timestamp=timestamp - 1)
    assert user1["uid"] != user2["uid"]
    # Reading current details should promote the higher-generation one.
    user = db.get_user("test@mozilla.com")
    assert user["generation"] == 2
    assert user["uid"] == user2["uid"]
    # And the other record should get marked as replaced.
    old_records = list(db.get_old_user_records(0))
    assert len(old_records) == 1


def test_node_reassignment_and_removal(db):
    """Test node reassignment and removal."""
    NODE1 = "https://phx12"
    NODE2 = "https://phx13"
    # note that NODE1 is created by default for all tests.
    db.add_node(NODE2, 100)
    # Assign four users, we should get two on each node.
    user1 = db.allocate_user("test1@mozilla.com")
    user2 = db.allocate_user("test2@mozilla.com")
    user3 = db.allocate_user("test3@mozilla.com")
    user4 = db.allocate_user("test4@mozilla.com")
    node_counts = defaultdict(lambda: 0)
    for user in (user1, user2, user3, user4):
        node_counts[user["node"]] += 1
    assert node_counts[NODE1] == 2
    assert node_counts[NODE2] == 2
    # Clear the assignments for NODE1, and re-assign.
    # The users previously on NODE1 should balance across both nodes,
    # giving 1 on NODE1 and 3 on NODE2.
    db.unassign_node(NODE1)
    node_counts = defaultdict(lambda: 0)
    for user in (user1, user2, user3, user4):
        new_user = db.get_user(user["email"])
        if user["node"] == NODE2:
            assert new_user["node"] == NODE2
        node_counts[new_user["node"]] += 1
    assert node_counts[NODE1] == 1
    assert node_counts[NODE2] == 3
    # Remove NODE2. Everyone should wind up on NODE1.
    db.remove_node(NODE2)
    for user in (user1, user2, user3, user4):
        new_user = db.get_user(user["email"])
        assert new_user["node"] == NODE1
    # The old users records pointing to NODE2 should have a NULL 'node'
    # property since it has been removed from the db.
    null_node_count = 0
    for row in db.get_old_user_records(0):
        if row.node is None:
            null_node_count += 1
        else:
            assert row.node == NODE1
    assert null_node_count == 3


def test_that_race_recovery_respects_generation_after_reassignment(db):
    """Test that race recovery respects generation after reassignment."""
    timestamp = get_timestamp()
    # Simulate race between clients with different generation numbers,
    # in which the out-of-date client gets a higher timestamp.
    user1 = db.allocate_user("test@mozilla.com", generation=1, timestamp=timestamp)
    user2 = db.allocate_user("test@mozilla.com", generation=2, timestamp=timestamp - 1)
    assert user1["uid"] != user2["uid"]
    # Force node re-assignment by marking all records as replaced.
    db.replace_user_records("test@mozilla.com", timestamp=timestamp + 1)
    # The next client to show up should get a new assignment, marked
    # with the correct generation number.
    user = db.get_user("test@mozilla.com")
    assert user["generation"] == 2
    assert user["uid"] != user1["uid"]
    assert user["uid"] != user2["uid"]


def test_that_we_can_allocate_users_to_a_specific_node(db):
    """Test that we can allocate users to a specific node."""
    node = "https://phx13"
    db.add_node(node, 50)
    # The new node is not selected by default, because of lower capacity.
    user = db.allocate_user("test1@mozilla.com")
    assert user["node"] != node
    # But we can force it using keyword argument.
    user = db.allocate_user("test2@mozilla.com", node=node)
    assert user["node"] == node


def test_that_we_can_move_users_to_a_specific_node(db):
    """Test that we can move users to a specific node."""
    node = "https://phx13"
    db.add_node(node, 50)
    # The new node is not selected by default, because of lower capacity.
    user = db.allocate_user("test@mozilla.com")
    assert user["node"] != node
    # But we can move them there explicitly using keyword argument.
    db.update_user(user, node=node)
    assert user["node"] == node
    # Sanity-check by re-reading it from the db.
    user = db.get_user("test@mozilla.com")
    assert user["node"] == node
    # Check that it properly respects client-state and generation.
    db.update_user(user, generation=12)
    db.update_user(user, client_state="XXX")
    db.update_user(user, generation=42, client_state="YYY", node="https://phx12")
    assert user["node"] == "https://phx12"
    assert user["generation"] == 42
    assert user["client_state"] == "YYY"
    assert sorted(user["old_client_states"]) == ["", "XXX"]
    # Sanity-check by re-reading it from the db.
    user = db.get_user("test@mozilla.com")
    assert user["node"] == "https://phx12"
    assert user["generation"] == 42
    assert user["client_state"] == "YYY"
    assert sorted(user["old_client_states"]) == ["", "XXX"]


def test_that_record_cleanup_frees_slots_on_the_node(db):
    """Test that record cleanup frees slots on the node."""
    node = "https://phx12"
    db.update_node(node, capacity=10, available=1, current_load=9)
    # We should only be able to allocate one more user to that node.
    user = db.allocate_user("test1@mozilla.com")
    assert user["node"] == node
    with pytest.raises(Exception):
        db.allocate_user("test2@mozilla.com")
    # But when we clean up the user's record, it frees up the slot.
    db.retire_user("test1@mozilla.com")
    db.delete_user_record(user["uid"])
    user = db.allocate_user("test2@mozilla.com")
    assert user["node"] == node


def test_gradual_release_of_node_capacity(db):
    """Test gradual release of node capacity."""
    node1 = "https://phx12"
    db.update_node(node1, capacity=8, available=1, current_load=4)
    node2 = "https://phx13"
    db.add_node(node2, capacity=6, available=1, current_load=4)
    # Two allocations should succeed without update, one on each node.
    user = db.allocate_user("test1@mozilla.com")
    assert user["node"] == node1
    user = db.allocate_user("test2@mozilla.com")
    assert user["node"] == node2
    # The next allocation attempt will release 10% more capacity,
    # which is one more slot for each node.
    user = db.allocate_user("test3@mozilla.com")
    assert user["node"] == node1
    user = db.allocate_user("test4@mozilla.com")
    assert user["node"] == node2
    # Now node2 is full, so further allocations all go to node1.
    user = db.allocate_user("test5@mozilla.com")
    assert user["node"] == node1
    user = db.allocate_user("test6@mozilla.com")
    assert user["node"] == node1
    # Until it finally reaches capacity.
    with pytest.raises(Exception):
        db.allocate_user("test7@mozilla.com")


def test_count_users(db):
    """Test count users."""
    user = db.allocate_user("test1@example.com")
    assert db.count_users() == 1
    old_timestamp = get_timestamp()
    time.sleep(0.01)
    # Adding users increases the count.
    user = db.allocate_user("rfkelly@mozilla.com")
    assert db.count_users() == 2
    # Updating a user doesn't change the count.
    db.update_user(user, client_state="aaaa")
    assert db.count_users() == 2
    # Looking back in time doesn't count newer users.
    assert db.count_users(old_timestamp) == 1
    # Retiring a user decreases the count.
    db.retire_user("test1@example.com")
    assert db.count_users() == 1


def test_first_seen_at(db):
    """Test first seen at."""
    EMAIL = "test1@example.com"
    user0 = db.allocate_user(EMAIL)
    user1 = db.get_user(EMAIL)
    assert user1["uid"] == user0["uid"]
    assert user1["first_seen_at"] == user0["first_seen_at"]
    # It should stay consistent if we re-allocate the user's node.
    time.sleep(0.1)
    db.update_user(user1, client_state="aaaa")
    user2 = db.get_user(EMAIL)
    assert user2["uid"] != user0["uid"]
    assert user2["first_seen_at"] == user0["first_seen_at"]
    # Until we purge their old node-assignment records.
    db.delete_user_record(user0["uid"])
    user3 = db.get_user(EMAIL)
    assert user3["uid"] == user2["uid"]
    assert user3["first_seen_at"] != user2["first_seen_at"]


def test_build_old_range(db):
    """Test build old range."""
    params = dict()
    sql = db._build_old_user_query(None, params)
    assert sql.text.find("uid > :start") < 0
    assert sql.text.find("uid < :end") < 0
    assert params.get("start") is None
    assert params.get("end") is None

    params = dict()
    rrange = (None, "abcd")
    sql = db._build_old_user_query(rrange, params)
    assert sql.text.find("uid > :start") < 0
    assert sql.text.find("uid < :end") > 0
    assert params.get("start") is None
    assert params.get("end") == rrange[1]

    params = dict()
    rrange = ("1234", "abcd")
    sql = db._build_old_user_query(rrange, params)
    assert sql.text.find("uid > :start") > 0
    assert sql.text.find("uid < :end") > 0
    assert params.get("start") == rrange[0]
    assert params.get("end") == rrange[1]
