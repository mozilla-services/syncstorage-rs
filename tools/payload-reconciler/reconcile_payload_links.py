# Reconcile GCS payload objects against Spanner payload_link changes.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Pub/Sub-driven reconciler for offloaded GCS payload objects.

Consumes the ``payload-link-changes`` Pub/Sub topic populated by the
``payload-link-dataflow`` flex template. For each mod in each change
record:

* New ``payload_link`` non-null: patch the GCS object's metadata to
  ``committed=true`` and ``customTime=MAX`` -- completing the 2-phase
  commit started by the syncserver write path.
* Old ``payload_link`` non-null and not equal to the new value: delete
  the GCS object (now orphaned because the row's link moved or the row
  was removed, including TTL row-deletion-policy deletes).

Both operations are idempotent and the script tolerates GCS 404s as
success, so Pub/Sub at-least-once delivery is safe; no ordering key is
required.

Two deployment modes, selected by ``RUN_BUDGET_SECONDS``:

* Set (K8s cronjob): drain the subscription up to the budget or the
  first idle poll, then exit 0. Cronjob cadence lives in
  webservices-infra.
* Unset (long-running pod): poll forever; never exit on idle. The
  deployment supervises restarts. Useful when finalize latency below
  the cronjob cadence matters.
"""

import datetime
import json
import logging
import os
import sys
import time
from typing import Any

from google.api_core import exceptions as gax_exceptions
from google.cloud import pubsub_v1
from google.cloud import storage

# Run by path (``python3 .../reconcile_payload_links.py``), so ``sys.path[0]``
# is this directory and the shared ``tools/common`` package is not importable
# without help. Add ``tools/`` before importing from it.
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from common.metrics import Metrics  # noqa: E402  (needs the path insert)
from utils import parse_gs_url  # noqa: E402

# Pinned far-future timestamp -- makes daysSinceCustomTime permanently
# negative so GCS lifecycle rules that GC uncommitted objects by
# daysSinceCustomTime > N can never touch a committed payload
# regardless of object age.
MAX_CUSTOM_TIME = datetime.datetime(
    2200, 12, 31, 23, 59, 59, tzinfo=datetime.timezone.utc
)

# Custom metadata key the syncserver writer sets to "false" on upload;
# the reconciler flips it to "true" once the Spanner write is durable.
# Must match payload_offload.rs::COMMITTED_METADATA_KEY.
COMMITTED_METADATA_KEY = "committed"

# Transaction tag syncstorage sets on the batch commit. A batch_bsos row removed
# under this tag is the commit handoff: its payload link just moved into the
# permanent bsos row in the same transaction, so its GCS object must be kept.
# Any other batch_bsos removal (TTL expiry or a user_collections delete, both
# cascade deletes that carry no such tag) is a genuine delete whose object
# should go. Must match BATCH_COMMIT_TRANSACTION_TAG in
# syncserver/src/web/transaction.rs. See STOR-668.
BATCH_COMMIT_TRANSACTION_TAG = "batch_commit"

logging.basicConfig(
    format='{"datetime": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)
log = logging.getLogger("payload-reconciler")

metrics = Metrics(
    namespace="payload_reconciler",
    host=os.environ.get("SYNC_STATSD_HOST") or os.environ.get("STATSD_HOST"),
    port=os.environ.get("SYNC_STATSD_PORT") or os.environ.get("STATSD_PORT"),
)


def get_env() -> tuple[str, str, str, int | None]:
    """Return ``(project, subscription, bucket, run_budget_seconds)``.

    ``run_budget_seconds`` is ``None`` when ``RUN_BUDGET_SECONDS`` is
    unset -- the long-running mode. Raises ``KeyError`` if any of the
    other required env vars is unset.
    """
    project = os.environ["PUBSUB_PROJECT_ID"]
    subscription = os.environ["PUBSUB_SUBSCRIPTION"]
    bucket = os.environ["GCS_PAYLOAD_BUCKET"]
    budget_env = os.environ.get("RUN_BUDGET_SECONDS")
    budget = int(budget_env) if budget_env else None
    return project, subscription, bucket, budget


def parse_commit_timestamp(value: str | None) -> datetime.datetime | None:
    """Parse a change record's ``commitTimestamp``.

    Spanner emits nanosecond precision (``2026-06-30T00:00:00.000000000Z``),
    which ``fromisoformat`` rejects, so the fraction is truncated to the
    microseconds it accepts. Returns ``None`` for a missing or unparseable
    value: the age metric is best effort and must never fail a message.
    """
    if not value:
        return None
    text = value.removesuffix("Z")
    whole, sep, frac = text.partition(".")
    if sep:
        text = f"{whole}.{frac[:6]}"
    try:
        parsed = datetime.datetime.fromisoformat(text)
    except ValueError:
        log.debug("unparseable commitTimestamp: %r", value)
        return None
    # Spanner commit timestamps are UTC; the trailing Z is stripped above.
    if parsed.tzinfo is None:
        parsed = parsed.replace(tzinfo=datetime.timezone.utc)
    return parsed


def _record_gcs_op(started: float, op: str, result: str) -> None:
    """Emit the round-trip time of one GCS call, tagged with ``op``/``result``.

    Separates our own GCS latency from the end-to-end ``finalize_age``, which
    also carries Dataflow and Pub/Sub delay. When the lag climbs, this says
    whether GCS is the reason.

    ``result`` is ``success``, ``not_found``, or ``error``. A 404 is its own
    value rather than folded into success, because it is a different round
    trip: the object was already gone, so there was nothing to write.
    """
    metrics.timing(
        "gcs_op",
        (time.monotonic() - started) * 1000,
        tags=[f"op:{op}", f"result:{result}"],
    )


def _record_finalize_age(commit_timestamp: datetime.datetime | None) -> None:
    """Emit the Spanner-commit-to-finalize lag in milliseconds.

    This is the window the object spends at ``committed=false``, reachable by
    the lifecycle policy. The 30 day policy is sized against it, so it is
    worth measuring rather than inferring from the cronjob cadence.
    """
    if commit_timestamp is None:
        return
    age_ms = (
        datetime.datetime.now(datetime.timezone.utc) - commit_timestamp
    ).total_seconds() * 1000
    if age_ms < 0:
        # Clock skew between Spanner and this pod. A negative lag is not
        # meaningful and would drag the aggregate down.
        log.debug("negative finalize age, skipping: %.0fms", age_ms)
        return
    metrics.timing("finalize_age", age_ms)


def finalize_object(
    gcs_client: storage.Client,
    bucket: str,
    name: str,
    commit_timestamp: datetime.datetime | None = None,
) -> None:
    """Patch metadata so the object is durable for the lifecycle GC.

    Sets ``committed=true`` and ``customTime=MAX_CUSTOM_TIME`` in one
    GCS round trip. 404 (object already deleted) is treated as success.

    ``commit_timestamp``, when supplied, is the change record's commit time
    and drives the ``finalize_age`` timing.
    """
    blob = gcs_client.bucket(bucket).blob(name)
    blob.metadata = {COMMITTED_METADATA_KEY: "true"}
    blob.custom_time = MAX_CUSTOM_TIME
    started = time.monotonic()
    outcome = "error"
    try:
        blob.patch()
        outcome = "success"
        metrics.incr("finalizes")
        _record_finalize_age(commit_timestamp)
    except gax_exceptions.NotFound:
        outcome = "not_found"
        log.debug("finalize 404: gs://%s/%s", bucket, name)
        metrics.incr("gcs_404", tags=["op:finalize"])
    finally:
        # Every outcome is a completed round trip, so time them all.
        _record_gcs_op(started, "finalize", outcome)


def delete_object(gcs_client: storage.Client, bucket: str, name: str) -> None:
    """Delete a GCS object. 404 is treated as success."""
    blob = gcs_client.bucket(bucket).blob(name)
    started = time.monotonic()
    outcome = "error"
    try:
        blob.delete()
        outcome = "success"
        metrics.incr("orphan_deletes")
    except gax_exceptions.NotFound:
        outcome = "not_found"
        log.debug("delete 404: gs://%s/%s", bucket, name)
        metrics.incr("gcs_404", tags=["op:delete"])
    finally:
        _record_gcs_op(started, "delete", outcome)


def _require_bucket(seen: str, expected: str) -> None:
    """Refuse to operate on objects outside the configured payload bucket."""
    if seen != expected:
        raise ValueError(
            f"refusing cross-bucket op: seen bucket={seen!r}, expected={expected!r}"
        )


def handle_message_body(
    gcs_client: storage.Client, expected_bucket: str, body: bytes
) -> None:
    """Process one Pub/Sub message body. Raises on unrecoverable parse errors.

    Idempotent at the GCS layer (set-then-set is a no-op; 404-on-delete is
    success), so re-delivery is safe.
    """
    record: dict[str, Any] = json.loads(body)
    table_name = record.get("tableName")
    transaction_tag = record.get("transactionTag")
    commit_timestamp = parse_commit_timestamp(record.get("commitTimestamp"))

    ops_performed = 0
    for mod in record.get("mods", []):
        old_values_str = mod.get("oldValues") or "{}"
        new_values_str = mod.get("newValues") or "{}"
        old_link = json.loads(old_values_str).get("payload_link")
        new_link = json.loads(new_values_str).get("payload_link")

        if new_link:
            bucket, name = parse_gs_url(new_link)
            _require_bucket(bucket, expected_bucket)
            finalize_object(gcs_client, bucket, name, commit_timestamp)
            ops_performed += 1

        if old_link and old_link != new_link:
            # Skip the delete only for a batch commit handoff: a batch_bsos row
            # removed under the batch commit transaction tag. On commit the link
            # moves into the permanent bsos row in the same transaction, so its
            # object must be kept (deleting it was the STOR-657 bug). Any other
            # batch_bsos removal (TTL expiry or a user_collections delete, both
            # cascade deletes that carry no such tag) is a genuine delete and its
            # object should go. See STOR-668.
            if (
                table_name == "batch_bsos"
                and new_link is None
                and transaction_tag == BATCH_COMMIT_TRANSACTION_TAG
            ):
                metrics.incr("batch_commit_skips")
                # Recognized and intentionally skipped, not filter noise.
                ops_performed += 1
                continue
            bucket, name = parse_gs_url(old_link)
            _require_bucket(bucket, expected_bucket)
            delete_object(gcs_client, bucket, name)
            ops_performed += 1

    if ops_performed == 0:
        # Defensive: the Dataflow filter should have dropped this record.
        # Counting noop_skips lets us alert when the filter regresses.
        metrics.incr("noop_skips")


def drain(
    project: str, subscription: str, bucket: str, budget_seconds: int | None
) -> None:
    """Pull messages with sync-pull.

    ``budget_seconds`` set -- cronjob mode: return when the queue idles
    or the budget elapses (whichever first).
    ``budget_seconds`` None -- long-running mode: poll forever; do not
    exit on idle. Restarts are supervised by the deployment.
    """
    sub_client = pubsub_v1.SubscriberClient()
    sub_path = sub_client.subscription_path(project, subscription)
    gcs_client = storage.Client()

    deadline = time.monotonic() + budget_seconds if budget_seconds is not None else None
    if deadline is not None:
        log.info(
            "draining %s for up to %ds (project=%s, bucket=%s)",
            sub_path,
            budget_seconds,
            project,
            bucket,
        )
    else:
        log.info(
            "draining %s indefinitely (no RUN_BUDGET_SECONDS) (project=%s, bucket=%s)",
            sub_path,
            project,
            bucket,
        )

    # Every other counter here is conditional on the event it names, so a run
    # that finalizes nothing emits nothing at all. `runs` is the unconditional
    # series that makes "quiet" distinguishable from "not running" on a
    # dashboard.
    metrics.incr("runs")
    started = time.monotonic()

    processed = 0
    try:
        processed = _drain_loop(sub_client, sub_path, gcs_client, bucket, deadline)
    finally:
        metrics.timing("drain_duration", (time.monotonic() - started) * 1000)
        metrics.incr("messages_processed", value=processed)


def _drain_loop(
    sub_client: pubsub_v1.SubscriberClient,
    sub_path: str,
    gcs_client: storage.Client,
    bucket: str,
    deadline: float | None,
) -> int:
    """Pull-and-handle until the budget elapses or the queue idles.

    Returns the number of messages processed. Per-message handler errors are
    caught and counted inside the loop, so this only propagates a pull or
    acknowledge failure, in which case the caller reports a count of zero.
    """
    processed = 0
    while True:
        if deadline is not None and time.monotonic() >= deadline:
            log.info("budget exhausted after %d messages", processed)
            # Distinct from an idle exit: a run that keeps ending this way is
            # not keeping up with the queue.
            metrics.incr("budget_exhausted")
            return processed

        try:
            response = sub_client.pull(
                request={
                    "subscription": sub_path,
                    "max_messages": 100,
                    "return_immediately": False,
                },
                timeout=10.0,
            )
        except gax_exceptions.DeadlineExceeded:
            continue

        if not response.received_messages:
            if deadline is not None:
                log.info("queue idle after %d messages; exiting", processed)
                return processed
            # Long-running: keep polling.
            continue

        ack_ids: list[str] = []
        for received in response.received_messages:
            try:
                handle_message_body(gcs_client, bucket, received.message.data)
                ack_ids.append(received.ack_id)
            except Exception:
                log.exception(
                    "handler error on message_id=%s; leaving unacked for retry / DLQ",
                    received.message.message_id,
                )
                metrics.incr("errors", tags=["kind:handler"])

        if ack_ids:
            sub_client.acknowledge(
                request={"subscription": sub_path, "ack_ids": ack_ids}
            )
        processed += len(response.received_messages)


if __name__ == "__main__":
    log.info("starting reconcile_payload_links.py")
    project, subscription, bucket, budget = get_env()
    drain(project, subscription, bucket, budget)
    log.info("completed reconcile_payload_links.py")
