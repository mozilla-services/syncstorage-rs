# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
import unittest

from tokenserver.test_authorization import TestAuthorization
from tokenserver.test_e2e import TestE2e
from tokenserver.test_misc import TestMisc
from tokenserver.test_node_assignment import TestNodeAssignment


def run_local_tests(include_browserid_specific_tests=False):
    test_classes = [TestAuthorization, TestMisc, TestNodeAssignment]

    return run_tests(test_classes)


def run_end_to_end_tests(verbosity=1):
    return run_tests([TestE2e], verbosity=verbosity)


def run_tests(test_cases, verbosity=1):
    loader = unittest.TestLoader()
    success = True

    for test_case in test_cases:
        suite = loader.loadTestsFromTestCase(test_case)
        runner = unittest.TextTestRunner(verbosity=verbosity)
        res = runner.run(suite)
        success = success and res.wasSuccessful()

    if success:
        return 0
    else:
        return 1
