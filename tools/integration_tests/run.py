#!/usr/bin/env python3

import os.path
import signal
import subprocess
import sys
import time

import psutil
from test_storage import TestStorage
from test_support import run_live_functional_tests
from tokenserver.run import run_end_to_end_tests, run_local_tests

DEBUG_BUILD = "target/debug/syncserver"
RELEASE_BUILD = "/app/bin/syncserver"


def terminate_process(process):
    proc = psutil.Process(pid=process.pid)
    child_proc = proc.children(recursive=True)
    for p in [proc] + child_proc:
        os.kill(p.pid, signal.SIGTERM)
    process.wait()


if __name__ == "__main__":
    # When run as a script, this file will execute the
    # functional tests against a live webserver.
    target_binary = None
    if os.path.exists(DEBUG_BUILD):
        target_binary = DEBUG_BUILD
    elif os.path.exists(RELEASE_BUILD):
        target_binary = RELEASE_BUILD
    else:
        raise RuntimeError(
            "Neither target/debug/syncserver \
                nor /app/bin/syncserver were found."
        )

    def start_server():
        the_server_subprocess = subprocess.Popen(target_binary, shell=True, env=os.environ)

        # TODO we should change this to watch for a log message on startup
        # to know when to continue instead of sleeping for a fixed amount
        time.sleep(20)

        return the_server_subprocess

    os.environ.setdefault("SYNC_MASTER_SECRET", "secret0")
    os.environ.setdefault("SYNC_CORS_MAX_AGE", "555")
    os.environ.setdefault("SYNC_CORS_ALLOWED_ORIGIN", "*")
    mock_fxa_server_url = os.environ["MOCK_FXA_SERVER_URL"]
    url = "%s/v2" % mock_fxa_server_url
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = mock_fxa_server_url
    the_server_subprocess = start_server()
    try:
        res = 0
        res |= run_live_functional_tests(TestStorage, sys.argv)
        os.environ["TOKENSERVER_AUTH_METHOD"] = "oauth"
        res |= run_local_tests()
    finally:
        terminate_process(the_server_subprocess)

    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = "https://oauth.stage.mozaws.net"
    the_server_subprocess = start_server()
    try:
        res |= run_end_to_end_tests()
    finally:
        terminate_process(the_server_subprocess)

    # Run the Tokenserver end-to-end tests without the JWK cached
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KTY"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__ALG"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__KID"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__FXA_CREATED_AT"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__USE"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__N"]
    del os.environ["SYNC_TOKENSERVER__FXA_OAUTH_PRIMARY_JWK__E"]

    the_server_subprocess = start_server()
    try:
        verbosity = int(os.environ.get("VERBOSITY", "1"))
        res |= run_end_to_end_tests(verbosity=verbosity)
    finally:
        terminate_process(the_server_subprocess)

    sys.exit(res)
