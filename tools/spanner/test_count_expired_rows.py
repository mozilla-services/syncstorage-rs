from unittest.mock import MagicMock
import logging

from tools.spanner import count_expired_rows


def test_spanner_read_data_counts_and_logs(monkeypatch, caplog):
    # Prepare mocks
    mock_instance = MagicMock()
    mock_database = MagicMock()
    mock_snapshot_ctx = MagicMock()
    mock_snapshot = MagicMock()
    mock_result = MagicMock()
    mock_result.one.return_value = [42]
    mock_snapshot.execute_sql.return_value = mock_result
    mock_snapshot_ctx.__enter__.return_value = mock_snapshot
    mock_database.snapshot.return_value = mock_snapshot_ctx

    # Patch spanner client and statsd
    monkeypatch.setattr(count_expired_rows, "client", MagicMock())
    count_expired_rows.client.instance.return_value = mock_instance
    mock_instance.database.return_value = mock_database

    mock_statsd = MagicMock()
    monkeypatch.setattr(count_expired_rows, "statsd", mock_statsd)
    mock_statsd.timer.return_value.__enter__.return_value = None
    mock_statsd.timer.return_value.__exit__.return_value = None

    # Patch from_env to return fixed values
    monkeypatch.setattr(
        count_expired_rows, "ids_from_env", lambda: ("inst", "db", "proj")
    )

    # Run function
    with caplog.at_level(logging.INFO):
        count_expired_rows.spanner_read_data("SELECT COUNT(*) FROM foo", "foo")

    # Check logs
    assert any("For inst:db proj" in m for m in caplog.messages)
    assert any("Found 42 expired rows in foo" in m for m in caplog.messages)

    # Check statsd calls
    mock_statsd.gauge.assert_called_with("syncstorage.expired_foo_rows", 42)
    mock_statsd.timer.assert_called_with("syncstorage.count_expired_foo_rows.duration")
    mock_database.snapshot.assert_called_once()
    mock_snapshot.execute_sql.assert_called_with("SELECT COUNT(*) FROM foo")
