# payload-reconciler

Pub/Sub-driven reconciler that closes the loop on GCS payload offload.

Reads the `payload-link-changes` Pub/Sub topic (populated by the
`tools/payload-link-dataflow/` flex template). For each mod inside each
change record:

- **New `payload_link` non-null** → patch the referenced GCS object's
  metadata to `committed=true` and `customTime=MAX`. Completes the
  2-phase commit started in `syncserver/src/web/payload_offload.rs`.
- **Old `payload_link` non-null and ≠ new** → delete the referenced GCS
  object. It is unreferenced now (the row's `payload_link` moved, was
  cleared, or the row was deleted — including TTL row-deletion-policy
  deletes).

Both operations are idempotent (re-setting metadata is a no-op; 404 on
delete is treated as success), so Pub/Sub at-least-once redelivery is
safe and no ordering key is needed.

## Files

| File | Purpose |
|---|---|
| `pyproject.toml` | Poetry project: `google-cloud-pubsub`, `google-cloud-storage`, `statsd`. |
| `reconcile_payload_links.py` | Entry point: sync-pull drain loop + per-message handler. |
| `utils.py` | `parse_gs_url()` — mirrors the Rust uploader's URL grammar. |
| `test_utils.py` | Tests for `parse_gs_url`. |
| `test_reconcile_payload_links.py` | Handler tests with a mocked GCS client. |

## Environment

| Variable | Required | Notes |
|---|---|---|
| `PUBSUB_PROJECT_ID` | yes | Project hosting the subscription. |
| `PUBSUB_SUBSCRIPTION` | yes | e.g. `payload-link-reconciler-sub`. |
| `GCS_PAYLOAD_BUCKET` | yes | The bucket the syncserver writes payloads into. Cross-bucket links abort the message and route it to the DLQ. |
| `RUN_BUDGET_SECONDS` | no | Selects deployment mode. **Set** (e.g. `240`): cronjob mode -- drain up to that many seconds then exit 0. **Unset**: long-running mode -- poll forever, never exit on idle; the deployment supervises restarts. Cronjob mode is the default deployment. |
| `STATSD_HOST` / `STATSD_PORT` | no | Standard `statsd.defaults.env` env-var pair. |

## Local install / tests

```bash
cd tools/payload-reconciler
poetry install
poetry run pytest
```

Unit tests cover `parse_gs_url` and the per-message handler. The
`drain` loop (sync-pull, budget vs. long-running exit conditions) has
no unit coverage — exercise it against the Pub/Sub emulator when
changing it.

## Metrics

Statsd counters (all under the `payload_reconciler.` prefix):

- `finalizes` — successful `committed=true` + `customTime=MAX` patch
- `orphan_deletes` — successful delete of an unreferenced object
- `noop_skips` — message processed with zero ops. Should be ~0 if the
  Dataflow filter is working; non-zero is the alarm bell that the
  filter regressed.
- `gcs_404.finalize` / `gcs_404.delete` — 404 on the respective op.
  Expected non-zero under at-least-once redelivery (idempotency tax);
  a sharp rise suggests upstream state is gone unexpectedly.
- `errors.handler` — Sustained non-zero values feed the DLQ.

## Deployment

**The reconciler ships in the primary syncserver image**, built from
the root `Dockerfile`. Its script and Python deps are baked in via
that Dockerfile's per-tool poetry-export / pip-wheel / pip-install
chain; K8s picks between the syncserver binary and the reconciler
script via the pod's `command:`.

Default: K8s cronjob (definition lives in webservices-infra), cadence
~5 min, exports `RUN_BUDGET_SECONDS` (e.g. `240`).

Alternative: long-running K8s Deployment. Omit `RUN_BUDGET_SECONDS`;
the pod polls forever. Choose this when finalize-flip latency below
the cronjob cadence matters (e.g. an operator wants offloaded
payloads to become `committed=true` within seconds of upload rather
than up to one cronjob cycle later).

Service account needs the same roles in either mode:

- `roles/pubsub.subscriber` on `payload-link-reconciler-sub`
- `roles/storage.objectAdmin` on the payload bucket (covers both the
  metadata `patch` and `delete` operations)

## Related

- Producer: `syncserver/src/web/payload_offload.rs`
- Change stream definition: `syncstorage-spanner/src/schema.ddl`
  (look for `payload_link_changes`)
- Upstream pipeline: `tools/payload-link-dataflow/`
