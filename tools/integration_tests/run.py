#!/usr/bin/env python3

import atexit
import os.path
import subprocess
import sys
from test_storage import TestStorage
from test_support import run_live_functional_tests
import time


DEBUG_BUILD = "target/debug/syncstorage"
RELEASE_BUILD = "/app/bin/syncstorage"

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
            "Neither target/debug/syncstorage \
                nor /app/bin/syncstorage were found."
        )
    the_server_subprocess = subprocess.Popen(
        "SYNC_MASTER_SECRET=secret0 " + target_binary, shell=True
    )
    # TODO we should change this to watch for a log message on startup
    # to know when to continue instead of sleeping for a fixed amount
    time.sleep(20)

    def stop_subprocess():
        the_server_subprocess.terminate()
        the_server_subprocess.wait()

    atexit.register(stop_subprocess)

    res = run_live_functional_tests(TestStorage, sys.argv)
    sys.exit(res)
