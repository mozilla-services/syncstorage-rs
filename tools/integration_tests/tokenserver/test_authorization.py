# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Authorization integration tests for the tokenserver."""

from integration_tests.tokenserver.helpers import (
    add_user,
    build_oauth_headers,
    get_user,
    unsafe_parse_token,
)


def test_unauthorized_error_status(ts_ctx):
    """Test unauthorized error status."""
    app = ts_ctx["app"]
    # Totally busted auth -> generic error.
    headers = {"Authorization": "Unsupported-Auth-Scheme IHACKYOU"}
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "errors": [{"description": "Unsupported", "location": "body", "name": ""}],
        "status": "error",
    }
    assert res.json == expected_error_response


def test_no_auth(ts_ctx):
    """Test no auth."""
    app = ts_ctx["app"]
    res = app.get("/1.0/sync/1.5", status=401)
    expected_error_response = {
        "status": "error",
        "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
    }
    assert res.json == expected_error_response


def test_invalid_client_state_in_key_id(ts_ctx):
    """Test invalid client state in key id."""
    app = ts_ctx["app"]
    additional_headers = {"X-KeyID": "1234-state!"}
    headers = build_oauth_headers(
        keys_changed_at=1234, client_state="aaaa", **additional_headers
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-credentials",
        "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
    }
    assert res.json == expected_error_response


def test_invalid_client_state_in_x_client_state(ts_ctx):
    """Test invalid client state in x client state."""
    app = ts_ctx["app"]
    additional_headers = {"X-Client-State": "state!"}
    headers = build_oauth_headers(
        generation=1234,
        keys_changed_at=1234,
        client_state="aaaa",
        **additional_headers,
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=400)
    expected_error_response = {
        "status": "error",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Invalid client state value",
            }
        ],
    }
    assert res.json == expected_error_response


def test_keys_changed_at_less_than_equal_to_generation(ts_ctx):
    """Test keys changed at less than equal to generation."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(db_conn, service_id, generation=1232, keys_changed_at=1234)
    # If keys_changed_at changes, that change must be less than or equal
    # to the new generation
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1236, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-keysChangedAt",
        "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
    }
    assert res.json == expected_error_response
    # If the keys_changed_at on the request matches that currently stored
    # on the user record, it does not need to be less than or equal to the
    # generation on the request
    headers = build_oauth_headers(
        generation=1233, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # A request with no generation is acceptable
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1235, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # A request with a keys_changed_at less than the new generation
    # is acceptable
    headers = build_oauth_headers(
        generation=1236, keys_changed_at=1235, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)


def test_disallow_reusing_old_client_state(ts_ctx):
    """Test disallow reusing old client state."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user record that has already been replaced
    add_user(db_conn, service_id, client_state="aaaa", replaced_at=1200)
    # Add the most up-to-date user record
    add_user(db_conn, service_id, client_state="bbbb")
    # A request cannot use a client state associated with a replaced user
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value stale value",
            }
        ],
    }
    assert res.json == expected_error_response
    # Using the last-seen client state is okay
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="bbbb"
    )
    res1 = app.get("/1.0/sync/1.5", headers=headers)
    # Using a new client state (with an updated generation and
    # keys_changed_at) is okay
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1235, client_state="cccc"
    )
    res2 = app.get("/1.0/sync/1.5", headers=headers)
    # This results in the creation of a new user record
    assert res1.json["uid"] != res2.json["uid"]


