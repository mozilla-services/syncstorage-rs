"""Pytest configuration and fixtures for integration tests."""

import contextlib
import logging
import os
import random
import time
import uuid

import hawkauthlib
import pytest
import webtest
from pyramid.interfaces import IAuthenticationPolicy
from pyramid.request import Request
from webtest import TestApp

from tools.integration_tests.test_support import get_test_configurator

# max number of attempts to check server heartbeat
SYNC_SERVER_STARTUP_MAX_ATTEMPTS = 35
SYNC_SERVER_URL = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

logger = logging.getLogger("tools.integration-tests")

if os.environ.get("SYNC_TEST_LOG_HTTP"):
    _orig_do_request = webtest.TestApp.do_request

    def _logged_do_request(self, req, *args, **kwargs):
        """Wrap request and response logging around original do_request."""
        logger.info(">> %s %s", req.method, req.url)
        if req.body:
            logger.info(">> BODY: %s", req.body)
        resp = _orig_do_request(self, req, *args, **kwargs)
        logger.info("<< %s", resp.status)
        logger.info("<< BODY: %s", resp.body)
        return resp

    webtest.TestApp.do_request = _logged_do_request


def _retry_send(func, *args, **kwargs):
    """Call a webtest method, retrying once on 409/503."""
    try:
        return func(*args, **kwargs)
    except webtest.AppError as ex:
        if "409 " not in ex.args[0] and "503 " not in ex.args[0]:
            raise
        time.sleep(0.01)
        return func(*args, **kwargs)


def retry_post_json(app, *args, **kwargs):
    """POST JSON with retry on transient errors."""
    return _retry_send(app.post_json, *args, **kwargs)


def retry_put_json(app, *args, **kwargs):
    """PUT JSON with retry on transient errors."""
    return _retry_send(app.put_json, *args, **kwargs)


def retry_delete(app, *args, **kwargs):
    """DELETE with retry on transient errors."""
    return _retry_send(app.delete, *args, **kwargs)


def _make_auth_state(config, host_url):
    """Generate hawk credentials for a new random user."""
    global_secret = os.environ.get("SYNC_MASTER_SECRET")
    policy = config.registry.getUtility(IAuthenticationPolicy)
    if global_secret is not None:
        policy.secrets._secrets = [global_secret]
    user_id = random.randint(1, 100000)
    fxa_uid = "DECAFBAD" + str(uuid.uuid4().hex)[8:]
    hashed_fxa_uid = str(uuid.uuid4().hex)
    fxa_kid = "0000000000000-DECAFBAD" + str(uuid.uuid4().hex)[8:]
    req = Request.blank(host_url)
    creds = policy.encode_hawk_id(
        req,
        user_id,
        extra={
            "hashed_fxa_uid": hashed_fxa_uid,
            "fxa_uid": fxa_uid,
            "fxa_kid": fxa_kid,
        },
    )
    auth_token, auth_secret = creds
    return {
        "user_id": user_id,
        "fxa_uid": fxa_uid,
        "hashed_fxa_uid": hashed_fxa_uid,
        "fxa_kid": fxa_kid,
        "auth_token": auth_token,
        "auth_secret": auth_secret,
    }


