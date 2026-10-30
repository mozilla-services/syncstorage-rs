# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Integration tests for the FxA-triggered syncstorage data deletion endpoint.

`DELETE /1.0/users/{fxa_uid}/storage` clears a user's sync data while leaving
their Tokenserver record intact. These tests cover both halves of that: the
data really goes away, and the account really survives.
"""

import time

from integration_tests.tokenserver.helpers import (
    FXA_EMAIL_DOMAIN,
    add_user,
    build_oauth_headers,
    delete_syncstorage_data,
    get_user,
)

FXA_UID = "deletedatauid"
EMAIL = f"{FXA_UID}@{FXA_EMAIL_DOMAIN}"


def test_delete_data_clears_storage_and_keeps_the_user(ts_local_node_ctx):
    """Data is removed; the Tokenserver record is left usable."""
    app = ts_local_node_ctx["app"]
    conn = ts_local_node_ctx["db_conn"]
    service_id = ts_local_node_ctx["service_id"]

    uid = add_user(conn, service_id, email=EMAIL)
    before = get_user(conn, uid)

    response = delete_syncstorage_data(app, FXA_UID)

    assert response.status_code == 200
    assert response.json["deleted"] is True

    # The account must survive: this is "reset sync data", not a deletion.
    # A bumped generation would lock the user out, and a replaced_at would
    # hand them a different node on their next sync.
    after = get_user(conn, uid)
    assert after["replaced_at"] is None
    assert after["generation"] == before["generation"]
    assert after["keys_changed_at"] == before["keys_changed_at"]
    assert after["nodeid"] == before["nodeid"]


def test_delete_data_leaves_the_user_able_to_get_a_token(ts_local_node_ctx):
    """After a delete, the next token request returns the same uid and node."""
    app = ts_local_node_ctx["app"]
    conn = ts_local_node_ctx["db_conn"]
    service_id = ts_local_node_ctx["service_id"]

    add_user(conn, service_id, email=EMAIL)
    headers = build_oauth_headers(
        generation=1234, keys_changed_at=1234, client_state="aaaa", user=FXA_UID
    )
    before = app.get("/1.0/sync/1.5", headers=headers).json

    delete_syncstorage_data(app, FXA_UID)

    after = app.get("/1.0/sync/1.5", headers=headers).json
    assert after["uid"] == before["uid"]
    assert after["api_endpoint"] == before["api_endpoint"]


def test_delete_data_acts_on_the_current_record_only(ts_local_node_ctx):
    """Only the newest record is deleted; superseded ones are purge's job."""
    app = ts_local_node_ctx["app"]
    conn = ts_local_node_ctx["db_conn"]
    service_id = ts_local_node_ctx["service_id"]

    now = int(time.time() * 1000)
    old_uid = add_user(
        conn,
        service_id,
        email=EMAIL,
        generation=1,
        keys_changed_at=1,
        client_state="aaaa",
        created_at=now - 1000,
        replaced_at=now - 500,
    )
    current_uid = add_user(
        conn,
        service_id,
        email=EMAIL,
        generation=5000,
        keys_changed_at=5000,
        client_state="bbbb",
        created_at=now,
    )
    assert old_uid != current_uid

    response = delete_syncstorage_data(app, FXA_UID)

    assert response.status_code == 200
    assert response.json["deleted"] is True
    # Neither record may be mutated by the delete.
    assert get_user(conn, current_uid)["replaced_at"] is None
    assert get_user(conn, old_uid)["generation"] == 1


def test_delete_data_unknown_user_is_404(ts_local_node_ctx):
    """An fxa_uid with no Tokenserver record is a 404."""
    app = ts_local_node_ctx["app"]

    response = delete_syncstorage_data(app, "nosuchuser", status=404)

    assert response.status_code == 404


def test_delete_data_rejects_bad_credentials(ts_local_node_ctx):
    """A wrong secret, and a missing header, are both 401."""
    app = ts_local_node_ctx["app"]
    conn = ts_local_node_ctx["db_conn"]
    add_user(conn, ts_local_node_ctx["service_id"], email=EMAIL)

    wrong = delete_syncstorage_data(app, FXA_UID, secret="w" * 32, status=401)
    assert wrong.status_code == 401

    missing = app.delete(f"/1.0/users/{FXA_UID}/storage", status=401)
    assert missing.status_code == 401
