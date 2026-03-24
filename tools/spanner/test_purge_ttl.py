import argparse
from types import SimpleNamespace
from unittest import mock
from unittest.mock import MagicMock

import pytest

from tools.spanner import purge_ttl


def test_parse_args_list_single_item() -> None:
    """A bare value (no brackets) is returned as a one-element list."""
    assert purge_ttl.parse_args_list("foo") == ["foo"]


def test_parse_args_list_multiple_items() -> None:
    """A bracketed comma-separated string is split into individual items."""
    assert purge_ttl.parse_args_list("[a,b,c]") == ["a", "b", "c"]


def test_get_expiry_condition_now() -> None:
    """'now' mode compares expiry against the current Spanner timestamp."""
    args = argparse.Namespace(expiry_mode="now")
    assert purge_ttl.get_expiry_condition(args) == "expiry < CURRENT_TIMESTAMP()"


def test_get_expiry_condition_midnight() -> None:
    """'midnight' mode truncates the comparison to the start of the UTC day."""
    args = argparse.Namespace(expiry_mode="midnight")
    assert (
        purge_ttl.get_expiry_condition(args)
        == 'expiry < TIMESTAMP_TRUNC(CURRENT_TIMESTAMP(), DAY, "UTC")'
    )


def test_get_expiry_condition_invalid() -> None:
    """An unrecognised expiry mode raises an exception."""
    args = argparse.Namespace(expiry_mode="invalid")
    with pytest.raises(Exception):
        purge_ttl.get_expiry_condition(args)


def test_add_conditions_no_collections_no_prefix() -> None:
    """No collection IDs and no prefix leaves the query and params unchanged."""
    args = argparse.Namespace(collection_ids=[], uid_prefixes=None)
    query, params, types = purge_ttl.add_conditions(
        args, "SELECT * FROM foo WHERE 1=1", None
    )
    assert query == "SELECT * FROM foo WHERE 1=1"
    assert params == {}
    assert types == {}


def test_add_conditions_with_collections_single() -> None:
    """A single collection ID adds an equality condition with an INT64 param type."""
    args = argparse.Namespace(collection_ids=["123"])
    query, params, types = purge_ttl.add_conditions(
        args, "SELECT * FROM foo WHERE 1=1", None
    )
    assert "collection_id = @collection_id" in query
    assert params["collection_id"] == "123"
    assert types["collection_id"] == purge_ttl.param_types.INT64


def test_add_conditions_with_collections_multiple() -> None:
    """Multiple collection IDs add an IN condition with per-ID named params."""
    args = argparse.Namespace(collection_ids=["1", "2"])
    query, params, types = purge_ttl.add_conditions(
        args, "SELECT * FROM foo WHERE 1=1", None
    )
    assert "collection_id in" in query
    assert params["collection_id_0"] == "1"
    assert params["collection_id_1"] == "2"
    assert types["collection_id_0"] == purge_ttl.param_types.INT64


def test_add_conditions_with_prefix() -> None:
    """A UID prefix adds a STARTS_WITH condition with a STRING param type."""
    args = argparse.Namespace(collection_ids=[])
    query, params, types = purge_ttl.add_conditions(
        args, "SELECT * FROM foo WHERE 1=1", "abc"
    )
    assert "STARTS_WITH(fxa_uid, @prefix)" in query
    assert params["prefix"] == "abc"
    assert types["prefix"] == purge_ttl.param_types.STRING


@mock.patch("tools.spanner.purge_ttl.statsd")
def test_deleter_dryrun(statsd_mock: MagicMock) -> None:
    """In dryrun mode the database is never contacted."""
    database = mock.Mock()
    statsd_mock.timer.return_value.__enter__.return_value = None
    statsd_mock.timer.return_value.__exit__.return_value = None
    purge_ttl.deleter(database, "batches", "DELETE FROM batches", dryrun=True)
    database.execute_partitioned_dml.assert_not_called()


@mock.patch("tools.spanner.purge_ttl.statsd")
def test_deleter_executes(statsd_mock: MagicMock) -> None:
    """Without dryrun, execute_partitioned_dml is called exactly once."""
    database = mock.Mock()
    statsd_mock.timer.return_value.__enter__.return_value = None
    statsd_mock.timer.return_value.__exit__.return_value = None
    database.execute_partitioned_dml.return_value = 42

    purge_ttl.deleter(database, "batches", "DELETE FROM batches", dryrun=False)
    database.execute_partitioned_dml.assert_called_once()


@mock.patch("tools.spanner.purge_ttl.deleter")
@mock.patch("tools.spanner.purge_ttl.add_conditions")
@mock.patch("tools.spanner.purge_ttl.get_expiry_condition")
@mock.patch("tools.spanner.purge_ttl.client")
def test_spanner_purge_both(
    client_mock: MagicMock,
    get_expiry_condition_mock: MagicMock,
    add_conditions_mock: MagicMock,
    deleter_mock: MagicMock,
) -> None:
    """spanner_purge calls deleter twice (batches + bso) when mode='both'."""
    args = argparse.Namespace(
        instance_id="inst",
        database_id="db",
        expiry_mode="now",
        auto_split=None,
        uid_prefixes=None,
        mode="both",
        dryrun=True,
        collection_ids=[],
    )

    instance = mock.Mock()
    database = mock.Mock()
    client_mock.instance.return_value = instance
    instance.database.return_value = database

    get_expiry_condition_mock.return_value = "expiry < CURRENT_TIMESTAMP()"
    add_conditions_mock.side_effect = [
        ("batch_query", {"a": 1}, {"a": 2}),
        ("bso_query", {"b": 3}, {"b": 4}),
    ]
    purge_ttl.spanner_purge(args)

    assert deleter_mock.call_count == 2
    deleter_mock.assert_any_call(
        database,
        name="batches",
        query="batch_query",
        params={"a": 1},
        param_types={"a": 2},
        prefix=None,
        dryrun=True,
    )
    deleter_mock.assert_any_call(
        database,
        name="bso",
        query="bso_query",
        params={"b": 3},
        param_types={"b": 4},
        prefix=None,
        dryrun=True,
    )


@mock.patch("argparse.ArgumentParser.parse_args")
def test_get_args_env_and_dsn(parse_args_mock: MagicMock) -> None:
    """When a DSN URL is provided, get_args overrides instance, database, and project IDs."""
    args = SimpleNamespace(
        instance_id="foo",
        database_id="bar",
        project_id="baz",
        sync_database_url="spanner://projects/proj/instances/inst/databases/db",
        collection_ids=[],
        uid_prefixes=[],
        auto_split=None,
        mode="both",
        expiry_mode="midnight",
        dryrun=False,
    )
    parse_args_mock.return_value = args
    result = purge_ttl.get_args()
    assert result.project_id == "proj"
    assert result.instance_id == "inst"
    assert result.database_id == "db"
