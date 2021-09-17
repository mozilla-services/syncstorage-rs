from fxa.oauth import Client
from fxa.errors import ClientError, TrustError
import json

DEFAULT_OAUTH_SCOPE = 'https://identity.mozilla.com/apps/oldsync'


def verify_token(token, server_url=None):
    client = Client(server_url=server_url)

    try:
        token_data = client.verify_token(token, DEFAULT_OAUTH_SCOPE)

        # Serialize the data to make it easier to parse in Rust
        return json.dumps(token_data)
    except (ClientError, TrustError):
        return None
