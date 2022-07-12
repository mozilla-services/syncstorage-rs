from base64 import urlsafe_b64encode as b64encode
import binascii
import jwt
import os
import time

import browserid
import browserid.jwt
from browserid.tests.support import make_assertion
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from locust import HttpUser, task, between

BROWSERID_AUDIENCE = os.environ['BROWSERID_AUDIENCE']
DEFAULT_OAUTH_SCOPE = 'https://identity.mozilla.com/apps/oldsync'
# This key is used to sign JWTs with a private key that does NOT
# correspond with the public key set on Tokenserver.
INVALID_OAUTH_PRIVATE_KEY = rsa.generate_private_key(
    public_exponent=65537,
    key_size=2048,
)
# We use a custom mockmyid site to synthesize valid assertions.
# It's hosted in a static S3 bucket so we don't swamp the live mockmyid server.
MOCKMYID_DOMAIN = "mockmyid.s3-us-west-2.amazonaws.com"
MOCKMYID_PRIVATE_KEY = browserid.jwt.DS128Key({
    "algorithm": "DS",
    "x": "385cb3509f086e110c5e24bdd395a84b335a09ae",
    "y": "738ec929b559b604a232a9b55a5295afc368063bb9c20fac4e53a74970a4db795"
         "6d48e4c7ed523405f629b4cc83062f13029c4d615bbacb8b97f5e56f0c7ac9bc1"
         "d4e23809889fa061425c984061fca1826040c399715ce7ed385c4dd0d40225691"
         "2451e03452d3c961614eb458f188e3e8d2782916c43dbe2e571251ce38262",
    "p": "ff600483db6abfc5b45eab78594b3533d550d9f1bf2a992a7a8daa6dc34f8045a"
         "d4e6e0c429d334eeeaaefd7e23d4810be00e4cc1492cba325ba81ff2d5a5b305a"
         "8d17eb3bf4a06a349d392e00d329744a5179380344e82a18c47933438f891e22a"
         "eef812d69c8f75e326cb70ea000c3f776dfdbd604638c2ef717fc26d02e17",
    "q": "e21e04f911d1ed7991008ecaab3bf775984309c3",
    "g": "c52a4a0ff3b7e61fdf1867ce84138369a6154f4afa92966e3c827e25cfa6cf508b"
         "90e5de419e1337e07a2e9e2a3cd5dea704d175f8ebf6af397d69e110b96afb17c7"
         "a03259329e4829b0d03bbc7896b15b4ade53e130858cc34d96269aa89041f40913"
         "6c7242a38895c9d5bccad4f389af1d7a4bd1398bd072dffa896233397a",
})
ONE_YEAR = 60 * 60 * 24 * 365
TOKENSERVER_PATH = '/1.0/sync/1.5'
# This is a private key used to "forge" valid tokens. The associated public
# key must be set using the SYNC_TOKENSERVER__FXA_PRIMARY_JWK_* environment
# variables on Tokenserver.
VALID_OAUTH_PRIVATE_KEY = private_key = serialization.load_pem_private_key(
    open(os.environ['OAUTH_PEM_FILE'], "rb").read(), password=None,
)


