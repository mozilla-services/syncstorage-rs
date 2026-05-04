# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Pytest fixtures for tokenserver integration tests.

Fixture hierarchy
─────────────────
ts_db_conn    — function-scoped SQLAlchemy connection
ts_app        — function-scoped WebTest TestApp
ts_service_id — function-scoped service ID (sync-1.5)
ts_ctx        — function-scoped composite: clears DB, seeds service + node,
                yields a plain dict consumed by test functions
fxa_auth      — session-scoped FxA OAuth token for test_e2e.py
                (session scope justified: FxA account creation is a slow
                network call to an external staging service; one account
                suffices for the whole test session)

Helper functions and constants live in helpers.py.
"""

import os
import random
import string
import time

import pytest
from sqlalchemy import create_engine

from integration_tests.tokenserver.helpers import (
    NODE_ID,
    NODE_URL,
    add_node,
    clear_db,
    get_db_mode,
    get_expected_node_type,
    get_or_add_service,
    make_app,
)


# ── Fixtures ──────────────────────────────────────────────────────────────────


@pytest.fixture(scope="function")
def ts_db_conn():
    """Function-scoped SQLAlchemy connection to the tokenserver database."""
    db_url = os.environ["SYNC_TOKENSERVER__DATABASE_URL"]
    # SQLAlchemy 1.4+ wants postgresql:// not postgres://
    if db_url.startswith("postgres://"):
        db_url = db_url.replace("postgres://", "postgresql://", 1)
    engine = create_engine(db_url)
    conn = engine.execution_options(isolation_level="AUTOCOMMIT").connect()
    yield conn
    conn.close()


@pytest.fixture(scope="function")
def ts_app():
    """Function-scoped WebTest TestApp pointing at the tokenserver host."""
    return make_app(os.environ["TOKENSERVER_HOST"])


@pytest.fixture(scope="function")
def ts_service_id(ts_db_conn):
    """Function-scoped service ID for sync-1.5, inserted if it does not exist."""
    return get_or_add_service(ts_db_conn, "sync-1.5", r"{node}/1.5/{uid}")


@pytest.fixture(scope="function")
def ts_ctx(ts_db_conn, ts_app, ts_service_id):
    """Full per-test tokenserver context.

    Clears the database, seeds the default service and node, then yields
    a dict that test functions can destructure:

        def test_foo(ts_ctx):
            app = ts_ctx["app"]
            ...
    """
    clear_db(ts_db_conn)
    add_node(ts_db_conn, ts_service_id, capacity=100, node=NODE_URL, id=NODE_ID)
    yield {
        "db_conn": ts_db_conn,
        "app": ts_app,
        "service_id": ts_service_id,
        "db_mode": get_db_mode(),
        "expected_node_type": get_expected_node_type(),
    }
    clear_db(ts_db_conn)


@pytest.fixture(scope="session")
def fxa_auth():
    """Session-scoped FxA OAuth token for test_e2e.py.

    Session scope is justified: creating a real FxA account requires a
    network round-trip to the FxA staging service plus email verification,
    which can take several seconds. One account is sufficient for all e2e
    tests in a single session.
    """
    from fxa.core import Client
    from fxa.errors import ClientError, ServerError
    from fxa.oauth import Client as OAuthClient
    from fxa.tests.utils import TestEmailAccount

    FXA_ACCOUNT_STAGE_HOST = "https://api-accounts.stage.mozaws.net"
    FXA_OAUTH_STAGE_HOST = "https://oauth.stage.mozaws.net"
    CLIENT_ID = "5882386c6d801776"
    SCOPE = "https://identity.mozilla.com/apps/oldsync"
    PASSWORD_CHARACTERS = string.ascii_letters + string.punctuation + string.digits

    password = "".join(random.choice(PASSWORD_CHARACTERS) for _ in range(32))
    acct = TestEmailAccount()
    client = Client(FXA_ACCOUNT_STAGE_HOST)
    oauth_client = OAuthClient(CLIENT_ID, None, server_url=FXA_OAUTH_STAGE_HOST)

    session = client.create_account(acct.email, password=password)
    # Poll for the verification email
    while not acct.messages:
        time.sleep(0.5)
        acct.fetch()
    for m in acct.messages:
        if "x-verify-code" in m["headers"]:
            session.verify_email_code(m["headers"]["x-verify-code"])

    oauth_token = oauth_client.authorize_token(session, SCOPE)

    yield {
        "session": session,
        "oauth_client": oauth_client,
        "oauth_token": oauth_token,
        "password": password,
        "acct": acct,
        "client": client,
    }

    acct.clear()
    try:
        client.destroy_account(acct.email, password)
    except (ServerError, ClientError) as ex:
        print(f"warning: Encountered error when cleaning up FxA account: {ex}")