@pytest.fixture(scope="function")
def st_ctx():
    """Functional test context for storage API tests.

    Sets up a Pyramid configurator, creates a TestApp with hawk signing,
    authenticates a random user, clears that user's data, and yields a
    context dict. Tears down configurator on exit.
    """
    ini_file = os.environ.get("MOZSVC_TEST_INI_FILE", "tests.ini")
    os.environ["MOZSVC_UUID"] = str(uuid.uuid4())
    if "MOZSVC_SQLURI" not in os.environ:
        os.environ["MOZSVC_SQLURI"] = "sqlite:///:memory:"
    if "MOZSVC_ONDISK_SQLURI" not in os.environ:
        ondisk = os.environ["MOZSVC_SQLURI"]
        if ":memory:" in ondisk:
            ondisk = "sqlite:////tmp/tests-sync-%s.db" % os.environ["MOZSVC_UUID"]
        os.environ["MOZSVC_ONDISK_SQLURI"] = ondisk

    # Locate tests.ini relative to test_storage.py
    this_dir = os.path.dirname(os.path.abspath(__file__))
    config = get_test_configurator(this_dir, ini_file)
    config.commit()
    config.make_wsgi_app()

    host_url = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")
    import urllib.parse as urlparse

    host_parts = urlparse.urlparse(host_url)
    app = TestApp(
        host_url,
        extra_environ={
            "HTTP_HOST": host_parts.netloc,
            "wsgi.url_scheme": host_parts.scheme or "http",
            "SERVER_NAME": host_parts.hostname,
            "REMOTE_ADDR": "127.0.0.1",
            "SCRIPT_NAME": host_parts.path,
        },
    )

    # Mutable auth state — shared with the do_request closure so that
    # switch_user() and the expired-token test can swap credentials at runtime.
    auth = _make_auth_state(config, host_url)
    auth_state = {
        "auth_token": auth["auth_token"],
        "auth_secret": auth["auth_secret"],
    }

    orig_do_request = app.do_request

    def new_do_request(req, *args, **kwds):
        hawkauthlib.sign_request(
            req, auth_state["auth_token"], auth_state["auth_secret"]
        )
        return orig_do_request(req, *args, **kwds)

    app.do_request = new_do_request

    root = "/1.5/%d" % auth["user_id"]
    retry_delete(app, root)

    ctx = {
        "app": app,
        "root": root,
        "user_id": auth["user_id"],
        "fxa_uid": auth["fxa_uid"],
        "hashed_fxa_uid": auth["hashed_fxa_uid"],
        "fxa_kid": auth["fxa_kid"],
        "auth_state": auth_state,
        "config": config,
        "host_url": host_url,
    }

    yield ctx

    config.end()
    del os.environ["MOZSVC_UUID"]


@contextlib.contextmanager
def switch_user(st_ctx):
    """Context manager: temporarily switch to a fresh random user.

    Updates both st_ctx and the auth_state dict (shared with the
    do_request closure) for the duration of the block, then restores
    the original user on exit.
    """
    orig_root = st_ctx["root"]
    orig_user_id = st_ctx["user_id"]
    orig_fxa_uid = st_ctx["fxa_uid"]
    orig_hashed_fxa_uid = st_ctx["hashed_fxa_uid"]
    orig_fxa_kid = st_ctx["fxa_kid"]
    orig_auth_token = st_ctx["auth_state"]["auth_token"]
    orig_auth_secret = st_ctx["auth_state"]["auth_secret"]

    config = st_ctx["config"]
    host_url = st_ctx["host_url"]
    app = st_ctx["app"]

    for _ in range(10):
        new_auth = _make_auth_state(config, host_url)
        if new_auth["user_id"] != orig_user_id:
            break
    else:
        raise RuntimeError("Failed to switch to new user id")

    st_ctx["auth_state"]["auth_token"] = new_auth["auth_token"]
    st_ctx["auth_state"]["auth_secret"] = new_auth["auth_secret"]
    st_ctx["user_id"] = new_auth["user_id"]
    st_ctx["fxa_uid"] = new_auth["fxa_uid"]
    st_ctx["hashed_fxa_uid"] = new_auth["hashed_fxa_uid"]
    st_ctx["fxa_kid"] = new_auth["fxa_kid"]
    new_root = "/1.5/%d" % new_auth["user_id"]
    st_ctx["root"] = new_root
    retry_delete(app, new_root)

    try:
        yield
    finally:
        st_ctx["auth_state"]["auth_token"] = orig_auth_token
        st_ctx["auth_state"]["auth_secret"] = orig_auth_secret
        st_ctx["user_id"] = orig_user_id
        st_ctx["fxa_uid"] = orig_fxa_uid
        st_ctx["hashed_fxa_uid"] = orig_hashed_fxa_uid
        st_ctx["fxa_kid"] = orig_fxa_kid
        st_ctx["root"] = orig_root
