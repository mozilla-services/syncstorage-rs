#!/usr/bin/env python3

import atexit
import subprocess
import sys
from test_storage import TestStorage
from test_support import run_live_functional_tests
import time


the_server_subprocess = subprocess.Popen('target/debug/syncstorage', shell=True)
time.sleep(20)


def stop_subprocess():
	the_server_subprocess.terminate()
	the_server_subprocess.wait()

	
atexit.register(stop_subprocess)


if __name__ == "__main__":
	# When run as a script, this file will execute the
	# functional tests against a live webserver.
	res = run_live_functional_tests(TestStorage, sys.argv)
	sys.exit(res)
