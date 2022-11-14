use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::Instant;

use actix_web::{dev::Payload, web::Data, FromRequest, HttpRequest};
use cadence::{
    BufferedUdpMetricSink, Counted, Metric, NopMetricSink, QueuingMetricSink, StatsdClient, Timed,
};
use futures::future;
use futures::future::Ready;
use slog::{Key, Record, KV};

use crate::error::ApiError;
use crate::server::ServerState;
use crate::tokenserver;
use crate::web::tags::Taggable;

#[derive(Debug, Clone)]
pub struct MetricTimer {
    pub label: String,
    pub start: Instant,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Default, Clone)]
pub struct Metrics {
    pub client: Option<StatsdClient>,
    pub tags: HashMap<String, String>,
    pub timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        let tags = self.tags.clone();
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_millis() as u64;
                trace!("⌚ Ending timer at nanos: {:?} : {:?}", &timer.label, lapse; &MetricTags(tags));
                let mut tagged = client.time_with_tags(&timer.label, lapse);
                // Include any "hard coded" tags.
                // tagged = tagged.with_tag("version", env!("CARGO_PKG_VERSION"));
                let tags = timer.tags.clone();
                let keys = tags.keys();
                for tag in keys {
                    tagged = tagged.with_tag(tag, tags.get(tag).unwrap())
                }
                match tagged.try_send() {
                    Err(e) => {
                        // eat the metric, but log the error
                        warn!("⚠️ Metric {} error: {:?} ", &timer.label, e);
                    }
                    Ok(v) => {
                        trace!("⌚ {:?}", v.as_metric_str());
                    }
                }
            }
        }
    }
}

impl FromRequest for Metrics {
    type Config = ();
    type Error = ();
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let client = {
            let syncstorage_metrics = req
                .app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone());
            let tokenserver_metrics = req
                .app_data::<Data<tokenserver::ServerState>>()
                .map(|state| state.metrics.clone());

            syncstorage_metrics.or(tokenserver_metrics)
        };

        if client.is_none() {
            warn!("⚠️ metric error: No App State");
        }

        future::ok(Metrics {
            client: client.as_deref().cloned(),
            tags: req.get_tags(),
            timer: None,
        })
    }
}

impl From<&StatsdClient> for Metrics {
    fn from(client: &StatsdClient) -> Self {
        Metrics {
            client: Some(client.clone()),
            tags: HashMap::default(),
            timer: None,
        }
    }
}

impl From<&ServerState> for Metrics {
    fn from(state: &ServerState) -> Self {
        Metrics {
            client: Some(*state.metrics.clone()),
            tags: HashMap::default(),
            timer: None,
        }
    }
}

impl Metrics {
    pub fn sink() -> StatsdClient {
        StatsdClient::builder("", NopMetricSink).build()
    }

    pub fn noop() -> Self {
        Self {
            client: Some(Self::sink()),
            timer: None,
            tags: HashMap::default(),
        }
    }

    pub fn start_timer(&mut self, label: &str, tags: Option<HashMap<String, String>>) {
        let mut mtags = self.tags.clone();
        if let Some(t) = tags {
            mtags.extend(t)
        }

        let mtags = MetricTags(mtags);
        trace!("⌚ Starting timer... {:?}", &label; &mtags);
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags: mtags.0,
        });
    }

    // increment a counter with no tags data.
    pub fn incr(&self, label: &str) {
        self.incr_with_tags(label, HashMap::default())
    }

    pub fn incr_with_tags(&self, label: &str, tags: HashMap<String, String>) {
        self.count_with_tags(label, 1, tags)
    }

    pub fn incr_with_tag(&self, label: &str, key: &str, value: &str) {
        let mut tags = HashMap::default();
        tags.insert(key.to_owned(), value.to_owned());

        self.incr_with_tags(label, tags);
    }

    // decrement a counter with no tags data.
    pub fn decr_with_tag(&self, label: &str, key: &str, value: &str) {
        let mut tags = HashMap::default();
        tags.insert(key.to_owned(), value.to_owned());

        self.count_with_tags(label, -1, tags);
    }

    pub fn count(&self, label: &str, count: i64) {
        self.count_with_tags(label, count, HashMap::default())
    }

    pub fn count_with_tags(&self, label: &str, count: i64, tags: HashMap<String, String>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.count_with_tags(label, count);
            let mut mtags = self.tags.clone();
            mtags.extend(tags);

            for key in mtags.keys().clone() {
                if let Some(val) = mtags.get(key) {
                    tagged = tagged.with_tag(key, val.as_ref());
                }
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    warn!("⚠️ Metric {} error: {:?} ", label, e; MetricTags(mtags));
                }
                Ok(v) => trace!("☑️ {:?}", v.as_metric_str()),
            }
        }
    }
}

pub fn metrics_from_opts(
    label: &str,
    host: Option<&str>,
    port: u16,
) -> Result<StatsdClient, ApiError> {
    let builder = if let Some(statsd_host) = host {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;

        let host = (statsd_host, port);
        let udp_sink = BufferedUdpMetricSink::from(host, socket)?;
        let sink = QueuingMetricSink::from(udp_sink);
        StatsdClient::builder(label, sink)
    } else {
        StatsdClient::builder(label, NopMetricSink)
    };
    Ok(builder
        .with_error_handler(|err| {
            warn!("⚠️ Metric send error:  {:?}", err);
        })
        .build())
}

/// A newtype used solely to allow us to implement KV on HashMap.
struct MetricTags(HashMap<String, String>);

impl KV for MetricTags {
    fn serialize(&self, _rec: &Record<'_>, serializer: &mut dyn slog::Serializer) -> slog::Result {
        for (key, val) in &self.0 {
            serializer.emit_str(Key::from(key.clone()), val)?;
        }
        Ok(())
    }
}
