# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Support utilities for storage integration tests.

Provides auth policy classes and secrets management used by conftest.py fixtures.
"""

import time
import tokenlib


class FixedSecrets(object):
    """Use a fixed set of secrets for all nodes.
    This class provides the same API as the Secrets class, but uses a
    single list of secrets for all nodes rather than using different
    secrets for each node.
    Options:
    - **secrets**: a list of hex-encoded secrets to use for all nodes.
    """

    def __init__(self, secrets):
        if isinstance(secrets, str):
            secrets = secrets.split()
        self._secrets = secrets

    def get(self, node):
        """Return the fixed list of secrets for any node."""
        return list(self._secrets)

    def keys(self):
        """Return an empty list since all nodes use the same fixed secrets."""
        return []


class PermissiveNonceCache(object):
    """Object for not really managing a cache of used nonce values.
    This class implements the timestamp/nonce checking interface required
    by hawkauthlib, but doesn't actually check them.  Instead it just logs
    timestamps that are too far out of the timestamp window for future
    analysis.
    """

    def __init__(self, log_window=60, get_time=None):
        self.log_window = log_window
        self.get_time = get_time or time.time

    def __len__(self):
        raise NotImplementedError

    def check_nonce(self, timestamp, nonce):
        """Check if the given timestamp+nonce is fresh."""
        now = self.get_time()
        skew = now - timestamp
        if abs(skew) > self.log_window:
            print("Large timestamp skew detected: %d", skew)
        return True


class TokenServerAuthenticationPolicy:
    """Authentication policy for use with Tokenserver auth tokens.

    This class provides token-based authentication using Mozilla Tokenserver
    authentication tokens as described. See our Tokenserver docs for more information.

    For verification of token signatures, this plugin can use either a
    single fixed secret (via the argument 'secret') or a file mapping
    node hostnames to secrets (via the argument 'secrets_file').  The
    two arguments are mutually exclusive.
    """

    def __init__(self, secrets=None, **kwds):
        if not secrets:
            # Using secret=None will cause tokenlib to use a randomly-generated
            # secret.  This is useful for getting started without having to
            # twiddle any configuration files, but probably not what anyone
            # wants to use long-term.
            secrets = None
            msgs = [
                "WARNING: using a randomly-generated token secret.",
                "You probably want to set 'secret' or 'secrets_file' in "
                "the [hawkauth] section of your configuration",
            ]
            for msg in msgs:
                print("warn:", msg)
        elif isinstance(secrets, (str, list)):
            secrets = FixedSecrets(secrets)
        elif isinstance(secrets, dict):
            secrets = FixedSecrets(secrets.pop("secrets", []))
        self.secrets = secrets
        self.nonce_cache = kwds.get("nonce_cache") or PermissiveNonceCache()

    def encode_hawk_id(self, request, userid, extra=None):
        """Encode the given userid into a Hawk id and secret key.
        This method is essentially the reverse of decode_hawk_id.  It is
        not needed for consuming authentication tokens, but is very useful
        when building them for testing purposes.
        """
        node_name = self._get_node_name(request)
        # There might be multiple secrets in use, if we're in the
        # process of transitioning from one to another.  Always use
        # the last one aka the "most recent" secret.
        secret = self._get_token_secrets(node_name)[-1]
        data = {"uid": userid, "node": node_name}
        if extra is not None:
            data.update(extra)
        tokenid = tokenlib.make_token(data, secret=secret)
        key = tokenlib.get_derived_secret(tokenid, secret=secret)
        return tokenid, key

    def _get_node_name(self, request):
        """Get the canonical node name for the given request."""
        # Secrets are looked up by hostname.
        # We need to normalize some port information for this work right.
        node_name = request.host_url
        if node_name.startswith("http:") and node_name.endswith(":80"):
            node_name = node_name[:-3]
        elif node_name.startswith("https:") and node_name.endswith(":443"):
            node_name = node_name[:-4]
        return node_name + request.script_name

    def _get_token_secrets(self, node_name):
        """Get the list of possible secrets for signing tokens."""
        if self.secrets is None:
            return [None]
        return self.secrets.get(node_name)
