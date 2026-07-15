# payload-link-dataflow

Custom Dataflow flex template that reads the `payload_link_changes`
Spanner change stream, drops records whose every mod has `payload_link`
NULL on both sides, and publishes the remainder to a Pub/Sub topic.

The downstream consumer is the `payload-reconciler` cronjob
(`tools/payload-reconciler/`), which finalizes newly-committed GCS
payload objects (`committed=true`, `customTime=MAX`) and deletes
orphaned ones.

## Directory layout

The Java source tree under `src/` is a standalone Apache Beam pipeline
we own and build directly. It is intentionally minimal: it uses Beam's
stock `SpannerIO.readChangeStream` and `PubsubIO.writeStrings`, plus the
filter and a small JSON serializer. We do **not** vendor or depend on
the upstream GoogleCloudPlatform/DataflowTemplates Java tree.

| Path | Purpose |
|---|---|
| `pom.xml` | Maven project (Apache Beam + Jackson + SLF4J). |
| `src/main/java/.../PayloadLinkChangesToPubSub.java` | The pipeline: read change stream → filter → serialize → publish. |
| `src/main/java/.../PayloadLinkOptions.java` | Flex template parameters. |
| `src/test/java/.../PayloadLinkChangesToPubSubTest.java` | Unit tests for the filter predicate. |
| `metadata.json` | Flex template parameter schema (consumed by `gcloud dataflow flex-template build`). |
| `Dockerfile` | Multi-stage build: Maven → flex template launcher base image. |
| `upstream-customization.patch` | **Reference documentation only.** Unified diff showing the **filter delta only** against the upstream template; see "Beyond the filter" below for the rest. Not a build input. |
| `upstream.txt` | The upstream commit SHA the reference patch was last reviewed against. Update when re-reviewing intent against upstream. |
| `generate-full-delta.sh` | On-demand: produces a unified diff of the **complete** delta between upstream (at the pinned SHA) and our impl. Not maintained as a checked-in artifact -- runs against the source tree at execution time. |
| `README.md` | This file. |

## Build

```bash
docker build -t payload-link-dataflow:local tools/payload-link-dataflow
```

`mvn package` runs the unit tests, so a change that breaks the filter
contract fails the image build.

For a local non-Docker build:

```bash
cd tools/payload-link-dataflow
mvn -B package
```

The bundled jar lands at
`target/payload-link-dataflow-bundled-1.0-SNAPSHOT.jar`.

## Publish + launch (operator workflow; lives in webservices-infra)

```bash
docker tag payload-link-dataflow:local <REGISTRY>/payload-link-dataflow:<TAG>
docker push <REGISTRY>/payload-link-dataflow:<TAG>

gcloud dataflow flex-template build \
  gs://<BUCKET>/templates/payload-link-dataflow.json \
  --image <REGISTRY>/payload-link-dataflow:<TAG> \
  --sdk-language JAVA \
  --metadata-file tools/payload-link-dataflow/metadata.json

gcloud dataflow flex-template run payload-link-dataflow-<DATE> \
  --template-file-gcs-location gs://<BUCKET>/templates/payload-link-dataflow.json \
  --parameters spannerProjectId=<PROJECT> \
  --parameters spannerInstanceId=<INSTANCE> \
  --parameters spannerDatabase=<DATABASE> \
  --parameters spannerMetadataInstanceId=<METADATA_INSTANCE> \
  --parameters spannerMetadataDatabase=<METADATA_DATABASE> \
  --parameters changeStreamName=payload_link_changes \
  --parameters pubsubTopic=projects/<PROJECT>/topics/payload-link-changes
```

The Dataflow job's service account needs:
- `roles/spanner.databaseReader` on the syncstorage database
- `roles/spanner.databaseUser` on the metadata database (the change
  stream connector writes its own partition state there)
- `roles/pubsub.publisher` on the destination topic
- `roles/dataflow.worker`

## Filter behaviour