class TokenserverTestUser(HttpUser):
    # An instance of this class represents a single Tokenserver user. Instances
    # will live for the entire duration of the load test. Based on the
    # `wait_time` class variable and the `@task` decorators, each user will
    # make sporadic requests to the server under test.

    wait_time = between(1, 5)

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Keep track of this user's generation number.
        self.generation_counter = 0
        self.client_state = binascii.hexlify(
            self.generation_counter.to_bytes(16, 'big')).decode('utf8')
        # Locust spawns a new instance of this class for each user. Using the
        # object ID as the FxA UID guarantees uniqueness.
        self.fxa_uid = id(self)
        self.email = "loadtest-%s@%s" % (self.fxa_uid, MOCKMYID_DOMAIN)

    @task(3000)
    def test_oauth_success(self):
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

    @task(100)
    def test_invalid_oauth(self):
        token = self._make_oauth_token(
            self.email,
            key=INVALID_OAUTH_PRIVATE_KEY
        )

        self._do_token_exchange_via_oauth(token, status=401)

    @task(100)
    def test_invalid_oauth_scope(self):
        token = self._make_oauth_token(
            self.email,
            scope="unrelated scopes",
        )

        self._do_token_exchange_via_oauth(token, status=401)

    @task(20)
    def test_encryption_key_change(self):
        # When a user's encryption keys change, the generation number and
        # keys_changed_at for the user both increase.
        self.generation_counter += 1
        self.client_state = binascii.hexlify(
            self.generation_counter.to_bytes(16, 'big')).decode('utf8')
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

    @task(20)
    def test_password_change(self):
        # When a user's password changes, the generation number increases.
        self.generation_counter += 1
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

    @task(100)
    def test_browserid_success(self):
        assertion = self._make_browserid_assertion(self.email)

        self._do_token_exchange_via_browserid(assertion)

    @task(3)
    def test_expired_browserid_assertion(self):
        assertion = self._make_browserid_assertion(
            self.email,
            exp=int(time.time() - ONE_YEAR) * 1000
        )

        self._do_token_exchange_via_browserid(assertion, status=401)

    @task(3)
    def test_browserid_email_issuer_mismatch(self):
        email = "loadtest-%s@%s" % (self.fxa_uid, "hotmail.com")
        assertion = self._make_browserid_assertion(email)

        self._do_token_exchange_via_browserid(assertion, status=401)

    @task(3)
    def test_browserid_invalid_audience(self):
        assertion = self._make_browserid_assertion(
            self.email,
            audience="http://123done.org"
        )

        self._do_token_exchange_via_browserid(assertion, status=401)

    @task(3)
    def test_browserid_invalid_issuer_priv_key(self):
        assertion = self._make_browserid_assertion(
            self.email,
            issuer="api.accounts.firefox.com"
        )

        self._do_token_exchange_via_browserid(assertion, status=401)

    def _make_oauth_token(self, email, key=VALID_OAUTH_PRIVATE_KEY, **fields):
        # For mock oauth tokens, we bundle the desired status code
        # and response body into a JSON blob for the mock verifier
        # to echo back to us.
        body = {}
        if "scope" not in fields:
            fields["scope"] = DEFAULT_OAUTH_SCOPE
        if "client_id" not in fields:
            fields["client_id"] = "x"
        sub, issuer = email.split("@", 1)
        body["sub"] = sub
        body["issuer"] = issuer
        body['fxa-generation'] = self.generation_counter
        body.update(fields)

        return jwt.encode(
            body,
            key,
            algorithm="RS256",
            headers={'typ': 'application/at+jwt'}
        )

    def _make_x_key_id_header(self):
        # In practice, the generation number and keys_changed_at may not be
        # the same, but for our purposes, making this assumption is sufficient:
        # the accuracy of the load test is unaffected.
        keys_changed_at = self.generation_counter
        raw_client_state = binascii.unhexlify(self.client_state)
        client_state = b64encode(raw_client_state).strip(b'=').decode('utf-8')

        return '%s-%s' % (keys_changed_at, client_state)

    def _make_browserid_assertion(self, email, **kwds):
        if "audience" not in kwds:
            kwds["audience"] = BROWSERID_AUDIENCE
        if "exp" not in kwds:
            kwds["exp"] = int((time.time() + ONE_YEAR) * 1000)
        if "issuer" not in kwds:
            kwds["issuer"] = MOCKMYID_DOMAIN
        if "issuer_keypair" not in kwds:
            kwds["issuer_keypair"] = (None, MOCKMYID_PRIVATE_KEY)
        kwds["idp_claims"] = {
            'fxa-generation': self.generation_counter,
            'fxa-keysChangedAt': self.generation_counter,
        }
        return make_assertion(email, **kwds)

    def _do_token_exchange_via_oauth(self, token, status=200):
        headers = {
            'Authorization': 'Bearer %s' % token,
            'X-KeyID': self._make_x_key_id_header(),
        }

        with self.client.get(TOKENSERVER_PATH,
                             catch_response=True,
                             headers=headers) as res:
            if res.status_code == status:
                res.success()

    def _do_token_exchange_via_browserid(self, assertion, status=200):
        headers = {
            'Authorization': 'BrowserID %s' % assertion,
            'X-Client-State': self.client_state
        }

        with self.client.get(TOKENSERVER_PATH,
                             catch_response=True,
                             headers=headers) as res:
            if res.status_code == status:
                res.success()
