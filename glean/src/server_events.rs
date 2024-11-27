//! This Server Events crate encapsulates the core functionality related to
//! emitting Glean server metrics.

// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// log type string used to identify logs to process in the Moz Data Pipeline.
const GLEAN_EVENT_MOZLOG_TYPE: &str = "glean-server-event";

// Code below is static, regardless of what is defined in `metrics.yaml`:

/// The GleanEventsLogger produces output in the required format for Glean to ingest.
/// Glean ingestion requires the output to be written to stdout. Writing to a different
/// output will require the consumer to handle any closing as appropriate for the Writer.
pub struct GleanEventsLogger {
    /// Application Id to identify application per Glean standards
    pub app_id: String,
    /// Version of application emitting the event
    pub app_display_version: String,
    /// Channel to differentiate logs from prod/beta/staging/development
    pub app_channel: String,
}

/// Struct containing request metadata. Record calls can be made with this being left empty.
/// Default impl empty values will be omitted in json from ping struct definition.
#[derive(Default, Serialize, Deserialize)]
pub struct RequestInfo {
    pub user_agent: String,
    pub ip_address: String,
}

/// Struct encapsulating client application data to construct Glean ping.
#[derive(Serialize, Deserialize, Debug)]
pub struct ClientInfo {
    telemetry_sdk_build: String,
    first_run_date: String,
    os: String,
    os_version: String,
    architecture: String,
    app_build: String,
    app_display_version: String,
    app_channel: String,
}

/// Ping metadata struct.
#[derive(Serialize, Deserialize, Debug)]
pub struct PingInfo {
    seq: u32,
    start_time: String,
    end_time: String,
}

impl Default for PingInfo {
    /// Default impl to create PingInfo.
    fn default() -> Self {
        // times are ISO-8601 strings, e.g. "2023-12-19T22:09:17.440Z"
        let now = Utc::now().to_rfc3339();
        PingInfo {
            seq: 0,
            start_time: now.clone(),
            end_time: now,
        }
    }
}

/// Struct containing ping metadata.
#[derive(Serialize, Deserialize, Debug)]
pub struct Ping {
    document_namespace: String,
    document_type: String,
    document_version: String,
    document_id: String,
    user_agent: Option<String>,
    ip_address: Option<String>,
    payload: String,
}

/// Glean Metrics type expressed by a String key of the supported metric types
/// ("string", "quantity", "event", "datetime", "boolean") and a HashMap
/// of each metric (defined in `metrics.yaml`) corresponding to its
/// serialized value.
type Metrics = HashMap<String, HashMap<String, serde_json::Value>>;

/// Struct defining the `Event` metric type.
#[derive(Debug, Serialize, Deserialize)]
pub struct GleanEvent {
    category: String,
    name: String,
    timestamp: i64,
    extra: HashMap<String, String>,
}

pub fn new_glean_event(category: &str, name: &str, extra: HashMap<String, String>) -> GleanEvent {
    GleanEvent {
        category: category.to_owned(),
        name: name.to_owned(),
        timestamp: Utc::now().timestamp_millis(),
        extra,
    }
}

/// Struct encapsulating the telemetry payload, including the metrics and events,
/// in addition to client and ping metadata.
#[derive(Serialize, Deserialize, Debug)]
struct PingPayload {
    client_info: ClientInfo,
    ping_info: PingInfo,
    metrics: Metrics,
    events: Vec<GleanEvent>,
}

/// Logging envelope that is serialized for emission to stdout.
#[derive(Serialize, Deserialize)]
struct LogEnvelope {
    // MozLog compliant format. https://wiki.mozilla.org/Firefox/Services/Logging
    #[serde(rename = "Type")]
    log_type: String,
    #[serde(rename = "Fields")]
    fields: Ping,
}

impl GleanEventsLogger {
    /// Create ClientInfo struct from values defined in GleanEventsLogger.
    fn create_client_info(&self) -> ClientInfo {
        // Fields with default values are required in the Glean schema, but not used in server context.
        ClientInfo {
            telemetry_sdk_build: "glean_parser v15.0.2.dev17+g81fec69a".to_owned(),
            first_run_date: "Unknown".to_owned(),
            os: "Unknown".to_owned(),
            os_version: "Unknown".to_owned(),
            architecture: "Unknown".to_owned(),
            app_build: "Unknown".to_owned(),
            app_display_version: self.app_display_version.clone(),
            app_channel: self.app_channel.clone(),
        }
    }

    /// Method used to encapsulate ping metadata and PingPayload.
    fn create_ping(
        &self,
        document_type: &str,
        config: &RequestInfo,
        payload: &PingPayload,
    ) -> Ping {
        Ping {
            document_namespace: self.app_id.clone(),
            document_type: document_type.to_owned(),
            document_version: "1".to_owned(),
            document_id: Uuid::new_v4().to_string(),
            user_agent: Some(config.user_agent.clone()),
            ip_address: Some(config.ip_address.clone()),
            payload: serde_json::to_string(payload).expect("unable to marshal payload to json."),
        }
    }

