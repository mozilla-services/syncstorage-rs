import unittest
import test_tokenserver

suite = unittest.TestSuite()
suite.addTest(unittest.findTestCases(test_tokenserver, 'test_generation_number_change'))
runner = unittest.TextTestRunner()
runner.run(suite)
