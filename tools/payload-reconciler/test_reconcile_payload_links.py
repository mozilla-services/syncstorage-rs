"""Tests for the per-message handler in reconcile_payload_links.py.

The handler is pure (no Pub/Sub IO), so tests mock the GCS client and
feed synthesized DataChangeRecord-shaped JSON bodies. Cases mirror the
shapes the Dataflow filter passes through.
"""

import json
from unittest.mock import MagicMock

import pytest
from google.api_core import exceptions as gax_exceptions

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


def _msg(
    mods: list[dict[str, str]], table: str = "bsos", transaction_tag: str = ""
) -> bytes:
    return json.dumps(
        {
            "commitTimestamp": "2026-06-29T00:00:00Z",
            "modType": "UPDATE",
            "tableName": table,
            "transactionTag": transaction_tag,
            "mods": mods,
        }
    ).encode()


@pytest.fixture
def statsd_incr(monkeypatch: pytest.MonkeyPatch) -> MagicMock:
    incr = MagicMock()
    monkeypatch.setattr(reconciler.metrics, "incr", incr)
    return incr


def _mod(old: str | None, new: str | None) -> dict[str, str]:
    """Build a mod dict in the wire shape (string-valued JSON fields)."""

    def encode(link: str | None) -> str:
        if link is None:
            return "{}"
        return json.dumps({"payload_link": link})

    return {"keys": "{}", "oldValues": encode(old), "newValues": encode(new)}


def test_insert_with_link_finalizes(statsd_incr: MagicMock) -> None:
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, LINK_A)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_called_once()
    blob.delete.assert_not_called()
    assert blob.metadata == {"committed": "true"}
    assert blob.custom_time == reconciler.MAX_CUSTOM_TIME
    statsd_incr.assert_any_call("finalizes")


def test_delete_with_old_link_deletes(statsd_incr: MagicMock) -> None:
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, None)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.delete.assert_called_once()
    blob.patch.assert_not_called()
    statsd_incr.assert_any_call("orphan_deletes")


def test_update_replace_does_both(statsd_incr: MagicMock) -> None:
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, LINK_B)]))

    # One blob created per .blob(name) call. Both ops happened.
    assert gcs.bucket.return_value.blob.call_count == 2
    statsd_incr.assert_any_call("finalizes")
    statsd_incr.assert_any_call("orphan_deletes")


def test_unchanged_link_finalizes_only_no_delete(statsd_incr: MagicMock) -> None:
    """Old == new: finalize only; the object is still referenced."""
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, LINK_A)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_called_once()
    blob.delete.assert_not_called()


def test_both_null_records_noop_skip(statsd_incr: MagicMock) -> None:
    """Inert noise that the Dataflow filter should have dropped."""
    gcs = _gcs_mock()

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, None)]))

    blob = gcs.bucket.return_value.blob.return_value
    blob.patch.assert_not_called()
    blob.delete.assert_not_called()
    statsd_incr.assert_any_call("noop_skips")


def test_batch_commit_handoff_is_skipped(statsd_incr: MagicMock) -> None:
    """A batch_bsos removal tagged as the batch commit is skipped, never deleted.

    On commit the link moves into bsos in the same transaction, so its object is
    still live (STOR-657); the batch commit transaction tag identifies it.
    """
    gcs = _gcs_mock()

    reconciler.handle_message_body(
        gcs,
        BUCKET,
        _msg(
            [_mod(LINK_A, None)],
            table="batch_bsos",
            transaction_tag=reconciler.BATCH_COMMIT_TRANSACTION_TAG,
        ),
    )

    blob = gcs.bucket.return_value.blob.return_value
    blob.delete.assert_not_called()
    blob.patch.assert_not_called()
    statsd_incr.assert_any_call("batch_commit_skips")


def test_batch_bsos_untagged_removal_deletes(statsd_incr: MagicMock) -> None:
    """An untagged batch_bsos removal is a genuine delete (TTL or storage delete).

    TTL expiry and user_collections deletes reach batch_bsos as cascade deletes
    with no batch commit tag, so the object must be removed. See STOR-668.
    """
    gcs = _gcs_mock()

    reconciler.handle_message_body(
        gcs, BUCKET, _msg([_mod(LINK_A, None)], table="batch_bsos")
    )

    blob = gcs.bucket.return_value.blob.return_value
    blob.delete.assert_called_once()
    statsd_incr.assert_any_call("orphan_deletes")


def test_batch_bsos_overwrite_still_deletes(statsd_incr: MagicMock) -> None:
    """Re-appending the same id in an open batch (link A to B) orphans A.

    That is a client UPDATE, not a removal, so A must still be deleted and B
    finalized; the commit-handoff skip only covers removals.
    """
    gcs = _gcs_mock()

    reconciler.handle_message_body(
        gcs, BUCKET, _msg([_mod(LINK_A, LINK_B)], table="batch_bsos")
    )

    assert gcs.bucket.return_value.blob.call_count == 2
    statsd_incr.assert_any_call("finalizes")
    statsd_incr.assert_any_call("orphan_deletes")


def test_finalize_404_is_success(statsd_incr: MagicMock) -> None:
    """A 404 on patch is treated as success (idempotency)."""
    gcs = _gcs_mock()
    gcs.bucket.return_value.blob.return_value.patch.side_effect = (
        gax_exceptions.NotFound("gone")
    )

    # Should not raise.
    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(None, LINK_A)]))

    statsd_incr.assert_any_call("gcs_404", tags=["op:finalize"])


def test_delete_404_is_success(statsd_incr: MagicMock) -> None:
    """A 404 on delete is treated as success (idempotency)."""
    gcs = _gcs_mock()
    gcs.bucket.return_value.blob.return_value.delete.side_effect = (
        gax_exceptions.NotFound("gone")
    )

    reconciler.handle_message_body(gcs, BUCKET, _msg([_mod(LINK_A, None)]))

    statsd_incr.assert_any_call("gcs_404", tags=["op:delete"])


def test_cross_bucket_link_is_rejected() -> None:
    """A payload_link referencing a different bucket aborts the message."""
    gcs = _gcs_mock()

    with pytest.raises(ValueError, match="refusing cross-bucket op"):
        reconciler.handle_message_body(
            gcs, BUCKET, _msg([_mod(None, "gs://other-bucket/u/c/b/uuid")])
        )


def test_multiple_mods_handled_independently(statsd_incr: MagicMock) -> None:
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
        if call.args and call.args[0] == "finalizes"
    )
    delete_count = sum(
        1
        for call in statsd_incr.call_args_list
        if call.args and call.args[0] == "orphan_deletes"
    )
    assert finalize_count == 2  # LINK_B from mod 1; LINK_A from mod 2
    assert delete_count == 2  # LINK_A from mod 1; LINK_B from mod 3
