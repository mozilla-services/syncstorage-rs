"""Pytest configuration for tokenserver tests, setting up the module path."""

import sys
import os

import pytest

from database import Database

sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))

# Default node used across tokenserver tests.
NODE_URL = "https://phx12"


@pytest.fixture(scope="function")
def db():
    """Per-test Database with service and base node, cleaned up after each test."""
    database = Database()
    # Start with a blank slate.
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    database.add_service("sync-1.5", r"{node}/1.5/{uid}")
    database.add_node(NODE_URL, 100)
    yield database
    # Clean up at the end, for good measure.
    database._execute_sql("DELETE FROM users").close()
    database._execute_sql("DELETE FROM nodes").close()
    database._execute_sql("DELETE FROM services").close()
    database.close()
