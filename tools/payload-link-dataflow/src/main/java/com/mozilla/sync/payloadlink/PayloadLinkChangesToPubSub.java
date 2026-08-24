package com.mozilla.sync.payloadlink;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.cloud.Timestamp;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import org.apache.beam.sdk.Pipeline;
import org.apache.beam.sdk.PipelineResult;
import org.apache.beam.sdk.io.gcp.pubsub.PubsubIO;
import org.apache.beam.sdk.io.gcp.spanner.SpannerConfig;
import org.apache.beam.sdk.io.gcp.spanner.SpannerIO;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.DataChangeRecord;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.Mod;
import org.apache.beam.sdk.options.PipelineOptionsFactory;
import org.apache.beam.sdk.options.ValueProvider.StaticValueProvider;
import org.apache.beam.sdk.transforms.Filter;
import org.apache.beam.sdk.transforms.MapElements;
import org.apache.beam.sdk.values.TypeDescriptors;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/**
 * Reads the {@code payload_link_changes} Spanner change stream, drops
 * records that carry no actionable {@code payload_link} value, and
 * publishes the rest to Pub/Sub as JSON.
 *
 * <p>The output JSON shape is intentionally minimal -- one object per
 * {@code DataChangeRecord} with {@code commitTimestamp}, {@code modType},
 * {@code tableName}, {@code transactionTag}, {@code isSystemTransaction},
 * and a {@code mods} array. Each mod carries {@code keys},
 * {@code oldValues}, and {@code newValues} as raw JSON strings (matching
 * the Spanner change-streams wire format), so the downstream Python
 * reconciler reads them with a second {@code json.loads} per mod. The
 * transaction fields let the reconciler tell a TTL delete
 * ({@code transactionTag} "RowDeletionPolicy", {@code isSystemTransaction}
 * true) from a client-driven batch commit handoff.
 */
public final class PayloadLinkChangesToPubSub {

    private static final Logger LOG =
        LoggerFactory.getLogger(PayloadLinkChangesToPubSub.class);

    private static final ObjectMapper MAPPER = new ObjectMapper();

    public static void main(String[] args) {
        PayloadLinkOptions options = PipelineOptionsFactory
            .fromArgs(args)
            .withValidation()
            .as(PayloadLinkOptions.class);

        // BundleFinalizer (used by SpannerIO.readChangeStream) requires the
        // Dataflow Portable Runner ("Runner V2"), and V2 for streaming jobs
        // requires Streaming Engine. Both are pipeline-shape invariants of
        // this source, not deployment choices -- pinning them here means
        // every launch path (flex-template, java -jar, tests) gets it right
        // without having to remember to pass the flags.
        List<String> experiments = new ArrayList<>(
            Optional.ofNullable(options.getExperiments()).orElse(List.of()));
        if (!experiments.contains("use_runner_v2")) {
            experiments.add("use_runner_v2");
        }
        options.setExperiments(experiments);
        options.setEnableStreamingEngine(true);

        run(options);
    }

    static PipelineResult run(PayloadLinkOptions options) {
        Pipeline pipeline = Pipeline.create(options);

        SpannerConfig spannerConfig = buildSpannerConfig(options);

        Timestamp startTimestamp = options.getStartTimestamp().isEmpty()
            ? Timestamp.now()
            : Timestamp.parseTimestamp(options.getStartTimestamp());
        Timestamp endTimestamp = options.getEndTimestamp().isEmpty()
            ? Timestamp.MAX_VALUE
            : Timestamp.parseTimestamp(options.getEndTimestamp());

        SpannerIO.ReadChangeStream readChangeStream = SpannerIO.readChangeStream()
            .withSpannerConfig(spannerConfig)
            .withMetadataInstance(options.getSpannerMetadataInstanceId())
            .withMetadataDatabase(options.getSpannerMetadataDatabase())
            .withChangeStreamName(options.getChangeStreamName())
            .withInclusiveStartAt(startTimestamp)
            .withInclusiveEndAt(endTimestamp);

        String metadataTable = options.getSpannerMetadataTableName();
        if (!metadataTable.isEmpty()) {
            readChangeStream = readChangeStream.withMetadataTable(metadataTable);
        }

        pipeline
            .apply("Read From Spanner Change Stream", readChangeStream)
            .apply(
                "Filter Payload Link Actionable",
                Filter.by(PayloadLinkChangesToPubSub::isPayloadLinkActionable))
            .apply(
                "Serialize To JSON",
                MapElements
                    .into(TypeDescriptors.strings())
                    .via(PayloadLinkChangesToPubSub::serializeRecord))
            .apply(
                "Write To Pub/Sub",
                PubsubIO.writeStrings().to(options.getPubsubTopic()));

        return pipeline.run();
    }

