from fxa.oauth import Client
from fxa.errors import ClientError, TrustError
import json

DEFAULT_OAUTH_SCOPE = 'https://identity.mozilla.com/apps/oldsync'


class FxaOAuthClient:
    def __init__(self, server_url=None, jwks=None):
        self._client = Client(server_url=server_url, jwks=jwks)

    def verify_token(self, token):
        try:
            token_data = self._client.verify_token(token, DEFAULT_OAUTH_SCOPE)

            # Serialize the data to make it easier to parse in Rust
            return json.dumps(token_data)
        except (ClientError, TrustError):
            return None
