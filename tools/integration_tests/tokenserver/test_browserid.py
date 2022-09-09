# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import json
import unittest
from tokenserver.test_support import TestCase


class TestBrowserId(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestBrowserId, self).setUp()

    def tearDown(self):
        super(TestBrowserId, self).tearDown()

    def _build_browserid_fxa_error_response(self, reason, status=200):
        body = {
            'body': {
                'status': 'failure'
            },
            'status': status
        }

        if reason:
            body['body']['reason'] = reason

        return {
            'Authorization': 'BrowserID %s' % json.dumps(body),
            'X-Client-State': 'aaaa'
        }

    def test_fxa_returns_status_not_ok(self):
        expected_error_response = {
            'status': 'error',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Resource is not available',
                    'name': ''
                }
            ]
        }
        # If FxA returns any status code other than 200, the client gets a 503
        headers = self._build_browserid_headers(client_state='aaaa',
                                                status=500)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

        headers = self._build_browserid_headers(client_state='aaaa',
                                                status=404)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

        headers = self._build_browserid_headers(client_state='aaaa',
                                                status=401)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

        headers = self._build_browserid_headers(client_state='aaaa',
                                                status=201)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

    def test_fxa_returns_invalid_response(self):
        # Craft a response that contains invalid JSON
        token = json.dumps({
            'body': {'test': True},
            'status': 200,
        }).replace('true', '')
        headers = {
            'Authorization': 'BrowserID %s' % token,
            'X-Client-State': 'aaaa'
        }
        expected_error_response = {
            'status': 'error',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Resource is not available',
                    'name': ''
                }
            ]
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

    def test_expired_token(self):
        expected_error_response = {
            'status': 'invalid-timestamp',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Unauthorized',
                    'name': ''
                }
            ]
        }
        # If the FxA response includes "expired" in the reason message,
        # the client gets a 401 and a message indicating an invalid timestamp
        headers = self._build_browserid_fxa_error_response('assertion expired')
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)
        # If the FxA response includes "issued later than" in the reason
        # message, the client gets a 401 and a message indicating an invalid
        # timestamp
        headers = self._build_browserid_fxa_error_response('issued later than')
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_other_reason_message(self):
        expected_error_response = {
            'status': 'invalid-credentials',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Unauthorized',
                    'name': ''
                }
            ]
        }
        # If the FxA response includes a reason that doesn't indicate an
        # invalid timestamp, a generic error is returned
        headers = self._build_browserid_fxa_error_response('invalid')
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_missing_reason_message(self):
        expected_error_response = {
            'status': 'invalid-credentials',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Unauthorized',
                    'name': ''
                }
            ]
        }
        # If the FxA response includes no reason, a generic error is returned
        headers = self._build_browserid_fxa_error_response(None)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_issuer_mismatch(self):
        expected_error_response = {
            'status': 'invalid-credentials',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Unauthorized',
                    'name': ''
                }
            ]
        }
        # If the issuer in the response doesn't match the issuer on
        # Tokenserver, a 401 is returned
        invalid_issuer = 'invalid.com'
        headers = self._build_browserid_headers(client_state='aaaa',
                                                issuer=invalid_issuer)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_fxa_error_response_not_ok(self):
        expected_error_response = {
            'status': 'error',
            'errors': [
                {
                    'location': 'body',
                    'description': 'Resource is not available',
                    'name': ''
                }
            ]
        }
        # If an FxA error response returns a status other than 200, the client
        # gets a 503 error
        headers = self._build_browserid_fxa_error_response('bad token',
                                                           status=401)
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=503)
        self.assertEqual(res.json, expected_error_response)

    def test_no_idp_claims(self):
        # A response from FxA that does not include idpClaims is still valid
        headers = self._build_browserid_headers(client_state='aaaa')
        self.app.get('/1.0/sync/1.5', headers=headers, status=200)

    def test_partial_idp_claims(self):
        # A response from FxA that includes a partially-filled idpClaims
        # object is still valid
        headers = self._build_browserid_headers(user='test1',
                                                client_state='aaaa',
                                                generation=1234)
        self.app.get('/1.0/sync/1.5', headers=headers, status=200)

        headers = self._build_browserid_headers(user='test2',
                                                client_state='aaaa',
                                                keys_changed_at=1234)
        self.app.get('/1.0/sync/1.5', headers=headers, status=200)

        headers = self._build_browserid_headers(user='test3',
                                                client_state='aaaa',
                                                device_id='id')
        self.app.get('/1.0/sync/1.5', headers=headers, status=200)

    def test_unverified_token(self):
        headers = self._build_browserid_headers(client_state='aaaa',
                                                token_verified=None)
        # Assertion should not be rejected if fxa-tokenVerified is unset
        self.app.get("/1.0/sync/1.5", headers=headers, status=200)
        # Assertion should not be rejected if fxa-tokenVerified is true
        headers = self._build_browserid_headers(client_state='aaaa',
                                                token_verified=True)
        self.app.get("/1.0/sync/1.5", headers=headers, status=200)
        # Assertion should be rejected if fxa-tokenVerified is false
        headers = self._build_browserid_headers(client_state='aaaa',
                                                token_verified=False)
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-credentials'
        }
        self.assertEqual(res.json, expected_error_response)
        # Assertion should be rejected if fxa-tokenVerified is null
        headers['Authorization'] = headers['Authorization'].replace('false',
                                                                    'null')
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_credentials_from_oauth_and_browserid(self):
        # Send initial credentials via oauth.
        oauth_headers = self._build_oauth_headers(generation=1234,
                                                  keys_changed_at=1234,
                                                  client_state='aaaa')
        res1 = self.app.get("/1.0/sync/1.5", headers=oauth_headers)
        # Send the same credentials via BrowserID
        browserid_headers = self._build_browserid_headers(generation=1234,
                                                          keys_changed_at=1234,
                                                          client_state='aaaa')
        res2 = self.app.get("/1.0/sync/1.5", headers=browserid_headers)
        # They should get the same node assignment.
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res2.json["api_endpoint"])
        # Earlier generation number via BrowserID -> invalid-generation
        browserid_headers = self._build_browserid_headers(generation=1233,
                                                          keys_changed_at=1234,
                                                          client_state='aaaa')
        res = self.app.get("/1.0/sync/1.5", headers=browserid_headers,
                           status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-generation'
        }
        self.assertEqual(res.json, expected_error_response)
        # Earlier keys_changed_at via BrowserID is not accepted.
        browserid_headers = self._build_browserid_headers(generation=1234,
                                                          keys_changed_at=1233,
                                                          client_state='aaaa')
        res = self.app.get("/1.0/sync/1.5", headers=browserid_headers,
                           status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-keysChangedAt'
        }
        self.assertEqual(res.json, expected_error_response)
        # Earlier generation number via OAuth -> invalid-generation
        oauth_headers = self._build_oauth_headers(generation=1233,
                                                  keys_changed_at=1234,
                                                  client_state='aaaa')
        res = self.app.get("/1.0/sync/1.5", headers=oauth_headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-generation'
        }
        self.assertEqual(res.json, expected_error_response)
        # Earlier keys_changed_at via OAuth is not accepted.
        oauth_headers = self._build_oauth_headers(generation=1234,
                                                  keys_changed_at=1233,
                                                  client_state='aaaa')
        res = self.app.get("/1.0/sync/1.5", headers=oauth_headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-keysChangedAt'
        }
        self.assertEqual(res.json, expected_error_response)
        # Change client-state via BrowserID.
        browserid_headers = self._build_browserid_headers(generation=1235,
                                                          keys_changed_at=1235,
                                                          client_state='bbbb')
        res1 = self.app.get("/1.0/sync/1.5", headers=browserid_headers)
        # Previous OAuth creds are rejected due to keys_changed_at update.
        oauth_headers = self._build_oauth_headers(generation=1235,
                                                  keys_changed_at=1234,
                                                  client_state='bbbb')
        res = self.app.get("/1.0/sync/1.5", headers=oauth_headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-keysChangedAt'
        }
        self.assertEqual(res.json, expected_error_response)
        # Previous OAuth creds are rejected due to generation update.
        oauth_headers = self._build_oauth_headers(generation=1234,
                                                  keys_changed_at=1235,
                                                  client_state='bbbb')
        res = self.app.get("/1.0/sync/1.5", headers=oauth_headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-generation'
        }
        self.assertEqual(res.json, expected_error_response)
        # Updated OAuth creds are accepted.
        oauth_headers = self._build_oauth_headers(generation=1235,
                                                  keys_changed_at=1235,
                                                  client_state='bbbb')
        res2 = self.app.get("/1.0/sync/1.5", headers=oauth_headers)
        # They should again get the same node assignment.
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"],
                         res2.json["api_endpoint"])

    def test_null_idp_claims(self):
        headers = self._build_browserid_headers(generation=1234,
                                                client_state='aaaa')
        headers['Authorization'] = headers['Authorization'].replace('1234',
                                                                    'null')
        # A null fxa-generation claim results in a 401
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-generation'
        }
        self.assertEqual(res.json, expected_error_response)
        # A null fxa-keysChangedAt claim results in a 401
        headers = self._build_browserid_headers(keys_changed_at=1234,
                                                client_state='aaaa')
        headers['Authorization'] = headers['Authorization'].replace('1234',
                                                                    'null')
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'invalid keysChangedAt',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-credentials'
        }
        self.assertEqual(res.json, expected_error_response)
        # A null fxa-tokenVerified claim results in a 401
        headers = self._build_browserid_headers(token_verified=True,
                                                client_state='aaaa')
        headers['Authorization'] = headers['Authorization'].replace('true',
                                                                    'null')
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-credentials'
        }
        self.assertEqual(res.json, expected_error_response)
        headers = self._build_browserid_headers(device_id="device id",
                                                client_state='aaaa')
        headers['Authorization'] = \
            headers['Authorization'].replace('"device id"', 'null')
        # A null fxa-deviceId claim is acceptable
        self.app.get("/1.0/sync/1.5", headers=headers)

    def test_uid_and_kid(self):
        browserid_headers = self._build_browserid_headers(user='testuser',
                                                          generation=1234,
                                                          keys_changed_at=1233,
                                                          client_state='aaaa')
        res = self.app.get("/1.0/sync/1.5", headers=browserid_headers)
        token = self.unsafelyParseToken(res.json["id"])
        self.assertEqual(token["uid"], res.json["uid"])
        self.assertEqual(token["fxa_uid"], "testuser")
        self.assertEqual(token["fxa_kid"], "0000000001233-qqo")
        self.assertNotEqual(token["hashed_fxa_uid"], token["fxa_uid"])
        self.assertEqual(token["hashed_fxa_uid"], res.json["hashed_fxa_uid"])
        self.assertIn("hashed_device_id", token)

    def test_generation_number_change(self):
        headers = self._build_browserid_headers(client_state="aaaa")
        # Start with no generation number.
        res1 = self.app.get("/1.0/sync/1.5", headers=headers)
        # Now send an explicit generation number.
        # The node assignment should not change.
        headers = self._build_browserid_headers(generation=1234,
                                                client_state="aaaa")
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res2.json["api_endpoint"])
        # Clients that don't report generation number are still allowed.
        headers = self._build_browserid_headers(client_state="aaaa")
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        headers = self._build_browserid_headers(device_id="nonsense",
                                                client_state="aaaa")
        headers['Authorization'] = \
            headers['Authorization'].replace("fxa-deviceId", "nonsense")
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        # But previous generation numbers get an invalid-generation response.
        headers = self._build_browserid_headers(generation=1233,
                                                client_state="aaaa")
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        self.assertEqual(res.json["status"], "invalid-generation")
        # Equal generation numbers are accepted.
        headers = self._build_browserid_headers(generation=1234,
                                                client_state="aaaa")
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res2.json["api_endpoint"])
        # Later generation numbers are accepted.
        # Again, the node assignment should not change.
        headers = self._build_browserid_headers(generation=1235,
                                                client_state="aaaa")
        res2 = self.app.get("/1.0/sync/1.5", headers=headers)
        self.assertEqual(res1.json["uid"], res2.json["uid"])
        self.assertEqual(res1.json["api_endpoint"], res2.json["api_endpoint"])
        # And that should lock out the previous generation number
        headers = self._build_browserid_headers(generation=1234,
                                                client_state="aaaa")
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        self.assertEqual(res.json["status"], "invalid-generation")

    def test_reverting_to_no_keys_changed_at(self):
        # Add a user that has no keys_changed_at set
        uid = self._add_user(generation=0, keys_changed_at=None,
                             client_state='aaaa')
        # Send a request with keys_changed_at
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=1234,
                                           client_state='aaaa')
        self.app.get('/1.0/sync/1.5', headers=headers)
        user = self._get_user(uid)
        # Confirm that keys_changed_at was set
        self.assertEqual(user['keys_changed_at'], 1234)
        # Send a request with no keys_changed_at
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=None,
                                           client_state='aaaa')
        # Once a keys_changed_at has been set, the server expects to receive
        # it from that point onwards
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-keysChangedAt',
            'errors': [
                {
                    'location': 'body',
                    'name': '',
                    'description': 'Unauthorized',
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)

    def test_zero_keys_changed_at_treated_as_null(self):
        # Add a user that has a zero keys_changed_at
        uid = self._add_user(generation=0, keys_changed_at=0,
                             client_state='aaaa')
        # Send a request with no keys_changed_at
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=None,
                                           client_state='aaaa')
        self.app.get('/1.0/sync/1.5', headers=headers)
        # The request should succeed and the keys_changed_at should be
        # unchanged
        user = self._get_user(uid)
        self.assertEqual(user['keys_changed_at'], 0)

    def test_reverting_to_no_client_state(self):
        # Add a user that has no client_state
        uid = self._add_user(generation=0, keys_changed_at=None,
                             client_state=None)
        # Send a request with no client state
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=None,
                                           client_state=None)
        # The request should succeed
        self.app.get('/1.0/sync/1.5', headers=headers)
        # Send a request that updates the client state
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=None,
                                           client_state='aaaa')
        # The request should succeed
        self.app.get('/1.0/sync/1.5', headers=headers)
        user = self._get_user(uid)
        # The client state should have been updated
        self.assertEqual(user['client_state'], 'aaaa')
        # Send another request with no client state
        headers = self._build_browserid_headers(generation=None,
                                           keys_changed_at=None,
                                           client_state=None)
        # The request should fail, since we are trying to revert to using no
        # client state after setting one
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value empty '
                                   'string',
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)