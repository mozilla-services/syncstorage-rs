# GCS Payload Reconciliation Pipeline

## Summary

When syncserver offloads a large BSO payload to Google Cloud Storage
(`syncserver/src/web/payload_offload.rs`), the GCS object is created
with custom metadata `committed=false` and `customTime=now`, and the
BSO row's `payload_link` column points at it. A separate pipeline must:

- **Finalize** newly-committed objects — flip metadata to
  `committed=true` and pin `customTime` to its maximum value
  (`9999-12-31T23:59:59Z`) so the bucket's lifecycle policy (see
  below) cannot reclaim them.
- **Garbage-collect orphans** — delete GCS objects whose row's
  `payload_link` was replaced (UPDATE) or removed (DELETE, including
  Spanner row-deletion-policy TTL deletes).

Objects whose syncserver upload failed — either the request never
reached the Spanner commit, or Spanner rolled the write back — *may*
never receive a finalize. Syncserver's write path attempts an
inline best-effort finalize as soon as such a case is detected, but
that attempt can itself fail (transient GCS error, process exit,
etc.), leaving the object stranded at `committed=false` with its
upload-time `customTime`. Anything that slips through is reaped by a
GCS **lifecycle policy** (configured out-of-band in
`webservices-infra/sync`) that deletes objects whose `customTime` is
older than N days. Flipping `customTime` to the max sentinel is what
protects committed objects from that policy — `daysSinceCustomTime`
goes permanently negative once finalized, so the policy cannot touch
them regardless of object age.

This document covers that pipeline. It consumes the
`payload_link_changes` Spanner change stream defined in
`syncstorage-spanner/src/schema.ddl`.

---

## Architecture

```text
Spanner change stream         Custom Dataflow             Pub/Sub                Reconciler
─────────────────────       ─────────────────────       ────────────────       ─────────────────────────
payload_link_changes  ──►   forked flex template   ──►  payload-link-changes ──►  Python cronjob:
(OLD_AND_NEW_VALUES,         (filters out records       topic + DLQ                - new link → finalize object
 7d retention)                with both old & new        pull subscription           (committed=true, customTime=MAX)
                              payload_link NULL)                                   - old link → delete object
                                                                                   - both idempotent
```

## Components

### 1. Spanner change stream — `payload_link_changes`

Defined in `syncstorage-spanner/src/schema.ddl`:

```sql
CREATE CHANGE STREAM payload_link_changes
    FOR bsos(payload_link), batch_bsos(payload_link)
    OPTIONS (
      retention_period = '7d',
      value_capture_type = 'OLD_AND_NEW_VALUES'
    );
```

Column-scoped: an UPDATE that does not touch `payload_link` produces no
record. INSERTs and DELETEs always produce a record, even when
`payload_link` is NULL — those are dropped at the next stage.

The Spanner DDL is not auto-applied; run `gcloud spanner databases ddl
update` against the target database after merging.

### 2. Custom Dataflow flex template — `tools/payload-link-dataflow/`

A standalone Apache Beam pipeline (Java, Beam 2.60.0) that:

1. Reads `payload_link_changes` via `SpannerIO.readChangeStream()`.
2. Applies a `Filter.by(isPayloadLinkActionable)` step that drops
   records whose every mod has `payload_link` NULL on both sides.
   Malformed records pass through so the reconciler / DLQ surfaces
   them — not the filter.
3. Serializes each surviving `DataChangeRecord` to JSON and publishes
   to a Pub/Sub topic.

