import os
import psutil
import signal
import subprocess
import time
import pytest
import requests
import logging

DEBUG_BUILD = "target/debug/syncserver"
RELEASE_BUILD = "/app/bin/syncserver"
# max number of attempts to check server heartbeat
SYNC_SERVER_STARTUP_MAX_ATTEMPTS = 35
JWK_CACHE_DISABLED = os.environ.get("JWK_CACHE_DISABLED")

logger = logging.getLogger("tokenserver.scripts.conftest")

# Local setup for fixtures


def _terminate_process(process):
    """
    Gracefully terminate the process and its children.
    """
    proc = psutil.Process(pid=process.pid)
    child_proc = proc.children(recursive=True)
    for p in [proc] + child_proc:
        os.kill(p.pid, signal.SIGTERM)
    process.wait()


def _wait_for_server_startup(max_attempts=SYNC_SERVER_STARTUP_MAX_ATTEMPTS):
    """
    Waits for the __heartbeat__ endpoint to return a 200, pausing for 1 second
    between attempts. Raises a RuntimeError if the server does not start after
    the specific number of attempts.
    """
    itter = 0
    while True:
        if itter >= max_attempts:
            raise RuntimeError("Server failed to start within the timeout period.")
        try:
            req = requests.get("http://localhost:8000/__heartbeat__", timeout=2)
            if req.status_code == 200:
                break
        except requests.exceptions.RequestException as e:
            logger.warning("Connection failed: %s", e)
        time.sleep(1)
        itter += 1


def _start_server():
    """
    Starts the syncserver process, waits for it to be running,
    and return the process handle.
    """
    target_binary = None
    if os.path.exists(DEBUG_BUILD):
        target_binary = DEBUG_BUILD
    elif os.path.exists(RELEASE_BUILD):
        target_binary = RELEASE_BUILD
    else:
        raise RuntimeError("Neither {DEBUG_BUILD} nor {RELEASE_BUILD} were found.")

    server_proc = subprocess.Popen(
        target_binary,
        shell=True,
        text=True,
        env=os.environ,
    )

    _wait_for_server_startup()

    return server_proc


def _server_manager():
    """
    Context manager to gracefully start and stop the server.
    """
    server_process = _start_server()
    try:
        yield server_process
    finally:
        _terminate_process(server_process)


def _set_local_test_env_vars():
    """
    Set environment variables for local testing.
    This function sets the necessary environment variables for the syncserver.
    """
    os.environ.setdefault("SYNC_MASTER_SECRET", "secret0")
    os.environ.setdefault("SYNC_CORS_MAX_AGE", "555")
    os.environ.setdefault("SYNC_CORS_ALLOWED_ORIGIN", "*")
    os.environ["MOZSVC_TEST_REMOTE"] = "localhost"
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = os.environ[
        "MOCK_FXA_SERVER_URL"
    ]


# Fixtures


@pytest.fixture(scope="session")
def setup_server_local_testing():
    """
    Fixture to set up the server for local testing.
    This fixture sets the necessary environment variables and
    starts the server.
    """
    _set_local_test_env_vars()
    yield from _server_manager()


@pytest.fixture(scope="session")
def setup_server_local_testing_with_oauth():
    """
    Fixture to set up the server for local testing with OAuth.
    This fixture sets the necessary environment variables and
    starts the server.
    """
    _set_local_test_env_vars()

    # Set OAuth-specific environment variables
    os.environ["TOKENSERVER_AUTH_METHOD"] = "oauth"

    # Start the server
    yield from _server_manager()


@pytest.fixture(scope="session")
def setup_server_end_to_end_testing():
    """
    Fixture to set up the server for end-to-end testing.
    This fixture sets the necessary environment variables and
    starts the server.
    """
    _set_local_test_env_vars()
    # debatable if this should ONLY be here since it was only
    # done against the "run_end_to_end_tests" prior, of if we
    # just do it in _set_local_test_env_vars...
    if JWK_CACHE_DISABLED:
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N"]
        del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E"]

    # Set OAuth-specific environment variables
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = (
        "https://oauth.stage.mozaws.net"
    )

    # Start the server
    yield from _server_manager()