    /**
     * Builds the change stream reader's {@link SpannerConfig}, optionally
     * assuming a Spanner database role for fine-grained access control.
     *
     * <p>The role scopes the change stream read only. Beam's
     * {@code MetadataSpannerConfigFactory} deliberately does not copy
     * {@code databaseRole} onto the connector's metadata database config, so
     * that connection still authenticates with the job service account's own
     * IAM grants. Per Spanner's FGAC guidance the metadata database therefore
     * has to be a different database than the one being read, otherwise the
     * database-level grant it needs overrides the role's restrictions.
     */
    static SpannerConfig buildSpannerConfig(PayloadLinkOptions options) {
        SpannerConfig spannerConfig = SpannerConfig.create()
            .withProjectId(options.getSpannerProjectId())
            .withInstanceId(options.getSpannerInstanceId())
            .withDatabaseId(options.getSpannerDatabase())
            .withRpcPriority(options.getRpcPriority());

        String databaseRole = options.getSpannerDatabaseRole();
        if (!databaseRole.isEmpty()) {
            spannerConfig =
                spannerConfig.withDatabaseRole(StaticValueProvider.of(databaseRole));
        }

        return spannerConfig;
    }

    /**
     * Returns true iff at least one mod in the record references a non-null
     * {@code payload_link} on either side. Records whose every mod has both
     * old and new {@code payload_link} NULL are inert for the reconciler
     * and are dropped here. Malformed records pass through so the
     * downstream reconciler / DLQ -- not this filter -- surface them.
     */
    static boolean isPayloadLinkActionable(DataChangeRecord record) {
        try {
            for (Mod mod : record.getMods()) {
                if (extractPayloadLink(mod.getOldValuesJson()) != null
                    || extractPayloadLink(mod.getNewValuesJson()) != null) {
                    return true;
                }
            }
            return false;
        } catch (Exception e) {
            LOG.warn("payload_link extraction failed; passing record through", e);
            return true;
        }
    }

    private static String extractPayloadLink(String json) throws Exception {
        if (json == null || json.isEmpty()) {
            return null;
        }
        JsonNode link = MAPPER.readTree(json).get("payload_link");
        if (link == null || link.isNull()) {
            return null;
        }
        String s = link.asText();
        return s.isEmpty() ? null : s;
    }

    static String serializeRecord(DataChangeRecord r) {
        try {
            ObjectNode root = MAPPER.createObjectNode();
            root.put("commitTimestamp", r.getCommitTimestamp().toString());
            root.put("modType", r.getModType().toString());
            root.put("tableName", r.getTableName());
            // TTL row-deletion-policy deletes carry transaction_tag
            // "RowDeletionPolicy" and is_system_transaction true; the reconciler
            // uses these to tell an abandoned batch from a batch commit handoff.
            root.put(
                "transactionTag",
                r.getTransactionTag() == null ? "" : r.getTransactionTag());
            root.put("isSystemTransaction", r.isSystemTransaction());
            ArrayNode modsArr = root.putArray("mods");
            for (Mod mod : r.getMods()) {
                ObjectNode modNode = modsArr.addObject();
                modNode.put("keys", mod.getKeysJson());
                modNode.put("oldValues", mod.getOldValuesJson());
                modNode.put("newValues", mod.getNewValuesJson());
            }
            return MAPPER.writeValueAsString(root);
        } catch (Exception e) {
            throw new RuntimeException("DataChangeRecord serialization failed", e);
        }
    }

    private PayloadLinkChangesToPubSub() {
        // Static utility class -- no instances.
    }
}