def test_generation_change_must_accompany_client_state_change(ts_ctx):
    """Test generation change must accompany client state change."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(db_conn, service_id, generation=1234, client_state="aaaa")
    # A request with a new client state must also contain a new generation
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value new "
                "value with no generation change",
            }
        ],
    }
    assert res.json == expected_error_response
    # A request with no generation is acceptable
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1235, client_state="bbbb"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # We can't use a generation of 1235 when setting a new client state
    # because the generation was set to be equal to the keys_changed_at
    # in the previous request, which was 1235
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1235, client_state="cccc"
    )
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value new "
                "value with no generation change",
            }
        ],
    }
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    assert res.json == expected_error_response
    # A change in client state is acceptable only with a change in
    # generation (if it is present)
    headers = build_oauth_headers(
        generation=1236, keys_changed_at=1236, client_state="cccc"
    )
    app.get("/1.0/sync/1.5", headers=headers)


def test_keys_changed_at_change_must_accompany_client_state_change(ts_ctx):
    """Test keys changed at change must accompany client state change."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(
        db_conn, service_id, generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    # A request with a new client state must also contain a new
    # keys_changed_at
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1234, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value new "
                "value with no keys_changed_at change",
            }
        ],
    }
    assert res.json == expected_error_response
    # A request with a new keys_changed_at is acceptable
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1235, client_state="bbbb"
    )
    app.get("/1.0/sync/1.5", headers=headers)


def test_generation_must_not_be_less_than_last_seen_value(ts_ctx):
    """Test generation must not be less than last seen value."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    uid = add_user(db_conn, service_id, generation=1234)
    # The generation in the request cannot be less than the generation
    # currently stored on the user record
    headers = build_oauth_headers(
        generation=1233, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-generation",
        "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
    }
    assert res.json == expected_error_response
    # A request with no generation is acceptable
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # A request with a generation equal to the last-seen generation is
    # acceptable
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # A request with a generation greater than the last-seen generation is
    # acceptable
    headers = build_oauth_headers(
        generation=1235, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    # This should not result in the creation of a new user
    assert res.json["uid"] == uid


def test_set_generation_unchanged_without_keys_changed_at_update(ts_ctx):
    """Test set generation unchanged without keys changed at update."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user who has never sent us a generation
    uid = add_user(
        db_conn, service_id, generation=0, keys_changed_at=1234, client_state="aaaa"
    )
    # Send a request without a generation that doesn't update keys_changed_at
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    user = get_user(db_conn, uid)
    # This should not have set the user's generation
    assert user["generation"] == 0
    # Send a request without a generation that updates keys_changed_at
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1235, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    user = get_user(db_conn, uid)
    # This should have set the user's generation
    assert user["generation"] == 1235