`PayloadLinkChangesToPubSub.isPayloadLinkActionable` keeps a
`DataChangeRecord` when any of its mods has a non-null `payload_link`
on either side. It drops only the case where **every** mod has both
old and new `payload_link` NULL -- the inert INSERT/DELETE noise from
column-scoped change streams. Malformed records pass through so the
reconciler / DLQ -- not this filter -- surfaces them.

## Beyond the filter: other differences from upstream

`upstream-customization.patch` expresses only the filter step. Our
pipeline additionally diverges from upstream's
`Cloud_Spanner_Change_Streams_to_PubSub` template in ways that diff
does not attempt to capture. Run `./generate-full-delta.sh` to see
the complete diff against the pinned upstream SHA; the substantive
differences are:

- **Sink swap.** `PubsubIO.writeStrings` + a custom `serializeRecord`,
  not upstream's `FileFormatFactorySpannerChangeStreamsToPubSub`.
  Emits raw JSON strings; no PubsubMessage attributes.
- **JSON wire format.** Emits only `{commitTimestamp, modType,
  tableName, mods[]}`. Omits fields upstream would emit:
  `serverTransactionId`, `rowType[]`,
  `numberOfRecordsInTransaction`,
  `numberOfPartitionsInTransaction`, `partitionToken`,
  `recordSequence`, `isLastRecordInTransactionInPartition`,
  `valueCaptureType`.
- **No Runner V2 experiment auto-injection.** Upstream mutates
  `options.experiments` to append `use_runner_v2`; we don't.
- **No `UncaughtExceptionLogger.register()`.** Uncaught exceptions
  flow to the runner's default handler.
- **No `@Template` / `TemplateCategory` annotations.** Upstream uses
  the `com.google.cloud.teleport.metadata` annotation-driven
  registrar for flex-template metadata. We register via
  `metadata.json` + `gcloud dataflow flex-template build` instead --
  simpler, no annotation-processor dependency.
- **No support for these upstream options:** `spannerDatabaseRole`,
  `useSpannerEmulatorHost`, `spannerHost`, `spannerMetadataTableName`,
  `spannerChangeStreamTvfNameList`, `outputMessageMetadata`,
  `outputDataFormat` (JSON/Avro switch), `pubsubAPI`. Our
  `PayloadLinkOptions` deliberately has a smaller surface (~10
  options vs. upstream's ~25).
- **No ValueProvider indirection on options.** Upstream wraps
  several config fields in `ValueProvider` for template
  parameterisation; we take plain strings.
- **No `enableStreamingEngine=true` / `streaming=true` mutation.**
  Beam infers streaming from the source; we don't force it.

These deltas were deliberate scope choices, not oversights. The
filter-only patch is kept as the primary review artifact because
it's small enough to eyeball, applies cleanly against the pinned
SHA, and is insensitive to upstream churn on features we don't use.

## Output wire format

Each surviving `DataChangeRecord` is serialized to a JSON Pub/Sub
message of the form:

```json
{
  "commitTimestamp": "2026-06-29T12:34:56.000000000Z",
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

Each mod field carries a **JSON string** (escaped) -- the downstream
Python reconciler reads them with a second `json.loads` per mod. This
matches the Spanner change-streams wire convention.

## Keeping the reference patch accurate

`upstream-customization.patch` is reference documentation, not a build
input. If upstream's pipeline shape evolves or our pipeline drifts in
ways that change the conceptual diff, regenerate it:

```bash
git clone https://github.com/GoogleCloudPlatform/DataflowTemplates.git upstream
cd upstream
git checkout <new-sha>
# Manually re-author the filter customization against the current
# upstream SpannerChangeStreamsToPubSub.java (in our src/ we own the
# whole pipeline; here we only need to express the *delta*).
git diff > ../tools/payload-link-dataflow/upstream-customization.patch
echo "<new-sha>" > ../tools/payload-link-dataflow/upstream.txt
```

After updating the SHA, run `./generate-full-delta.sh` and skim the
output to check that our impl's other divergences (sink swap, JSON
shape, dropped options -- see "Beyond the filter" above) still make
sense against upstream's new state. If upstream restructured
something we deliberately dropped, update the "Beyond the filter"
bullets accordingly.

This is purely a documentation refresh -- it does not affect the build.
