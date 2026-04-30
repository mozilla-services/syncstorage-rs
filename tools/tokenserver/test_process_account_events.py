# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Tests for the tokenserver process_account_events module."""

import json

import pytest
from testfixtures import LogCapture

from process_account_events import process_account_event


EMAIL = "test@example.com"
UID = "test"
ISS = "example.com"


def message_body(**kwds):
    """Build a JSON-encoded SNS message body from the given keyword arguments."""
    return json.dumps({"Message": json.dumps(kwds)})


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

    # These update_user calls previously failed (SYNC-3633)
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
    # Mark the current record as replaced. This can probably only occur
    # during a race condition in row creation
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
