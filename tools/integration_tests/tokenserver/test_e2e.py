# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""End-to-end integration tests for the tokenserver."""

import hmac
import json
import jwt
import string
import tokenlib
from base64 import urlsafe_b64decode
from hashlib import sha256

from cryptography.hazmat.backends import default_backend
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from integration_tests.tokenserver.helpers import (
    FXA_METRICS_HASH_SECRET,
    TOKEN_SIGNING_SECRET,
    unsafe_parse_token,
)

# This is the client ID used for Firefox Desktop. The FxA team confirmed that
# this is the proper client ID to be using for these integration tests.
CLIENT_ID = "5882386c6d801776"
DEFAULT_TOKEN_DURATION = 3600
FXA_ACCOUNT_STAGE_HOST = "https://api-accounts.stage.mozaws.net"
FXA_OAUTH_STAGE_HOST = "https://oauth.stage.mozaws.net"
PASSWORD_CHARACTERS = string.ascii_letters + string.punctuation + string.digits
PASSWORD_LENGTH = 32
SCOPE = "https://identity.mozilla.com/apps/oldsync"


def _fxa_metrics_hash(value: str) -> str:
    """Compute the FxA metrics hash for a given value."""
    hasher = hmac.new(FXA_METRICS_HASH_SECRET.encode("utf-8"), b"", sha256)
    hasher.update(value.encode("utf-8"))
    return hasher.hexdigest()


def _get_bad_token() -> str:
    """Generate a JWT signed with an untrusted key."""
    key = rsa.generate_private_key(
        backend=default_backend(), public_exponent=65537, key_size=2048
    )
    pem = key.private_bytes(
        encoding=serialization.Encoding.PEM,
        format=serialization.PrivateFormat.TraditionalOpenSSL,
        encryption_algorithm=serialization.NoEncryption(),
    )
    claims = {"sub": "fake sub", "iat": 12345, "exp": 12345}
    return jwt.encode(claims, pem.decode("utf-8"), algorithm="RS256")


def test_unauthorized_oauth_error_status(ts_ctx, fxa_auth):
    """Test unauthorized oauth error status."""
    app = ts_ctx["app"]
    oauth_client = fxa_auth["oauth_client"]
    session = fxa_auth["session"]

    # Totally busted auth -> generic error.
    headers = {
        "Authorization": "Unsupported-Auth-Scheme IHACKYOU",
        "X-KeyID": "1234-qqo",
    }
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "errors": [{"description": "Unsupported", "location": "body", "name": ""}],
        "status": "error",
    }
    assert res.json == expected_error_response

    bad_token = _get_bad_token()
    headers = {"Authorization": f"Bearer {bad_token}", "X-KeyID": "1234-qqo"}
    # Bad token -> 'invalid-credentials'
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    expected_error_response = {
        "errors": [{"description": "Unauthorized", "location": "body", "name": ""}],
        "status": "invalid-credentials",
    }
    assert res.json == expected_error_response

    # Untrusted scopes -> 'invalid-credentials'
    bad_scope_token = oauth_client.authorize_token(session, "bad_scope")
    headers = {"Authorization": f"Bearer {bad_scope_token}", "X-KeyID": "1234-qqo"}
    res = app.get("/1.0/sync/1.5", headers=headers, status=401)
    assert res.json == expected_error_response


def test_valid_oauth_request(ts_ctx, fxa_auth):
    """Test valid oauth request."""
    app = ts_ctx["app"]
    expected_node_type = ts_ctx["expected_node_type"]
    session = fxa_auth["session"]
    oauth_token = fxa_auth["oauth_token"]

    headers = {"Authorization": f"Bearer {oauth_token}", "X-KeyID": "1234-qqo"}
    # Send a valid request, allocating a new user
    res = app.get("/1.0/sync/1.5", headers=headers)
    fxa_uid = session.uid

    # Verify the token signature using tokenlib
    raw = urlsafe_b64decode(res.json["id"])
    payload = raw[:-32]
    signature = raw[-32:]
    payload_str = payload.decode("utf-8")
    payload_dict = json.loads(payload_str)
    # The `id` payload should include a field indicating the origin of the token
    assert payload_dict["tokenserver_origin"] == "rust"
    signing_secret = TOKEN_SIGNING_SECRET
    tm = tokenlib.TokenManager(secret=signing_secret)
    expected_signature = tm._get_signature(payload_str.encode("utf8"))
    # Using compare_digest here is good practice even in a test context
    assert hmac.compare_digest(expected_signature, signature)
    # Check that the given key is a secret derived from the hawk ID
    expected_secret = tokenlib.get_derived_secret(res.json["id"], secret=signing_secret)
    assert res.json["key"] == expected_secret
    # Check to make sure the remainder of the fields are valid
    assert res.json["api_endpoint"].startswith("https://example.com/1.5/")
    assert res.json["duration"] == 3600
    assert res.json["hashalg"] == "sha256"
    assert res.json["hashed_fxa_uid"] == _fxa_metrics_hash(fxa_uid)[:32]
    # Verify the node_type matches the syncstorage backend being tested
    assert res.json["node_type"] == expected_node_type
    # The response should have an X-Timestamp header
    assert "X-Timestamp" in res.headers
    assert int(res.headers["X-Timestamp"]) is not None
    token = unsafe_parse_token(res.json["id"])
    assert "hashed_device_id" in token
    assert token["uid"] == res.json["uid"]
    assert token["fxa_uid"] == fxa_uid
    assert token["fxa_kid"] == "0000000001234-qqo"
    assert token["hashed_fxa_uid"] != token["fxa_uid"]
    assert token["hashed_fxa_uid"] == res.json["hashed_fxa_uid"]
    assert "hashed_device_id" in token
