# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import cryptography
import json
import jwt
import os
import mock
import mysql.connector
import random
import string
import time
import unittest
import urllib.parse as urlparse

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.backends import default_backend
from fxa.tools.bearer import get_bearer_token
from fxa.core import Client
from fxa.oauth import Client as OAuthClient
from fxa.tests.utils import TestEmailAccount
from pyramid import testing
from testfixtures import LogCapture
from test_support import load_into_settings
from webtest import TestApp

from tokenlib.utils import decode_token_bytes

here = os.path.dirname(__file__)

CLIENT_ID = '5882386c6d801776'
FXA_ACCOUNT_STAGE_HOST = 'https://api-accounts.stage.mozaws.net'
FXA_OAUTH_STAGE_HOST = 'https://oauth.stage.mozaws.net'
NODE_URL='https://example.com'
PASSWORD_CHARACTERS = string.ascii_letters + string.punctuation + string.digits
PASSWORD_LENGTH = 32
SCOPE = 'https://identity.mozilla.com/apps/oldsync'
SECONDS_IN_A_YEAR = 60 * 60 * 24 * 365

# TODO:
# - figure out metrics logging
# - figure out strange nondeterminism
# - clean up code (should we really be using so many class vars?)
#   (where should we store constants?)
# - determine whether some tests should run as these integration tests or unit
#   tests in rust instead
# - how to run unit tests?

