# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""Pytest fixtures for storage integration tests.

Fixture hierarchy
─────────────────
st_ctx — function-scoped composite: sets up Pyramid configurator, creates a
         hawk-signed TestApp, seeds a random user, clears that user's data,
         and yields a plain dict consumed by test functions.

Helper functions and constants live in helpers.py.
"""

import os
import uuid

import pytest

from tools.integration_tests.helpers import (
    make_auth_state,
    make_test_app,
    retry_delete,
)
from tools.integration_tests.test_support import get_test_configurator


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

    # Locate tests.ini relative to this file
    this_dir = os.path.dirname(os.path.abspath(__file__))
    config = get_test_configurator(this_dir, ini_file)
    config.commit()
    config.make_wsgi_app()

    host_url = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

    auth = make_auth_state(config, host_url)
    auth_state = {
        "auth_token": auth["auth_token"],
        "auth_secret": auth["auth_secret"],
    }

    app = make_test_app(host_url, auth_state)

    root = "/1.5/%d" % auth["user_id"]
    retry_delete(app, root)

    yield {
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

    config.end()
    del os.environ["MOZSVC_UUID"]
