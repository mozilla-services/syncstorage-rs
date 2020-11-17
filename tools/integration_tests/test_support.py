# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
""" Base test class, with an instanciated app.
"""

import contextlib
import functools
from konfig import Config, SettingsDict
import hawkauthlib
import os
import optparse
from pyramid.authorization import ACLAuthorizationPolicy
from pyramid.config import Configurator
from pyramid.interfaces import IAuthenticationPolicy
from pyramid.request import Request
from pyramid.util import DottedNameResolver
from pyramid_hawkauth import HawkAuthenticationPolicy
import random
import re
import csv
import binascii
from collections import defaultdict
import sys
import time
import tokenlib
import urllib.parse as urlparse
import unittest2
import uuid
from webtest import TestApp
from zope.interface import implementer


global_secret = None
VALID_FXA_ID_REGEX = re.compile("^[A-Za-z0-9=\\-_]{1,64}$")


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
        return self._secrets.keys()

    def load(self, filename):
        if not isinstance(filename, (list, tuple)):
            filename = [filename]

        for name in filename:
            with open(name, 'rb') as f:

                reader = csv.reader(f, delimiter=',')
                for line, row in enumerate(reader):
                    if len(row) < 2:
                        continue
                    node = row[0]
                    if node in self._secrets:
                        raise ValueError("Duplicate node line %d" % line)
                    secrets = []
                    for secret in row[1:]:
                        secret = secret.split(':')
                        if len(secret) != 2:
                            raise ValueError("Invalid secret line %d" % line)
                        secrets.append(tuple(secret))
                    secrets.sort()
                    self._secrets[node] = secrets

    def save(self, filename):
        with open(filename, 'wb') as f:
            writer = csv.writer(f, delimiter=',')
            for node, secrets in self._secrets.items():
                secrets = ['%s:%s' % (timestamp, secret)
                           for timestamp, secret in secrets]
                secrets.insert(0, node)
                writer.writerow(secrets)

    def get(self, node):
        return [secret for timestamp, secret in self._secrets[node]]

    def add(self, node, size=256):
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
        return list(self._secrets)

    def keys(self):
        return []


def resolve_name(name, package=None):
    """Resolve dotted name into a python object.
    This function resolves a dotted name as a reference to a python object,
    returning whatever object happens to live at that path.  It's a simple
    convenience wrapper around pyramid's DottedNameResolver.
    The optional argument 'package' specifies the package name for relative
    imports.  If not specified, only absolute paths will be supported.
    """
    return DottedNameResolver(package).resolve(name)


def load_into_settings(filename, settings):
    """Load config file contents into a Pyramid settings dict.
    This is a helper function for initialising a Pyramid settings dict from
    a config file.  It flattens the config file sections into dotted settings
    names and updates the given dictionary in place.
    You would typically use this when constructing a Pyramid Configurator
    object, like so::
        def main(global_config, **settings):
            config_file = global_config['__file__']
            load_info_settings(config_file, settings)
            config = Configurator(settings=settings)
    """
    filename = os.path.expandvars(os.path.expanduser(filename))
    filename = os.path.abspath(os.path.normpath(filename))
    config = Config(filename)

    # Konfig keywords are added to every section when present, we have to
    # filter them out, otherwise plugin.load_from_config and
    # plugin.load_from_settings are unable to create instances.
    konfig_keywords = ['extends', 'overrides']

    # Put values from the config file into the pyramid settings dict.
    for section in config.sections():
        setting_prefix = section.replace(":", ".")
        for name, value in config.get_map(section).items():
            if name not in konfig_keywords:
                settings[setting_prefix + "." + name] = value

    # Store a reference to the Config object itself for later retrieval.
    settings['config'] = config
    return config


def get_test_configurator(root, ini_file="tests.ini"):
    """Find a file with testing settings, turn it into a configurator."""
    ini_dir = root
    while True:
        ini_path = os.path.join(ini_dir, ini_file)
        if os.path.exists(ini_path):
            break
        if ini_path == ini_file or ini_path == "/" + ini_file:
            raise RuntimeError("cannot locate " + ini_file)
        ini_dir = os.path.split(ini_dir)[0]
    # print("finding configurator for", ini_path)
    config = get_configurator({"__file__": ini_path})
    authz_policy = ACLAuthorizationPolicy()
    config.set_authorization_policy(authz_policy)
    authn_policy = TokenServerAuthenticationPolicy.from_settings(
        config.get_settings())
    config.set_authentication_policy(authn_policy)
    return config


