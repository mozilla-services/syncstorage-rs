# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this file,
# You can obtain one at http://mozilla.org/MPL/2.0/.
"""End-to-end tests for the payload-link reconciliation pipeline.

Requires the docker-compose.e2e.reconciliation.yaml stack: syncserver
with GCS offload enabled, fake-gcs-server, Pub/Sub emulator, the
payload-link publisher (Python by default, optionally Java), and the
reconciler running in long-running mode.

These tests self-skip when ``GCS_PAYLOAD_BUCKET`` is unset in the
environment -- so the same file lives happily under
``tools/integration_tests/`` and gets picked up by the existing
spanner e2e suite without failing (skipped entries are visible signal,
not noise). The reconciliation compose overlay sets that env var on
the ``e2e-tests`` container so the tests un-skip there.
"""

import datetime
import json
import os
import time
from typing import Any, Callable, TypeVar

import pytest
from google.cloud import storage

pytestmark = pytest.mark.skipif(
    not os.environ.get("GCS_PAYLOAD_BUCKET"),
    reason="requires the reconciliation compose stack (GCS_PAYLOAD_BUCKET env var)",
)


PAYLOAD_BUCKET = os.environ.get("GCS_PAYLOAD_BUCKET", "")
PAYLOAD_PREFIX = os.environ.get("GCS_PAYLOAD_PREFIX", "v1")
POLL_TIMEOUT_SECONDS = int(os.environ.get("RECONCILER_POLL_TIMEOUT", "30"))
POLL_INTERVAL_SECONDS = 0.5

# Standard sync collection also present in this stack's offload list.
OFFLOAD_COLLECTION = "crypto"

# Standard sync collection deliberately *absent* from this stack's
# SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS. Its rows never carry a
# payload_link, so syncserver excludes its write transactions from the change
# stream. Keep it out of that compose setting or
# test_non_offloaded_writes_are_excluded stops testing anything.
INLINE_COLLECTION = "bookmarks"

CHANGE_STREAM_NAME = os.environ.get(
    "SPANNER_CHANGE_STREAM_NAME", "payload_link_changes"
)

# Back-date the change-stream read window by this much. Commit timestamps come
# from Spanner's clock and the window's from ours; the containers share the
# host clock so any skew is tiny, but a window that starts a moment late would
# silently miss the records under test. Mirrors the publisher's
# POLL_OVERLAP_SECONDS. Overlapping only pulls in unrelated earlier records,
# which the BSO-id filter drops.
CHANGE_STREAM_OVERLAP = datetime.timedelta(seconds=5)

# Positional indices into the READ_<stream> TVF row structs. Mirrors the row
# shape documented in
# tools/payload-link-dataflow/payload-link-publisher-py/publisher.py -- see the
# comment block there for the full layout.
_CR_DATA_CHANGE_RECORD = 0
_CR_CHILD_PARTITIONS_RECORD = 2
_CPR_START_TS = 0
_CPR_CHILD_PARTITIONS = 2
_CHILD_TOKEN = 0
_DCR_MODS = 6
_MOD_KEYS = 0

# Large enough that the offload code path is exercised regardless of
# any future size-threshold gate on the write side.
LARGE_PAYLOAD = "x" * 300_000


T = TypeVar("T")


@pytest.fixture(scope="module")
def gcs() -> storage.Client:
    """Storage client pointed at fake-gcs-server via STORAGE_EMULATOR_HOST."""
    return storage.Client()


@pytest.fixture(scope="module")
def spanner_database() -> Any:
    """Spanner database handle for reading the change stream directly.

    Skips rather than fails where the stack does not wire Spanner in, so
    this file stays usable from the plain spanner e2e suite. The import is
    local for the same reason: a missing google-cloud-spanner must not break
    collection of the other tests in this module.
    """
    if not os.environ.get("SPANNER_EMULATOR_HOST"):
        pytest.skip("requires SPANNER_EMULATOR_HOST (reconciliation compose stack)")
    try:
        from google.cloud import spanner
    except ImportError:
        pytest.skip("requires google-cloud-spanner")
    client = spanner.Client(project=os.environ["SPANNER_PROJECT_ID"])
    instance = client.instance(os.environ["SPANNER_INSTANCE_ID"])
    return instance.database(os.environ["SPANNER_DATABASE_ID"])


def _wait_for(predicate: Callable[[], T], description: str) -> T:
    """Poll ``predicate`` until truthy or timeout; return its final value."""
    deadline = time.monotonic() + POLL_TIMEOUT_SECONDS
    last: T | None = None
    while time.monotonic() < deadline:
        last = predicate()
        if last:
            return last
        time.sleep(POLL_INTERVAL_SECONDS)
    pytest.fail(f"timeout waiting for: {description}")


