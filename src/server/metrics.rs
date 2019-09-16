use std::net::UdpSocket;

use actix_web::{error::ErrorInternalServerError, Error, HttpRequest};
use cadence::{BufferedUdpMetricSink, Counted, NopMetricSink, QueuingMetricSink, StatsdClient};

use crate::error::ApiError;
use crate::server::ServerState;
use crate::settings::Settings;

#[derive(Debug, Clone)]
pub struct Metrics {
    client: Option<StatsdClient>,
}

impl From<&HttpRequest> for Metrics {
    fn from(req: &HttpRequest) -> Self {
        Metrics {
            client: match req.app_data::<ServerState>() {
                Some(v) => Some(*v.metrics.clone()),
                None => {
                    dbg!("⚠️ metric error: No App State");
                    None
                }
            },
        }
    }
}

impl From<&actix_web::web::Data<ServerState>> for Metrics {
    fn from(state: &actix_web::web::Data<ServerState>) -> Self {
        Metrics {
            client: Some(*state.metrics.clone()),
        }
    }
}

impl Metrics {
    pub fn sink() -> StatsdClient {
        StatsdClient::builder("", NopMetricSink).build()
    }

    // TODO: Return this as a metric string for testing/debugging?
    pub fn incr(self, label: &str) {
        if self.client.is_some() {
            match self.client.unwrap().incr(label) {
                Err(e) => {
                    // eat the metric, but log the error
                    dbg!("⚠️ Metric {} error: {:?} ", label, e);
                }
                Ok(_v) => {
                    // v.as_metric_str()
                }
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
            dbg!("⚠️ Metric send error:", err);
        })
        .build())
}