def get_configurator(global_config, **settings):
    """Create a pyramid Configurator and populate it with sensible defaults.
    This function is a helper to create and pre-populate a Configurator
    object using the given paste-deploy settings dicts.  It uses the
    mozsvc.config module to flatten the config paste-deploy config file
    into the settings dict so that non-mozsvc pyramid apps can read values
    from it easily.
    """
    # Populate a SettingsDict with settings from the deployment file.
    settings = SettingsDict(settings)
    config_file = global_config.get('__file__')
    if config_file is not None:
        load_into_settings(config_file, settings)
    # Update with default pyramid settings, and then insert for all to use.
    config = Configurator(settings={})
    settings.setdefaults(config.registry.settings)
    config.registry.settings = settings
    return config


def restore_env(*keys):
    """Decorator that ensures os.environ gets restored after a test.

    Given a list of environment variable keys, this decorator will save the
    current values of those environment variables at the start of the call
    and restore them to those values at the end.
    """
    def decorator(func):
        @functools.wraps(func)
        def wrapper(*args, **kwds):
            values = [os.environ.get(key) for key in keys]
            try:
                return func(*args, **kwds)
            finally:
                for key, value in zip(keys, values):
                    if value is None:
                        os.environ.pop(key, None)
                    else:
                        os.environ[key] = value
        return wrapper
    return decorator


class TestCase(unittest2.TestCase):
    """TestCase with some generic helper methods."""

    def setUp(self):
        super(TestCase, self).setUp()
        self.config = self.get_configurator()

    def tearDown(self):
        self.config.end()
        super(TestCase, self).tearDown()

    def get_configurator(self):
        """Load the configurator to use for the tests."""
        # Load config from the .ini file.
        # print("get_configurator", self, getattr(self, "TEST_INI_FILE", None))
        if not hasattr(self, "ini_file"):
            if hasattr(self, "TEST_INI_FILE"):
                self.ini_file = self.TEST_INI_FILE
            else:
                # The file to use may be specified in the environment.
                self.ini_file = os.environ.get("MOZSVC_TEST_INI_FILE",
                                               "tests.ini")
        __file__ = sys.modules[self.__class__.__module__].__file__
        config = get_test_configurator(__file__, self.ini_file)
        config.begin()
        return config

    """
    def make_request(self, *args, **kwds):
        config = kwds.pop("config", self.config)
        return make_request(config, *args, **kwds)
    """


class StorageTestCase(TestCase):
    """TestCase class with automatic cleanup of database files."""

    @restore_env("MOZSVC_TEST_INI_FILE")
    def setUp(self):
        # Put a fresh UUID into the environment.
        # This can be used in e.g. config files to create unique paths.
        os.environ["MOZSVC_UUID"] = str(uuid.uuid4())
        # Ensure a default sqluri if none is provided in the environment.
        # We use an in-memory sqlite db by default, except for tests that
        # explicitly require an on-disk file.
        if "MOZSVC_SQLURI" not in os.environ:
            os.environ["MOZSVC_SQLURI"] = "sqlite:///:memory:"
        if "MOZSVC_ONDISK_SQLURI" not in os.environ:
            ondisk_sqluri = os.environ["MOZSVC_SQLURI"]
            if ":memory:" in ondisk_sqluri:
                ondisk_sqluri = "sqlite:////tmp/tests-sync-%s.db"
                ondisk_sqluri %= (os.environ["MOZSVC_UUID"],)
            os.environ["MOZSVC_ONDISK_SQLURI"] = ondisk_sqluri
        # Allow subclasses to override default ini file.
        if hasattr(self, "TEST_INI_FILE"):
            if "MOZSVC_TEST_INI_FILE" not in os.environ:
                os.environ["MOZSVC_TEST_INI_FILE"] = self.TEST_INI_FILE
        super(StorageTestCase, self).setUp()

    def tearDown(self):
        self._cleanup_test_databases()
        # clear the pyramid threadlocals
        self.config.end()
        super(StorageTestCase, self).tearDown()
        del os.environ["MOZSVC_UUID"]

    def get_configurator(self):
        config = super(StorageTestCase, self).get_configurator()
        # config.include("syncstorage")
        return config

    def _cleanup_test_databases(self):
        """Clean up any database used during the tests."""
        # Find and clean up any in-use databases
        for key, storage in self.config.registry.items():
            if not key.startswith("syncstorage:storage:"):
                continue
            while hasattr(storage, "storage"):
                storage = storage.storage
            # For server-based dbs, drop the tables to clear them.
            if storage.dbconnector.driver in ("mysql", "postgres"):
                with storage.dbconnector.connect() as c:
                    c.execute('DROP TABLE bso')
                    c.execute('DROP TABLE user_collections')
                    c.execute('DROP TABLE collections')
                    c.execute('DROP TABLE batch_uploads')
                    c.execute('DROP TABLE batch_upload_items')
            # Explicitly free any pooled connections.
            storage.dbconnector.engine.dispose()
        # Find any sqlite database files and delete them.
        for key, value in self.config.registry.settings.items():
            if key.endswith(".sqluri"):
                sqluri = urlparse.urlparse(value)
                if sqluri.scheme == 'sqlite' and ":memory:" not in value:
                    if os.path.isfile(sqluri.path):
                        os.remove(sqluri.path)


