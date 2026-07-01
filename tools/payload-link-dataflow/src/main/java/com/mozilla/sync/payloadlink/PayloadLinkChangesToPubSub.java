package com.mozilla.sync.payloadlink;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import com.google.cloud.Timestamp;
import org.apache.beam.sdk.Pipeline;
import org.apache.beam.sdk.PipelineResult;
import org.apache.beam.sdk.io.gcp.pubsub.PubsubIO;
import org.apache.beam.sdk.io.gcp.spanner.SpannerConfig;
import org.apache.beam.sdk.io.gcp.spanner.SpannerIO;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.DataChangeRecord;
import org.apache.beam.sdk.io.gcp.spanner.changestreams.model.Mod;
import org.apache.beam.sdk.options.PipelineOptionsFactory;
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
 * {@code tableName}, and a {@code mods} array. Each mod carries
 * {@code keys}, {@code oldValues}, and {@code newValues} as raw JSON
 * strings (matching the Spanner change-streams wire format), so the
 * downstream Python reconciler reads them with a second
 * {@code json.loads} per mod.
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
        run(options);
    }

    static PipelineResult run(PayloadLinkOptions options) {
        Pipeline pipeline = Pipeline.create(options);

        SpannerConfig spannerConfig = SpannerConfig.create()
            .withProjectId(options.getSpannerProjectId())
            .withInstanceId(options.getSpannerInstanceId())
            .withDatabaseId(options.getSpannerDatabase())
            .withRpcPriority(options.getRpcPriority());

        Timestamp startTimestamp = options.getStartTimestamp().isEmpty()
            ? Timestamp.now()
            : Timestamp.parseTimestamp(options.getStartTimestamp());
        Timestamp endTimestamp = options.getEndTimestamp().isEmpty()
            ? Timestamp.MAX_VALUE
            : Timestamp.parseTimestamp(options.getEndTimestamp());

        pipeline
            .apply(
                "Read From Spanner Change Stream",
                SpannerIO.readChangeStream()
                    .withSpannerConfig(spannerConfig)
                    .withMetadataInstance(options.getSpannerMetadataInstanceId())
                    .withMetadataDatabase(options.getSpannerMetadataDatabase())
                    .withChangeStreamName(options.getChangeStreamName())
                    .withInclusiveStartAt(startTimestamp)
                    .withInclusiveEndAt(endTimestamp))
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
