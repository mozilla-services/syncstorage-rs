//! MozLog formatting for tracing events

use serde::{ser::SerializeMap, Serialize, Serializer};
use std::{fmt, io, time::SystemTime};
use tracing::{field::Visit, Event, Level, Subscriber};
use tracing_subscriber::{
    fmt::{
        format::{self, FormatEvent, FormatFields},
        FmtContext,
    },
    registry::LookupSpan,
};

/// Top-level formatter for a tracing [Event]
pub struct EventFormatter {
    logger: String,
    hostname: String,
    pid: u32,
}

impl EventFormatter {
    pub fn new() -> Self {
        Self {
            logger: format!("{}-{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            hostname: match hostname::get() {
                Ok(h) => h.to_string_lossy().to_string(),
                Err(_) => "<unknown>".to_owned(),
            },
            pid: std::process::id(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for EventFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        // This closure returns a `serde_json::Result` which allows for nicer ergonomics with the
        // `?` operator.  We map this to a `fmt::Result` at the bottom of the method.
        //
        // It serializes `event`, writes it out to `writer`, then returns the writer back so we can
        // write out the trailing newline.
        let format_with_serde = move || -> serde_json::Result<format::Writer<'_>> {
            let mut serializer = serde_json::Serializer::new(WriteAdaptor(writer));
            let mut map = serializer.serialize_map(Some(7))?;
            map.serialize_entry(
                "Timestamp",
                &SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos(),
            )?;
            map.serialize_entry("Type", "syncserver:log")?;
            map.serialize_entry("Logger", &self.logger)?;
            map.serialize_entry("Hostname", &self.hostname)?;
            map.serialize_entry("EnvVersion", "2.0")?;
            map.serialize_entry("Pid", &self.pid)?;
            map.serialize_entry(
                "Severity",
                &match *event.metadata().level() {
                    Level::ERROR => 3,
                    Level::WARN => 4,
                    Level::INFO => 5,
                    Level::DEBUG => 6,
                    Level::TRACE => 7,
                },
            )?;
            map.serialize_entry("Fields", &SerializableEvent(event))?;
            map.end()?;
            Ok(serializer.into_inner().0)
        };
        let mut writer = format_with_serde().map_err(|_| fmt::Error)?;
        writeln!(writer)
    }
}

/// Newtype that wraps `tracing::Event` and implements `serde::Serialize`.  This allows us to
/// serialize the event into the `Fields` field.
struct SerializableEvent<'a, 'event>(&'a Event<'event>);

impl<'a, 'event> Serialize for SerializableEvent<'a, 'event> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let map = serializer.serialize_map(self.0.fields().size_hint().1)?;
        let mut visitor = SerdeFieldVisitor::new(map);
        self.0.record(&mut visitor);
        visitor.end()
    }
}

/// Implements `tracing::Visit` by serializing the fields to a `serde` map.  This is how we
/// serialize the `tracing::Event` with `serde`.
struct SerdeFieldVisitor<S>
where
    S: SerializeMap,
{
    map: S,
    error: Option<S::Error>,
}

impl<S> SerdeFieldVisitor<S>
where
    S: SerializeMap,
{
    fn new(serializer: S) -> Self {
        Self {
            map: serializer,
            error: None,
        }
    }

    fn should_serialize_field(&self, field: &tracing::field::Field) -> bool {
        !field.name().starts_with("slog.")
    }

    fn serialize_entry<V: Serialize + ?Sized>(&mut self, name: &str, value: &V) {
        if let Err(e) = self.map.serialize_entry(name, value) {
            // This is a bit awkward because serde's methods are failable, while the tracing::Visit
            // methods aren't.  The best we can do is store the error and return it when
            // `check_error()` is eventually called.  In practice this will probably be okay, since
            // the serializer will only fail on IO errors
            self.error = Some(e)
        }
    }

    fn end(mut self) -> Result<S::Ok, S::Error> {
        match self.error.take() {
            Some(e) => Err(e),
            None => self.map.end(),
        }
    }
}

impl<S> Visit for SerdeFieldVisitor<S>
where
    S: SerializeMap,
{
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value)
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), value)
        }
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &value.to_string())
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if self.should_serialize_field(field) {
            self.serialize_entry(field.name(), &format!("{value:?}"))
        }
    }
}

// Adapts tracing-subscriber's `Writer` struct to implement `std::io::Write`
//
// This is needed because `tracing` using the `std::fmt::Write` trait while `serde` uses
// `std::io::Write`.
struct WriteAdaptor<'writer>(format::Writer<'writer>);

impl<'writer> io::Write for WriteAdaptor<'writer> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.0
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
