# Poll the payload_link_changes Spanner change stream and publish
# actionable records to Pub/Sub.
#
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/.
"""Dev/E2E-only Python variant of the payload-link Dataflow publisher.

Prod continues to use the Java flex template at ``tools/payload-link-dataflow/``.
This variant exists so the docker-compose e2e stack can run without a
JVM and without Dataflow. See README.md for scope.

Behaviour: polls the ``READ_payload_link_changes`` TVF, follows
partition splits, filters records with no actionable ``payload_link``,
and publishes surviving records to Pub/Sub in the exact JSON shape the
Java publisher produces so the reconciler consumes both identically.

The TVF's row shape is hardcoded below rather than introspected. If a
future Spanner rev changes it, tests will fail loudly and the field
map needs updating in one place -- acceptable for a dev/E2E tool.
"""

import datetime
import json
import logging
import os
import sys
import time
from typing import Any, Iterator

from google.api_core import exceptions as gax_exceptions
from google.cloud import spanner
from google.cloud import pubsub_v1  # type: ignore[attr-defined]
from google.cloud.spanner_v1 import param_types
from statsd.defaults.env import statsd

from utils import is_payload_link_actionable

CHANGE_STREAM_NAME = os.environ.get(
    "SPANNER_CHANGE_STREAM_NAME", "payload_link_changes"
)

logging.basicConfig(
    format='{"datetime": "%(asctime)s", "level": "%(levelname)s", "message": "%(message)s"}',
    stream=sys.stdout,
    level=logging.INFO,
)
log = logging.getLogger("payload-link-publisher")


def get_env() -> dict[str, Any]:
    """Read env vars. Raises ``KeyError`` if a required var is unset."""
    return {
        "spanner_project": os.environ["SPANNER_PROJECT_ID"],
        "spanner_instance": os.environ["SPANNER_INSTANCE_ID"],
        "spanner_database": os.environ["SPANNER_DATABASE_ID"],
        "pubsub_project": os.environ["PUBSUB_PROJECT_ID"],
        "pubsub_topic": os.environ["PUBSUB_TOPIC"],
        "heartbeat_ms": int(os.environ.get("HEARTBEAT_MS", "1000")),
        "poll_overlap_seconds": int(os.environ.get("POLL_OVERLAP_SECONDS", "5")),
        "poll_interval_seconds": float(os.environ.get("POLL_INTERVAL_SECONDS", "2")),
    }


def _to_json_string(value: Any) -> str:
    """Normalize a Spanner ``JSON`` column value to a JSON string.

    The Python Spanner client returns ``JSON`` columns as ``JsonObject``
    (dict-/list-like) values, not strings. The reconciler's wire
    contract expects JSON *strings* (matching what the Java flex
    template emits and Spanner's change-streams wire format), so
    encode when necessary.
    """
    if value is None or value == "":
        return "{}"
    if isinstance(value, str):
        return value
    try:
        return json.dumps(value)
    except TypeError:
        return "{}"


def serialize_record(dcr: dict[str, Any]) -> str:
    """Serialize a DataChangeRecord to the reconciler's expected JSON shape.

    Matches ``PayloadLinkChangesToPubSub.serializeRecord`` (Java):

        {
          "commitTimestamp": "…",
          "modType": "INSERT|UPDATE|DELETE",
          "tableName": "bsos|batch_bsos",
          "mods": [
            {"keys": "<json-string>", "oldValues": "<json-string>", "newValues": "<json-string>"}
          ]
        }

    The mod-value fields carry JSON *strings* (double-parsed by the
    reconciler), matching Spanner's change-streams wire convention.
    """
    commit_ts = dcr.get("commit_timestamp")
    if isinstance(commit_ts, datetime.datetime):
        commit_ts_str = commit_ts.isoformat()
    else:
        commit_ts_str = str(commit_ts) if commit_ts is not None else ""

    mods_out: list[dict[str, str]] = []
    for mod in dcr.get("mods", []) or []:
        mods_out.append(
            {
                "keys": _to_json_string(mod.get("keys")),
                "oldValues": _to_json_string(mod.get("old_values")),
                "newValues": _to_json_string(mod.get("new_values")),
            }
        )

    payload = {
        "commitTimestamp": commit_ts_str,
        "modType": dcr.get("mod_type") or "",
        "tableName": dcr.get("table_name") or "",
        "mods": mods_out,
    }
    return json.dumps(payload)