def test_set_generation_with_keys_changed_at_initialization(ts_ctx):
    """Test set generation with keys changed at initialization."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user who has never sent us a generation or a keys_changed_at
    uid = add_user(
        db_conn, service_id, generation=0, keys_changed_at=None, client_state="aaaa"
    )
    # Send a request without a generation that updates keys_changed_at
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    user = get_user(db_conn, uid)
    # This should have set the user's generation
    assert user["generation"] == 1234


def test_fxa_kid_change(ts_ctx):
    """Test fxa kid change."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(
        db_conn, service_id, generation=1234, keys_changed_at=None, client_state="aaaa"
    )
    # An OAuth client shows up, setting keys_changed_at.
    # (The value matches generation number above, because in this scenario
    # FxA hasn't been updated to track and report keysChangedAt yet).
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    token0 = unsafe_parse_token(res.json["id"])
    # Reject keys_changed_at lower than the value previously seen
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1233, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-keysChangedAt",
        "errors": [{"location": "body", "name": "", "description": "Unauthorized"}],
    }
    assert res.json == expected_error_response
    # Reject greater keys_changed_at with no corresponding update to
    # generation
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=2345, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    assert res.json == expected_error_response
    # Accept equal keys_changed_at
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    app.get("/1.0/sync/1.5", headers=headers)
    # Accept greater keys_changed_at with new generation
    headers = build_oauth_headers(
        generation=2345, keys_changed_at=2345, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    token = unsafe_parse_token(res.json["id"])
    assert token["fxa_kid"] == "0000000002345-u7s"
    assert token["uid"] != token0["uid"]
    assert token["node"] == token0["node"]


def test_client_specified_duration(ts_ctx):
    """Test client specified duration."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(
        db_conn, service_id, generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    # It's ok to request a shorter-duration token.
    res = app.get("/1.0/sync/1.5?duration=12", headers=headers)
    assert res.json["duration"] == 12
    # But you can't exceed the server's default value.
    res = app.get("/1.0/sync/1.5?duration=4000", headers=headers)
    assert res.json["duration"] == 3600
    # And nonsense values are ignored.
    res = app.get("/1.0/sync/1.5?duration=lolwut", headers=headers)
    assert res.json["duration"] == 3600
    res = app.get("/1.0/sync/1.5?duration=-1", headers=headers)
    assert res.json["duration"] == 3600


# Although all servers are now writing keys_changed_at, we still need this
# case to be handled. See this PR for more information:
# https://github.com/mozilla-services/tokenserver/pull/176
def test_kid_change_during_gradual_tokenserver_rollout(ts_ctx):
    """Test kid change during gradual tokenserver rollout."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Let's start with a user already in the db, with no keys_changed_at.
    uid = add_user(
        db_conn, service_id, generation=1234, client_state="aaaa", keys_changed_at=None
    )
    user1 = get_user(db_conn, uid)
    # User hits updated tokenserver node, writing keys_changed_at to db.
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1200, client_state="aaaa"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    # That should not have triggered a node re-assignment.
    user2 = get_user(db_conn, res.json["uid"])
    assert user1["uid"] == user2["uid"]
    assert user1["nodeid"] == user2["nodeid"]
    # That should have written keys_changed_at into the db.
    assert user2["generation"] == 1234
    assert user2["keys_changed_at"] == 1200
    # User does a password reset on their Firefox Account.
    headers = build_oauth_headers(
        generation=2345, keys_changed_at=2345, client_state="bbbb"
    )
    # They sync again, but hit a tokenserver node that isn't updated yet.
    # This would trigger the allocation of a new user, so we simulate this
    # by adding a new user. We set keys_changed_at to be the last-used
    # value, since we are simulating a server that doesn't pay attention
    # to keys_changed_at.
    uid = add_user(
        db_conn, service_id, generation=2345, keys_changed_at=1200, client_state="bbbb"
    )
    user2 = get_user(db_conn, uid)
    assert user1["uid"] != user2["uid"]
    assert user1["nodeid"] == user2["nodeid"]
    # They sync again, hitting an updated tokenserver node.
    # This should succeed, despite keys_changed_at appearing to have
    # changed without any corresponding change in generation number.
    res = app.get("/1.0/sync/1.5", headers=headers)
    # That should not have triggered a second user allocation.
    user1 = user2
    user2 = get_user(db_conn, res.json["uid"])
    assert user2["uid"] == user1["uid"]
    assert user2["nodeid"] == user1["nodeid"]


def test_update_client_state(ts_ctx):
    """Test update client state."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    uid = add_user(
        db_conn, service_id, generation=0, keys_changed_at=None, client_state=""
    )
    user1 = get_user(db_conn, uid)
    # The user starts out with no client_state
    assert user1["generation"] == 0
    assert user1["client_state"] == ""
    seen_uids = {uid}
    orig_node = user1["nodeid"]
    # Changing client_state allocates a new user, resulting in a new uid
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    user2 = get_user(db_conn, res.json["uid"])
    assert user2["uid"] not in seen_uids
    assert user2["nodeid"] == orig_node
    assert user2["generation"] == 1234
    assert user2["keys_changed_at"] == 1234
    assert user2["client_state"] == "bbbb"
    seen_uids.add(user2["uid"])
    # We can change the client state even if no generation is present on
    # the request
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1235, client_state="cccc"
    )
    res = app.get("/1.0/sync/1.5", headers=headers)
    user3 = get_user(db_conn, res.json["uid"])
    assert user3["uid"] not in seen_uids
    assert user3["nodeid"] == orig_node
    # When keys_changed_at changes and generation is not present on the
    # request, generation is set to be the same as keys_changed_at
    assert user3["generation"] == 1235
    assert user3["keys_changed_at"] == 1235
    assert user3["client_state"] == "cccc"
    seen_uids.add(user3["uid"])
    # We cannot change client_state without a change in keys_changed_at
    headers = build_oauth_headers(
        generation=None, keys_changed_at=1235, client_state="dddd"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value new "
                "value with no keys_changed_at change",
            }
        ],
    }
    assert expected_error_response == res.json
    # We cannot use a previously-used client_state
    headers = build_oauth_headers(
        generation=1236, keys_changed_at=1236, client_state="bbbb"
    )
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "status": "invalid-client-state",
        "errors": [
            {
                "location": "header",
                "name": "X-Client-State",
                "description": "Unacceptable client-state value stale value",
            }
        ],
    }
    assert expected_error_response == res.json


def test_set_generation_from_no_generation(ts_ctx):
    """Test set generation from no generation."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user that has no generation set
    uid = add_user(
        db_conn, service_id, generation=0, keys_changed_at=None, client_state="aaaa"
    )
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    # Send a request to set the generation
    app.get("/1.0/sync/1.5", headers=headers)
    user = get_user(db_conn, uid)
    # Ensure that the user had the correct generation set
    assert user["generation"] == 1234


def test_set_keys_changed_at_from_no_keys_changed_at(ts_ctx):
    """Test set keys changed at from no keys changed at."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user that has no keys_changed_at set
    uid = add_user(
        db_conn, service_id, generation=1234, keys_changed_at=None, client_state="aaaa"
    )
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    # Send a request to set the keys_changed_at
    app.get("/1.0/sync/1.5", headers=headers)
    user = get_user(db_conn, uid)
    # Ensure that the user had the correct generation set
    assert user["keys_changed_at"] == 1234


def test_x_client_state_must_have_same_client_state_as_key_id(ts_ctx):
    """Test x client state must have same client state as key id."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    add_user(db_conn, service_id, client_state="aaaa")
    additional_headers = {"X-Client-State": "bbbb"}
    headers = build_oauth_headers(
        generation=1234,
        keys_changed_at=1234,
        client_state="aaaa",
        **additional_headers,
    )
    # If present, the X-Client-State header must have the same client
    # state as the X-KeyID header
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "errors": [{"description": "Unauthorized", "location": "body", "name": ""}],
        "status": "invalid-client-state",
    }
    assert res.json == expected_error_response
    headers["X-Client-State"] = "aaaa"
    app.get("/1.0/sync/1.5", headers=headers)


