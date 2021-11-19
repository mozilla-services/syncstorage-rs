# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.

import sys
import unittest

import test_database
from test_database import TestDatabase
from test_process_account_events import TestProcessAccountEvents
from test_purge_old_records import TestPurgeOldRecords
from test_scripts import TestScripts

if __name__ == "__main__":
    # loader = unittest.TestLoader()
    # test_cases = [TestDatabase, TestPurgeOldRecords, TestProcessAccountEvents,
    #               TestScripts]

    # res = 0
    # for test_case in test_cases:
    #     suite = loader.loadTestsFromTestCase(test_case)
    #     runner = unittest.TextTestRunner()
    #     if not runner.run(suite).wasSuccessful():
    #         res = 1
    
    suite = unittest.TestSuite()
    suite.addTest(unittest.findTestCases(test_database, 'test_cleanup_of_old_records'))
    runner = unittest.TextTestRunner()

    res = 0
    if not runner.run(suite).wasSuccessful():
        res = 1

    sys.exit(res)