class FunctionalTestCase(TestCase):
    """TestCase for writing functional tests using WebTest.
    This TestCase subclass provides an easy mechanism to write functional
    tests using WebTest.  It exposes a TestApp instance as self.app.
    If the environment variable MOZSVC_TEST_REMOTE is set to a URL, then
    self.app will be a WSGIProxy application that forwards all requests to
    that server.  This allows the functional tests to be easily run against
    a live server instance.
    """

    def setUp(self):
        super(FunctionalTestCase, self).setUp()

        # now that we're testing against a rust server, we're always distant.
        # but some tests don't run if we're set to distant. so let's set
        # distant to false, figure out which tests we still want, and
        # delete the ones that don't work with distant = True along
        # with the need for self.distant.
        self.distant = False
        self.host_url = "http://localhost:8000"
        # This call implicitly commits the configurator. We probably still
        # want it for the side effects.
        self.config.make_wsgi_app()

        host_url = urlparse.urlparse(self.host_url)
        self.app = TestApp(self.host_url, extra_environ={
            "HTTP_HOST": host_url.netloc,
            "wsgi.url_scheme": host_url.scheme or "http",
            "SERVER_NAME": host_url.hostname,
            "REMOTE_ADDR": "127.0.0.1",
            "SCRIPT_NAME": host_url.path,
        })


class StorageFunctionalTestCase(FunctionalTestCase, StorageTestCase):
    """Abstract base class for functional testing of a storage API."""

    def setUp(self):
        super(StorageFunctionalTestCase, self).setUp()

        # Generate userid and auth token crednentials.
        # This can be overridden by subclasses.
        self.config.commit()
        self._authenticate()

        # Monkey-patch the app to sign all requests with the token.
        def new_do_request(req, *args, **kwds):
            hawkauthlib.sign_request(req, self.auth_token, self.auth_secret)
            return orig_do_request(req, *args, **kwds)
        orig_do_request = self.app.do_request
        self.app.do_request = new_do_request

    def basic_testing_authenticate(self):
        # For basic testing, use a random uid and sign our own tokens.
        # Subclasses might like to override this and use a live tokenserver.
        pass

    def _authenticate(self):
        policy = self.config.registry.getUtility(IAuthenticationPolicy)
        if global_secret is not None:
            policy.secrets._secrets = [global_secret]
        self.user_id = random.randint(1, 100000)
        auth_policy = self.config.registry.getUtility(IAuthenticationPolicy)
        req = Request.blank(self.host_url)
        creds = auth_policy.encode_hawk_id(
            req, self.user_id, extra={
                # Include a hashed_fxa_uid to trigger uid/kid extraction
                "hashed_fxa_uid": str(uuid.uuid4()),
                "fxa_uid": str(uuid.uuid4()),
                "fxa_kid": str(uuid.uuid4()),
            }
        )
        self.auth_token, self.auth_secret = creds

    @contextlib.contextmanager
    def _switch_user(self):
        # It's hard to reliably switch users when testing a live server.
        if self.distant:
            raise unittest2.SkipTest("Skipped when testing a live server")
        # Temporarily authenticate as a different user.
        orig_user_id = self.user_id
        orig_auth_token = self.auth_token
        orig_auth_secret = self.auth_secret
        try:
            # We loop because the userids are randomly generated,
            # so there's a small change we'll get the same one again.
            for retry_count in range(10):
                self._authenticate()
                if self.user_id != orig_user_id:
                    break
            else:
                raise RuntimeError("Failed to switch to new user id")
            yield
        finally:
            self.user_id = orig_user_id
            self.auth_token = orig_auth_token
            self.auth_secret = orig_auth_secret

    def _cleanup_test_databases(self):
        # Don't cleanup databases unless we created them ourselves.
        if not self.distant:
            super(StorageFunctionalTestCase, self)._cleanup_test_databases()


MOCKMYID_DOMAIN = "mockmyid.s3-us-west-2.amazonaws.com"
MOCKMYID_PRIVATE_KEY = None
MOCKMYID_PRIVATE_KEY_DATA = {
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
}


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


