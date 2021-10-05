# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
from base64 import urlsafe_b64encode as b64encode
import json
import os
import math
import mysql.connector
import time
import urllib.parse as urlparse

from sqlalchemy import create_engine
from tokenlib.utils import decode_token_bytes
from webtest import TestApp


class TestCase:
    FXA_EMAIL_DOMAIN = 'api-accounts.stage.mozaws.net'
    FXA_METRICS_HASH_SECRET = 'secret'
    NODE_ID = 800
    NODE_URL = 'https://example.com'
    SYNC_1_1_SERVICE_ID = 1
    SYNC_1_5_SERVICE_ID = 2
    SYNC_1_5_SERVICE_NAME = 'sync-1.5'
    TOKEN_SIGNING_SECRET = 'secret'
    TOKENSERVER_HOST = os.environ['TOKENSERVER_HOST']

    def setUp(self):
        engine = create_engine(os.environ['SYNC_TOKENSERVER__DATABASE_URL'])
        self.database = engine.execution_options(isolation_level="AUTOCOMMIT").connect()

        host_url = urlparse.urlparse(self.TOKENSERVER_HOST)
        self.app = TestApp(self.TOKENSERVER_HOST, extra_environ={
            'HTTP_HOST': host_url.netloc,
            'wsgi.url_scheme': host_url.scheme or 'http',
            'SERVER_NAME': host_url.hostname,
            'REMOTE_ADDR': '127.0.0.1',
            'SCRIPT_NAME': host_url.path,
        })
        # Start each test with a blank slate.
        cursor = self._execute_sql(('DELETE FROM users'), ())
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM nodes'), ())
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM services'), ())
        cursor.close()

        # Ensure the necessary services exists in the db.
        self._add_service('sync-1.1', '{node}/1.1/{uid}',
                          self.SYNC_1_1_SERVICE_ID)
        self._add_service('sync-1.5', '{node}/1.5/{uid}',
                          self.SYNC_1_5_SERVICE_ID)

        # Ensure we have a node with enough capacity to run the tests.
        self._add_node(capacity=100, node=self.NODE_URL, id=self.NODE_ID)

    def tearDown(self):
        # And clean up at the end, for good measure.
        cursor = self._execute_sql(('DELETE FROM users'), ())
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM nodes'), ())
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM services'), ())
        cursor.close()

        self.database.close()

    def _forge_oauth_token(self, generation=None, sub='test', scope='scope'):
        claims = {
            'fxa-generation': generation,
            'sub': sub,
            'client_id': 'client ID',
            'scope': scope,
            'fxa-profileChangedAt': None
        }
        header = b64encode(b'{}').strip(b'=').decode('utf-8')
        claims = b64encode(json.dumps(claims).encode('utf-8')) \
            .strip(b'=').decode('utf-8')
        signature = b64encode(b'signature').strip(b'=').decode('utf-8')

        return '%s.%s.%s' % (header, claims, signature)

    def _add_node(self, service=SYNC_1_5_SERVICE_NAME, capacity=100,
                  available=100, node=NODE_URL, id=None, current_load=0,
                  backoff=0, downed=0):
        service_id = self._get_service_id(service)
        query = 'INSERT INTO nodes (service, node, available, capacity, \
            current_load, backoff, downed'
        data = (service_id, node, available, capacity, current_load, backoff,
                downed)

        if id:
            query += ', id) VALUES(%s, %s, %s, %s, %s, %s, %s, %s)'
            data += (id,)
        else:
            query += ') VALUES(%s, %s, %s, %s, %s, %s, %s)'

        cursor = self._execute_sql(query, data)
        cursor.close()

        return self._last_insert_id()

    def _get_node(self, id):
        query = 'SELECT * FROM nodes WHERE id=%s'
        cursor = self._execute_sql(query, (id,))
        (id, service, node, available, current_load, capacity, downed,
         backoff) = cursor.fetchone()
        cursor.close()

        return {
            'id': id,
            'service': service,
            'node': node,
            'available': available,
            'current_load': current_load,
            'capacity': capacity,
            'downed': downed,
            'backoff': backoff
        }

    def _last_insert_id(self):
        cursor = self._execute_sql('SELECT LAST_INSERT_ID()', ())
        (id,) = cursor.fetchone()
        cursor.close()

        return id

    def _add_service(self, service_name, pattern, id):
        query = 'INSERT INTO services (service, pattern, id) \
            VALUES(%s, %s, %s)'
        cursor = self._execute_sql(query, (service_name, pattern, id))
        cursor.close()

    def _add_user(self, service=SYNC_1_5_SERVICE_ID, email=None,
                  generation=1234, client_state='616161', created_at=None,
                  nodeid=NODE_ID, keys_changed_at=1234, replaced_at=None):
        query = '''
            INSERT INTO users (service, email, generation, client_state, \
                created_at, nodeid, keys_changed_at, replaced_at)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s);
        '''
        created_at = created_at or math.trunc(time.time() * 1000)
        cursor = self._execute_sql(query,
                                   (service,
                                    email or 'test@%s' % self.FXA_EMAIL_DOMAIN,
                                    generation, client_state,
                                    created_at, nodeid, keys_changed_at,
                                    replaced_at))
        cursor.close()

        return self._last_insert_id()

    def _get_user(self, uid):
        query = 'SELECT * FROM users WHERE uid = %s'
        cursor = self._execute_sql(query, (uid,))

        (uid, service, email, generation, client_state, created_at,
         replaced_at, nodeid, keys_changed_at) = cursor.fetchone()
        cursor.close()

        return {
            'uid': uid,
            'service': service,
            'email': email,
            'generation': generation,
            'client_state': client_state,
            'created_at': created_at,
            'replaced_at': replaced_at,
            'nodeid': nodeid,
            'keys_changed_at': keys_changed_at
        }

    def _get_replaced_users(self, service_id, email):
        query = 'SELECT * FROM users WHERE service = %s AND email = %s AND \
            replaced_at IS NOT NULL'
        cursor = self._execute_sql(query, (service_id, email))

        users = []
        for user in cursor.fetchall():
            (uid, service, email, generation, client_state, created_at,
             replaced_at, nodeid, keys_changed_at) = user

            user_dict = {
                'uid': uid,
                'service': service,
                'email': email,
                'generation': generation,
                'client_state': client_state,
                'created_at': created_at,
                'replaced_at': replaced_at,
                'nodeid': nodeid,
                'keys_changed_at': keys_changed_at
            }
            users.append(user_dict)

        cursor.close()
        return users

    def _get_service_id(self, service):
        query = 'SELECT id FROM services WHERE service = %s'
        cursor = self._execute_sql(query, (service,))
        (service_id,) = cursor.fetchone()
        cursor.close()

        return service_id

    def _count_users(self):
        query = 'SELECT COUNT(DISTINCT(uid)) FROM users'
        cursor = self._execute_sql(query, ())
        (count,) = cursor.fetchone()
        cursor.close()

        return count

    def _execute_sql(self, query, args):
        cursor = self.database.execute(query, args)

        return cursor

    def unsafelyParseToken(self, token):
        # For testing purposes, don't check HMAC or anything...
        return json.loads(decode_token_bytes(token)[:-32].decode('utf8'))
