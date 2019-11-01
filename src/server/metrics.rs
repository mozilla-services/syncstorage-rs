use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::Instant;

use actix_web::{error::ErrorInternalServerError, Error, HttpRequest};
use cadence::{
    BufferedUdpMetricSink, Counted, Metric, NopMetricSink, QueuingMetricSink, StatsdClient, Timed,
};

use crate::error::ApiError;
use crate::server::ServerState;
use crate::settings::Settings;

pub type Tags = HashMap<String, String>;

#[derive(Debug, Clone)]
pub struct MetricTimer {
    pub label: String,
    pub start: Instant,
    pub tags: Option<Tags>,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    client: Option<StatsdClient>,
    timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_nanos() as u64;
                trace!("⌚ Ending timer at nanos: {:?} : {:?}", &timer.label, lapse);
                let mut tagged = client.time_with_tags(&timer.label, lapse);
                // Include any "hard coded" tags.
                // tagged = tagged.with_tag("version", env!("CARGO_PKG_VERSION"));
                let tags = timer.tags.clone().unwrap_or_default();
                let keys = tags.keys();
                for tag in keys {
                    tagged = tagged.with_tag(tag, &tags.get(tag).unwrap())
                }
                match tagged.try_send() {
                    Err(e) => {
                        // eat the metric, but log the error
                        debug!("⚠️ Metric {} error: {:?} ", &timer.label, e);
                    }
                    Ok(v) => {
                        trace!("⌚ {:?}", v.as_metric_str());
                    }
                }
            }
        }
    }
}

impl From<&HttpRequest> for Metrics {
    fn from(req: &HttpRequest) -> Self {
        Metrics {
            client: match req.app_data::<ServerState>() {
                Some(v) => Some(*v.metrics.clone()),
                None => {
                    debug!("⚠️ metric error: No App State");
                    None
                }
            },
            timer: None,
        }
    }
}

impl From<&StatsdClient> for Metrics {
    fn from(client: &StatsdClient) -> Self {
        Metrics {
            client: Some(client.clone()),
            timer: None,
        }
    }
}

impl From<&actix_web::web::Data<ServerState>> for Metrics {
    fn from(state: &actix_web::web::Data<ServerState>) -> Self {
        Metrics {
            client: Some(*state.metrics.clone()),
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
        }
    }

    pub fn start_timer(&mut self, label: &str, tags: Option<Tags>) {
        trace!("⌚ Starting timer... {:?}", &label);
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags,
        });
    }

    // increment a counter with no tags data.
    pub fn incr(self, label: &str) {
        self.incr_with_tags(label, None)
    }

    pub fn incr_with_tags(self, label: &str, tags: Option<HashMap<String, String>>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.incr_with_tags(label);
            let tags = tags.unwrap_or_default();
            let keys = tags.keys();
            for tag in keys {
                tagged = tagged.with_tag(tag, &tags.get(tag).unwrap())
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    debug!("⚠️ Metric {} error: {:?} ", label, e);
                }
                Ok(v) => trace!("☑️ {:?}", v.as_metric_str()),
            }
        }
    }
}

pub fn metrics_from_req(req: &HttpRequest) -> Result<Box<StatsdClient>, Error> {
    Ok(req
        .app_data::<ServerState>()
        .ok_or_else(|| ErrorInternalServerError("Could not get state"))
        .unwrap()
        .metrics
        .clone())
}

/// Create a cadence StatsdClient from the given options
pub fn metrics_from_opts(opts: &Settings) -> Result<StatsdClient, ApiError> {
    let builder = if let Some(statsd_host) = opts.statsd_host.as_ref() {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;

        let host = (statsd_host.as_str(), opts.statsd_port);
        let udp_sink = BufferedUdpMetricSink::from(host, socket)?;
        let sink = QueuingMetricSink::from(udp_sink);
        StatsdClient::builder(opts.statsd_label.as_ref(), sink)
    } else {
        StatsdClient::builder(opts.statsd_label.as_ref(), NopMetricSink)
    };
    Ok(builder
        .with_error_handler(|err| {
            debug!("⚠️ Metric send error:  {:?}", err);
        })
        .build())
}