def test_zero_generation_treated_as_null(ts_ctx):
    """Test zero generation treated as null."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user that has a generation set
    uid = add_user(
        db_conn, service_id, generation=1234, keys_changed_at=1234, client_state="aaaa"
    )
    headers = build_oauth_headers(
        generation=0, keys_changed_at=1234, client_state="aaaa"
    )
    # Send a request with a generation of 0
    app.get("/1.0/sync/1.5", headers=headers)
    # Ensure that the request succeeded and that the user's generation
    # was not updated
    user = get_user(db_conn, uid)
    assert user["generation"] == 1234


def test_zero_keys_changed_at_treated_as_null(ts_ctx):
    """Test zero keys changed at treated as null."""
    db_conn = ts_ctx["db_conn"]
    app = ts_ctx["app"]
    service_id = ts_ctx["service_id"]
    # Add a user that has no keys_changed_at set
    uid = add_user(
        db_conn, service_id, generation=1234, keys_changed_at=None, client_state="aaaa"
    )
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=0, client_state="aaaa"
    )
    # Send a request with a keys_changed_at of 0
    app.get("/1.0/sync/1.5", headers=headers)
    # Ensure that the request succeeded and that the user's
    # keys_changed_at was not updated
    user = get_user(db_conn, uid)
    assert user["keys_changed_at"] is None
