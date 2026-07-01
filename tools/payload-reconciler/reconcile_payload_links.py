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
from statsd.defaults.env import statsd

from utils import parse_gs_url

# Pinned far-future timestamp -- makes daysSinceCustomTime permanently
# negative so GCS lifecycle rules that GC uncommitted objects by
# daysSinceCustomTime > N can never touch a committed payload
# regardless of object age.
MAX_CUSTOM_TIME = datetime.datetime(
    9999, 12, 31, 23, 59, 59, tzinfo=datetime.timezone.utc
)

# Custom metadata key the syncserver writer sets to "false" on upload;
# the reconciler flips it to "true" once the Spanner write is durable.
# Must match payload_offload.rs::COMMITTED_METADATA_KEY.
COMMITTED_METADATA_KEY = "committed"

logging.basicConfig(
    format='{"datetime": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)
log = logging.getLogger("payload-reconciler")


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


def finalize_object(gcs_client: storage.Client, bucket: str, name: str) -> None:
    """Patch metadata so the object is durable for the lifecycle GC.

    Sets ``committed=true`` and ``customTime=MAX_CUSTOM_TIME`` in one
    GCS round trip. 404 (object already deleted) is treated as success.
    """
    blob = gcs_client.bucket(bucket).blob(name)
    blob.metadata = {COMMITTED_METADATA_KEY: "true"}
    blob.custom_time = MAX_CUSTOM_TIME
    try:
        blob.patch()
        statsd.incr("payload_reconciler.finalizes")
    except gax_exceptions.NotFound:
        log.warning("finalize 404: gs://%s/%s", bucket, name)
        statsd.incr("payload_reconciler.gcs_404.finalize")


def delete_object(gcs_client: storage.Client, bucket: str, name: str) -> None:
    """Delete a GCS object. 404 is treated as success."""
    blob = gcs_client.bucket(bucket).blob(name)
    try:
        blob.delete()
        statsd.incr("payload_reconciler.orphan_deletes")
    except gax_exceptions.NotFound:
        log.warning("delete 404: gs://%s/%s", bucket, name)
        statsd.incr("payload_reconciler.gcs_404.delete")


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
    ops_performed = 0
    for mod in record.get("mods", []):
        old_values_str = mod.get("oldValues") or "{}"
        new_values_str = mod.get("newValues") or "{}"
        old_link = json.loads(old_values_str).get("payload_link")
        new_link = json.loads(new_values_str).get("payload_link")

        if new_link:
            bucket, name = parse_gs_url(new_link)
            _require_bucket(bucket, expected_bucket)
            finalize_object(gcs_client, bucket, name)
            ops_performed += 1

        if old_link and old_link != new_link:
            bucket, name = parse_gs_url(old_link)
            _require_bucket(bucket, expected_bucket)
            delete_object(gcs_client, bucket, name)
            ops_performed += 1

    if ops_performed == 0:
        # Defensive: the Dataflow filter should have dropped this record.
        # Counting noop_skips lets us alert when the filter regresses.
        statsd.incr("payload_reconciler.noop_skips")


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

    processed = 0
    while True:
        if deadline is not None and time.monotonic() >= deadline:
            log.info("budget exhausted after %d messages", processed)
            return

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
                return
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
                statsd.incr("payload_reconciler.errors.handler")

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