    /// Method called by each ping-specific record method.
    /// The goal is to construct the ping, wrap it in the envelope and print to stdout.
    fn record(
        &self,
        document_type: &str,
        request_info: &RequestInfo,
        metrics: Metrics,
        events: Vec<GleanEvent>,
    ) {
        let telemetry_payload: PingPayload = PingPayload {
            client_info: self.create_client_info(),
            ping_info: PingInfo::default(),
            metrics,
            events,
        };

        let ping: Ping = self.create_ping(document_type, request_info, &telemetry_payload);

        let envelope: LogEnvelope = LogEnvelope {
            log_type: GLEAN_EVENT_MOZLOG_TYPE.to_owned(),
            fields: ping,
        };
        let envelope_json =
            serde_json::to_string(&envelope).expect("unable to marshal payload to json.");
        println!("{}", envelope_json);
    }
}

// Code below is generated based on the provided `metrics.yaml` file:

// Metrics of the `event` type. Anything defined in `extra_keys` has it's own struct field.
// The appended `Event` term to any metric of the event type implies the ping event.

/// Struct containing metadata defined in `extra_keys` if they are defined. Otherwise empty.
pub struct SyncstorageGetCollectionsEvent {
    // Metadata for event in `extra_keys`.
}

// Implementing the EventsPingEvent trait for the generated struct SyncstorageGetCollectionsEvent
impl EventsPingEvent for SyncstorageGetCollectionsEvent {
    /// Create a GleanEvent for the above-defined Event struct (SyncstorageGetCollectionsEvent).
    /// Any metadata `extra` values are passed into the extra HashMap.
    fn glean_event(&self) -> GleanEvent {
        // Any `extra_keys` will be output below to be inserted into `extra`.
        // If there are none, an empty, immutable HashMap is created.
        let extra: HashMap<String, String> = HashMap::new();

        new_glean_event("syncstorage", "get_collections", extra)
    }
}

/// Marker trait for events per ping.
pub trait EventsPingEvent {
    fn glean_event(&self) -> GleanEvent;
}

/// Struct containing defined metrics and event(s) from `metrics.yaml`.
/// Encompasses the core Glean Ping Event and its data.
pub struct EventsPing {
    pub syncstorage_device_family: String, // Device family from which sync action was initiated. Desktop PC, Tablet, Mobile, and Other.
    pub syncstorage_hashed_device_id: String, // Hashed device id that is associated with a given account.
    pub syncstorage_hashed_fxa_uid: String, // User identifier. Uses `hashed_fxa_uid` for accurate count of sync actions.
    pub syncstorage_platform: String, // Platform from which sync action was initiated. Firefox Desktop, Fenix, or Firefox iOS.
    pub event: Option<Box<dyn EventsPingEvent>>, // valid event of `EventsPingEvent` for this ping.
}

impl GleanEventsLogger {
    /// General `record_events_ping` function for core Glean Ping Event - Record and submit `events` ping.
    /// Collects a HashMap of parametrized key value pairs and events to be recorded.
    pub fn record_events_ping(&self, request_info: &RequestInfo, params: &EventsPing) {
        // Define the outer `Metrics` map that holds the metric type.
        let mut metrics = Metrics::new();
        // Create the inner metric value map to insert into `Metrics`.
        let mut string_map: HashMap<String, Value> = HashMap::new();
        // Create corresponding metric value maps to insert into `Metrics`.
        string_map.insert(
            "syncstorage.device_family".to_owned(),
            Value::String(params.syncstorage_device_family.to_string()),
        );
        string_map.insert(
            "syncstorage.hashed_device_id".to_owned(),
            Value::String(params.syncstorage_hashed_device_id.to_string()),
        );
        string_map.insert(
            "syncstorage.hashed_fxa_uid".to_owned(),
            Value::String(params.syncstorage_hashed_fxa_uid.to_string()),
        );
        string_map.insert(
            "syncstorage.platform".to_owned(),
            Value::String(params.syncstorage_platform.to_string()),
        );
        metrics.insert("string".to_owned(), string_map);

        let mut events: Vec<GleanEvent> = Vec::new();
        if let Some(event) = &params.event {
            events.push(event.glean_event());
        }

        self.record("events", request_info, metrics, events);
    }
}

impl GleanEventsLogger {
    /// Record and submit `events` ping while omitting user request info.
    pub fn record_events_ping_without_user_info(&self, params: &EventsPing) {
        self.record_events_ping(&RequestInfo::default(), params)
    }
}
