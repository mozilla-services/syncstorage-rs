import json
from base64 import urlsafe_b64encode as b64encode
from locust import HttpUser, task, between

DEFAULT_OAUTH_SCOPE = 'https://identity.mozilla.com/apps/oldsync'
MOCKMYID_DOMAIN = "mockmyid.s3-us-west-2.amazonaws.com"
TOKENSERVER_PATH = '/1.0/sync/1.5'

# An instance of this class represents a single Tokenserver user. Instances 
# will live for the entire duration of the load test. Based on the `wait_time`
# class variable and the `@task` decorators, each user will make sporadic
# requests to the server under test.
class TokenserverTestUser(HttpUser):
    wait_time = between(1, 5)

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        # Keep track of this user's generation number.
        self.generation_counter = 0
        self.x_key_id = self._make_x_key_id_header()
        # Locust spawns a new instance of this class for each user. Using the
        # object ID as the FxA UID guarantees uniqueness.
        self.fxa_uid = id(self)
        self.email = "loadtest-%s@%s" % (self.fxa_uid, MOCKMYID_DOMAIN)

    @task(1000)
    def test_success(self):
        token = self._make_oauth_token(self.email)

        self._do_token_exchange(token)

    @task(5)
    def test_invalid_scope(self):
        token = self._make_oauth_token(
            user=str(self.fxa_uid),
            scope=["unrelated", "scopes"],
        )

        self._do_token_exchange(token, status=401)

    @task(5)
    def test_invalid_token(self):
        token = self._make_oauth_token(status=400, errno=108)

        self._do_token_exchange(token, status=401)

    @task(5)
    def test_encryption_key_change(self):
        # When a user's encryption keys change, the generation number and
        # keys_changed_at for the user both increase.
        self.generation_counter += 1
        self.x_key_id = self._make_x_key_id_header()
        token = self._make_oauth_token(self.email)

        self._do_token_exchange(token)

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
        body['generation'] = self.generation_counter
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
        client_state = b64encode(
            str(keys_changed_at).encode('utf8')).strip(b'=').decode('utf-8')
        
        return '%s-%s' % (keys_changed_at, client_state)

    def _do_token_exchange(self, token, status=200):
        headers = {
            'Authorization': 'Bearer %s' % token,
            'X-KeyID': self.x_key_id,
        }

        with self.client.get(TOKENSERVER_PATH, catch_response=True, headers=headers) as res:
            if res.status_code == status:
                res.success()