@implementer(IAuthenticationPolicy)
class TokenServerAuthenticationPolicy(HawkAuthenticationPolicy):
    """Pyramid authentication policy for use with Tokenserver auth tokens.
    This class provides an IAuthenticationPolicy implementation based on
    the Mozilla TokenServer authentication tokens as described here:
        https://docs.services.mozilla.com/token/
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
            msgs = ["WARNING: using a randomly-generated token secret.",
                    "You probably want to set 'secret' or 'secrets_file' in "
                    "the [hawkauth] section of your configuration"]
            for msg in msgs:
                print("warn:", msg)
        elif isinstance(secrets, (str, list)):
            secrets = FixedSecrets(secrets)
        elif isinstance(secrets, dict):
            secrets = resolve_name(secrets.pop("backend"))(**secrets)
        self.secrets = secrets
        if kwds.get("nonce_cache") is None:
            kwds["nonce_cache"] = PermissiveNonceCache()
        super(TokenServerAuthenticationPolicy, self).__init__(**kwds)

    @classmethod
    def _parse_settings(cls, settings):
        """Parse settings for an instance of this class."""
        supercls = super(TokenServerAuthenticationPolicy, cls)
        kwds = supercls._parse_settings(settings)
        # collect leftover settings into a config for a Secrets object,
        # wtih some b/w compat for old-style secret-handling settings.
        secrets_prefix = "secrets."
        secrets = {}
        if "secrets_file" in settings:
            if "secret" in settings:
                raise ValueError("can't use both 'secret' and 'secrets_file'")
            secrets["backend"] = "test_support.Secrets"
            secrets["filename"] = settings.pop("secrets_file")
        elif "secret" in settings:
            secrets["backend"] = "test_support.FixedSecrets"
            secrets["secrets"] = settings.pop("secret")
        for name in settings.keys():
            if name.startswith(secrets_prefix):
                secrets[name[len(secrets_prefix):]] = settings.pop(name)
        kwds['secrets'] = secrets
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


@implementer(IAuthenticationPolicy)
class SyncStorageAuthenticationPolicy(TokenServerAuthenticationPolicy):
    """Pyramid authentication policy with special handling of expired tokens.

    This class extends the standard mozsvc TokenServerAuthenticationPolicy
    to (carefully) allow some access by holders of expired tokens.  Presenting
    an expired token will result in a principal of "expired:<uid>" rather than
    just "<uid>", allowing this case to be specially detected and handled for
    some resources without interfering with the usual authentication rules.
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
        elif "device_id" in data:
            user["hashed_device_id"] = data.get("device_id")
            if not VALID_FXA_ID_REGEX.match(user["hashed_device_id"]):
                raise ValueError("invalid device_id in token data")
        return user


def run_live_functional_tests(TestCaseClass, argv=None):
    """Execute the given suite of testcases against a live server."""
    if argv is None:
        argv = sys.argv

    # This will only work using a StorageFunctionalTestCase subclass,
    # since we override the _authenticate() method.
    assert issubclass(TestCaseClass, StorageFunctionalTestCase)

    usage = "Usage: %prog [options] <server-url>"
    parser = optparse.OptionParser(usage=usage)
    parser.add_option("-x", "--failfast", action="store_true",
                      help="stop after the first failed test")
    parser.add_option("", "--config-file",
                      help="name of the config file in use by the server")
    parser.add_option("", "--use-token-server", action="store_true",
                      help="the given URL is a tokenserver, not an endpoint")
    parser.add_option("", "--email",
                      help="email address to use for tokenserver tests")
    parser.add_option("", "--audience",
                      help="assertion audience to use for tokenserver tests")

    try:
        opts, args = parser.parse_args(argv)
    except SystemExit as e:
        return e.args[0]
    if len(args) != 2:
        parser.print_usage()
        return 2

    url = args[1]
    if opts.config_file is not None:
        os.environ["MOZSVC_TEST_INI_FILE"] = opts.config_file

    # If we're not using the tokenserver, the default implementation of
    # _authenticate will do just fine.  We optionally accept the token
    # signing secret in the url hash fragement.
    if opts.email is not None:
        msg = "cant specify email address unless using live tokenserver"
        raise ValueError(msg)
    if opts.audience is not None:
        msg = "cant specify audience unless using live tokenserver"
        raise ValueError(msg)
    host_url = urlparse.urlparse(url)
    if host_url.fragment:
        global global_secret
        global_secret = host_url.fragment
        host_url = host_url._replace(fragment="")
    os.environ["MOZSVC_TEST_REMOTE"] = 'localhost'

    # Now use the unittest2 runner to execute them.
    suite = unittest2.TestSuite()
    import test_storage
    test_prefix = os.environ.get("SYNC_TEST_PREFIX", "test")
    suite.addTest(unittest2.findTestCases(test_storage, test_prefix))
    # suite.addTest(unittest2.makeSuite(LiveTestCases, prefix=test_prefix))
    runner = unittest2.TextTestRunner(
        stream=sys.stderr,
        failfast=opts.failfast,
        verbosity=2,
    )
    res = runner.run(suite)
    if not res.wasSuccessful():
        return 1
    return 0


# Tell over-zealous test discovery frameworks that this isn't a real test.
run_live_functional_tests.__test__ = False
