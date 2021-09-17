# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import unittest

from tokenserver.test_support import TestCase


class TestMisc(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestMisc, self).setUp()

    def tearDown(self):
        super(TestMisc, self).tearDown()

    def test_unknown_app(self):
        headers = {
            'Authorization': 'Bearer %s' % self._forge_oauth_token(),
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/xXx/token', headers=headers, status=404)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unsupported application',
                    'location': 'url',
                    'name': 'application'
                }
            ],
            'status': 'error'
        }
        self.assertEqual(res.json, expected_error_response)

    def test_unknown_version(self):
        headers = {
            'Authorization': 'Bearer %s' % self._forge_oauth_token(),
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.2', headers=headers, status=404)
        expected_error_response = {
            'errors': [
                {
                    'description': 'Unsupported application version',
                    'location': 'url',
                    'name': '1.2'
                }
            ],
            'status': 'error'
        }
        self.assertEqual(res.json, expected_error_response)

    def test_valid_app(self):
        self._add_user()
        headers = {
            'Authorization': 'Bearer %s' % self._forge_oauth_token(),
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        self.assertIn('https://example.com/1.5', res.json['api_endpoint'])
        self.assertIn('duration', res.json)
        self.assertEquals(res.json['duration'], 300)

    def test_current_user_is_the_most_up_to_date(self):
        # Add some users
        self._add_user(generation=1234, created_at=1234)
        self._add_user(generation=1235, created_at=1234)
        self._add_user(generation=1234, created_at=1235)
        uid = self._add_user(generation=1236, created_at=1233)
        # Users are sorted by (generation, created_at), so the fourth user
        # record is considered to be the current user
        oauth_token = self._forge_oauth_token(generation=1236)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1234-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        self.assertEqual(res.json['uid'], uid)

    def test_user_creation_when_most_current_user_is_replaced(self):
        # Add some users
        uid1 = self._add_user(generation=1234, created_at=1234)
        uid2 = self._add_user(generation=1235, created_at=1235)
        uid3 = self._add_user(generation=1236, created_at=1236,
                              replaced_at=1237)
        seen_uids = [uid1, uid2, uid3]
        # Because the current user (the one with uid3) has been replaced, a new
        # user record is created
        oauth_token = self._forge_oauth_token(generation=1237)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1237-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        self.assertNotIn(res.json['uid'], seen_uids)

    def test_old_users_marked_as_replaced_in_race_recovery(self):
        # Add some users
        uid1 = self._add_user(generation=1234, created_at=1234)
        uid2 = self._add_user(generation=1235, created_at=1235)
        uid3 = self._add_user(generation=1236, created_at=1240)
        # Make a request
        oauth_token = self._forge_oauth_token(generation=1236)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1236-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # uid3 is associated with the current user
        self.assertEqual(res.json['uid'], uid3)
        # The users associated with uid1 and uid2 have replaced_at set to be
        # equal to created_at on the current user record
        user1 = self._get_user(uid1)
        user2 = self._get_user(uid2)
        self.assertEqual(user1['replaced_at'], 1240)
        self.assertEqual(user2['replaced_at'], 1240)

    def test_user_updates_with_new_client_state(self):
        # Start with a single user in the database
        uid = self._add_user(generation=1234, keys_changed_at=1234,
                             client_state='616161')
        # Send a request, updating the generation, keys_changed_at, and
        # client_state
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YmJi'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # A new user should have been created
        self.assertEqual(self._count_users(), 2)
        self.assertNotEqual(uid, res.json['uid'])
        # The new user record should have the updated generation,
        # keys_changed_at, and client_state
        user = self._get_user(res.json['uid'])
        self.assertEqual(user['generation'], 1235)
        self.assertEqual(user['keys_changed_at'], 1235)
        self.assertEqual(user['client_state'], '626262')
        # The old user record should not have the updated values
        user = self._get_user(uid)
        self.assertEqual(user['generation'], 1234)
        self.assertEqual(user['keys_changed_at'], 1234)
        self.assertEqual(user['client_state'], '616161')
        # Get all the replaced users
        email = 'test@%s' % self.FXA_EMAIL_DOMAIN
        replaced_users = self._get_replaced_users(self.SYNC_1_5_SERVICE_ID,
                                                  email)
        # Only one user should be replaced
        self.assertEqual(len(replaced_users), 1)
        # The replaced user record should have the old generation,
        # keys_changed_at, and client_state
        replaced_user = replaced_users[0]
        self.assertEqual(replaced_user['generation'], 1234)
        self.assertEqual(replaced_user['keys_changed_at'], 1234)
        self.assertEqual(replaced_user['client_state'], '616161')

    def test_user_updates_with_same_client_state(self):
        # Start with a single user in the database
        uid = self._add_user(generation=1234, keys_changed_at=1234)
        # Send a request, updating the generation and keys_changed_at but not
        # the client state
        oauth_token = self._forge_oauth_token(generation=1235)
        headers = {
            'Authorization': 'Bearer %s' % oauth_token,
            'X-KeyID': '1235-YWFh'
        }
        res = self.app.get('/1.0/sync/1.5', headers=headers)
        # A new user should not have been created
        self.assertEqual(self._count_users(), 1)
        self.assertEqual(uid, res.json['uid'])
        # The user record should have been updated
        user = self._get_user(uid)
        self.assertEqual(user['generation'], 1235)
        self.assertEqual(user['keys_changed_at'], 1235)
