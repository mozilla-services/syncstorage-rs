"""Pytest configuration and fixtures for integration tests."""

import os
import logging

# max number of attempts to check server heartbeat
SYNC_SERVER_STARTUP_MAX_ATTEMPTS = 35
SYNC_SERVER_URL = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

logger = logging.getLogger("tools.integration-tests")

if os.environ.get("SYNC_TEST_LOG_HTTP"):
    import webtest

    _orig_do_request = webtest.TestApp.do_request

    def _logged_do_request(self, req, *args, **kwargs):
        """Wrap request and response logging around original do_request."""
        logger.info(">> %s %s", req.method, req.url)
        if req.body:
            logger.info(">> BODY: %s", req.body)
        resp = _orig_do_request(self, req, *args, **kwargs)
        logger.info("<< %s", resp.status)
        logger.info("<< BODY: %s", resp.body)
        return resp

    webtest.TestApp.do_request = _logged_do_request
