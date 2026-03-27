"""Pytest configuration and fixtures for integration tests."""

import os
import time
import pytest
import requests  # type: ignore[import-untyped]
import logging

# max number of attempts to check server heartbeat
SYNC_SERVER_STARTUP_MAX_ATTEMPTS = 35
SYNC_SERVER_URL = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

logger = logging.getLogger("tokenserver.scripts.conftest")

# Local setup for fixtures


def _wait_for_server_startup(server_url, max_attempts=SYNC_SERVER_STARTUP_MAX_ATTEMPTS):
    """Wait for the __heartbeat__ endpoint to return a 200, pausing for 1 second
    between attempts. Raise a RuntimeError if the server does not start after
    the specific number of attempts.
    """
    for attempt in range(max_attempts):
        try:
            req = requests.get(f"{server_url}/__heartbeat__", timeout=2)
            if req.status_code == 200:
                return
        except requests.exceptions.RequestException as e:
            logger.warning("Connection failed: %s", e)
        time.sleep(1)

    raise RuntimeError(
        f"Server at {server_url} failed to start within the timeout period."
    )


# Fixtures


@pytest.fixture(scope="session", autouse=True)
def setup_server():
    """Wait for a server to be ready.  The server should be started prior to
    running the tests.  The server url can be set with SYNC_SERVER_URL; the
    default value is http://localhost:8000.
    """
    _wait_for_server_startup(SYNC_SERVER_URL)
