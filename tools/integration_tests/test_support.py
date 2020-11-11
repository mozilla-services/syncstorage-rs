# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
""" Base test class, with an instanciated app.
"""

import atexit
import subprocess

the_server_subprocess = subprocess.Popen('target/debug/syncstorage', shell=True)
time.sleep(20)

def stop_subprocess():
    the_server_subprocess.terminate()
    the_server_subprocess.wait()
    
atexit.register(stop_subprocess)

import contextlib
import functools
import json
from konfig import Config, SettingsDict
import hawkauthlib
import os
import optparse
from pyramid.config import Configurator
from pyramid.interfaces import IAuthenticationPolicy
from pyramid.request import Request
import random
import requests
import sys
import urllib.parse as urlparse
import unittest2
import uuid


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

    config = get_configurator({"__file__": ini_path})
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

    def make_request(self, *args, **kwds):
        config = kwds.pop("config", self.config)
        return make_request(config, *args, **kwds)


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
        ##config.include("syncstorage")
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

        # Test against a live server if instructed so by the environment.
        # Otherwise, test against an in-process WSGI application.
        test_remote = os.environ.get("MOZSVC_TEST_REMOTE")
        if not test_remote:
            self.distant = False
            self.host_url = "http://localhost:5000"
            # This call implicity commits the configurator.
            application = self.config.make_wsgi_app()
        else:
            self.distant = True
            self.host_url = test_remote
            application = WSGIProxyApp(test_remote)
            # Explicitly commit so that calling code can introspect the config.
            self.config.commit()

        host_url = urlparse.urlparse(self.host_url)
        self.app = TestApp(application, extra_environ={
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

    def _authenticate(self):
        # For basic testing, use a random uid and sign our own tokens.
        # Subclasses might like to override this and use a live tokenserver.
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
            for retry_count in xrange(10):
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


def authenticate_to_token_server(url, email=None, audience=None):
    """Authenticate to the given token-server URL.

    This function generates a testing assertion for the specified email
    address, passes it to the specified token-server URL, and returns the
    resulting dict of authentication data.  It's useful for testing things
    that depend on having a live token-server.
    """
    # These modules are not (yet) hard dependencies of syncstorage,
    # so only import them is we really need them.
    global MOCKMYID_PRIVATE_KEY
    if MOCKMYID_PRIVATE_KEY is None:
        from browserid.jwt import DS128Key
        MOCKMYID_PRIVATE_KEY = DS128Key(MOCKMYID_PRIVATE_KEY_DATA)
    if email is None:
        email = "user%s@%s" % (random.randint(1, 100000), MOCKMYID_DOMAIN)
    if audience is None:
        audience = urlparse.urlparse(url)._replace(path="")
        audience = urlparse.urlunparse(audience)
    import browserid.tests.support
    assertion = browserid.tests.support.make_assertion(
        email=email,
        audience=audience,
        issuer=MOCKMYID_DOMAIN,
        issuer_keypair=(None, MOCKMYID_PRIVATE_KEY),
    )
    r = requests.get(url, headers={
        "Authorization": "BrowserID " + assertion,
    })
    r.raise_for_status()
    creds = json.loads(r.content)
    for key in ("id", "key", "api_endpoint"):
        creds[key] = creds[key].encode("ascii")
    return creds


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
    if not opts.use_token_server:
        if opts.email is not None:
            msg = "cant specify email address unless using live tokenserver"
            raise ValueError(msg)
        if opts.audience is not None:
            msg = "cant specify audience unless using live tokenserver"
            raise ValueError(msg)
        host_url = urlparse.urlparse(url)
        secret = None
        if host_url.fragment:
            secret = host_url.fragment
            host_url = host_url._replace(fragment="")
        os.environ["MOZSVC_TEST_REMOTE"] = host_url.geturl()

        class LiveTestCases(TestCaseClass):
            def _authenticate(self):
                policy = self.config.registry.getUtility(IAuthenticationPolicy)
                if secret is not None:
                    policy.secrets._secrets = [secret]
                return super(LiveTestCases, self)._authenticate()

    # If we're using a live tokenserver, then we need to get some credentials
    # and an endpoint URL.
    else:
        creds = authenticate_to_token_server(url, opts.email, opts.audience)

        # Point the tests at the given endpoint URI, after stripping off
        # the trailing /2.0/UID component.
        host_url = urlparse.urlparse(creds["api_endpoint"])
        host_path = host_url.path.rstrip("/")
        host_path = "/".join(host_path.split("/")[:-2])
        host_url = host_url._replace(path=host_path)
        os.environ["MOZSVC_TEST_REMOTE"] = host_url.geturl()

        # Customize the tests to use the provisioned auth credentials.
        class LiveTestCases(TestCaseClass):
            def _authenticate(self):
                self.user_id = creds["uid"]
                self.auth_token = creds["id"].encode("ascii")
                self.auth_secret = creds["key"].encode("ascii")

    # Now use the unittest2 runner to execute them.
    suite = unittest2.TestSuite()
    test_prefix = os.environ.get("SYNC_TEST_PREFIX", "test")
    suite.addTest(unittest2.makeSuite(LiveTestCases, prefix=test_prefix))
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