def _put_bso(st_ctx: dict[str, Any], bso_id: str, payload: str = LARGE_PAYLOAD) -> None:
    from tools.integration_tests.helpers import retry_put_json

    retry_put_json(
        st_ctx["app"],
        f"{st_ctx['root']}/storage/{OFFLOAD_COLLECTION}/{bso_id}",
        {"payload": payload, "sortindex": 1},
    )


def _list_blobs(gcs: storage.Client, prefix: str) -> list[storage.Blob]:
    return list(gcs.bucket(PAYLOAD_BUCKET).list_blobs(prefix=prefix))


def _prefix_for(st_ctx: dict[str, Any]) -> str:
    return f"{PAYLOAD_PREFIX}/{st_ctx['fxa_uid']}/"


def _change_records(
    database: Any,
    start_ts: datetime.datetime,
    end_ts: datetime.datetime,
) -> list[Any]:
    """Collect raw DataChangeRecords from the change stream in a time window.

    A change-stream query hands back child partition tokens rather than all
    the data, so the partition tree has to be walked. The bounded
    ``end_timestamp`` is what makes each query terminate -- the publisher's
    equivalent (``poll_partition``) passes NULL and streams forever by
    design.
    """
    from google.cloud.spanner_v1 import param_types

    sql = f"""
        SELECT * FROM READ_{CHANGE_STREAM_NAME}(
            start_timestamp => @start,
            end_timestamp => @end,
            partition_token => @partition,
            heartbeat_milliseconds => 1000
        )
    """  # nosec B608
    records: list[Any] = []
    # (token, start_ts); None is the root partition.
    pending: list[tuple[str | None, datetime.datetime]] = [(None, start_ts)]
    seen: set[str | None] = set()
    while pending:
        token, partition_start = pending.pop()
        if token in seen:
            continue
        seen.add(token)
        with database.snapshot() as snapshot:
            rows = snapshot.execute_sql(
                sql,
                params={
                    "start": partition_start,
                    "end": end_ts,
                    "partition": token,
                },
                param_types={
                    "start": param_types.TIMESTAMP,
                    "end": param_types.TIMESTAMP,
                    "partition": param_types.STRING,
                },
            )
            for row in rows:
                for change_record in row[0] or []:
                    if change_record is None:
                        continue
                    for dcr in change_record[_CR_DATA_CHANGE_RECORD] or []:
                        if dcr is not None:
                            records.append(dcr)
                    for cpr in change_record[_CR_CHILD_PARTITIONS_RECORD] or []:
                        if cpr is None:
                            continue
                        child_start = cpr[_CPR_START_TS]
                        if not isinstance(child_start, datetime.datetime):
                            child_start = partition_start
                        for child in cpr[_CPR_CHILD_PARTITIONS] or []:
                            if child is not None and child[_CHILD_TOKEN]:
                                pending.append((child[_CHILD_TOKEN], child_start))
    return records


def _bso_ids(records: list[Any]) -> set[str]:
    """Every BSO id named in these records' primary keys.

    The id is the one field that ties a record back to the write that made
    it without having to resolve collection ids, and the tests use ids unique
    enough not to collide with other users' traffic on the shared stream.
    """
    ids: set[str] = set()
    for dcr in records:
        for mod in dcr[_DCR_MODS] or []:
            if mod is None:
                continue
            keys = mod[_MOD_KEYS]
            if isinstance(keys, str):
                try:
                    keys = json.loads(keys)
                except json.JSONDecodeError:
                    continue
            if not isinstance(keys, dict):
                continue
            # bsos keys on bso_id, batch_bsos on batch_bso_id.
            for field in ("bso_id", "batch_bso_id"):
                value = keys.get(field)
                if value:
                    ids.add(str(value))
    return ids


def test_upload_finalizes_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """A newly-offloaded object gets committed=true + customTime=MAX."""
    bso_id = "reconcile-upload"
    _put_bso(st_ctx, bso_id)

    def committed_blob() -> storage.Blob | None:
        for blob in _list_blobs(gcs, _prefix_for(st_ctx)):
            blob.reload()
            if (blob.metadata or {}).get("committed") == "true":
                return blob
        return None

    blob = _wait_for(committed_blob, "committed=true on new object")
    assert blob is not None
    # customTime should be pinned to the far-future sentinel.
    assert blob.custom_time is not None
    assert blob.custom_time.year == 2200


