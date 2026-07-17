package com.mozilla.sync.payloadlink;

import com.google.cloud.spanner.Options.RpcPriority;
import org.apache.beam.runners.dataflow.options.DataflowPipelineOptions;
import org.apache.beam.sdk.options.Default;
import org.apache.beam.sdk.options.Description;
import org.apache.beam.sdk.options.Validation.Required;

public interface PayloadLinkOptions extends DataflowPipelineOptions {
    @Required
    @Description("GCP project that owns the Spanner instance.")
    String getSpannerProjectId();
    void setSpannerProjectId(String value);

    @Required
    @Description("Spanner instance holding the syncstorage database.")
    String getSpannerInstanceId();
    void setSpannerInstanceId(String value);

    @Required
    @Description("Syncstorage database (contains bsos and batch_bsos).")
    String getSpannerDatabase();
    void setSpannerDatabase(String value);

    @Required
    @Description("Spanner instance where the change stream's metadata table lives.")
    String getSpannerMetadataInstanceId();
    void setSpannerMetadataInstanceId(String value);

    @Required
    @Description("Spanner database where the change stream's metadata table lives.")
    String getSpannerMetadataDatabase();
    void setSpannerMetadataDatabase(String value);

    @Description("Spanner change stream to consume.")
    @Default.String("payload_link_changes")
    String getChangeStreamName();
    void setChangeStreamName(String value);

    @Required
    @Description("Fully-qualified Pub/Sub topic to publish actionable change records to.")
    String getPubsubTopic();
    void setPubsubTopic(String value);

    @Description("Inclusive start timestamp (RFC 3339). Empty -> pipeline-launch time.")
    @Default.String("")
    String getStartTimestamp();
    void setStartTimestamp(String value);

    @Description("Inclusive end timestamp (RFC 3339). Empty -> unbounded.")
    @Default.String("")
    String getEndTimestamp();
    void setEndTimestamp(String value);

    @Description("Spanner RPC priority for the change stream reader.")
    @Default.Enum("HIGH")
    RpcPriority getRpcPriority();
    void setRpcPriority(RpcPriority value);
}