The pipeline is **not** vendored from
[`GoogleCloudPlatform/DataflowTemplates`](https://github.com/GoogleCloudPlatform/DataflowTemplates).
We own a small standalone source tree under `src/`; the upstream
`Cloud_Spanner_Change_Streams_to_PubSub` template is referenced for
intent comparison via `upstream-customization.patch` (documentation
only — not a build input).

**Build / publish** (operator runs from `webservices-infra`):

```bash
docker build -t <REGISTRY>/syncserver-payload-link-dataflow:<TAG> \
  tools/payload-link-dataflow
docker push <REGISTRY>/syncserver-payload-link-dataflow:<TAG>

gcloud dataflow flex-template build \
  gs://<BUCKET>/templates/syncserver-payload-link-dataflow.json \
  --image <REGISTRY>/syncserver-payload-link-dataflow:<TAG> \
  --sdk-language JAVA \
  --metadata-file tools/payload-link-dataflow/metadata.json
```

**Launch parameters** (full list in
`tools/payload-link-dataflow/metadata.json`):

- `spannerProjectId`, `spannerInstanceId`, `spannerDatabase` — the
  syncstorage Spanner database.
- `spannerMetadataInstanceId`, `spannerMetadataDatabase` — where the
  change-stream connector keeps its partition-state table. Recommend
  a dedicated database in prod for isolation.
- `changeStreamName=payload_link_changes`.
- `pubsubTopic=projects/<PROJECT>/topics/payload-link-changes`.

**Service account requires:**

- `roles/spanner.databaseReader` on the syncstorage database.
- `roles/spanner.databaseUser` on the metadata database.
- `roles/pubsub.publisher` on the destination topic.
- `roles/dataflow.worker`.

### 3. Pub/Sub topic + DLQ

Provisioned from `webservices-infra/sync`:

- Topic: `payload-link-changes`
- Dead-letter topic: `payload-link-changes-dlq`
- Pull subscription: `payload-link-reconciler-sub`
  - 60s ack deadline
  - 7d message retention
  - DLQ routing after 5 delivery attempts

### 4. Reconciler — `tools/payload-reconciler/`

Python script with one job: pull messages, perform GCS operations, ack.
Sync-pull drain loop with two deployment modes selected by whether
`RUN_BUDGET_SECONDS` is set:

- **Cronjob mode** (default deployment): `RUN_BUDGET_SECONDS` set (e.g.
  `240`). The script drains the subscription up to that many seconds
  or until the queue idles, then exits 0. K8s cronjob at ~5 min
  cadence.
- **Long-running mode**: `RUN_BUDGET_SECONDS` unset. The script polls
  forever, never exiting on idle. Deploy as a K8s Deployment when
  finalize-flip latency below the cronjob cadence matters.

**Per-message handling** (`reconcile_payload_links.py:handle_message_body`):

For each mod in the change record:

- New `payload_link` non-null → `blob.patch()` setting
  `metadata.committed = "true"` and `customTime =
  "9999-12-31T23:59:59Z"`.
- Old `payload_link` non-null and ≠ new → `blob.delete()`.

Both operations tolerate `404 NotFound` as success — see *Failure
modes* below.

**Environment**

| Variable | Required | Default | Notes |
|---|---|---|---|
| `PUBSUB_PROJECT_ID` | yes | — | Project hosting the subscription. |
| `PUBSUB_SUBSCRIPTION` | yes | — | `payload-link-reconciler-sub` in prod. |
| `GCS_PAYLOAD_BUCKET` | yes | — | Cross-bucket links abort the message. |
| `RUN_BUDGET_SECONDS` | no | — | Set (e.g. `240`) → cronjob mode; drain up to N seconds then exit 0. Unset → long-running mode; poll forever, never exit on idle. |
| `STATSD_HOST`, `STATSD_PORT` | no | — | Standard `statsd.defaults.env` pair. |

**Service account requires:**

- `roles/pubsub.subscriber` on `payload-link-reconciler-sub`.
- `roles/storage.objectAdmin` on the payload bucket (covers both the
  metadata `patch` and the `delete` operation).

**Deployment.** Default is a K8s cronjob (~5 min cadence) with
`RUN_BUDGET_SECONDS` set. When lower finalize-flip latency matters,
deploy as a K8s Deployment without `RUN_BUDGET_SECONDS` to run
long-running. Manifests live in `webservices-infra/sync`.

---

## Wire format

Each surviving change record reaches the reconciler as a JSON Pub/Sub
message:

```json
{
  "commitTimestamp": "2026-06-30T00:00:00.000000000Z",
  "modType": "UPDATE",
  "tableName": "bsos",
  "mods": [
    {
      "keys": "{\"fxa_uid\":\"...\",\"fxa_kid\":\"...\",\"collection_id\":1,\"bso_id\":\"...\"}",
      "oldValues": "{\"payload_link\":\"gs://bucket/u/c/b/uuid-1\"}",
      "newValues": "{\"payload_link\":\"gs://bucket/u/c/b/uuid-2\"}"
    }
  ]
}
```

Mod fields (`keys`, `oldValues`, `newValues`) carry **JSON strings**
that the reconciler parses with a second `json.loads` — this matches
Spanner's change-streams wire convention.

---

## Failure modes

| Symptom | Cause | Behaviour |
|---|---|---|
| Spike in `payload_reconciler.noop_skips` | Dataflow filter is letting inert records through | Investigate; should be ~0 if the filter works. Records still ack — no harm but extra Pub/Sub cost. |
| Sustained `payload_reconciler.gcs_404.finalize` | Lifecycle rule reclaimed the object before the reconciler finalized it, OR the same message redelivered after a successful prior run (at-least-once tax) | Acceptable up to a low background level. Sharp rise = lifecycle window too aggressive vs. cronjob cadence. |
| Sustained `payload_reconciler.gcs_404.delete` | Object was already deleted (redelivery or concurrent cleanup) | Acceptable; idempotent by design. |
| Messages in `payload-link-changes-dlq` | Repeated handler exceptions on the same message after 5 retries (malformed JSON, cross-bucket link, GCS auth failure) | Inspect the DLQ payload; fix and re-publish or discard. The main subscription continues to drain. |
| `payload_reconciler.errors.handler` non-zero | Same as above before reaching DLQ. | Same. |

A `payload_link` pointing at a bucket other than `GCS_PAYLOAD_BUCKET`
raises `ValueError` and the message is left unacked — it retries up to
the DLQ rather than mutating an unrelated bucket. This is a hard guard.

---

## Keeping the reference patch accurate

`tools/payload-link-dataflow/upstream-customization.patch` is
documentation only — not a build input. If the upstream
`Cloud_Spanner_Change_Streams_to_PubSub` pipeline evolves in ways that
change the conceptual diff, refresh it (see the README in that
directory). This is a doc refresh, not a code change.
