# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import unittest

from tokenserver.test_authorization import TestAuthorization
from tokenserver.test_e2e import TestE2e
from tokenserver.test_misc import TestMisc
from tokenserver.test_node_assignment import TestNodeAssignment


def run_local_tests():
    return run_tests([TestAuthorization, TestMisc, TestNodeAssignment])


def run_end_to_end_tests():
    return run_tests([TestE2e])


def run_tests(test_cases):
    loader = unittest.TestLoader()
    success = True

    for test_case in test_cases:
        suite = loader.loadTestsFromTestCase(test_case)
        runner = unittest.TextTestRunner()
        res = runner.run(suite)
        success = success and res.wasSuccessful()

    if success:
        return 0
    else:
        return 1
