from base64 import urlsafe_b64encode as b64encode
import binascii
import jwt
import os

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from locust import HttpUser, task, between

DEFAULT_OAUTH_SCOPE = "https://identity.mozilla.com/apps/oldsync"

# To create an invalid token, we sign the JWT with a private key that doesn't
# correspond with the public key set on Tokenserver. To accomplish this, we
# just generate a new private key with every run of the load tests.
INVALID_OAUTH_PRIVATE_KEY = rsa.generate_private_key(
    public_exponent=65537,
    key_size=2048,
)

# We use a custom mockmyid site to synthesize valid assertions.
# It's hosted in a static S3 bucket so we don't swamp the live mockmyid server.
MOCKMYID_DOMAIN = "mockmyid.s3-us-west-2.amazonaws.com"
ONE_YEAR = 60 * 60 * 24 * 365
TOKENSERVER_PATH = "/1.0/sync/1.5"

# This is a private key used to "forge" valid tokens. The associated public
# key must be set using the SYNC_TOKENSERVER__FXA_PRIMARY_JWK_* environment
# variables on Tokenserver.
VALID_OAUTH_PRIVATE_KEY = private_key = serialization.load_pem_private_key(
    open(os.environ["OAUTH_PEM_FILE"], "rb").read(),
    password=None,
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
            self.generation_counter.to_bytes(16, "big")
        ).decode("utf8")
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
        token = self._make_oauth_token(self.email, key=INVALID_OAUTH_PRIVATE_KEY)

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
            self.generation_counter.to_bytes(16, "big")
        ).decode("utf8")
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

    @task(20)
    def test_password_change(self):
        # When a user's password changes, the generation number increases.
        self.generation_counter += 1
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

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
        body["fxa-generation"] = self.generation_counter
        body.update(fields)

        return jwt.encode(
            body, key, algorithm="RS256", headers={"typ": "application/at+jwt"}
        )

    def _make_x_key_id_header(self):
        # In practice, the generation number and keys_changed_at may not be
        # the same, but for our purposes, making this assumption is sufficient:
        # the accuracy of the load test is unaffected.
        keys_changed_at = self.generation_counter
        raw_client_state = binascii.unhexlify(self.client_state)
        client_state = b64encode(raw_client_state).strip(b"=").decode("utf-8")

        return "%s-%s" % (keys_changed_at, client_state)

    def _do_token_exchange_via_oauth(self, token, status=200):
        headers = {
            "Authorization": "Bearer %s" % token,
            "X-KeyID": self._make_x_key_id_header(),
        }

        with self.client.get(
            TOKENSERVER_PATH, catch_response=True, headers=headers
        ) as res:
            if res.status_code == status:
                res.success()