def test_upload_writes_metadata(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """An offloaded object records which BSO it came from."""
    bso_id = "reconcile-metadata"
    payload = LARGE_PAYLOAD
    _put_bso(st_ctx, bso_id, payload=payload)

    def blob_for_bso() -> storage.Blob | None:
        for blob in _list_blobs(gcs, _prefix_for(st_ctx)):
            blob.reload()
            if (blob.metadata or {}).get("bso_id") == bso_id:
                return blob
        return None

    blob = _wait_for(blob_for_bso, f"object with bso_id={bso_id}")
    assert blob is not None
    metadata = blob.metadata or {}
    assert metadata["bso_id"] == bso_id
    assert metadata["fxa_kid"] == st_ctx["fxa_kid"]
    assert metadata["original_size"] == str(len(payload))
    assert metadata["format_version"] == "1"


def test_update_deletes_old_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """Replacing payload_link deletes the prior object."""
    bso_id = "reconcile-update"
    _put_bso(st_ctx, bso_id, payload="a" * 300_000)
    old = _wait_for(
        lambda: (_list_blobs(gcs, _prefix_for(st_ctx)) or [None])[0],
        "initial object exists",
    )
    assert old is not None

    # Rewrite -- publisher path -> reconciler -> old object deleted, new committed.
    _put_bso(st_ctx, bso_id, payload="b" * 300_000)

    _wait_for(
        lambda: not gcs.bucket(PAYLOAD_BUCKET).blob(old.name).exists(),
        "old object deleted",
    )


def test_delete_removes_object(st_ctx: dict[str, Any], gcs: storage.Client) -> None:
    """Deleting the BSO row deletes its GCS object."""
    bso_id = "reconcile-delete"
    _put_bso(st_ctx, bso_id)
    blob = _wait_for(
        lambda: (_list_blobs(gcs, _prefix_for(st_ctx)) or [None])[0],
        "object exists after upload",
    )
    assert blob is not None

    st_ctx["app"].delete(f"{st_ctx['root']}/storage/{OFFLOAD_COLLECTION}/{bso_id}")

    _wait_for(
        lambda: not gcs.bucket(PAYLOAD_BUCKET).blob(blob.name).exists(),
        "object deleted after row delete",
    )


def test_non_offloaded_writes_are_excluded(
    st_ctx: dict[str, Any], spanner_database: Any
) -> None:
    """A write to a non-offloaded collection never reaches the change stream.

    Syncserver sets ``TransactionOptions.exclude_txn_from_change_streams`` on
    writes to collections outside ``gcs_payload_offload_collections``, and
    ``payload_link_changes`` opts in via ``allow_txn_exclusion`` (schema.ddl).
    Those collections hold no payload_link, so their records would reach the
    publisher only to be filtered out, having cost retention space to get
    there.

    This is the one check that the flag lands where Spanner reads it: it's
    written as an unknown protobuf field (see
    ``EXCLUDE_TXN_FROM_CHANGE_STREAMS_FIELD`` in
    syncstorage-spanner/src/db/db_impl.rs), so a wrong field number would be
    skipped silently and every transaction would keep being captured.

    The inline write goes first on purpose. Once the offloaded record shows
    up, the stream has provably advanced past the inline write's commit, so
    the absence assertion is real rather than a race with how fast records
    become visible.
    """
    from tools.integration_tests.helpers import retry_put_json

    start = datetime.datetime.now(datetime.timezone.utc) - CHANGE_STREAM_OVERLAP
    inline_id = "exclusion-probe-inline"
    offloaded_id = "exclusion-probe-offloaded"

    retry_put_json(
        st_ctx["app"],
        f"{st_ctx['root']}/storage/{INLINE_COLLECTION}/{inline_id}",
        {"payload": "inline payload", "sortindex": 1},
    )
    _put_bso(st_ctx, offloaded_id)

    def saw_offloaded() -> set[str] | None:
        now = datetime.datetime.now(datetime.timezone.utc)
        ids = _bso_ids(_change_records(spanner_database, start, now))
        return ids if offloaded_id in ids else None

    ids = _wait_for(saw_offloaded, f"change record for {offloaded_id}")
    assert ids is not None
    assert inline_id not in ids, (
        f"a {INLINE_COLLECTION} write reached the change stream: "
        "exclude_txn_from_change_streams had no effect. Check that the "
        "ALTER CHANGE STREAM setting allow_txn_exclusion in schema.ddl was "
        "applied, and that this Spanner emulator revision honors it."
    )
