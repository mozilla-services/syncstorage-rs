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

## Offboarding a collection

**Do not remove a collection from
`SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS` until its GCS objects are
gone.**

`SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS` picks which writes reach
the `payload_link_changes` change stream: writes to any collection outside the
list are excluded from it, because such collections hold no `payload_link` and
their change records would only be discarded by the reconciler.

A collection that was offloaded keeps its `payload_link` rows after the name is
removed from the list. From the moment it leaves the list, its deletes stop
producing change records, the reconciler never learns of them, and every one of
its remaining GCS objects is orphaned permanently -- the reconciler is the only
thing that deletes them.

To offboard:

1. Stop writes to the collection, or accept that new writes land inline.
2. Remove the collection's `payload_link` rows (a delete of each BSO, or let
   the row-deletion-policy TTL expire them) and wait for the reconciler to
   delete the corresponding objects. Confirm the bucket prefix is empty.
3. Only then remove the name from
   `SYNC_SYNCSTORAGE__GCS_PAYLOAD_OFFLOAD_COLLECTIONS` and deploy.

If a collection has already been removed prematurely, re-add it, drain it as
above, then remove it again. Objects orphaned in the meantime have to be
reconciled by hand against the `bsos`/`batch_bsos` rows.

See [Configuration](../config.md#syncstorage-payload-off-load) for all settings.

