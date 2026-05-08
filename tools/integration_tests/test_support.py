# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Support utilities for storage integration tests.

Provides AuthConfig, auth policy classes, and secrets management
used by conftest.py fixtures.
"""

import csv
import binascii
from collections import defaultdict
import os
import re
import time
import tokenlib


VALID_FXA_ID_REGEX = re.compile("^[A-Za-z0-9=\\-_]{1,64}$")


class AuthConfig:
    """Minimal config holder for test fixtures.

    Holds the auth policy used for hawk token signing/verification.
    """

    def __init__(self, auth_policy=None):
        self.auth_policy = auth_policy


class Secrets(object):
    """Load node-specific secrets from a file.
    This class provides a method to get a list of secrets for a node
    ordered by timestamps. The secrets are stored in a CSV file which
    is loaded when the object is created.
    Options:
    - **filename**: a list of file paths, or a single path.
    """

    def __init__(self, filename=None):
        self._secrets = defaultdict(list)
        if filename is not None:
            self.load(filename)

    def keys(self):
        """Return all node keys stored in secrets."""
        return self._secrets.keys()

    def load(self, filename):
        """Load secrets from the given filename or list of filenames."""
        if not isinstance(filename, (list, tuple)):
            filename = [filename]

        for name in filename:
            with open(name, "rb") as f:
                reader = csv.reader(f, delimiter=",")
                for line, row in enumerate(reader):
                    if len(row) < 2:
                        continue
                    node = row[0]
                    if node in self._secrets:
                        raise ValueError("Duplicate node line %d" % line)
                    secrets = []
                    for secret in row[1:]:
                        secret = secret.split(":")
                        if len(secret) != 2:
                            raise ValueError("Invalid secret line %d" % line)
                        secrets.append(tuple(secret))
                    secrets.sort()
                    self._secrets[node] = secrets

    def save(self, filename):
        """Save secrets to the given filename in CSV format."""
        with open(filename, "wb") as f:
            writer = csv.writer(f, delimiter=",")
            for node, secrets in self._secrets.items():
                secrets = [
                    "%s:%s" % (timestamp, secret) for timestamp, secret in secrets
                ]
                secrets.insert(0, node)
                writer.writerow(secrets)

    def get(self, node):
        """Return list of secrets for the given node."""
        return [secret for timestamp, secret in self._secrets[node]]

    def add(self, node, size=256):
        """Add a new randomly generated secret for the given node."""
        timestamp = str(int(time.time()))
        secret = binascii.b2a_hex(os.urandom(size))[:size]
        # The new secret *must* sort at the end of the list.
        # This forbids you from adding multiple secrets per second.
        try:
            if timestamp <= self._secrets[node][-1][0]:
                assert False, "You can only add one secret per second"
        except IndexError:
            pass
        self._secrets[node].append((timestamp, secret))


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


def get_test_configurator():
    """Build a AuthConfig with an auth policy using the configured secret.

    The secret is read from SYNC_MASTER_SECRET if set, otherwise falls back
    to the well-known local development default.
    """
    secret = os.environ.get("SYNC_MASTER_SECRET", "TED KOPPEL IS A ROBOT")
    policy = TokenServerAuthenticationPolicy(secrets=secret)
    return AuthConfig(auth_policy=policy)


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

    @classmethod
    def from_settings(cls, settings=None, prefix="hawkauth.", **extra):
        """Construct a policy instance from deployment settings.

        Extracts settings with the given prefix, strips the prefix, parses
        them via _parse_settings, and passes the result to the constructor.
        """
        if settings is None:
            settings = {}
        hawkauth_settings = {}
        for name in settings:
            if name.startswith(prefix):
                hawkauth_settings[name[len(prefix) :]] = settings[name]
        hawkauth_settings.update(extra)
        kwds = cls._parse_settings(hawkauth_settings)
        for unknown_setting in hawkauth_settings:
            raise ValueError("unknown hawkauth setting: %s" % unknown_setting)
        return cls(**kwds)

    @classmethod
    def _parse_settings(cls, settings):
        """Parse settings for an instance of this class."""
        kwds = {}
        # Consume base hawk settings that are no longer used by this standalone
        # class, but may appear in the settings dict from legacy configuration.
        for key in (
            "find_groups",
            "master_secret",
            "nonce_cache",
            "decode_hawk_id",
            "encode_hawk_id",
        ):
            val = settings.pop(key, None)
            if val is not None:
                kwds[key] = val
        # collect leftover settings into a config for a Secrets object,
        # with some b/w compat for old-style secret-handling settings.
        secrets_prefix = "secrets."
        secrets = {}
        if "secrets_file" in settings:
            if "secret" in settings:
                raise ValueError("can't use both 'secret' and 'secrets_file'")
            secrets["backend"] = "tools.integration_tests.test_support.Secrets"
            secrets["filename"] = settings.pop("secrets_file")
        elif "secret" in settings:
            secrets["backend"] = "tools.integration_tests.test_support.FixedSecrets"
            secrets["secrets"] = settings.pop("secret")
        for name in list(settings.keys()):
            if name.startswith(secrets_prefix):
                secrets[name[len(secrets_prefix) :]] = settings.pop(name)
        kwds["secrets"] = secrets
        return kwds

    def decode_hawk_id(self, request, tokenid):
        """Decode a Hawk token id into its userid and secret key.
        This method determines the appropriate secrets to use for the given
        request, then passes them on to tokenlib to handle the given Hawk
        token.
        If the id is invalid then ValueError will be raised.
        """
        # There might be multiple secrets in use, if we're in the
        # process of transitioning from one to another.  Try each
        # until we find one that works.
        node_name = self._get_node_name(request)
        secrets = self._get_token_secrets(node_name)
        for secret in secrets:
            try:
                data = tokenlib.parse_token(tokenid, secret=secret)
                userid = data["uid"]
                token_node_name = data["node"]
                if token_node_name != node_name:
                    raise ValueError("incorrect node for this token")
                key = tokenlib.get_derived_secret(tokenid, secret=secret)
                break
            except (ValueError, KeyError):
                pass
        else:
            print("warn: Authentication Failed: invalid hawk id")
            raise ValueError("invalid Hawk id")
        return userid, key

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


class SyncStorageAuthenticationPolicy(TokenServerAuthenticationPolicy):
    """Authentication policy with special handling of expired tokens.

    Extends TokenServerAuthenticationPolicy to allow limited access by holders
    of expired tokens. Presenting an expired token results in a userid of
    "expired:<uid>" rather than just "<uid>", allowing calling code to detect
    and handle this case explicitly.
    """

    def __init__(self, secrets=None, **kwds):
        self.expired_token_timeout = kwds.pop("expired_token_timeout", None)
        if self.expired_token_timeout is None:
            self.expired_token_timeout = 300
        super(SyncStorageAuthenticationPolicy, self).__init__(secrets, **kwds)

    @classmethod
    def _parse_settings(cls, settings):
        """Parse settings for an instance of this class."""
        supercls = super(SyncStorageAuthenticationPolicy, cls)
        kwds = supercls._parse_settings(settings)
        expired_token_timeout = settings.pop("expired_token_timeout", None)
        if expired_token_timeout is not None:
            kwds["expired_token_timeout"] = int(expired_token_timeout)
        return kwds

    def decode_hawk_id(self, request, tokenid):
        """Decode a Hawk token id into its userid and secret key.

        This method determines the appropriate secrets to use for the given
        request, then passes them on to tokenlib to handle the given Hawk
        token.  If the id is invalid then ValueError will be raised.

        Unlike the superclass method, this implementation allows expired
        tokens to be used up to a configurable timeout.  The effective userid
        for expired tokens is changed to be "expired:<uid>".
        """
        now = time.time()
        node_name = self._get_node_name(request)
        # There might be multiple secrets in use,
        # so try each until we find one that works.
        secrets = self._get_token_secrets(node_name)
        for secret in secrets:
            try:
                tm = tokenlib.TokenManager(secret=secret)
                # Check for a proper valid signature first.
                # If that failed because of an expired token, check if
                # it falls within the allowable expired-token window.
                try:
                    data = self._parse_token(tm, tokenid, now)
                    userid = data["uid"]
                except tokenlib.errors.ExpiredTokenError:
                    recently = now - self.expired_token_timeout
                    data = self._parse_token(tm, tokenid, recently)
                    # We replace the uid with a special string to ensure that
                    # calling code doesn't accidentally treat the token as
                    # valid. If it wants to use the expired uid, it will have
                    # to explicitly dig it back out from `request.user`.
                    data["expired_uid"] = data["uid"]
                    userid = data["uid"] = "expired:%d" % (data["uid"],)
            except tokenlib.errors.InvalidSignatureError:
                # Token signature check failed, try the next secret.
                continue
            except TypeError as e:
                # Something went wrong when validating the contained data.
                raise ValueError(str(e))
            else:
                # Token signature check succeeded, quit the loop.
                break
        else:
            # The token failed to validate using any secret.
            print("warn Authentication Failed: invalid hawk id")
            raise ValueError("invalid Hawk id")

        # Let the app access all user data from the token.
        request.user.update(data)
        request.metrics["metrics_uid"] = data.get("hashed_fxa_uid")
        request.metrics["metrics_device_id"] = data.get("hashed_device_id")

        # Sanity-check that we're on the right node.
        if data["node"] != node_name:
            msg = "incorrect node for this token: %s"
            raise ValueError(msg % (data["node"],))

        # Calculate the matching request-signing secret.
        key = tokenlib.get_derived_secret(tokenid, secret=secret)

        return userid, key

    def encode_hawk_id(self, request, userid, extra=None):
        """Encode the given userid into a Hawk id and secret key.

        This method is essentially the reverse of decode_hawk_id.  It is
        not needed for consuming authentication tokens, but is very useful
        when building them for testing purposes.

        Unlike its superclass method, this one allows the caller to specify
        a dict of additional user data to include in the auth token.
        """
        node_name = self._get_node_name(request)
        secret = self._get_token_secrets(node_name)[-1]
        data = {"uid": userid, "node": node_name}
        if extra is not None:
            data.update(extra)
        tokenid = tokenlib.make_token(data, secret=secret)
        key = tokenlib.get_derived_secret(tokenid, secret=secret)
        return tokenid, key

    def _parse_token(self, tokenmanager, tokenid, now):
        """Parse, validate and normalize user data from a tokenserver token.

        This is a thin wrapper around tokenmanager.parse_token to apply
        some extra validation to the contained user data.  The data is
        signed and trusted, but it's still coming from outside the system
        so it's good defense-in-depth to validate it at our app boundary.

        We also deal with some historical baggage by renaming fields
        as needed.
        """
        data = tokenmanager.parse_token(tokenid, now=now)
        user = {}

        # It should always contain an integer userid.
        try:
            user["uid"] = data["uid"]
        except KeyError:
            raise ValueError("missing uid in token data")
        else:
            if not isinstance(user["uid"], int) or user["uid"] < 0:
                raise ValueError("invalid uid in token data")

        # It should always contain a string node name.
        try:
            user["node"] = data["node"]
        except KeyError:
            raise ValueError("missing node in token data")
        else:
            if not isinstance(user["node"], str):
                raise ValueError("invalid node in token data")

        # It might contain additional user identifiers for
        # storage and metrics purposes.
        #
        # There's some historical baggage here.
        #
        # Old versions of tokenserver would send a hashed "metrics uid" as the
        # "fxa_uid" key, attempting a small amount of anonymization.  Newer
        # versions of tokenserver send the raw uid as "fxa_uid" and the hashed
        # version as "hashed_fxa_uid".  The raw version may be used associating
        # stored data with a specific user, but the hashed version is the one
        # that we want for metrics.

        if "hashed_fxa_uid" in data:
            user["hashed_fxa_uid"] = data["hashed_fxa_uid"]
            if not VALID_FXA_ID_REGEX.match(user["hashed_fxa_uid"]):
                raise ValueError("invalid hashed_fxa_uid in token data")
            try:
                user["fxa_uid"] = data["fxa_uid"]
            except KeyError:
                raise ValueError("missing fxa_uid in token data")
            else:
                if not VALID_FXA_ID_REGEX.match(user["fxa_uid"]):
                    raise ValueError("invalid fxa_uid in token data")
            try:
                user["fxa_kid"] = data["fxa_kid"]
            except KeyError:
                raise ValueError("missing fxa_kid in token data")
            else:
                if not VALID_FXA_ID_REGEX.match(user["fxa_kid"]):
                    raise ValueError("invalid fxa_kid in token data")
        elif "fxa_uid" in data:
            user["hashed_fxa_uid"] = data["fxa_uid"]
            if not VALID_FXA_ID_REGEX.match(user["hashed_fxa_uid"]):
                raise ValueError("invalid fxa_uid in token data")

        if "hashed_device_id" in data:
            user["hashed_device_id"] = data["hashed_device_id"]
            if not VALID_FXA_ID_REGEX.match(user["hashed_device_id"]):
                raise ValueError("invalid hashed_device_id in token data")
        """
        elif "device_id" in data:
            user["hashed_device_id"] = data.get("device_id")
            if not VALID_FXA_ID_REGEX.match(user["hashed_device_id"]):
                raise ValueError("invalid device_id in token data")
        """
        return user