class TestService(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.acct = TestEmailAccount()
        cls.client = Client(FXA_ACCOUNT_STAGE_HOST)
        cls.oauth_client = OAuthClient(CLIENT_ID, None, server_url=FXA_OAUTH_STAGE_HOST)
        cls.fxa_password = TestService._generate_password()
        cls.session = cls.client.create_account(cls.acct.email, password=cls.fxa_password)

        print(cls.acct.email)
        print(cls.fxa_password)

        while not cls.acct.messages:
            time.sleep(0.5)
            cls.acct.fetch()
        for m in cls.acct.messages:
            if "x-verify-code" in m["headers"]:
                cls.session.verify_email_code(m["headers"]["x-verify-code"])

        cls.oauth_token = cls.oauth_client.authorize_token(cls.session, SCOPE)

        cls.database = mysql.connector.connect(
            user=os.environ['TOKENSERVER_DATABASE_USER'],
            password=os.environ['TOKENSERVER_DATABASE_PASSWORD'],
            host=os.environ['TOKENSERVER_DATABASE_HOST'],
            database=os.environ['TOKENSERVER_DATABASE_NAME']
        )

    @classmethod
    def tearDownClass(cls):
        cls.acct.clear()
        cls.client.destroy_account(cls.acct.email, cls.fxa_password)

        cls.database.close()

    def setUp(self):
        # self.config = testing.setUp()
        # settings = {}
        # load_into_settings(self.get_ini(), settings)
        # self.config.add_settings(settings)
        host = "http://localhost:5000"
        host_url = urlparse.urlparse(host)
        self.app = TestApp(host, extra_environ={
            "HTTP_HOST": host_url.netloc,
            "wsgi.url_scheme": host_url.scheme or "http",
            "SERVER_NAME": host_url.hostname,
            "REMOTE_ADDR": "127.0.0.1",
            "SCRIPT_NAME": host_url.path,
        })
        #self.logs = LogCapture()
        # Start each test with a blank slate.
        cursor = self._execute_sql(('DELETE FROM users'), ())
        TestService.database.commit()
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM nodes'), ())
        TestService.database.commit()
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM services'), ())
        TestService.database.commit()
        cursor.close()
        # Ensure the necessary service exists in the db.
        self._add_service('sync-1.1', '{node}/1.1/{uid}')
        self._add_service('sync-1.5', '{node}/1.5/{uid}')
        # Ensure we have a node with enough capacity to run the tests.
        self._add_node('sync-1.5', 100, NODE_URL, id=800)

    def tearDown(self):
        # And clean up at the end, for good measure.
        cursor = self._execute_sql(('DELETE FROM users'), ())
        TestService.database.commit()
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM nodes'), ())
        TestService.database.commit()
        cursor.close()

        cursor = self._execute_sql(('DELETE FROM services'), ())
        TestService.database.commit()
        cursor.close()

        #self.logs.uninstall()

    def _add_node(self, service, capacity, node, id=None):
        service_id = self._get_service_id(service)
        query = 'INSERT INTO nodes (service, node, available, capacity'
        data = (service_id, node, 100, capacity)

        if id:
            query += ', id) VALUES(%s, %s, %s, %s, %s)'
            data += (id,)
        else:
            query += ') VALUES(%s, %s, %s, %s)'

        cursor = self._execute_sql(query, data)
        TestService.database.commit()
        cursor.close()

    def _add_service(self, service_name, pattern):
        query = 'INSERT INTO services (service, pattern) VALUES(%s, %s)'
        cursor = self._execute_sql(query, (service_name, pattern))
        TestService.database.commit()
        cursor.close()

    # def _add_user(self, service_name, pattern):
    #     query = 'INSERT INTO users (service, pattern) VALUES(%s, %s)'
    #     cursor = self._execute_sql(query, (service_name, pattern))
    #     TestService.database.commit()
    #     cursor.close()
    
    @staticmethod
    def _generate_password():
        return ''.join(random.choice(PASSWORD_CHARACTERS) for i in range(PASSWORD_LENGTH))
        
    def _get_token_with_bad_scope(self):
        bad_scope = 'bad_scope'

        return get_bearer_token(TestService.acct.email,
                                TestService.fxa_password,
                                scopes=[bad_scope],
                                account_server_url=FXA_ACCOUNT_STAGE_HOST,
                                oauth_server_url=FXA_OAUTH_STAGE_HOST,
                                client_id=CLIENT_ID)

    def _get_bad_token(self):
        key = rsa.generate_private_key(backend=default_backend(), public_exponent=65537, \
            key_size=2048)
        pem = key.private_bytes(encoding=serialization.Encoding.PEM,
            format=serialization.PrivateFormat.TraditionalOpenSSL,
            encryption_algorithm=serialization.NoEncryption())
        private_key = pem.decode('utf-8')
        claims = {
            'sub': 'fake sub',
            'iat': 12345,
            'exp': 12345,
        }

        return jwt.encode(claims, private_key, algorithm='RS256')
    
    def _get_user(self, service_name, email):
        query = 'SELECT * FROM users WHERE service = %s AND email = %s'
        
        cursor = self._execute_sql(query, (service_name, email))
        user = cursor.fetchone()
        cursor.close()

        return user

    def _change_password(self):
        new_password = TestService._generate_password()
        TestService.session.change_password(TestService.fxa_password, new_password)
        TestService.fxa_password = new_password
        # Refresh the session
        TestService.session = TestService.client.login(TestService.acct.email, TestService.fxa_password)
        # Refresh the OAuth token
        TestService.oauth_token = TestService.oauth_client.authorize_token(TestService.session, SCOPE)

    def _update_user(self, service_name, email, generation=None,
                    client_state=None, keys_changed_at=None, node=None):
        query = 'UPDATE users SET '
        data = ()

        if generation:
            query += 'generation = %s'
            data += (generation,)

        if client_state:
            query += 'client_state = %s'
            data += (client_state,)

        if keys_changed_at:
            query += 'keys_changed_at = %s'
            data += (keys_changed_at,)

        if node:
            query += 'node = %s'
            data += (node,)

        query += 'WHERE service = %s AND email = %s'

        cursor = self._execute_sql(query, data)
        TestService.database.commit()
        cursor.close()

    def _get_service_id(self, service):
        query = 'SELECT id FROM services WHERE service = %s'
        cursor = self._execute_sql(query, (service,))
        (service_id,) = cursor.fetchone()
        cursor.close()

        return service_id

    def _execute_sql(self, query, args):
        cursor = TestService.database.cursor()
        cursor.execute(query, args)

        return cursor

    def assertExceptionWasLogged(self, msg):
        for r in self.logs.records:
            if r.msg == msg:
                assert r.exc_info is not None
                break
        else:
            assert False, "exception with message %r was not logged" % (msg,)

    def assertMessageWasNotLogged(self, msg):
        for r in self.logs.records:
            if r.msg == msg:
                assert False, "message %r was unexpectedly logged" % (msg,)

    def assertMetricWasLogged(self, key):
        """Check that a metric was logged during the request."""
        for r in self.logs.records:
            if key in r.__dict__:
                break
        else:
            assert False, "metric %r was not logged" % (key,)

    def clearLogs(self):
        del self.logs.records[:]

    def unsafelyParseToken(self, token):
        # For testing purposes, don't check HMAC or anything...
        return json.loads(decode_token_bytes(token)[:-32].decode("utf8"))

    def test_unknown_app(self):
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        resp = self.app.get('/1.0/xXx/token', headers=headers, status=404)
        self.assertTrue('errors' in resp.json)

    def test_invalid_client_state(self):
        headers = {'X-Client-State': 'state!'}
        resp = self.app.get('/1.0/sync/1.5', headers=headers, status=400)
        self.assertEquals(resp.json['errors'][0]['location'], 'header')
        self.assertEquals(resp.json['errors'][0]['name'], 'X-Client-State')

    def test_no_auth(self):
        self.app.get('/1.0/sync/1.5', status=401)

    def test_valid_app(self):
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        self.assertIn('https://example.com/1.5', res.json['api_endpoint'])
        self.assertIn('duration', res.json)
        self.assertEquals(res.json['duration'], 3600)
        #self.assertMetricWasLogged('token.assertion.verify_success')
        #self.clearLogs()

    def test_unknown_pattern(self):
        # sync 1.1 is defined in the .ini file, but  no pattern exists for it.
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        self.app.get('/1.0/sync/1.1', headers=headers, status=503)

    def test_discovery(self):
        res = self.app.get('/')
        self.assertEqual(res.json, {
            'auth': 'http://localhost:5000',
            'services': {
                'sync': ['1.1', '1.5'],
            },
            'oauth': {
                'default_issuer': 'api-accounts.stage.mozaws.net',
                'scope': 'https://identity.mozilla.com/apps/oldsync',
                'server_url': 'https://oauth.stage.mozaws.net/v1',
            }
        })

    def test_unauthorized_error_status(self):
        # Totally busted auth -> generic error.
        headers = {'Authorization': 'Unsupported-Auth-Scheme IHACKYOU'}
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json['status'], 'error')

        token = self._get_bad_token()
        headers = {'Authorization': 'Bearer %s' % token}
        # Bad token -> "invalid-credentials"
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json['status'], 'invalid-credentials')
        # self.assertMetricWasLogged('token.oauth.errno.108')
        # self.assertMessageWasNotLogged('Unexpected verification error')
        # Untrusted scopes -> "invalid-credentials"
        token = self._get_token_with_bad_scope()
        headers = {'Authorization': 'Bearer %s' % token}
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json['status'], 'invalid-credentials')
        # self.assertMessageWasNotLogged('Unexpected verification error')
        # self.clearLogs()

    def test_generation_number_change(self):
        oauth_token_1 = TestService.oauth_token
        headers = {"Authorization": "Bearer %s" % oauth_token_1}
        res1 = self.app.get("/1.0/sync/1.5", headers=headers)
        # Equal generation numbers are accepted.
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res2.json["api_endpoint"])
        # Changing the password of the FxA account results in a higher
        # generation number.
        self._change_password()
        oauth_token_2 = TestService.oauth_token
        # Later generation numbers are accepted.
        # Again, the node assignment should not change.
        headers = {"Authorization": "Bearer %s" % oauth_token_2}
        res3 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res3.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res3.json["api_endpoint"])
        # Previous generation numbers get an invalid-generation response.
        headers = {"Authorization": "Bearer %s" % oauth_token_1}
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        self.assertEqual(res.json["status"], "invalid-generation")

    """
    TODO This test passes when it's run against a freshly-started tokenserver
    instance but fails every subsequent run. I think it has something to do
    with users being assigned to services that no longer exist (same issue as
    test_fxa_kid_change I think)
    """
    # def test_client_state_change(self):
    #     # Start with no client-state header.
    #     headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     uid0 = res.json['uid']
    #     # No change == same uid.
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     self.assertEqual(res.json['uid'], uid0)
    #     # Changing client-state header requires changing generation and
    #     # keys_changed_at.
    #     headers['X-KeyID'] = '1234-YWFh'
    #     res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
    #     self.assertEqual(res.json['status'], 'invalid-client-state')
    #     desc = res.json['errors'][0]['description']
    #     self.assertTrue(desc.endswith('new value with no generation change'))
    #     self._change_password()
    #     headers['Authorization'] = 'Bearer %s' % TestService.oauth_token
    #     # Change the client-state header, get a new uid.
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     uid1 = res.json['uid']
    #     self.assertNotEqual(uid1, uid0)
    #     # No change == same uid.
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     self.assertEqual(res.json['uid'], uid1)
    #     # Send a client-state header, get a new uid.
    #     headers["X-KeyID"] = "1236-YmJi"
    #     self._change_password()
    #     headers['Authorization'] = 'Bearer %s' % TestService.oauth_token
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     uid2 = res.json['uid']
    #     self.assertNotEqual(uid2, uid0)
    #     self.assertNotEqual(uid2, uid1)
    #     # No change == same uid.
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     self.assertEqual(res.json['uid'], uid2)
    #     # Use a previous client-state, get an auth error.
    #     headers['X-KeyID'] = '1236-YWFh'
    #     res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
    #     self.assertEqual(res.json['status'], 'invalid-client-state')
    #     desc = res.json['errors'][0]['description']
    #     self.assertTrue(desc.endswith('stale value'))
    #     self._change_password()
    #     headers['Authorization'] = 'Bearer %s' % TestService.oauth_token
    #     headers["X-KeyID"] = "1237-YWFh"
    #     res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
    #     self.assertEqual(res.json['status'], 'invalid-client-state')

    """
    TODO This test passes when it's run against a freshly-started tokenserver
    instance but fails every subsequent run. I think it has something to do
    with users being assigned to services that no longer exist. Here is a
    database dump from one of the failing runs:

    nodes: [(800, 1729, 'https://spanner.example.com', 100, 0, 100, 0, 0), (1476, 1728, 'https://example.com', 100, 0, 100, 0, 0)]
    services: [(1728, 'sync-1.1', '{node}/1.1/{uid}'), (1729, 'sync-1.5', '{node}/1.5/{uid}')]
    users: [(388, 1727, '9476852beb014721a76e1489d1707cb1@api-accounts.stage.mozaws.net', 1623349743075, '616161', 1623349744653, None, 800, 1234)]

    As you can see, the user is assigned to service 1727, which doesn't exist.
    I can't figure out why this is happening.
    """
    # def test_fxa_kid_change(self):
    #     # keys_changed_at <= generation --> good
    #     # keys_changed_at > generation --> bad
    #     # Now an OAuth client shows up, setting keys_changed_at.
    #     # (The value matches generation number above, beause in this scenario
    #     # FxA hasn't been updated to track and report keysChangedAt yet).
    #     headers = {
    #         "Authorization": "Bearer %s" % TestService.oauth_token,
    #         "X-KeyID": "1234-YWFh",
    #     }
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     token0 = self.unsafelyParseToken(res.json["id"])
    #     # Reject keys_changed_at lower than the value previously seen
    #     headers["X-KeyID"] = "1233-YWFh"
    #     res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
    #     self.assertEqual(res.json["status"], "invalid-keysChangedAt")
    #     # Reject greater keys_changed_at with no corresponding update to generation
    #     headers["X-KeyID"] = "2345-YmJi"
    #     res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
    #     self.assertEqual(res.json["status"], "invalid-client-state")
    #     # Change the password to update the generation number
    #     self._change_password()
    #     decoded = jwt.decode(TestService.oauth_token, options={'verify_signature': False})
    #     # Accept greater keys_changed_at with new generation
    #     headers["Authorization"] = "Bearer %s" % TestService.oauth_token
    #     res = self.app.get('/1.0/sync/1.5', headers=headers)
    #     token = self.unsafelyParseToken(res.json["id"])
    #     self.assertEqual(token["fxa_kid"], "0000000002345-YmJi")
    #     self.assertNotEqual(token["uid"], token0["uid"])
    #     self.assertEqual(token["node"], token0["node"])

    # TODO this might be better written as a unit test for the new version;
    # there's no way to manipulate the JWT claims after they're signed by
    # FxA (we'd need to add a test mode)
    # TODO what is this testing?
    # def test_kid_change_during_gradual_tokenserver_rollout(self):
    #     # Let's start with a user already in the db, with no keys_changed_at.
    #     # user0 = self.backend.allocate_user("sync-1.1", "test@mozilla.com",
    #     #                                    generation=1234,
    #     #                                    client_state="616161")
    #     headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
    #     self.app.get('/1.0/sync/1.5', headers=headers)
    #     # User hits updated tokenserver node, writing keys_changed_at to db.
    #     headers = {
    #         "Authorization": "BrowserID %s" % self._getassertion(),
    #         "X-Client-State": "616161",
    #     }
    #     mock_response = {
    #         "status": "okay",
    #         "email": "test@mozilla.com",
    #         "idpClaims": {
    #             "fxa-generation": 1234,
    #             "fxa-keysChangedAt": 1200,
    #         },
    #     }
    #     with self.mock_browserid_verifier(response=mock_response):
    #         self.app.get('/1.0/sync/1.1', headers=headers)
    #     # That should not have triggered a node re-assignment.
    #     user1 = self.backend.get_user("sync-1.1", mock_response["email"])
    #     self.assertEqual(user1['uid'], user0['uid'])
    #     self.assertEqual(user1['node'], user0['node'])
    #     # That should have written keys_changed_at into the db.
    #     self.assertEqual(user1["generation"], 1234)
    #     self.assertEqual(user1["keys_changed_at"], 1200)
    #     # User does a password reset on their Firefox Account.
    #     mock_response["idpClaims"]["fxa-generation"] = 2345
    #     mock_response["idpClaims"]["fxa-keysChangedAt"] = 2345
    #     headers["X-Client-State"] = "626262"
    #     # They sync again, but hit a tokenserver node that isn't updated yet.
    #     # Simulate this by writing the updated data directly to the backend,
    #     # which should trigger a node re-assignment.
    #     self.backend.update_user("sync-1.1", user1,
    #                              generation=2345,
    #                              client_state="626262")
    #     self.assertNotEqual(user1['uid'], user0['uid'])
    #     self.assertEqual(user1['node'], user0['node'])
    #     # They sync again, hitting an updated tokenserver node.
    #     # This should succeed, despite keys_changed_at appearing to have
    #     # changed without any corresponding change in generation number.
    #     with self.mock_browserid_verifier(response=mock_response):
    #         res = self.app.get('/1.0/sync/1.1', headers=headers)
    #     token = self.unsafelyParseToken(res.json["id"])
    #     self.assertEqual(token["fxa_kid"], "0000000002345-YmJi")
    #     # That should not have triggered a second node re-assignment.
    #     user2 = self.backend.get_user("sync-1.1", mock_response["email"])
    #     self.assertEqual(user2['uid'], user1['uid'])
    #     self.assertEqual(user2['node'], user1['node'])

    def test_client_state_cannot_revert_to_empty(self):
        # Start with a client-state header.
        headers = {
            'Authorization': 'Bearer %s' % TestService.oauth_token,
            'X-Client-State': 'aaaa',
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        uid0 = res.json['uid']
        # Sending no client-state will fail.
        del headers['X-Client-State']
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json['status'], 'invalid-client-state')
        desc = res.json['errors'][0]['description']
        self.assertTrue(desc.endswith('empty string'))
        headers['X-Client-State'] = ''
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json['status'], 'invalid-client-state')
        desc = res.json['errors'][0]['description']
        self.assertTrue(desc.endswith('empty string'))
        # And the uid will be unchanged.
        headers['X-Client-State'] = 'aaaa'
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        self.assertEqual(res.json['uid'], uid0)

    def test_client_specified_duration(self):
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        # It's ok to request a shorter-duration token.
        res = self.app.get('/1.0/sync/1.5?duration=12', headers=headers)
        self.assertEquals(res.json['duration'], 12)
        # But you can't exceed the server's default value.
        res = self.app.get('/1.0/sync/1.5?duration=4000', headers=headers)
        self.assertEquals(res.json['duration'], 3600)
        # And nonsense values are ignored.
        res = self.app.get('/1.0/sync/1.5?duration=lolwut', headers=headers)
        self.assertEquals(res.json['duration'], 3600)
        res = self.app.get('/1.0/sync/1.5?duration=-1', headers=headers)
        self.assertEquals(res.json['duration'], 3600)

    # TODO we can't really emulate this test remotely since it involves updating
    # settings mid-flight
    # def test_allow_new_users(self):
    #     # New users are allowed by default.
    #     settings = self.config.registry.settings
    #     self.assertEquals(settings.get('tokenserver.allow_new_users'), None)
    #     assertion = self._getassertion(email="newuser1@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     self.app.get('/1.0/sync/1.1', headers=headers, status=200)
    #     # They're allowed if we explicitly allow them.
    #     settings['tokenserver.allow_new_users'] = True
    #     assertion = self._getassertion(email="newuser2@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     self.app.get('/1.0/sync/1.1', headers=headers, status=200)
    #     # They're not allowed if we explicitly disable them.
    #     settings['tokenserver.allow_new_users'] = False
    #     assertion = self._getassertion(email="newuser3@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     res = self.app.get('/1.0/sync/1.1', headers=headers, status=401)
    #     self.assertEqual(res.json['status'], 'new-users-disabled')
    #     # But existing users are still allowed.
    #     assertion = self._getassertion(email="newuser1@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     self.app.get('/1.0/sync/1.1', headers=headers, status=200)
    #     assertion = self._getassertion(email="newuser2@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     self.app.get('/1.0/sync/1.1', headers=headers, status=200)

    # TODO can't really test metrics logging
    # def test_metrics_uid_logging(self):
    #     assert "fxa.metrics_uid_secret_key" in self.config.registry.settings
    #     assertion = self._getassertion(email="newuser2@test.com")
    #     headers = {'Authorization': 'BrowserID %s' % assertion}
    #     self.app.get('/1.0/sync/1.5', headers=headers, status=200)
    #     self.assertMetricWasLogged('uid')
    #     self.assertMetricWasLogged('uid.first_seen_at')
    #     self.assertMetricWasLogged('metrics_uid')
    #     self.assertMetricWasLogged('metrics_device_id')

    def test_uid_and_kid_from_oauth_token(self):
        uid = TestService.session.email.split('@')[0]
        headers_oauth = {
            "Authorization": "Bearer %s" % TestService.oauth_token,
            "X-KeyID": "12-YWFh",
        }
        res = self.app.get("/1.0/sync/1.5", headers=headers_oauth)
        token = self.unsafelyParseToken(res.json["id"])
        self.assertEqual(token["uid"], res.json["uid"])
        self.assertEqual(token["fxa_uid"], uid)
        self.assertEqual(token["fxa_kid"], "0000000000012-YWFh")
        self.assertNotEqual(token["hashed_fxa_uid"], token["fxa_uid"])
        self.assertEqual(token["hashed_fxa_uid"], res.json["hashed_fxa_uid"])
        self.assertIn("hashed_device_id", token)

    def test_metrics_uid_is_returned_in_response(self):
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=200)
        self.assertTrue('hashed_fxa_uid' in res.json)

    def test_node_type_is_returned_in_response(self):
        headers = {'Authorization': 'Bearer %s' % TestService.oauth_token}
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=200)
        self.assertEqual(res.json['node_type'], 'example')
