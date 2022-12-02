#!/usr/bin/env python3

import argparse
import os.path
import psutil
import signal
import subprocess
import sys
from test_storage import TestStorage
from test_support import run_live_functional_tests
import time
from tokenserver.run import run_end_to_end_tests, run_local_tests

DEBUG_BUILD = "target/debug/syncserver"
RELEASE_BUILD = "/app/bin/syncserver"

if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Run the syncstorage and tokenserver integration tests"
    )
    parser.add_argument(
        '--syncstorage', action="store_true",
        help="Run the syncstorage integration tests")
    parser.add_argument(
        '--tokenserver-local', action="store_true",
        help="Run the local tokenserver integration tests")
    parser.add_argument(
        '--tokenserver-e2e', action="store_true",
        help="Run the e2e tokenserver integration tests")
    args = parser.parse_args()

    res = 0

    if args.syncstorage:
        res |= run_live_functional_tests(TestStorage, sys.argv)

    if args.tokenserver_local and args.tokenserver_e2e:
        raise RuntimeError(
            "It is impossible to run the local and e2e tokenserver tests \
                against the same server."
        )
    elif args.tokenserver_local:
        # TODO: handle oauth vs. browserid tests
        res |= run_local_tests(include_browserid_specific_tests=False)
    elif args.tokenserver_e2e:
        res |= run_end_to_end_tests()

# def terminate_process(process):
#     proc = psutil.Process(pid=process.pid)
#     child_proc = proc.children(recursive=True)
#     for p in [proc] + child_proc:
#         os.kill(p.pid, signal.SIGTERM)
#     process.wait()


if __name__ == "__main__":
    # When run as a script, this file will execute the
    # functional tests against a live webserver.
    # target_binary = None
    # if os.path.exists(DEBUG_BUILD):
    #     target_binary = DEBUG_BUILD
    # elif os.path.exists(RELEASE_BUILD):
    #     target_binary = RELEASE_BUILD
    # else:
    #     raise RuntimeError(
    #         "Neither target/debug/syncserver \
    #             nor /app/bin/syncserver were found."
    #     )

    # def start_server():
    #     the_server_subprocess = subprocess.Popen(
    #         target_binary, shell=True, env=os.environ
    #     )

    #     # TODO we should change this to watch for a log message on startup
    #     # to know when to continue instead of sleeping for a fixed amount
    #     time.sleep(20)

    #     return the_server_subprocess

    os.environ.setdefault("SYNC_MASTER_SECRET", "secret0")
    os.environ.setdefault("SYNC_CORS_MAX_AGE", "555")
    os.environ.setdefault("SYNC_CORS_ALLOWED_ORIGIN", "*")
    mock_fxa_server_url = os.environ["MOCK_FXA_SERVER_URL"]
    url = "%s/v2" % mock_fxa_server_url
    os.environ["SYNC_TOKENSERVER__FXA_BROWSERID_SERVER_URL"] = url
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = mock_fxa_server_url
    the_server_subprocess = start_server()
    try:
        res = 0
        res |= run_live_functional_tests(TestStorage, sys.argv)
        os.environ["TOKENSERVER_AUTH_METHOD"] = "oauth"
        res |= run_local_tests(include_browserid_specific_tests=False)
        os.environ["TOKENSERVER_AUTH_METHOD"] = "browserid"
        res |= run_local_tests(include_browserid_specific_tests=True)
    finally:
        terminate_process(the_server_subprocess)

    os.environ["SYNC_TOKENSERVER__FXA_BROWSERID_SERVER_URL"] = \
        "https://verifier.stage.mozaws.net/v2"
    os.environ["SYNC_TOKENSERVER__FXA_OAUTH_SERVER_URL"] = \
        "https://oauth.stage.mozaws.net"
    the_server_subprocess = start_server()
    try:
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
        res |= run_end_to_end_tests()
    finally:
        terminate_process(the_server_subprocess)

    sys.exit(res)
