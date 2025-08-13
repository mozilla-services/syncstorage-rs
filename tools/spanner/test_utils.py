import pytest

from tools.spanner.utils import ids_from_env


@pytest.fixture(autouse=True)
def reset_env(monkeypatch):
    # Reset environment variables before each test
    for var in [
        "SYNC_SYNCSTORAGE__DATABASE_URL",
        "INSTANCE_ID",
        "DATABASE_ID",
        "GOOGLE_CLOUD_PROJECT",
    ]:
        monkeypatch.delenv(var, raising=False)


def test_ids_from_env_parses_url(monkeypatch):
    """Test with passed in DSN"""
    monkeypatch.setenv(
        "SYNC_SYNCSTORAGE__DATABASE_URL",
        "spanner://projects/proj/instances/inst/databases/db",
    )
    dsn = "SYNC_SYNCSTORAGE__DATABASE_URL"
    instance_id, database_id, project_id = ids_from_env(dsn)
    assert project_id == "proj"
    assert instance_id == "inst"
    assert database_id == "db"


def test_ids_from_env_with_missing_url(monkeypatch):
    """Test ensures that default env vars set id values."""
    monkeypatch.setenv("INSTANCE_ID", "foo")
    monkeypatch.setenv("DATABASE_ID", "bar")
    monkeypatch.setenv("GOOGLE_CLOUD_PROJECT", "baz")
    instance_id, database_id, project_id = ids_from_env()
    assert instance_id == "foo"
    assert database_id == "bar"
    assert project_id == "baz"


def test_from_env_with_invalid_url(monkeypatch):
    monkeypatch.setenv("SYNC_SYNCSTORAGE__DATABASE_URL", "notaspanner://foo")
    monkeypatch.setenv("INSTANCE_ID", "default")
    monkeypatch.setenv("DATABASE_ID", "default-db")
    monkeypatch.setenv("GOOGLE_CLOUD_PROJECT", "default-proj")

    instance_id, database_id, project_id = ids_from_env()
    assert instance_id == "default"
    assert database_id == "default-db"
    assert project_id == "default-proj"
