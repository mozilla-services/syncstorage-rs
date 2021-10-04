# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import unittest
from tokenserver.test_support import TestCase


class TestAuthorization(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestAuthorization, self).setUp()

    def tearDown(self):
        super(TestAuthorization, self).tearDown()

    def test_unauthorized_error_status(self):
        # Totally busted auth -> generic error.
        headers = {'Authorization': 'Unsupported-Auth-Scheme IHACKYOU'}
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)

        expected_error_response = {
            'errors': [
                {
                    'description': 'Unsupported',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'error'
        }
        self.assertEqual(res.json, expected_error_response)

    def test_no_auth(self):
        self.app.get('/1.0/sync/1.5', status=401)

    def test_invalid_client_state(self):
        headers = {'X-KeyID': '1234-state!'}
        resp = self.app.get('/1.0/sync/1.5', headers=headers, status=401)

        expected_error_response = {
            'status': 'error',
            'errors': [
                {
                    'location': 'body',
                    'name': '',
                    'description': 'Unauthorized'
                }
            ]
        }
        self.assertEqual(resp.json, expected_error_response)

    def test_keys_changed_at_less_than_equal_to_generation(self):
        self._add_user(generation=1232, keys_changed_at=1234)
        # If keys_changed_at changes, that change must be less than or equal
        # the new generation
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1236-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-keysChangedAt',
            'errors': [
                {
                    'location': 'body',
                    'name': '',
                    'description': 'Unauthorized'
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)
        # If the keys_changed_at on the request matches that currently stored
        # on the user record, it does not need to be less than or equal to the
        # generation on the request
        oauth_token = self._forge_oauth_token(generation=1233)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)
        # A request with no generation is acceptable
        oauth_token = self._forge_oauth_token(generation=None)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YWFh'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)
        # A request with a keys_changed_at less than the new generation
        # is acceptable
        oauth_token = self._forge_oauth_token(generation=1236)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YWFh'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)

    def test_disallow_reusing_old_client_state(self):
        # Add a user record that has already been replaced
        self._add_user(client_state='616161', replaced_at=1200)
        # Add the most up-to-date user record
        self._add_user(client_state='626262')
        # A request cannot use a client state associated with a replaced user
        oauth_token = self._forge_oauth_token()
        # (Note that YWFh is base64 for 'aaa', which is 0x616161 in hex)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value stale '
                                   'value'
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)
        # Using the last-seen client state is okay
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YmJi'
        }
        res1 = self.app.get('/1.0/sync/1.5', headers=headers)
        # Using a new client state (with an updated generation and
        # keys_changed_at) is okay
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-Y2Nj'
        }
        res2 = self.app.get('/1.0/sync/1.5', headers=headers)
        # This results in the creation of a new user record
        self.assertNotEqual(res1.json['uid'], res2.json['uid'])

    def test_generation_change_must_accompany_client_state_change(self):
        self._add_user(generation=1234, client_state='616161')
        # A request with a new client state must also contain a new generation
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YmJi'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value new '
                                   'value with no generation change'
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)
        # A request with no generation is acceptable
        oauth_token = self._forge_oauth_token(generation=None)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YmJi'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)
        # We can't use a generation of 1235 when setting a new client state
        # because the generation was set to be equal to the keys_changed_at
        # in the previous request, which was 1235
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-Y2Nj'
        }
        expected_error_response = {
           'status': 'invalid-client-state',
           'errors': [
               {
                   'location': 'header',
                   'name': 'X-Client-State',
                   'description': 'Unacceptable client-state value new '
                                  'value with no generation change'
               }
           ]
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)
        # A change in client state is acceptable only with a change in
        # generation (if it is present)
        oauth_token = self._forge_oauth_token(generation=1236)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1236-Y2Nj'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)

    def test_keys_changed_at_change_must_accompany_client_state_change(self):
        self._add_user(generation=1234, keys_changed_at=1234,
                       client_state='616161')
        # A request with a new client state must also contain a new
        # keys_changed_at
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YmJi'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value new '
                                   'value with no keys_changed_at change'
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)
        # A request with a new keys_changed_at is acceptable
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YmJi'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)

    def test_generation_must_not_be_less_than_last_seen_value(self):
        uid = self._add_user(generation=1234)
        # The generation in the request cannot be less than the generation
        # currently stored on the user record
        oauth_token = self._forge_oauth_token(generation=1233)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-generation',
            'errors': [
                {
                    'location': 'body',
                    'name': '',
                    'description': 'Unauthorized',
                }
            ]
        }
        self.assertEqual(res.json, expected_error_response)
        # A request with no generation is acceptable
        oauth_token = self._forge_oauth_token(generation=None)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)
        # A request with a generation equal to the last-seen generation is
        # acceptable
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        self.app.get('/1.0/sync/1.5', headers=headers)
        # A request with a generation greater than the last-seen generation is
        # acceptable
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # This should not result in the creation of a new user
        self.assertEqual(res.json['uid'], uid)

    def test_fxa_kid_change(self):
        self._add_user(generation=1234, keys_changed_at=None,
                       client_state='616161')
        # An OAuth client shows up, setting keys_changed_at.
        # (The value matches generation number above, beause in this scenario
        # FxA hasn't been updated to track and report keysChangedAt yet).
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh',
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        token0 = self.unsafelyParseToken(res.json['id'])
        # Reject keys_changed_at lower than the value previously seen
        headers['X-KeyID'] = '1233-YWFh'
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
        # Reject greater keys_changed_at with no corresponding update to
        # generation
        headers['X-KeyID'] = '2345-YmJi'
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)
        # Accept equal keys_changed_at
        headers['X-KeyID'] = '1234-YWFh'
        self.app.get('/1.0/sync/1.5', headers=headers)
        # Accept greater keys_changed_at with new generation
        headers['X-KeyID'] = '2345-YmJi'
        oauth_token = self._forge_oauth_token(generation=2345)
        headers['Authorization'] = 'Bearer %s' % oauth_token
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        token = self.unsafelyParseToken(res.json['id'])
        self.assertEqual(token['fxa_kid'], '0000000002345-YmJi')
        self.assertNotEqual(token['uid'], token0['uid'])
        self.assertEqual(token['node'], token0['node'])

    def test_client_specified_duration(self):
        self._add_user(generation=1234, keys_changed_at=1234,
                       client_state='616161')
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh',
        }
        # It's ok to request a shorter-duration token.
        res = self.app.get('/1.0/sync/1.5?duration=12', headers=headers)
        self.assertEquals(res.json['duration'], 12)
        # But you can't exceed the server's default value.
        res = self.app.get('/1.0/sync/1.5?duration=4000', headers=headers)
        self.assertEquals(res.json['duration'], 300)
        # And nonsense values are ignored.
        res = self.app.get('/1.0/sync/1.5?duration=lolwut', headers=headers)
        self.assertEquals(res.json['duration'], 300)
        res = self.app.get('/1.0/sync/1.5?duration=-1', headers=headers)
        self.assertEquals(res.json['duration'], 300)

    # Although all servers are now writing keys_changed_at, we still need this
    # case to be handled. See this PR for more information:
    # https://github.com/mozilla-services/tokenserver/pull/176
    def test_kid_change_during_gradual_tokenserver_rollout(self):
        # Let's start with a user already in the db, with no keys_changed_at.
        uid = self._add_user(generation=1234, client_state='616161',
                             keys_changed_at=None)
        user1 = self._get_user(uid)
        # User hits updated tokenserver node, writing keys_changed_at to db.
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1200-YWFh',
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # That should not have triggered a node re-assignment.
        user2 = self._get_user(res.json['uid'])
        self.assertEqual(user1['uid'], user2['uid'])
        self.assertEqual(user1['nodeid'], user2['nodeid'])
        # That should have written keys_changed_at into the db.
        self.assertEqual(user2['generation'], 1234)
        self.assertEqual(user2['keys_changed_at'], 1200)
        # User does a password reset on their Firefox Account.
        oauth_token = self._forge_oauth_token(generation=2345)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '2345-YmJi',
        }
        # They sync again, but hit a tokenserver node that isn't updated yet.
        # This would trigger the allocation of a new user, so we simulate this
        # by adding a new user. We set keys_changed_at to be the last-used
        # value, since we are simulating a server that doesn't pay attention
        # to keys_changed_at.
        uid = self._add_user(generation=2345, keys_changed_at=1200, client_state='626262')
        user2 = self._get_user(uid)
        self.assertNotEqual(user1['uid'], user2['uid'])
        self.assertEqual(user1['nodeid'], user2['nodeid'])
        # They sync again, hitting an updated tokenserver node.
        # This should succeed, despite keys_changed_at appearing to have
        # changed without any corresponding change in generation number.
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # That should not have triggered a second user allocation.
        user1 = user2
        user2 = self._get_user(res.json['uid'])
        self.assertEqual(user2['uid'], user1['uid'])
        self.assertEqual(user2['nodeid'], user1['nodeid'])

    def test_update_client_state(self):
        uid = self._add_user(generation=0, keys_changed_at=None,
                             client_state='')
        user1 = self._get_user(uid)
        # The user starts out with no client_state
        self.assertEqual(user1['generation'], 0)
        self.assertEqual(user1['client_state'], '')
        seen_uids = set((uid,))
        orig_node = user1['nodeid']
        # Changing client_state allocates a new user, resulting in a new uid
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YmJi'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        user2 = self._get_user(res.json['uid'])
        self.assertTrue(user2['uid'] not in seen_uids)
        self.assertEqual(user2['nodeid'], orig_node)
        self.assertEqual(user2['generation'], 1234)
        self.assertEqual(user2['keys_changed_at'], 1234)
        self.assertEqual(user2['client_state'], '626262')
        seen_uids.add(user2['uid'])
        # We can change the client state even if no generation is present on
        # the request
        oauth_token = self._forge_oauth_token(generation=None)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-Y2Nj'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        user3 = self._get_user(res.json['uid'])
        self.assertTrue(user3['uid'] not in seen_uids)
        self.assertEqual(user3['nodeid'], orig_node)
        # When keys_changed_at changes and generation is not present on the
        # request, generation is set to be the same as keys_changed_at
        self.assertEqual(user3['generation'], 1235)
        self.assertEqual(user3['keys_changed_at'], 1235)
        self.assertEqual(user3['client_state'], '636363')
        seen_uids.add(user3['uid'])
        # We cannot change client_state without a change in keys_changed_at
        oauth_token = self._forge_oauth_token(generation=None)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-ZGRk'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value new '
                                   'value with no keys_changed_at change'
                }
            ]
        }
        self.assertEqual(expected_error_response, res.json)
        # We cannot use a previously-used client_state
        oauth_token = self._forge_oauth_token(generation=1236)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1236-YmJi'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'status': 'invalid-client-state',
            'errors': [
                {
                    'location': 'header',
                    'name': 'X-Client-State',
                    'description': 'Unacceptable client-state value stale '
                                   'value'
                }
            ]
        }
        self.assertEqual(expected_error_response, res.json)

    def test_set_generation_from_no_generation(self):
        # Add a user that has no generation set
        uid = self._add_user(generation=0, keys_changed_at=None,
                             client_state='616161')
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        # Send a request to set the generation
        self.app.get('/1.0/sync/1.5', headers=headers)
        user = self._get_user(uid)
        # Ensure that the user had the correct generation set
        self.assertEqual(user['generation'], 1234)

    def test_set_keys_changed_at_from_no_keys_changed_at(self):
        # Add a user that has no keys_changed_at set
        uid = self._add_user(generation=1234, keys_changed_at=None,
                             client_state='616161')
        oauth_token = self._forge_oauth_token(generation=1234)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        # Send a request to set the keys_changed_at
        self.app.get('/1.0/sync/1.5', headers=headers)
        user = self._get_user(uid)
        # Ensure that the user had the correct generation set
        self.assertEqual(user['keys_changed_at'], 1234)

    def test_x_client_state_must_have_same_client_state_as_key_id(self):
        self._add_user(client_state='616161')
        headers = {
            'Authorization': 'Bearer %s' % self._forge_oauth_token(),
            'X-KeyID': '1234-YWFh',
            'X-Client-State': '626262'
        }
        # If present, the X-Client-State header must have the same client
        # state as the X-KeyID header
        res = self.app.get('/1.0/sync/1.5', headers=headers, status=401)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unauthorized',
                    'location': 'body',
                    'name': ''
                }
            ],
            'status': 'invalid-client-state'
        }
        self.assertEqual(res.json, expected_error_response)
        headers['X-Client-State'] = '616161'
        res = self.app.get('/1.0/sync/1.5', headers=headers)
