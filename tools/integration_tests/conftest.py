"""Pytest configuration and fixtures for integration tests."""

import os
import logging

SYNC_SERVER_URL = os.environ.get("SYNC_SERVER_URL", "http://localhost:8000")

logger = logging.getLogger("tokenserver.scripts.conftest")