# --- Change-stream row shape (hardcoded, observed on Spanner emulator
# v1.5.52 and matches Cloud Spanner's documented ChangeRecord shape) ---
#
# Row: (ChangeRecord ARRAY<STRUCT<...>>,)
#
#   ChangeRecord STRUCT[3]:
#     [0] data_change_record        ARRAY<STRUCT>  (DCRs when this row is a data change)
#     [1] heartbeat_record          ARRAY<STRUCT>  (unused here)
#     [2] child_partitions_record   ARRAY<STRUCT>  (partition splits)
#
#   DataChangeRecord STRUCT (only the fields we consume are named):
#     [0] commit_timestamp                          TIMESTAMP
#     [4] table_name                                STRING
#     [6] mods                                      ARRAY<STRUCT<Mod>>
#     [7] mod_type                                  STRING (INSERT|UPDATE|DELETE)
#
#   Mod STRUCT: [keys JSON, new_values JSON, old_values JSON]
#     (Note: emulator orders new before old; we key by name in the dict.)
#
#   ChildPartitionsRecord STRUCT:
#     [0] start_timestamp           TIMESTAMP  (start_ts for the child partitions)
#     [2] child_partitions          ARRAY<STRUCT<token STRING, parent_partition_tokens ARRAY<STRING>>>

_DCR_COMMIT_TS = 0
_DCR_TABLE_NAME = 4
_DCR_MODS = 6
_DCR_MOD_TYPE = 7

_MOD_KEYS = 0
_MOD_NEW_VALUES = 1
_MOD_OLD_VALUES = 2

_CR_DATA_CHANGE_RECORD = 0
_CR_CHILD_PARTITIONS_RECORD = 2

_CPR_START_TS = 0
_CPR_CHILD_PARTITIONS = 2

_CHILD_TOKEN = 0
_CHILD_PARENT_TOKENS = 1


def _dcr_to_dict(raw_dcr: Any) -> dict[str, Any] | None:
    """Convert a positional DataChangeRecord to the dict the rest of this
    module expects. Returns ``None`` if the record itself is null.
    """
    if raw_dcr is None:
        return None
    mods_out: list[dict[str, Any]] = []
    for raw_mod in raw_dcr[_DCR_MODS] or []:
        if raw_mod is None:
            continue
        mods_out.append(
            {
                "keys": raw_mod[_MOD_KEYS],
                "new_values": raw_mod[_MOD_NEW_VALUES],
                "old_values": raw_mod[_MOD_OLD_VALUES],
            }
        )
    return {
        "commit_timestamp": raw_dcr[_DCR_COMMIT_TS],
        "table_name": raw_dcr[_DCR_TABLE_NAME],
        "mods": mods_out,
        "mod_type": raw_dcr[_DCR_MOD_TYPE],
    }


def _extract_child_partitions(
    cpr_array: Any,
) -> Iterator[tuple[str, datetime.datetime | None, list[str]]]:
    """Yield ``(token, start_ts, parent_tokens)`` for each announced child."""
    for cpr in cpr_array or []:
        if cpr is None:
            continue
        start_ts = (
            cpr[_CPR_START_TS]
            if isinstance(cpr[_CPR_START_TS], datetime.datetime)
            else None
        )
        for child in cpr[_CPR_CHILD_PARTITIONS] or []:
            if child is None:
                continue
            token = child[_CHILD_TOKEN]
            if not token:
                continue
            parents = list(child[_CHILD_PARENT_TOKENS] or [])
            yield (token, start_ts, parents)


