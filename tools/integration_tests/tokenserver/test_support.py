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

from sqlalchemy import create_engine
from sqlalchemy.sql import text as sqltext
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
        engine = create_engine(os.environ["SYNC_TOKENSERVER__DATABASE_URL"])
        self.database = engine.execution_options(isolation_level="AUTOCOMMIT").connect()
        self.db_mode = os.environ["SYNC_TOKENSERVER__DATABASE_URL"].split(":")[0]
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
        cursor = self._execute_sql(sqltext(("DELETE FROM users")), {})
        cursor.close()

        cursor = self._execute_sql((sqltext("DELETE FROM nodes")), {})
        cursor.close()

        self.service_id = self._add_service("sync-1.5", r"{node}/1.5/{uid}")

        # Ensure we have a node with enough capacity to run the tests.
        self._add_node(capacity=100, node=self.NODE_URL, id=self.NODE_ID)

    def tearDown(self):
        # And clean up at the end, for good measure.
        cursor = self._execute_sql(sqltext(("DELETE FROM users")), {})
        cursor.close()

        cursor = self._execute_sql(sqltext(("DELETE FROM nodes")), {})
        cursor.close()

        cursor = self._execute_sql(sqltext(("DELETE FROM services")), {})
        cursor.close()

        self.database.close()

    def _build_oauth_headers(
        self,
        generation=None,
        user="test",
        keys_changed_at=None,
        client_state=None,
        status=200,
        **additional_headers,
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
        headers["Authorization"] = f"Bearer {json.dumps(body)}"
        client_state = binascii.unhexlify(client_state)
        client_state = b64encode(client_state).strip(b"=").decode("utf-8")
        headers["X-KeyID"] = f"{keys_changed_at}-{client_state}"
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
        if not id:
            params = {
                "service": self.service_id,
                "node": node,
                "available": available,
                "capacity": capacity,
                "current_load": current_load,
                "backoff": backoff,
                "downed": downed,
            }
            query = sqltext("""\
            insert into nodes (service, node, available, capacity, \
                current_load, backoff, downed)
            values (:service, :node, :available, :capacity, :current_load,
                    :backoff, :downed)
            """)
            query_pg = sqltext("""\
            insert into nodes (service, node, available, capacity, \
                current_load, backoff, downed)
            values (:service, :node, :available, :capacity, :current_load,
                    :backoff, :downed)
            RETURNING id
            """)
        else:
            query = sqltext("""\
            insert into nodes (service, node, available, capacity, \
                current_load, backoff, downed, id)
            values (:service, :node, :available, :capacity, :current_load,
                    :backoff, :downed, :id)
            """)
            query_pg = sqltext("""\
            insert into nodes (service, node, available, capacity, \
                current_load, backoff, downed, id)
            values (:service, :node, :available, :capacity, :current_load,
                    :backoff, :downed, :id)
            RETURNING id
            """)
            params = {
                "service": self.service_id,
                "node": node,
                "available": available,
                "capacity": capacity,
                "current_load": current_load,
                "backoff": backoff,
                "downed": downed,
                "id": id,
            }

        if self.db_mode == "postgres":
            cursor = self._execute_sql(query_pg, params)
        else:
            cursor = self._execute_sql(query, params)
        cursor.close()

        if self.db_mode == "postgres":
            return cursor.fetchone()[0]
        else:
            return cursor.lastrowid

    def _get_node(self, id):
        query = sqltext("select * from nodes where id = :id")
        cursor = self._execute_sql(query, {"id": id})
        (id, service, node, available, current_load, capacity, downed, backoff) = (
            cursor.fetchone()
        )
        cursor.close()

        return {
            "id": id,
            "service": service,
            "node": node,
            "available": available,
            "current_load": current_load,
            "capacity": capacity,
            "downed": downed,
            "backoff": backoff,
        }

    def _add_service(self, service, pattern):
        """Add definition for a new service."""
        if self.db_mode == "postgres":
            insert_sql = sqltext("""
            insert into services (service, pattern)
            values (:service, :pattern)
            RETURNING id
        """)
        else:
            insert_sql = sqltext("""
          insert into services (service, pattern)
          values (:service, :pattern)
        """)
        cursor = self._execute_sql(insert_sql, {"service": service, "pattern": pattern})
        cursor.close()
        if self.db_mode == "postgres":
            return cursor.fetchone()[0]
        else:
            return cursor.lastrowid

    def _add_user(
        self,
        email=None,
        nodeid=NODE_ID,
        generation=1234,
        keys_changed_at=1234,
        client_state="aaaa",
        created_at=None,
        replaced_at=None,
    ):
        if self.db_mode == "postgres":
            insert_sql = sqltext("""\
            insert into users (service, email, nodeid, generation, keys_changed_at, client_state, created_at, replaced_at)
            values (:service, :email, :nodeid, :generation, :keys_changed_at, :client_state, :created_at, :replaced_at)
            RETURNING uid
        """)
        else:
            insert_sql = sqltext("""\
            insert into
                users
                (service, email, nodeid, generation, keys_changed_at, client_state,
                created_at, replaced_at)
            values
                (:service, :email, :nodeid, :generation, :keys_changed_at,
                :client_state, :created_at, :replaced_at)
            """)

        created_at = created_at or math.trunc(time.time() * 1000)
        params = {
            "service": self.service_id,
            "email": email or f"test@{self.FXA_EMAIL_DOMAIN}",
            "nodeid": nodeid,
            "generation": generation,
            "keys_changed_at": keys_changed_at,
            "client_state": client_state,
            "created_at": created_at,
            "replaced_at": replaced_at,
        }
        cursor = self._execute_sql(insert_sql, params)
        cursor.close()
        if self.db_mode == "postgres":
            return cursor.fetchone()[0]
        else:
            return cursor.lastrowid

    def _get_user(self, uid):
        query = sqltext("select * from users where uid = :uid")
        cursor = self._execute_sql(query, {"uid": uid})
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

    def _get_replaced_users(self, service, email):
        query = sqltext("""\
                select * from users
                 where service = :service
                   and email = :email
                   and replaced_at is not null
                """)
        params = {"service": service, "email": email}
        cursor = self._execute_sql(query, params)

        users = []
        for user in cursor.fetchall():
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
            ) = user

            user_dict = {
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
            users.append(user_dict)

        cursor.close()
        return users

    def _get_service_id(self, service):
        query = sqltext("select id from services where service = :service")
        cursor = self._execute_sql(query, {"service": service})
        (service_id,) = cursor.fetchone()
        cursor.close()

        return service_id

    def _count_users(self):
        query = sqltext("select COUNT(DISTINCT(uid)) from users")
        cursor = self._execute_sql(query, {})
        (count,) = cursor.fetchone()
        cursor.close()

        return count

    def _execute_sql(self, *args, **kwds):
        """Execute SQL statement. *args is the query and **kwds are the keyword
        argument parameters."""
        cursor = self.database.execute(*args, **kwds)
        return cursor

    def unsafelyParseToken(self, token):
        # For testing purposes, don't check HMAC or anything...
        return json.loads(decode_token_bytes(token)[:-32].decode("utf8"))
