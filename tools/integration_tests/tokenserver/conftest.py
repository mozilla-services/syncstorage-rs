import sys
import os
import psutil
import signal
import subprocess
import time
import pytest
import requests
from urllib.parse import urlparse
import threading
import logging

DEBUG_BUILD = "target/debug/syncserver"
RELEASE_BUILD = "/app/bin/syncserver"
SYNC_SERVER_STARTUP_TIMEOUT = 15  # seconds


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
        raise RuntimeError(
            "Neither {DEBUG_BUILD} nor {RELEASE_BUILD} were found."
        )

    server_process = subprocess.Popen(
        target_binary,
        shell=True,
        env=os.environ,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True
    )

    # def stream_output(process):
    #     for line in iter(process.stdout.readline, ''):
    #         print(f"😭 {line}", end="")

    # threading.Thread(target=stream_output, args=(server_process,), daemon=True).start()

    # Wait for the server to start
    itter = 0
    for _ in range(SYNC_SERVER_STARTUP_TIMEOUT):
        itter += 1
        if itter > SYNC_SERVER_STARTUP_TIMEOUT - 1:
            raise RuntimeError(
                "Server failed to start within the timeout period."
            )
        try:
            req = requests.get("http://localhost:8000/__heartbeat__", timeout=2)
            if req.status_code == 200:
                break
        except requests.exceptions.RequestException as e:
            logger = logging.getLogger(__name__)
            logger.warning(f"Connection failed: {e}")
        time.sleep(1)


    host_url = urlparse(os.environ.get("TOKENSERVER_HOST_WITH_FRAGMENT"))
    if host_url.fragment:
        os.environ.setdefault("global_secret", host_url.fragment)
    os.environ["MOZSVC_TEST_REMOTE"] = "localhost"

    return server_process


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
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = os.environ["MOCK_FXA_SERVER_URL"]

# Fixtures

@pytest.fixture(scope="class")
def setup_server_local_testing():
    """
    Fixture to set up the server for local testing.
    This fixture sets the necessary environment variables and starts the server.
    """
    print("Using setup_server_local_testing fixture")
    _set_local_test_env_vars()
    yield from _server_manager()


@pytest.fixture(scope="class")
def setup_server_local_testing_with_oauth():
    """
    Fixture to set up the server for local testing with OAuth.
    This fixture sets the necessary environment variables and starts the server.
    """
    _set_local_test_env_vars()

    # Set OAuth-specific environment variables
    os.environ["TOKENSERVER_AUTH_METHOD"] = "oauth"

    # Start the server
    yield from _server_manager()

@pytest.fixture(scope="class")
def setup_server_end_to_end_testing():
    """
    Fixture to set up the server for end-to-end testing.
    This fixture sets the necessary environment variables and starts the server.
    """
    _set_local_test_env_vars()

    # Set OAuth-specific environment variables
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = \
        "https://oauth.stage.mozaws.net"

    # Start the server
    yield from _server_manager()

# ## Probably need to rename this to something like 'start_live_functional_server'
# @pytest.fixture(scope="session")
# def setup_server():

#     # Set environment variables
#     os.environ.setdefault("SYNC_MASTER_SECRET", "secret0")
#     os.environ.setdefault("SYNC_CORS_MAX_AGE", "555")
#     os.environ.setdefault("SYNC_CORS_ALLOWED_ORIGIN", "*")
#     os.environ.setdefault("MOZSVC_TEST_REMOTE", "localhost")

#     os.environ["TOKENSERVER_AUTH_METHOD"] = "oauth"

#     url = "http://localhost:8000#secret0"
#     host_url = urllib.parse.urlparse(url)
#     if host_url.fragment:
#         global global_secret
#         global_secret = host_url.fragment
#         host_url = host_url._replace(fragment="")
#     os.environ["MOZSVC_TEST_REMOTE"] = host_url.netloc


#     # I think these are just for running `run_end_to_end_tests`, need to investigate 
#     # mock_fxa_server_url = os.environ["MOCK_FXA_SERVER_URL"]
#     # os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = mock_fxa_server_url

#     # Start the server
#     yield from manage_server()



## This needs to be revisited - the old setup would delete the vars and re-run the `run_end_to_end_tests`
#       the challenge is that they target the same test suite but we'd
#       need two separate fixtures to setup the env and we can't
#       really do that.
#
#       One option would be to set a env_var from the container to indicate which path, 
#       then a single fixture could be used to set the env vars and then run the tests. 
#       But that's messy Another option is to duplicate the tests, but that's also messy.
@pytest.fixture(scope="class")
def setup_server_without_oauth_vars():
    """
    Fixture that deletes OAuth-specific environment variables
    and starts the server.
    """
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E"]
    
    server_process = _start_server()
    
    yield server_process
    
    _terminate_process(server_process)