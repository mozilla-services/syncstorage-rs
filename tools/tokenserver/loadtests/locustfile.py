import binascii
import json

from base64 import urlsafe_b64encode as b64encode
from locust import HttpUser, task, between

DEFAULT_OAUTH_SCOPE = 'https://identity.mozilla.com/apps/oldsync'
FAKE_DOMAIN = "fake-domain.com"
TOKENSERVER_PATH = '/1.0/sync/1.5'


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
        self.email = "loadtest-%s@%s" % (self.fxa_uid, FAKE_DOMAIN)

    @task(3000)
    def test_oauth_success(self):
        token = self._make_oauth_token(self.email)

        self._do_token_exchange_via_oauth(token)

    @task(100)
    def test_invalid_oauth(self):
        token = self._make_oauth_token(status=400)

        self._do_token_exchange_via_oauth(token, status=401)

    @task(100)
    def test_invalid_oauth_scope(self):
        token = self._make_oauth_token(
            user=str(self.fxa_uid),
            scope=["unrelated", "scopes"],
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
    def test_invalid_browserid(self):
        assertion = self._make_browserid_assertion(status='failure')

        self._do_token_exchange_via_browserid(assertion, status=401)

    @task(3)
    def test_expired_browserid_assertion(self):
        assertion = self._make_browserid_assertion(self.email,
                                                   status='failure',
                                                   reason='expired')

        self._do_token_exchange_via_browserid(assertion, 401)

    @task(3)
    def test_browserid_issuer_mismatch(self):
        assertion = self._make_browserid_assertion(self.email,
                                                   issuer='wrong.issuer')

        self._do_token_exchange_via_browserid(assertion, 401)

    def _make_oauth_token(self, user=None, status=200, **fields):
        # For mock oauth tokens, we bundle the desired status code
        # and response body into a JSON blob for the mock verifier
        # to echo back to us.
        body = {}
        if status < 400:
            if user is None:
                raise ValueError("Must specify user for valid oauth token")
            if "scope" not in fields:
                fields["scope"] = [DEFAULT_OAUTH_SCOPE]
            if "client_id" not in fields:
                fields["client_id"] = "x"
        if user is not None:
            parts = user.split("@", 1)
            if len(parts) == 1:
                body["user"] = user
            else:
                body["user"] = parts[0]
                body["issuer"] = parts[1]
        body['fxa-generation'] = self.generation_counter
        body.update(fields)
        return json.dumps({
          "status": status,
          "body": body
        })

    def _make_x_key_id_header(self):
        # In practice, the generation number and keys_changed_at may not be
        # the same, but for our purposes, making this assumption is sufficient:
        # the accuracy of the load test is unaffected.
        keys_changed_at = self.generation_counter
        raw_client_state = binascii.unhexlify(self.client_state)
        client_state = b64encode(raw_client_state).strip(b'=').decode('utf-8')

        return '%s-%s' % (keys_changed_at, client_state)

    def _make_browserid_assertion(self, email=None, status='okay',
                                  reason='error', **fields):
        body = {'status': status}
        if email is not None:
            body["email"] = email

        if status == 'okay':
            body['idpClaims'] = {
                'fxa-generation': self.generation_counter,
                'fxa-keysChangedAt': self.generation_counter,
                'fxa-tokenVerified': fields.get('token_verified', True)
            }
            body['issuer'] = fields.get('issuer', FAKE_DOMAIN)
        else:
            body['reason'] = reason
        body.update(fields)

        return json.dumps({
          "status": 200,
          "body": body
        })

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
