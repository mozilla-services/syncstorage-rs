# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Helper functions and constants for tokenserver integration tests.

These are plain module-level utilities — no pytest fixtures here.
Test files import directly from this module; conftest.py imports
whatever it needs to seed fixtures.
"""

import binascii
import json
import math
import os
import time
import uuid
import urllib.parse as urlparse
from base64 import urlsafe_b64encode as b64encode

from fxa_delete.make_delete_assertion import TYP as DELETE_ASSERTION_TYP
from fxa_delete.make_delete_assertion import make_delete_assertion
from sqlalchemy.sql import text as sqltext
from tokenlib.utils import decode_token_bytes
from webtest import TestApp

DEFAULT_OAUTH_SCOPE = "https://identity.mozilla.com/apps/oldsync"

NODE_ID = 800
NODE_URL = "https://example.com"
FXA_EMAIL_DOMAIN = "api-accounts.stage.mozaws.net"
TOKEN_SIGNING_SECRET = os.environ.get("SYNC_MASTER_SECRET", "secret0")
FXA_METRICS_HASH_SECRET = os.environ.get("SYNC_MASTER_SECRET", "secret0")

#: Signing key for delete assertions. In production FxA signs these; the e2e
#: stack configures the public half of this key as a tokenserver JWK.
DELETE_SIGNING_KEY_PEM = os.environ.get(
    "FXA_DELETE_SIGNING_KEY_PEM",
    os.path.join(
        os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
        "fxa_delete",
        "testing",
        "delete_test_key.pem",
    ),
)
DELETE_DATA_AUDIENCE = os.environ.get(
    "SYNC_TOKENSERVER__FXA_DELETE_DATA_AUDIENCE",
    "urn:mozilla:fxa:tokenserver:sync-data-delete",
)
DELETE_DATA_ISSUER = os.environ.get(
    "SYNC_TOKENSERVER__FXA_DELETE_DATA_ISSUER", "https://accounts.firefox.com"
)

#: The syncstorage instance the tests actually talk to. The default node
#: (NODE_URL) is unroutable on purpose, which is fine for token issuance but
#: not for a delete that has to reach a real node.
LOCAL_NODE_URL = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")


# ── DB-mode helpers ───────────────────────────────────────────────────────────


def get_db_mode() -> str:
    """Derive db_mode from the SYNC_TOKENSERVER__DATABASE_URL env var."""
    return os.environ["SYNC_TOKENSERVER__DATABASE_URL"].split(":")[0]


def get_expected_node_type() -> str:
    """Derive expected node_type from the SYNC_SYNCSTORAGE__DATABASE_URL env var."""
    syncstorage_url = os.environ.get("SYNC_SYNCSTORAGE__DATABASE_URL", "spanner://")
    node_type = syncstorage_url.split(":")[0]
    if node_type == "postgresql":
        return "postgres"
    if node_type.startswith("mysql"):
        return "mysql"
    return node_type


# ── SQL helpers ───────────────────────────────────────────────────────────────


def execute_sql(conn, query, params=None):
    """Execute a SQL statement and return the cursor."""
    return conn.execute(query, params or {})


def clear_db(conn) -> None:
    """Delete all users and nodes.

    Services are intentionally not cleared: tokenserver may have cached
    its service_id and a DELETE would invalidate that cache mid-run.
    """
    execute_sql(conn, sqltext("DELETE FROM users"), {}).close()
    execute_sql(conn, sqltext("DELETE FROM nodes"), {}).close()


def get_service_id(conn, service: str):
    """Return the ID for the given service name, or None if not found."""
    cursor = execute_sql(
        conn,
        sqltext("select id from services where service = :service"),
        {"service": service},
    )
    row = cursor.fetchone()
    cursor.close()
    return None if row is None else row[0]


def add_service(conn, service: str, pattern: str) -> int:
    """Insert a services row and return its ID."""
    db_mode = get_db_mode()
    if db_mode == "postgres":
        sql = sqltext(
            "insert into services (service, pattern) values (:service, :pattern) RETURNING id"
        )
        cursor = execute_sql(conn, sql, {"service": service, "pattern": pattern})
        result: int = cursor.fetchone()[0]
    else:
        sql = sqltext(
            "insert into services (service, pattern) values (:service, :pattern)"
        )
        cursor = execute_sql(conn, sql, {"service": service, "pattern": pattern})
        result = cursor.lastrowid
    cursor.close()
    return result


def get_or_add_service(conn, service: str, pattern: str) -> int:
    """Return existing service ID, inserting a new row if it does not exist."""
    service_id = get_service_id(conn, service)
    if service_id is not None:
        return int(service_id)
    return add_service(conn, service, pattern)


def add_node(
    conn,
    service_id: int,
    capacity: int = 100,
    available: int = 100,
    node: str = NODE_URL,
    id: int | None = None,
    current_load: int = 0,
    backoff: int = 0,
    downed: int = 0,
) -> int:
    """Insert a nodes row and return its ID."""
    db_mode = get_db_mode()
    params = {
        "service": service_id,
        "node": node,
        "available": available,
        "capacity": capacity,
        "current_load": current_load,
        "backoff": backoff,
        "downed": downed,
    }
    if id is not None:
        params["id"] = id
        cols = "service, node, available, capacity, current_load, backoff, downed, id"
        vals = ":service, :node, :available, :capacity, :current_load, :backoff, :downed, :id"
    else:
        cols = "service, node, available, capacity, current_load, backoff, downed"
        vals = (
            ":service, :node, :available, :capacity, :current_load, :backoff, :downed"
        )

    result: int
    if db_mode == "postgres":
        sql = sqltext(f"insert into nodes ({cols}) values ({vals}) RETURNING id")  # nosec B608 - cols/vals are hardcoded literals, not user input
        cursor = execute_sql(conn, sql, params)
        result = cursor.fetchone()[0]
    else:
        sql = sqltext(f"insert into nodes ({cols}) values ({vals})")  # nosec B608
        cursor = execute_sql(conn, sql, params)
        result = cursor.lastrowid
    cursor.close()
    return result


def seeded_ctx(conn, app, service_id: int, node_url: str):
    """Yield a per-test tokenserver context seeded with `node_url`."""
    clear_db(conn)
    add_node(conn, service_id, capacity=100, node=node_url, id=NODE_ID)
    yield {
        "db_conn": conn,
        "app": app,
        "service_id": service_id,
        "db_mode": get_db_mode(),
        "expected_node_type": get_expected_node_type(),
    }
    clear_db(conn)


def get_node(conn, node_id: int) -> dict:
    """Return a node dict by ID."""
    cursor = execute_sql(
        conn, sqltext("select * from nodes where id = :id"), {"id": node_id}
    )
    (id_, service, node, available, current_load, capacity, downed, backoff) = (
        cursor.fetchone()
    )
    cursor.close()
    return {
        "id": id_,
        "service": service,
        "node": node,
        "available": available,
        "current_load": current_load,
        "capacity": capacity,
        "downed": downed,
        "backoff": backoff,
    }


def add_user(
    conn,
    service_id: int,
    email: str | None = None,
    nodeid: int = NODE_ID,
    generation: int = 1234,
    keys_changed_at: int | None = 1234,
    client_state: str = "aaaa",
    created_at: int | None = None,
    replaced_at: int | None = None,
) -> int:
    """Insert a users row and return its uid."""
    db_mode = get_db_mode()
    created_at = created_at or math.trunc(time.time() * 1000)
    params = {
        "service": service_id,
        "email": email or f"test@{FXA_EMAIL_DOMAIN}",
        "nodeid": nodeid,
        "generation": generation,
        "keys_changed_at": keys_changed_at,
        "client_state": client_state,
        "created_at": created_at,
        "replaced_at": replaced_at,
    }
    result: int
    if db_mode == "postgres":
        sql = sqltext("""\
            insert into users
                (service, email, nodeid, generation, keys_changed_at,
                 client_state, created_at, replaced_at)
            values
                (:service, :email, :nodeid, :generation, :keys_changed_at,
                 :client_state, :created_at, :replaced_at)
            RETURNING uid
        """)
        cursor = execute_sql(conn, sql, params)
        result = cursor.fetchone()[0]
    else:
        sql = sqltext("""\
            insert into users
                (service, email, nodeid, generation, keys_changed_at,
                 client_state, created_at, replaced_at)
            values
                (:service, :email, :nodeid, :generation, :keys_changed_at,
                 :client_state, :created_at, :replaced_at)
        """)
        cursor = execute_sql(conn, sql, params)
        result = cursor.lastrowid
    cursor.close()
    return result


def get_user(conn, uid: int) -> dict:
    """Return a user dict by uid."""
    cursor = execute_sql(
        conn, sqltext("select * from users where uid = :uid"), {"uid": uid}
    )
    (
        uid,
        service,
        email,
        generation,
        client_state,
        created_at,
        replaced_at,
        nodeid,
        keys_changed_at,
    ) = cursor.fetchone()
    cursor.close()
    return {
        "uid": uid,
        "service": service,
        "email": email,
        "generation": generation,
        "client_state": client_state,
        "created_at": created_at,
        "replaced_at": replaced_at,
        "nodeid": nodeid,
        "keys_changed_at": keys_changed_at,
    }


def get_replaced_users(conn, service_id: int, email: str) -> list:
    """Return a list of user dicts for records with a non-null replaced_at."""
    cursor = execute_sql(
        conn,
        sqltext("""\
            select * from users
             where service = :service
               and email = :email
               and replaced_at is not null
        """),
        {"service": service_id, "email": email},
    )
    users = []
    for row in cursor.fetchall():
        (
            uid,
            service,
            email,
            generation,
            client_state,
            created_at,
            replaced_at,
            nodeid,
            keys_changed_at,
        ) = row
        users.append(
            {
                "uid": uid,
                "service": service,
                "email": email,
                "generation": generation,
                "client_state": client_state,
                "created_at": created_at,
                "replaced_at": replaced_at,
                "nodeid": nodeid,
                "keys_changed_at": keys_changed_at,
            }
        )
    cursor.close()
    return users


def count_users(conn) -> int:
    """Return the count of distinct user UIDs."""
    cursor = execute_sql(conn, sqltext("select COUNT(DISTINCT(uid)) from users"), {})
    (count,) = cursor.fetchone()
    cursor.close()
    return int(count)


# ── Auth helpers ──────────────────────────────────────────────────────────────


def build_oauth_headers(
    generation: int | None = None,
    user: str = "test",
    keys_changed_at: int | None = None,
    client_state: str | None = None,
    status: int = 200,
    **additional_headers: str,
) -> dict:
    """Build OAuth Bearer + X-KeyID headers for a test request."""
    claims = {
        "user": user,
        "generation": generation,
        "client_id": "fake client id",
        "scope": [DEFAULT_OAUTH_SCOPE],
    }
    if generation is not None:
        claims["generation"] = generation
    body = {"body": claims, "status": status}
    headers = {}
    headers["Authorization"] = f"Bearer {json.dumps(body)}"
    client_state_bytes = binascii.unhexlify(client_state or "")
    client_state_b64 = b64encode(client_state_bytes).strip(b"=").decode("utf-8")
    headers["X-KeyID"] = f"{keys_changed_at}-{client_state_b64}"
    headers.update(additional_headers)
    return headers


def make_app(host: str) -> TestApp:
    """Build a WebTest TestApp pointing at the given host URL."""
    host_url = urlparse.urlparse(host)
    return TestApp(
        host,
        extra_environ={
            "HTTP_HOST": host_url.netloc,
            "wsgi.url_scheme": host_url.scheme or "http",
            "SERVER_NAME": host_url.hostname,
            "REMOTE_ADDR": "127.0.0.1",
            "SCRIPT_NAME": host_url.path,
        },
    )


def unsafe_parse_token(token: str) -> dict:
    """Parse a tokenlib token without verifying its HMAC signature."""
    return json.loads(decode_token_bytes(token)[:-32].decode("utf8"))  # type: ignore[no-any-return]


def make_delete_token(
    fxa_uid: str,
    sub: str | None = None,
    audience: str | None = None,
    issuer: str | None = None,
    ttl: int = 30,
    typ: str = DELETE_ASSERTION_TYP,
    jti: str | None = "",
    key_path: str | None = None,
) -> str:
    """Mint a delete assertion, with per-claim overrides for the failure cases.

    `sub` defaults to the uid in the path; a test overrides it to prove the two
    must agree. `jti` defaults to a fresh uuid; pass None to omit it.
    """
    with open(key_path or DELETE_SIGNING_KEY_PEM, "rb") as handle:
        pem = handle.read()
    # Annotated local: mypy cannot see through PyJWT's untyped `encode`, so
    # the minter's declared `-> str` arrives here as Any.
    token: str = make_delete_assertion(
        pem,
        sub if sub is not None else fxa_uid,
        audience=audience or DELETE_DATA_AUDIENCE,
        issuer=issuer or DELETE_DATA_ISSUER,
        ttl=ttl,
        typ=typ,
        jti=str(uuid.uuid4()) if jti == "" else jti,
    )
    return token


def delete_syncstorage_data(app, fxa_uid: str, **kwargs):
    """Call the FxA data deletion endpoint for `fxa_uid`.

    Mints the assertion the endpoint expects with the same reference
    implementation given to the FxA Auth Server. Keyword arguments not consumed
    by the minter are passed through to WebTest, so a test can assert on a
    specific status.
    """
    mint_keys = {"sub", "audience", "issuer", "ttl", "typ", "jti", "key_path"}
    mint = {k: kwargs.pop(k) for k in list(kwargs) if k in mint_keys}
    token = make_delete_token(fxa_uid, **mint)
    return app.delete(
        f"/1.0/users/{fxa_uid}/storage",
        headers={"Authorization": f"Bearer {token}"},
        **kwargs,
    )
