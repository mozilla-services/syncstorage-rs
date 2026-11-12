//! MozLog JSON and human formatting for tracing events.
//!
//! The JSON output matches what slog-mozlog-json 0.1 produced, which downstream consumers
//! parse: top-level `Logger`, `Type`, `Hostname`, `Pid`, `Severity`, `Timestamp` (ns since the
//! epoch) and `Fields`, with no `EnvVersion`. `Fields` starts with `msg`, followed by every
//! event field as its own key. Started from the formatter in
//! https://github.com/mozilla-services/syncstorage-rs/pull/1515

use std::fmt;

use serde::{Serialize, Serializer, ser::SerializeMap};
use serde_json::Value;
use tracing::{Event, Level, Subscriber, field::Field, field::Visit};
use tracing_subscriber::{
    field::RecordFields,
    fmt::{
        FmtContext,
        format::{FormatEvent, FormatFields, Writer},
    },
    registry::LookupSpan,
};

/// Formats each event as one MozLog JSON line.
pub struct MozLogFormatter {
    logger: String,
    msg_type: String,
    hostname: String,
    pid: u32,
}

impl MozLogFormatter {
    pub fn new(hostname: String) -> Self {
        Self {
            logger: format!("{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            msg_type: format!("{}:log", env!("CARGO_PKG_NAME")),
            hostname,
            pid: std::process::id(),
        }
    }
}

/// The MozLog severity (syslog levels) slog-mozlog-json used: tracing has no critical (2).
fn severity(level: &Level) -> u8 {
    match *level {
        Level::ERROR => 3,
        Level::WARN => 4,
        Level::INFO => 6,
        Level::DEBUG | Level::TRACE => 7,
    }
}

/// One output line. Field order matches slog-mozlog-json.
#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct MozLogLine<'a> {
    logger: &'a str,
    #[serde(rename = "Type")]
    msg_type: &'a str,
    hostname: &'a str,
    pid: u32,
    severity: u8,
    timestamp: i64,
    fields: EventFields,
}

impl<S, N> FormatEvent<S, N> for MozLogFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let line = MozLogLine {
            logger: &self.logger,
            msg_type: &self.msg_type,
            hostname: &self.hostname,
            pid: self.pid,
            severity: severity(event.metadata().level()),
            timestamp: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            fields: EventFields::from(event),
        };
        let json = serde_json::to_string(&line).map_err(|_| fmt::Error)?;
        writeln!(writer, "{json}")
    }
}

/// An event's fields: tracing's `message` as `msg`, then every other field in order.
#[derive(Default)]
struct EventFields {
    msg: Option<String>,
    fields: Vec<(&'static str, Value)>,
}

impl From<&Event<'_>> for EventFields {
    fn from(event: &Event<'_>) -> Self {
        let mut fields = Self::default();
        event.record(&mut fields);
        fields
    }
}

impl EventFields {
    fn push(&mut self, field: &Field, value: Value) {
        // `msg` holds the message, so a field with that name would repeat the key.
        let name = match field.name() {
            "msg" => "msg_field",
            name => name,
        };
        self.fields.push((name, value));
    }
}

impl Visit for EventFields {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.push(field, value.into())
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.push(field, value.into())
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.push(field, value.into())
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.push(field, serde_json::to_value(value).unwrap_or(Value::Null))
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.push(field, serde_json::to_value(value).unwrap_or(Value::Null))
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.push(field, value.into())
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => self.msg = Some(value.to_owned()),
            _ => self.push(field, value.into()),
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.push(field, value.to_string().into())
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        // `message` is usually `format_args!`, whose Debug output is its Display output.
        match field.name() {
            "message" => self.msg = Some(format!("{value:?}")),
            _ => self.push(field, format!("{value:?}").into()),
        }
    }
}

impl Serialize for EventFields {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(1 + self.fields.len()))?;
        // slog records always had a message.
        map.serialize_entry("msg", self.msg.as_deref().unwrap_or_default())?;
        for (key, value) in &self.fields {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// Human-readable fields: the message, then ` key=value` pairs (strings unquoted), like
/// slog_term's output.
pub struct HumanFields;

impl<'w> FormatFields<'w> for HumanFields {
    fn format_fields<R: RecordFields>(&self, mut writer: Writer<'w>, fields: R) -> fmt::Result {
        let mut visitor = HumanVisitor {
            writer: &mut writer,
            result: Ok(()),
            first: true,
        };
        fields.record(&mut visitor);
        visitor.result
    }
}

struct HumanVisitor<'a, 'w> {
    writer: &'a mut Writer<'w>,
    result: fmt::Result,
    first: bool,
}

impl Visit for HumanVisitor<'_, '_> {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_debug(field, &format_args!("{value}"))
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if self.result.is_err() {
            return;
        }
        let space = if std::mem::take(&mut self.first) {
            ""
        } else {
            " "
        };
        self.result = match field.name() {
            "message" => write!(self.writer, "{space}{value:?}"),
            name => write!(self.writer, "{space}{name}={value:?}"),
        };
    }
}
