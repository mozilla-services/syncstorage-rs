# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
from base64 import urlsafe_b64decode
import hmac
import json
import jwt
import pytest
import random
import string
import time
import tokenlib
import unittest

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.backends import default_backend
from fxa.core import Client
from fxa.oauth import Client as OAuthClient
from fxa.errors import ClientError, ServerError
from fxa.tests.utils import TestEmailAccount
from hashlib import sha256

from integration_tests.tokenserver.test_support import TestCase

# This is the client ID used for Firefox Desktop. The FxA team confirmed that
# this is the proper client ID to be using for these integration tests.
CLIENT_ID = "5882386c6d801776"
DEFAULT_TOKEN_DURATION = 3600
FXA_ACCOUNT_STAGE_HOST = "https://api-accounts.stage.mozaws.net"
FXA_OAUTH_STAGE_HOST = "https://oauth.stage.mozaws.net"
PASSWORD_CHARACTERS = string.ascii_letters + string.punctuation + string.digits
PASSWORD_LENGTH = 32
SCOPE = "https://identity.mozilla.com/apps/oldsync"


@pytest.mark.usefixtures("setup_server_end_to_end_testing")
class TestE2e(TestCase, unittest.TestCase):
    def setUp(self):
        super(TestE2e, self).setUp()

    def tearDown(self):
        super(TestE2e, self).tearDown()

    @classmethod
    def setUpClass(cls):
        # Create an ephemeral email account to use to create an FxA account
        cls.acct = TestEmailAccount()
        cls.client = Client(FXA_ACCOUNT_STAGE_HOST)
        cls.oauth_client = OAuthClient(CLIENT_ID, None, server_url=FXA_OAUTH_STAGE_HOST)
        cls.fxa_password = cls._generate_password()
        # Create an FxA account for these end-to-end tests
        cls.session = cls.client.create_account(
            cls.acct.email, password=cls.fxa_password
        )
        # Loop until we receive the verification email from FxA
        while not cls.acct.messages:
            time.sleep(0.5)
            cls.acct.fetch()
        # Find the message containing the verification code and verify the
        # code
        for m in cls.acct.messages:
            if "x-verify-code" in m["headers"]:
                cls.session.verify_email_code(m["headers"]["x-verify-code"])
        # Create an OAuth token to be used for the end-to-end tests
        cls.oauth_token = cls.oauth_client.authorize_token(cls.session, SCOPE)

    @classmethod
    def tearDownClass(cls):
        cls.acct.clear()
        # A teardown of some of the tests can produce a 401 error because
        # of a race condition, where the record had already been removed.
        # This causes `destroy_account` to return an error if it attempts
        # to parse the invalid JSON response.
        # It's also possible that the `destroy_account` is rejected due to
        # missing authentication. It is not known why the authentication
        # is considered missing.
        # This traps for those events.
        try:
            cls.client.destroy_account(cls.acct.email, cls.fxa_password)
        except (ServerError, ClientError) as ex:
            print(f"warning: Encountered error when cleaning up: {ex}")

    @staticmethod
    def _generate_password():
        r = range(PASSWORD_LENGTH)

        return "".join(random.choice(PASSWORD_CHARACTERS) for i in r)

    def _get_oauth_token_with_bad_scope(self):
        bad_scope = "bad_scope"
        return self.oauth_client.authorize_token(self.session, bad_scope)

    def _get_bad_token(self):
        key = rsa.generate_private_key(
            backend=default_backend(), public_exponent=65537, key_size=2048
        )
        format = serialization.PrivateFormat.TraditionalOpenSSL
        algorithm = serialization.NoEncryption()
        pem = key.private_bytes(
            encoding=serialization.Encoding.PEM,
            format=format,
            encryption_algorithm=algorithm,
        )
        private_key = pem.decode("utf-8")
        claims = {
            "sub": "fake sub",
            "iat": 12345,
            "exp": 12345,
        }

        return jwt.encode(claims, private_key, algorithm="RS256")

    def _extract_keys_changed_at_from_assertion(self, assertion):
        token = assertion.split("~")[-2]
        claims = jwt.decode(token, options={"verify_signature": False})

        return claims["fxa-keysChangedAt"]

    @classmethod
    def _change_password(cls):
        new_password = cls._generate_password()
        cls.session.change_password(cls.fxa_password, new_password)
        cls.fxa_password = new_password

    # Adapted from the original Tokenserver:
    # https://github.com/mozilla-services/tokenserver/blob/master/tokenserver/util.py#L24
    def _fxa_metrics_hash(self, value):
        hasher = hmac.new(self.FXA_METRICS_HASH_SECRET.encode("utf-8"), b"", sha256)
        hasher.update(value.encode("utf-8"))
        return hasher.hexdigest()

    def test_unauthorized_oauth_error_status(self):
        # Totally busted auth -> generic error.
        headers = {
            "Authorization": "Unsupported-Auth-Scheme IHACKYOU",
            "X-KeyID": "1234-qqo",
        }
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            "errors": [{"description": "Unsupported", "location": "body", "name": ""}],
            "status": "error",
        }
        self.assertEqual(res.json, expected_error_response)
        token = self._get_bad_token()
        headers = {"Authorization": f"Bearer {token}", "X-KeyID": "1234-qqo"}
        # Bad token -> 'invalid-credentials'
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        expected_error_response = {
            "errors": [{"description": "Unauthorized", "location": "body", "name": ""}],
            "status": "invalid-credentials",
        }
        self.assertEqual(res.json, expected_error_response)
        # Untrusted scopes -> 'invalid-credentials'
        token = self._get_oauth_token_with_bad_scope()
        headers = {"Authorization": f"Bearer {token}", "X-KeyID": "1234-qqo"}
        res = self.app.get("/1.0/sync/1.5", headers=headers, status=401)
        self.assertEqual(res.json, expected_error_response)

    def test_valid_oauth_request(self):
        oauth_token = self.oauth_token
        headers = {"Authorization": f"Bearer {oauth_token}", "X-KeyID": "1234-qqo"}
        # Send a valid request, allocating a new user
        res = self.app.get("/1.0/sync/1.5", headers=headers)
        fxa_uid = self.session.uid
        # Retrieve the user from the database
        user = self._get_user(res.json["uid"])
        # First, let's verify that the token we received is valid. To do this,
        # we can unpack the hawk header ID into the payload and its signature
        # and then construct a tokenlib token to compute the signature
        # ourselves. To obtain a matching signature, we use the same secret as
        # is used by Tokenserver.
        raw = urlsafe_b64decode(res.json["id"])
        payload = raw[:-32]
        signature = raw[-32:]
        payload_str = payload.decode("utf-8")
        payload_dict = json.loads(payload_str)
        # The `id` payload should include a field indicating the origin of the
        # token
        self.assertEqual(payload_dict["tokenserver_origin"], "rust")
        signing_secret = self.TOKEN_SIGNING_SECRET
        tm = tokenlib.TokenManager(secret=signing_secret)
        expected_signature = tm._get_signature(payload_str.encode("utf8"))
        # Using the #compare_digest method here is not strictly necessary, as
        # this is not a security-sensitive situation, but it's good practice
        self.assertTrue(hmac.compare_digest(expected_signature, signature))
        # Check that the given key is a secret derived from the hawk ID
        expected_secret = tokenlib.get_derived_secret(
            res.json["id"], secret=signing_secret
        )
        self.assertEqual(res.json["key"], expected_secret)
        # Check to make sure the remainder of the fields are valid
        self.assertEqual(res.json["uid"], user["uid"])
        self.assertEqual(
            res.json["api_endpoint"], f"{self.NODE_URL}/1.5/{user['uid']}"
        )
        self.assertEqual(res.json["duration"], DEFAULT_TOKEN_DURATION)
        self.assertEqual(res.json["hashalg"], "sha256")
        self.assertEqual(
            res.json["hashed_fxa_uid"], self._fxa_metrics_hash(fxa_uid)[:32]
        )
        self.assertEqual(res.json["node_type"], "spanner")
        # The response should have an X-Timestamp header that contains the
        # number of seconds since the UNIX epoch
        self.assertIn("X-Timestamp", res.headers)
        self.assertIsNotNone(int(res.headers["X-Timestamp"]))
        token = self.unsafelyParseToken(res.json["id"])
        self.assertIn("hashed_device_id", token)
        self.assertEqual(token["uid"], res.json["uid"])
        self.assertEqual(token["fxa_uid"], fxa_uid)
        self.assertEqual(token["fxa_kid"], "0000000001234-qqo")
        self.assertNotEqual(token["hashed_fxa_uid"], token["fxa_uid"])
        self.assertEqual(token["hashed_fxa_uid"], res.json["hashed_fxa_uid"])
        self.assertIn("hashed_device_id", token)
