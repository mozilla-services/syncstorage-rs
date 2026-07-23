# payload-link-publisher-py

Dev/E2E-only Python variant of the payload-link Dataflow publisher.

**Prod uses the Java flex template one directory up.** This variant
exists so the docker-compose e2e stack can run without a JVM, without
Dataflow, and without minute-scale iteration times.

## Design summary

- **Not Apache Beam.** Python's Beam SDK has no Spanner change-stream
  connector; a Beam pipeline in Python would either require a
  cross-language Java expansion (JVM back in) or a custom Source
  (more code than the whole non-Beam approach). Instead we poll the
  `READ_payload_link_changes` TVF directly via
  `google.cloud.spanner.Client`.
- **Wire-format identical to the Java job.** `serialize_record`
  produces the exact JSON shape `PayloadLinkChangesToPubSub.serializeRecord`
  produces, so the reconciler consumes either interchangeably.
- **Partition tracking.** Follows partition splits: reads `_root`,
  picks up child-partition tokens from any `child_partitions_record`,
  reads each child from its advertised `start_timestamp`, and retires
  a parent when a child announces it. Handles `OUT_OF_RANGE` by
  dropping ended partitions. Required in practice — the Spanner
  emulator routes DataChangeRecords to child partitions almost
  immediately, so a `_root`-only reader sees zero DCRs.
- **In-memory cursor.** On restart, resumes from `now - POLL_OVERLAP_SECONDS`
  on `_root`. Fine for tests (compose brings the pod up alongside the
  stream); would lose events across restarts in prod.
- **Hardcoded row-shape offsets.** Rather than introspect the
  `READ_<change_stream>` TVF's ChangeRecord / DataChangeRecord /
  ChildPartitionsRecord STRUCT layout at runtime, `publisher.py`
  documents the field positions and accesses them positionally. If a
  future Spanner rev reorders the STRUCT, tests fail loudly and one
  offset map gets updated. Acceptable trade-off for a dev/E2E tool.

## Files

| File | Purpose |
|---|---|
| `pyproject.toml` | Poetry project: `google-cloud-spanner`, `google-cloud-pubsub`, `statsd`. |
| `publisher.py` | Entry point: poll TVF → filter → serialize → publish. |
| `utils.py` | `extract_payload_link`, `is_payload_link_actionable` — mirrors the Java filter. |
| `test_publisher.py` | Unit tests for the pure functions. |
| `Dockerfile` | `python:3.13-slim` + poetry install. |

## Environment

| Variable | Required | Default | Notes |
|---|---|---|---|
| `SPANNER_PROJECT_ID` | yes | — | |
| `SPANNER_INSTANCE_ID` | yes | — | |
| `SPANNER_DATABASE_ID` | yes | — | |
| `PUBSUB_PROJECT_ID` | yes | — | |
| `PUBSUB_TOPIC` | yes | — | e.g. `payload-link-changes` |
| `SPANNER_CHANGE_STREAM_NAME` | no | `payload_link_changes` | |
| `HEARTBEAT_MS` | no | `1000` | TVF heartbeat interval. |
| `POLL_OVERLAP_SECONDS` | no | `5` | Restart re-reads this window. |
| `SPANNER_EMULATOR_HOST` | no | — | Auto-honored by `google-cloud-spanner`. |
| `PUBSUB_EMULATOR_HOST` | no | — | Auto-honored by `google-cloud-pubsub`. |
| `STATSD_HOST` / `STATSD_PORT` | no | — | Standard `statsd.defaults.env` pair. |

## Local install / tests

```bash
cd tools/payload-link-dataflow/payload-link-publisher-py
poetry install
poetry run pytest
poetry run mypy .
poetry run ruff check .
```

`poetry.lock` is generated on first install (not yet checked in).

Unit tests cover `extract_payload_link`, `is_payload_link_actionable`,
`serialize_record`, and `_to_json_string`. Not covered by unit tests:
`poll_partition` (needs a live Spanner emulator) and
`publish_if_actionable` (needs a Pub/Sub emulator). Both are
exercised end-to-end by the docker-compose reconciliation stack
(`make docker_run_reconciliation_e2e_tests`).

## Metrics

- `payload_link_publisher.published` — records published to Pub/Sub
- `payload_link_publisher.filtered` — inert records dropped by
  `is_payload_link_actionable`
- `payload_link_publisher.errors.publish` — publish failures

## Related

- Java flex template (prod): `tools/payload-link-dataflow/` (parent dir)
- Change stream definition: `syncstorage-spanner/src/schema.ddl`
  (`payload_link_changes`)
- Downstream reconciler: `tools/payload-reconciler/`
- Runbook: `docs/src/tools/payload_link_reconciler.md`