def poll_partition(
    database: Any,
    partition_token: str | None,
    start_ts: datetime.datetime,
    heartbeat_ms: int,
) -> Iterator[tuple[str, Any]]:
    """Poll one change-stream partition.

    ``partition_token`` may be ``None`` for the ``_root`` partition.
    Yields ``("dcr", dcr_dict)`` per DataChangeRecord and
    ``("child", (token, start_ts, parent_tokens))`` per announced
    child partition; the caller enqueues children and retires parents.
    """
    sql = f"""
        SELECT *
        FROM READ_{CHANGE_STREAM_NAME}(
            start_timestamp => @start,
            end_timestamp => NULL,
            partition_token => @partition,
            heartbeat_milliseconds => @heartbeat
        )
    """  # nosec B608
    with database.snapshot() as snapshot:
        results = snapshot.execute_sql(
            sql,
            params={
                "start": start_ts,
                "heartbeat": heartbeat_ms,
                "partition": partition_token,
            },
            param_types={
                "start": param_types.TIMESTAMP,
                "heartbeat": param_types.INT64,
                "partition": param_types.STRING,
            },
        )
        for row in results:
            for change_record in row[0] or []:
                if change_record is None:
                    continue
                for raw_dcr in change_record[_CR_DATA_CHANGE_RECORD] or []:
                    dcr = _dcr_to_dict(raw_dcr)
                    if dcr is not None:
                        yield ("dcr", dcr)
                for child in _extract_child_partitions(
                    change_record[_CR_CHILD_PARTITIONS_RECORD]
                ):
                    yield ("child", child)


def publish_if_actionable(
    publisher: pubsub_v1.PublisherClient, topic_path: str, dcr: dict[str, Any]
) -> None:
    """Publish the record iff at least one mod has an actionable payload_link."""
    mods = dcr.get("mods", []) or []
    if not is_payload_link_actionable(mods):
        statsd.incr("payload_link_publisher.filtered")
        return
    data = serialize_record(dcr).encode("utf-8")
    future = publisher.publish(topic_path, data)
    future.result(timeout=30)
    statsd.incr("payload_link_publisher.published")


def main() -> None:
    """Run the publisher"""
    env = get_env()

    spanner_client = spanner.Client(project=env["spanner_project"])
    instance = spanner_client.instance(env["spanner_instance"])  # type: ignore[no-untyped-call]
    database = instance.database(env["spanner_database"])

    publisher = pubsub_v1.PublisherClient()
    topic_path = publisher.topic_path(env["pubsub_project"], env["pubsub_topic"])

    log.info(
        "starting: change_stream=%s topic=%s poll_overlap=%ds poll_interval=%.1fs",
        CHANGE_STREAM_NAME,
        topic_path,
        env["poll_overlap_seconds"],
        env["poll_interval_seconds"],
    )

    # Partition tracking. Each partition has its own ``[start_ts,
    # end_ts)`` window. When a partition splits it announces its
    # children (with the ``start_ts`` they should be read from) and
    # then ends. We follow the tree: add children, retire parents,
    # drop anything that reports OUT_OF_RANGE.
    active_partitions: dict[str | None, datetime.datetime | None] = {None: None}

    while True:
        # _root has no advertised start_ts; use now - overlap so we
        # catch anything just landed. Child partitions use their own.
        root_start_ts = datetime.datetime.now(datetime.UTC) - datetime.timedelta(
            seconds=env["poll_overlap_seconds"]
        )

        additions: dict[str, datetime.datetime | None] = {}
        removals: set[str | None] = set()

        for partition, part_start_ts in list(active_partitions.items()):
            read_start_ts = part_start_ts or root_start_ts
            try:
                for kind, item in poll_partition(
                    database, partition, read_start_ts, env["heartbeat_ms"]
                ):
                    if kind == "dcr":
                        try:
                            publish_if_actionable(publisher, topic_path, item)
                        except Exception:
                            log.exception("publish failed for a record")
                            statsd.incr("payload_link_publisher.errors.publish")
                    elif kind == "child":
                        token, child_start_ts, parent_tokens = item
                        if token not in active_partitions:
                            additions[token] = child_start_ts
                        for parent in parent_tokens:
                            if parent in active_partitions:
                                removals.add(parent)
            except gax_exceptions.OutOfRange:
                log.info(
                    "partition ended (OUT_OF_RANGE); dropping partition=%r",
                    partition,
                )
                removals.add(partition)
            except Exception:
                log.exception(
                    "change-stream poll cycle failed on partition=%r",
                    partition,
                )
                statsd.incr("payload_link_publisher.errors.poll")

        if additions:
            log.info(
                "discovered %d new child partition(s): %s",
                len(additions),
                sorted(additions.keys()),
            )
            active_partitions.update(additions)
        for retired in removals:
            active_partitions.pop(retired, None)

        time.sleep(env["poll_interval_seconds"])


if __name__ == "__main__":
    log.info("starting publisher.py")
    main()
    log.info("publisher.py exited (never expected)")
