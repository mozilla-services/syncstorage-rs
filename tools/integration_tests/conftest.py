# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Pytest fixtures for storage integration tests.

Fixture hierarchy
─────────────────
st_ctx — function-scoped composite: creates a hawk-signed TestApp, seeds a
         random user, clears that user's data, and yields a plain dict
         consumed by test functions.

Helper functions and constants live in helpers.py.
"""

import os

import pytest

from tools.integration_tests.helpers import (
    make_auth_state,
    make_test_app,
    retry_delete,
)
from tools.integration_tests.test_support import (
    FixedSecrets,
    TokenServerAuthenticationPolicy,
)


@pytest.fixture(scope="function")
def st_ctx():
    """Functional test context for storage API tests.

    Creates a TestApp with hawk signing, authenticates a random user,
    clears that user's data, and yields a context dict.
    """
    secret = os.environ.get("SYNC_MASTER_SECRET", "TED KOPPEL IS A ROBOT")
    auth_policy = TokenServerAuthenticationPolicy(secrets=FixedSecrets(secret))
    host_url = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

    auth = make_auth_state(auth_policy, host_url)
    auth_state = {
        "auth_token": auth["auth_token"],
        "auth_secret": auth["auth_secret"],
    }

    app = make_test_app(host_url, auth_state)

    root = "/1.5/%d" % auth["user_id"]
    retry_delete(app, root)

    yield {
        "app": app,
        "root": root,
        "user_id": auth["user_id"],
        "fxa_uid": auth["fxa_uid"],
        "hashed_fxa_uid": auth["hashed_fxa_uid"],
        "fxa_kid": auth["fxa_kid"],
        "auth_state": auth_state,
        "auth_policy": auth_policy,
        "host_url": host_url,
    }
