# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.

import pytest
from unittest.mock import Mock, MagicMock, patch
from argparse import Namespace

from purge_ttl import (
    add_conditions,
    get_expiry_condition,
    parse_args_list,
    get_db_engine,
    exec_delete,
)


class TestParseArgsList:
    def test_empty_string(self):
        assert parse_args_list("") == []

    def test_empty_list(self):
        assert parse_args_list("[]") == []

    def test_single_item(self):
        assert parse_args_list("123") == ["123"]

    def test_single_item_in_list(self):
        assert parse_args_list("[foo]") == ["foo"]

    def test_multiple_items(self):
        assert parse_args_list("[1,wibble,quux]") == ["1", "wibble", "quux"]


class TestAddConditions:
    def test_no_conditions(self):
        args = Namespace(collection_ids=[])
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"

        actual_query, actual_params = add_conditions(args, query)

        assert actual_query == "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"
        assert actual_params == {}

    def test_single_collection_id(self):
        args = Namespace(collection_ids=["5"])
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"

        actual_query, actual_params = add_conditions(args, query)

        assert "collection_id = :collection_id" in actual_query
        assert actual_params == {"collection_id": "5"}

    def test_multiple_collection_ids(self):
        args = Namespace(collection_ids=["6", "7"])
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"

        actual_query, actual_params = add_conditions(args, query)

        assert "collection_id IN" in actual_query
        assert ":collection_id_0" in actual_query
        assert ":collection_id_1" in actual_query
        assert actual_params == {
            "collection_id_0": "6",
            "collection_id_1": "7",
        }

    def test_filters_empty_collection_ids(self):
        args = Namespace(collection_ids=["1", "", "23"])
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"

        _, actual_params = add_conditions(args, query)

        assert actual_params == {"collection_id_0": "1", "collection_id_1": "23"}


class TestGetExpiryCondition:
    def test_expiry_mode_now(self):
        args = Namespace(expiry_mode="now")
        actual_condition = get_expiry_condition(args)
        assert actual_condition == "expiry < CURRENT_TIMESTAMP"

    def test_expiry_mode_midnight(self):
        args = Namespace(expiry_mode="midnight")
        actual_condition = get_expiry_condition(args)
        assert "DATE_TRUNC" in actual_condition
        assert "UTC" in actual_condition

    def test_invalid_expiry_mode(self):
        args = Namespace(expiry_mode="invalid")
        with pytest.raises(Exception, match="Invalid expiry mode"):
            get_expiry_condition(args)


class TestGetDbEngine:
    @patch('purge_ttl.sqlalchemy.create_engine')
    def test_postgresql_url(self, mock_create_engine):
        url = "postgresql://root:secretz@localhost/db"
        get_db_engine(url)
        mock_create_engine.assert_called_once_with(url)

    @patch('purge_ttl.sqlalchemy.create_engine')
    def test_postgres_url(self, mock_create_engine):
        url = "postgres://root:secretz@localhost/db"
        get_db_engine(url)
        mock_create_engine.assert_called_once_with("postgresql://root:secretz@localhost/db")

    def test_invalid_scheme(self):
        url = "invalid://leaf@localhost/db"
        with pytest.raises(ValueError, match="Unsupported database scheme"):
            get_db_engine(url)


class TestExecDelete:
    @patch('purge_ttl.statsd')
    def test_dryrun(self, mock_statsd):
        mock_timer = MagicMock()
        mock_statsd.timer.return_value.__enter__ = Mock(return_value=mock_timer)
        mock_statsd.timer.return_value.__exit__ = Mock(return_value=False)

        mock_db_engine = Mock()
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"
        params = {}

        exec_delete(mock_db_engine, "test", query, params, dryrun=True)

        mock_db_engine.connect.assert_not_called()

    @patch('purge_ttl.statsd')
    def test_query_execution(self, mock_statsd):
        mock_timer = MagicMock()
        mock_statsd.timer.return_value.__enter__ = Mock(return_value=mock_timer)
        mock_statsd.timer.return_value.__exit__ = Mock(return_value=False)

        mock_result = Mock()
        mock_result.rowcount = 42

        mock_conn = MagicMock()
        mock_conn.__enter__ = Mock(return_value=mock_conn)
        mock_conn.__exit__ = Mock(return_value=False)
        mock_conn.execute.return_value = mock_result

        mock_transaction = MagicMock()
        mock_transaction.__enter__ = Mock(return_value=mock_transaction)
        mock_transaction.__exit__ = Mock(return_value=False)
        mock_conn.begin.return_value = mock_transaction

        mock_db_engine = Mock()
        mock_db_engine.connect.return_value = mock_conn

        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"
        params = {"collection_id": 12}

        exec_delete(mock_db_engine, "test", query, params, dryrun=False)

        mock_db_engine.connect.assert_called_once()
        mock_conn.execute.assert_called_once()

    @patch('purge_ttl.statsd')
    def test_metrics(self, mock_statsd):
        mock_timer = MagicMock()
        mock_statsd.timer.return_value.__enter__ = Mock(return_value=mock_timer)
        mock_statsd.timer.return_value.__exit__ = Mock(return_value=False)

        mock_db_engine = Mock()
        query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"

        exec_delete(mock_db_engine, "bso", query, dryrun=True)

        mock_statsd.timer.assert_called_once_with("syncstorage.purge_ttl.bso_duration")


class TestIntegration:
    def test_full_query(self):
        args = Namespace(collection_ids=["8"], expiry_mode="now")
        query = "DELETE FROM bso WHERE "
        expiry = get_expiry_condition(args)
        query += expiry

        actual_query, params = add_conditions(args, query)

        assert "DELETE FROM bso WHERE" in actual_query
        assert "expiry < CURRENT_TIMESTAMP" in actual_query
        assert "collection_id = :collection_id" in actual_query

        assert params == {"collection_id": "8"}

    def test_batches_and_bso_queries(self):
        args = Namespace(collection_ids=["1"], expiry_mode="now")

        batches_query = "DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP"
        actual_batches_query, actual_batches_params = add_conditions(args, batches_query)

        bsos_query = "DELETE FROM bso WHERE expiry < CURRENT_TIMESTAMP"
        actual_bsos_query, actual_bso_params = add_conditions(args, bsos_query)

        assert "batches" in actual_batches_query
        assert "bso" in actual_bsos_query
        assert "collection_id = :collection_id" in actual_batches_query
        assert "collection_id = :collection_id" in actual_bsos_query
        assert actual_batches_params == actual_bso_params


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
