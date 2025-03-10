# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
from base64 import urlsafe_b64encode as b64encode
import binascii
import json
import os
import math
import time
import urllib.parse as urlparse

from sqlalchemy import (
    create_engine,
    event,
    select,
    delete,
    insert,
    and_,
    func,
    distinct,
)
from sqlalchemy.pool import NullPool
from sqlalchemy.engine import Engine
from sqlalchemy.orm import close_all_sessions, Session
from tokenlib.utils import decode_token_bytes
from webtest import TestApp

from tokenserver.tables import Users, Nodes, Services

DEFAULT_OAUTH_SCOPE = "https://identity.mozilla.com/apps/oldsync"


class TestCase:
    FXA_EMAIL_DOMAIN = "api-accounts.stage.mozaws.net"
    FXA_METRICS_HASH_SECRET = os.environ.get("SYNC_MASTER_SECRET", "secret0")
    NODE_ID = 800
    NODE_URL = "https://example.com"
    TOKEN_SIGNING_SECRET = os.environ.get("SYNC_MASTER_SECRET", "secret0")
    TOKENSERVER_HOST = os.environ["TOKENSERVER_HOST"]

    @classmethod
    def setUpClass(cls):
        cls._build_auth_headers = cls._build_oauth_headers

    def setUp(self):
        self._db_connect()

        host_url = urlparse.urlparse(self.TOKENSERVER_HOST)
        self.app = TestApp(
            self.TOKENSERVER_HOST,
            extra_environ={
                "HTTP_HOST": host_url.netloc,
                "wsgi.url_scheme": host_url.scheme or "http",
                "SERVER_NAME": host_url.hostname,
                "REMOTE_ADDR": "127.0.0.1",
                "SCRIPT_NAME": host_url.path,
            },
        )

        # Start each test with a blank slate.
        with Session(self.engine) as session, session.begin():
            session.execute(delete(Users))
            session.execute(delete(Nodes))
            session.execute(delete(Services))

        self.service_id = self._add_service("sync-1.5", r"{node}/1.5/{uid}")

        # Ensure we have a node with enough capacity to run the tests.
        self._add_node(capacity=100, node=self.NODE_URL, id=self.NODE_ID)

    def tearDown(self):
        # And clean up at the end, for good measure.
        with Session(self.engine) as session, session.begin():
            session.execute(delete(Users))
            session.execute(delete(Nodes))
            session.execute(delete(Services))

        # Ensure that everything is saved in db
        close_all_sessions()
        self.engine.dispose()

    def _build_oauth_headers(
        self,
        generation=None,
        user="test",
        keys_changed_at=None,
        client_state=None,
        status=200,
        **additional_headers
    ):
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
        headers["Authorization"] = "Bearer %s" % json.dumps(body)
        client_state = binascii.unhexlify(client_state)
        client_state = b64encode(client_state).strip(b"=").decode("utf-8")
        headers["X-KeyID"] = "%s-%s" % (keys_changed_at, client_state)
        headers.update(additional_headers)

        return headers

    def _add_node(
        self,
        capacity=100,
        available=100,
        node=NODE_URL,
        id=None,
        current_load=0,
        backoff=0,
        downed=0,
    ):
        query = insert(Nodes).values(
            service=self.service_id,
            node=node,
            available=available,
            capacity=capacity,
            current_load=current_load,
            backoff=backoff,
            downed=downed,
        )

        if id is not None:
            query = insert(Nodes).values(
                service=self.service_id,
                node=node,
                available=available,
                capacity=capacity,
                current_load=current_load,
                backoff=backoff,
                downed=downed,
                id=id,
            )

        with Session(self.engine) as session, session.begin():
            result = session.execute(query)
            lastrowid = result.lastrowid

        return lastrowid

    def _get_node(self, id):
        query = select(Nodes).where(Nodes.id == id)

        with Session(self.engine) as session:
            result = session.execute(query)
            (node,) = result.fetchone()

        return node._asdict()

    def _add_service(self, service_name, pattern):
        query = insert(Services).values(service=service_name, pattern=pattern)
        with Session(self.engine) as session, session.begin():
            result = session.execute(query)
            lastrowid = result.lastrowid

        return lastrowid

    def _add_user(
        self,
        email=None,
        generation=1234,
        client_state="aaaa",
        created_at=None,
        nodeid=NODE_ID,
        keys_changed_at=1234,
        replaced_at=None,
    ):
        created_at = created_at or math.trunc(time.time() * 1000)
        query = insert(Users).values(
            service=self.service_id,
            email=email or "test:%s" % self.FXA_EMAIL_DOMAIN,
            generation=generation,
            client_state=client_state,
            created_at=created_at,
            nodeid=nodeid,
            keys_changed_at=keys_changed_at,
            replaced_at=replaced_at,
        )
        with Session(self.engine) as session, session.begin():
            result = session.execute(query)
            lastrowid = result.lastrowid

        return lastrowid

    def _get_user(self, uid):
        query = select(Users).where(Users.uid == uid)
        with Session(self.engine) as session:
            result = session.execute(query)
            (user,) = result.fetchone()

        return user._asdict()

    def _get_replaced_users(self, service_id, email):
        query = select(Users).where(
            and_(
                Users.service == service_id,
                and_(Users.email == email, Users.replaced_at is not None),
            )
        )
        with Session(self.engine) as session:
            result = session.execute(query)
            users = result.fetchall()

        users_dicts = []
        for user in users:
            users_dicts.append(user._asdict())

        return users_dicts

    def _get_service_id(self, service):
        query = select(Services.id).where(Services.service == service)
        with Session(self.engine) as session:
            result = session.execute(query)
            (service_id,) = result.fetchone()

        return service_id

    def _count_users(self):
        query = select(func.count(distinct(Users.uid)))
        with Session(self.engine) as session:
            result = session.execute(query)
            (count,) = result.fetchone()

        return count

    def _clear_nodes(self):
        with Session(self.engine) as session, session.begin():
            session.execute(delete(Nodes))

    def _db_connect(self):
        self.engine = create_engine(
            os.environ["SYNC_TOKENSERVER__DATABASE_URL"], poolclass=NullPool
        )
        if self.engine.name == "sqlite":

            @event.listens_for(Engine, "connect")
            def set_sqlite_pragma(dbapi_connection, connection_record):
                cursor = dbapi_connection.cursor()
                cursor.execute("PRAGMA journal_mode = WAL;")
                cursor.execute("PRAGMA synchronous = NORMAL;")
                cursor.execute("PRAGMA foreign_keys = ON;")
                cursor.execute("PRAGMA busy_timeout = 10000")
                cursor.close()
                dbapi_connection.commit()

    def unsafelyParseToken(self, token):
        # For testing purposes, don't check HMAC or anything...
        return json.loads(decode_token_bytes(token)[:-32].decode("utf8"))
