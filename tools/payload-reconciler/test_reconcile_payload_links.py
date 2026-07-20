"""Tests for the per-message handler in reconcile_payload_links.py.

The handler is pure (no Pub/Sub IO), so tests mock the GCS client and
feed synthesized DataChangeRecord-shaped JSON bodies. Cases mirror the
shapes the Dataflow filter passes through.
"""

import json
from unittest.mock import MagicMock

import pytest
from google.api_core import exceptions as gax_exceptions
from statsd.defaults.env import statsd as statsd_singleton

import reconcile_payload_links as reconciler

BUCKET = "test-payloads"
LINK_A = f"gs://{BUCKET}/u/c/b/uuid-a"
LINK_B = f"gs://{BUCKET}/u/c/b/uuid-b"


def _gcs_mock() -> MagicMock:
    """Mock storage.Client. All ``.bucket(...).blob(...)`` calls share one
    blob mock, so per-blob assertions in tests see the same object the code
    actually invoked ``.patch()`` / ``.delete()`` on.
    """
    return MagicMock()


def _msg(mods: list[dict[str, str]]) -> bytes:
    return json.dumps(
        {
            "commitTimestamp": "2026-06-29T00:00:00Z",
            "modType": "UPDATE",
            "tableName": "bsos",
            "mods": mods,
        }
    ).encode()


def _mod(old: str | None, new: str | None) -> dict[str, str]:
    """Build a mod dict in the wire shape (string-valued JSON fields)."""

    def encode(link: str | None) -> str:
        if link is None:
            return "{}"
        return json.dumps({"payload_link": link})

    return {"keys": "{}", "oldValues": encode(old), "newValues": encode(new)}


def test_insert_with_link_finalizes(monkeypatch: pytest.MonkeyPatch) -> None:
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, LINK_A)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_called_once()
    blob.delete.assert_not_called()
    assert blob.metadata == {"committed": "true"}
    assert blob.custom_time == reconciler.MAX_CUSTOM_TIME
    statsd_incr.assert_any_call("payload_reconciler.finalizes")


def test_delete_with_old_link_deletes(monkeypatch: pytest.MonkeyPatch) -> None:
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, None)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.delete.assert_called_once()
    blob.patch.assert_not_called()
    statsd_incr.assert_any_call("payload_reconciler.orphan_deletes")


def test_update_replace_does_both(monkeypatch: pytest.MonkeyPatch) -> None:
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, LINK_B)]))

    # One blob created per .blob(name) call. Both ops happened.
    assert gcs.bucket.return_value.blob.call_count == 2
    statsd_incr.assert_any_call("payload_reconciler.finalizes")
    statsd_incr.assert_any_call("payload_reconciler.orphan_deletes")


def test_unchanged_link_finalizes_only_no_delete(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    """Old == new: finalize only; the object is still referenced."""
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, LINK_A)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_called_once()
    blob.delete.assert_not_called()


def test_both_null_records_noop_skip(monkeypatch: pytest.MonkeyPatch) -> None:
    """Inert noise that the Dataflow filter should have dropped."""
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, None)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_not_called()
    blob.delete.assert_not_called()
    statsd_incr.assert_any_call("payload_reconciler.noop_skips")


def test_finalize_404_is_success(monkeypatch: pytest.MonkeyPatch) -> None:
    """A 404 on patch is treated as success (idempotency)."""
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()
    gcs.bucket.return_value.blob.return_value.patch.side_effect = (
        gax_exceptions.NotFound("gone")
    )

    # Should not raise.
    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, LINK_A)]))

    statsd_incr.assert_any_call("payload_reconciler.gcs_404.finalize")


def test_delete_404_is_success(monkeypatch: pytest.MonkeyPatch) -> None:
    """A 404 on delete is treated as success (idempotency)."""
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()
    gcs.bucket.return_value.blob.return_value.delete.side_effect = (
        gax_exceptions.NotFound("gone")
    )

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, None)]))

    statsd_incr.assert_any_call("payload_reconciler.gcs_404.delete")


def test_cross_bucket_link_is_rejected(monkeypatch: pytest.MonkeyPatch) -> None:
    """A payload_link referencing a different bucket aborts the message."""
    gcs = _gcs_mock()

    with pytest.raises(ValueError, match="refusing cross-bucket op"):
        reconciler.handle_message_body(
            gcs, BUCKET, _msg([_mod(None, "gs://other-bucket/u/c/b/uuid")])
        )


def test_multiple_mods_handled_independently(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    statsd_incr = MagicMock()
    monkeypatch.setattr(statsd_singleton, "incr", statsd_incr)
    gcs = _gcs_mock()

    reconciler.handle_message_body(
        gcs,
        BUCKET,
        _msg([_mod(LINK_A, LINK_B), _mod(None, LINK_A), _mod(LINK_B, None)]),
    )

    # One finalize call for each of the new links + the unchanged-same case (none here).
    # Three deletes/finalizes total spread across mods: (delete A, finalize B), (finalize A), (delete B)
    finalize_count = sum(
        1
        for call in statsd_incr.call_args_list
        if call.args and call.args[0] == "payload_reconciler.finalizes"
    )
    delete_count = sum(
        1
        for call in statsd_incr.call_args_list
        if call.args and call.args[0] == "payload_reconciler.orphan_deletes"
    )
    assert finalize_count == 2  # LINK_B from mod 1; LINK_A from mod 2
    assert delete_count == 2  # LINK_A from mod 1; LINK_B from mod 3
