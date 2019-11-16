use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::Instant;

use actix_web::{error::ErrorInternalServerError, http, Error, HttpRequest};
use cadence::{
    BufferedUdpMetricSink, Counted, Metric, NopMetricSink, QueuingMetricSink, StatsdClient, Timed,
};

use crate::error::ApiError;
use crate::server::user_agent::parse_user_agent;
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
    tags: Option<Tags>,
    timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_millis() as u64;
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
        let ua = req.headers().get(http::header::USER_AGENT);
        let mut tags: Option<Tags> = None;
        if let Some(ua_string) = ua {
            tags = Some(Self::default_tags(ua_string.to_str().unwrap_or("")));
        }
        Metrics {
            client: match req.app_data::<ServerState>() {
                Some(v) => Some(*v.metrics.clone()),
                None => {
                    debug!("⚠️ metric error: No App State");
                    None
                }
            },
            tags,
            timer: None,
        }
    }
}

impl From<&StatsdClient> for Metrics {
    fn from(client: &StatsdClient) -> Self {
        Metrics {
            client: Some(client.clone()),
            tags: None,
            timer: None,
        }
    }
}

impl From<&actix_web::web::Data<ServerState>> for Metrics {
    fn from(state: &actix_web::web::Data<ServerState>) -> Self {
        Metrics {
            client: Some(*state.metrics.clone()),
            tags: None,
            timer: None,
        }
    }
}

impl Metrics {
    pub fn sink() -> StatsdClient {
        StatsdClient::builder("", NopMetricSink).build()
    }

    pub fn default_tags(user_agent: &str) -> Tags {
        let mut tags = Tags::new();

        let (ua_result, metrics_os, metrics_browser) = parse_user_agent(user_agent);

        tags.insert("ua.os.family".to_owned(), metrics_os.to_owned());
        tags.insert("ua.browser.family".to_owned(), metrics_browser.to_owned());
        tags.insert("ua.name".to_owned(), ua_result.name.to_owned());
        tags.insert(
            "ua.os.ver".to_owned(),
            ua_result.os_version.to_owned().to_string(),
        );
        tags.insert("ua.browser.ver".to_owned(), ua_result.version.to_owned());
        tags
    }

    pub fn noop() -> Self {
        Self {
            client: Some(Self::sink()),
            timer: None,
            tags: None,
        }
    }

    pub fn start_timer(&mut self, label: &str, tags: Option<Tags>) {
        let mut mtags = self.tags.clone().unwrap_or_default();
        if let Some(t) = tags {
            mtags.extend(t)
        }
        trace!("⌚ Starting timer... {:?}", &label);
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags: if !mtags.is_empty() { Some(mtags) } else { None },
        });
    }

    // increment a counter with no tags data.
    pub fn incr(self, label: &str) {
        self.incr_with_tags(label, None)
    }

    pub fn incr_with_tags(self, label: &str, tags: Option<Tags>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.incr_with_tags(label);
            let mut mtags = self.tags.clone().unwrap_or_default();
            mtags.extend(tags.unwrap_or_default());
            let keys = mtags.keys();
            for tag in keys {
                tagged = tagged.with_tag(tag, &mtags.get(tag).unwrap())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags() {
        let tags = Metrics::default_tags(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0",
        );
        let mut result = HashMap::<String, String>::new();
        result.insert("ua.os.ver".to_owned(), "NT 10.0".to_owned());
        result.insert("ua.os.family".to_owned(), "Windows".to_owned());
        result.insert("ua.browser.ver".to_owned(), "72.0".to_owned());
        result.insert("ua.name".to_owned(), "Firefox".to_owned());
        result.insert("ua.browser.family".to_owned(), "Firefox".to_owned());

        assert_eq!(tags, result)
    }
}
