# Onboarding an Offloaded Collection

The offload feature requires the Spanner build.  It applies to every payload in
an opted-in collection, regardless of size.  Existing inline records stay
inline until rewritten.  Enabling payload-offloading has no direct effect on
limits.

## Prerequisites

- Apply the payload-offload DDL in `syncstorage-spanner/src/schema.ddl`.
- Provision the payload bucket, lifecycle policy, IAM, Dataflow job, Pub/Sub
  subscription and reconciler for the target environment. See
  [GCP Infrastructure](../tools/payload-offload-infrastructure.md) and
  [Reconciliation Pipeline](../tools/payload_link_reconciler.md).
- Confirm syncserver can upload, read and delete objects, and the reconciler
  can finalize and delete them. The reconciler's `GCS_PAYLOAD_BUCKET` must match
  syncserver's bucket.

## Configure and deploy

- Set `SYNC_SYNCSTORAGE__GCS_PAYLOAD_BUCKET` to the bucket name (without
  `gs://`) if it's not yet set.
- Append the collection name to
  `SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS`. Use commas without
  spaces, e.g. `bookmarks,newtab-images`.
- If larger payloads are needed, merge an entry into
   `SYNC_SYNCSTORAGE__LIMITS__COLLECTIONS`:

   ```json
   {"newtab-images":{"max_record_payload_bytes":20971520,"max_post_bytes":26214400,"max_request_bytes":26218496}}
   ```

   Unspecified fields inherit global limits.

Deploy, and check whether `GET /1.5/1/info/configuration` advertises the
intended collection overrides, if configured.

See [Configuration](../config.md#syncstorage-payload-off-load) for all settings.

