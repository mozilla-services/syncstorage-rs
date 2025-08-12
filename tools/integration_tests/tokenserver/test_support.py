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

from sqlalchemy import create_engine, text
from sqlalchemy.engine import Connection
from tokenlib.utils import decode_token_bytes
from webtest import TestApp

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
        self.engine = create_engine(
            os.environ["SYNC_TOKENSERVER__DATABASE_URL"],
             future=True)
        self.connection = self.engine.connect()
        host_url = urlparse.urlparse(self.TOKENSERVER_HOST)
        self.app = TestApp(self.TOKENSERVER_HOST, extra_environ={
            "HTTP_HOST": host_url.netloc,
            "wsgi.url_scheme": host_url.scheme or "http",
            "SERVER_NAME": host_url.hostname,
            "REMOTE_ADDR": "127.0.0.1",
            "SCRIPT_NAME": host_url.path,
        })

        # Start each test with a blank slate.
        self._execute_sql("DELETE FROM users", ())
        self._execute_sql("DELETE FROM nodes", ())

        self.service_id = self._add_service("sync-1.5", r"{node}/1.5/{uid}")

        # Ensure we have a node with enough capacity to run the tests.
        self._add_node(capacity=100, node=self.NODE_URL, id=self.NODE_ID)

    def tearDown(self):
        # And clean up at the end, for good measure.
        self._execute_sql("DELETE FROM users", ())
        self._execute_sql("DELETE FROM nodes", ())
        self._execute_sql("DELETE FROM services", ())
        self.connection.close()
        self.engine.dispose()

    def _build_oauth_headers(self, generation=None, user="test",
                             keys_changed_at=None, client_state=None,
                             status=200, **additional_headers):
        claims = {
            "user": user,
            "generation": generation,
            "client_id": "fake client id",
            "scope": [DEFAULT_OAUTH_SCOPE],
        }

        if generation is not None:
            claims["generation"] = generation

        body = {
            "body": claims,
            "status": status
        }

        headers = {}
        headers["Authorization"] = "Bearer %s" % json.dumps(body)
        client_state = binascii.unhexlify(client_state)
        client_state = b64encode(client_state).strip(b"=").decode("utf-8")
        headers["X-KeyID"] = "%s-%s" % (keys_changed_at, client_state)
        headers.update(additional_headers)

        return headers

    def _add_node(self, capacity=100, available=100, node=NODE_URL, id=None,
                  current_load=0, backoff=0, downed=0):
        if id:
            query = """
                INSERT INTO nodes (service, node, available, capacity, current_load, backoff, downed, id)
                VALUES (:service, :node, :available, :capacity, :current_load, :backoff, :downed, :id)
            """
            data = {
                "service": self.service_id,
                "node": node,
                "available": available,
                "capacity": capacity,
                "current_load": current_load,
                "backoff": backoff,
                "downed": downed,
                "id": id,
            }
        else:
            query = """
                INSERT INTO nodes (service, node, available, capacity, current_load, backoff, downed)
                VALUES (:service, :node, :available, :capacity, :current_load, :backoff, :downed)
            """
            data = {
                "service": self.service_id,
                "node": node,
                "available": available,
                "capacity": capacity,
                "current_load": current_load,
                "backoff": backoff,
                "downed": downed,
            }

        self._execute_sql(query, data)
        return self._last_insert_id()

    def _get_node(self, id):
        query = "SELECT * FROM nodes WHERE id=:id"
        result = self._execute_sql(query, {"id": id})
        row = result.fetchone()
        result.close()

        # SQLAlchemy 2.0 returns Row objects with ._mapping
        if row is None:
            return None
        row = row._mapping

        return {
            "id": row["id"],
            "service": row["service"],
            "node": row["node"],
            "available": row["available"],
            "current_load": row["current_load"],
            "capacity": row["capacity"],
            "downed": row["downed"],
            "backoff": row["backoff"]
        }

    def _last_insert_id(self):
        result = self._execute_sql("SELECT LAST_INSERT_ID()", {})
        id_ = result.scalar()
        result.close()
        return id_

    def _add_service(self, service_name, pattern):
        query = """
            INSERT INTO services (service, pattern)
            VALUES(:service, :pattern)
        """
        self._execute_sql(query, {"service": service_name, "pattern": pattern})
        return self._last_insert_id()

    def _add_user(self, email=None, generation=1234, client_state="aaaa",
                  created_at=None, nodeid=NODE_ID, keys_changed_at=1234,
                  replaced_at=None):
        query = """
            INSERT INTO users (service, email, generation, client_state, created_at, nodeid, keys_changed_at, replaced_at)
            VALUES (:service, :email, :generation, :client_state, :created_at, :nodeid, :keys_changed_at, :replaced_at)
        """
        created_at = created_at or math.trunc(time.time() * 1000)
        data = {
            "service": self.service_id,
            "email": email or f"test@{self.FXA_EMAIL_DOMAIN}",
            "generation": generation,
            "client_state": client_state,
            "created_at": created_at,
            "nodeid": nodeid,
            "keys_changed_at": keys_changed_at,
            "replaced_at": replaced_at,
        }
        self._execute_sql(query, data)
        return self._last_insert_id()

    def _get_user(self, uid):
        query = "SELECT * FROM users WHERE uid = :uid"
        result = self._execute_sql(query, {"uid": uid})
        row = result.fetchone()
        result.close()

        if row is None:
            return None
        row = row._mapping

        return {
            "uid": row["uid"],
            "service": row["service"],
            "email": row["email"],
            "generation": row["generation"],
            "client_state": row["client_state"],
            "created_at": row["created_at"],
            "replaced_at": row["replaced_at"],
            "nodeid": row["nodeid"],
            "keys_changed_at": row["keys_changed_at"]
        }

    def _get_replaced_users(self, service_id, email):
        query = """
            SELECT * FROM users
            WHERE service = :service_id AND email = :email AND replaced_at IS NOT NULL
        """
        result = self._execute_sql(query, {"service_id": service_id, "email": email})
        users = []
        for row in result:
            row = row._mapping
            user_dict = {
                "uid": row["uid"],
                "service": row["service"],
                "email": row["email"],
                "generation": row["generation"],
                "client_state": row["client_state"],
                "created_at": row["created_at"],
                "replaced_at": row["replaced_at"],
                "nodeid": row["nodeid"],
                "keys_changed_at": row["keys_changed_at"]
            }
            users.append(user_dict)
        result.close()
        return users

    def _get_service_id(self, service):
        query = "SELECT id FROM services WHERE service = :service"
        result = self._execute_sql(query, {"service": service})
        service_id = result.scalar()
        result.close()
        return service_id

    def _count_users(self):
        query = "SELECT COUNT(DISTINCT(uid)) FROM users"
        result = self._execute_sql(query, {})
        count = result.scalar()
        result.close()
        return count

    def _execute_sql(self, query, args):
        # SQLAlchemy future mode requires text()
        if isinstance(args, tuple):
            # Convert tuple to dict for named queries
            # This should only happen for legacy calls
            raise ValueError("Use a dict for SQL arguments!")
        stmt = text(query)
        return self.connection.execute(stmt, args)

    def unsafelyParseToken(self, token):
        # For testing purposes, don't check HMAC or anything...
        return json.loads(decode_token_bytes(token)[:-32].decode("utf8"))
    