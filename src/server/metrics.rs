use std::net::UdpSocket;
use std::time::Instant;

use actix_web::{error::ErrorInternalServerError, web::Data, Error, HttpRequest};
use cadence::{
    BufferedUdpMetricSink, Counted, Metric, NopMetricSink, QueuingMetricSink, StatsdClient, Timed,
};

use crate::error::ApiError;
use crate::server::ServerState;
use crate::settings::Settings;
use crate::web::tags::Tags;

#[derive(Debug, Clone)]
pub struct MetricTimer {
    pub label: String,
    pub start: Instant,
    pub tags: Tags,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    client: Option<StatsdClient>,
    tags: Option<Tags>,
    timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        let tags = self.tags.clone().unwrap_or_default();
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_millis() as u64;
                trace!("⌚ Ending timer at nanos: {:?} : {:?}", &timer.label, lapse;
                "ua.os.family" => tags.get("ua.os.family"),
                "ua.browser.family" => tags.get("ua.browser.family"),
                "ua.name" => tags.get("ua.name"),
                "ua.os.ver" => tags.get("ua.os.ver"),
                "ua.browser.ver" => tags.get("ua.browser.ver"));
                let mut tagged = client.time_with_tags(&timer.label, lapse);
                // Include any "hard coded" tags.
                // tagged = tagged.with_tag("version", env!("CARGO_PKG_VERSION"));
                let tags = timer.tags.tags.clone();
                let keys = tags.keys();
                for tag in keys {
                    tagged = tagged.with_tag(tag, &tags.get(tag).unwrap())
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

impl From<&HttpRequest> for Metrics {
    fn from(req: &HttpRequest) -> Self {
        let exts = req.extensions();
        let def_tags = Tags::from_request_head(req.head());
        let tags = exts.get::<Tags>().unwrap_or_else(|| &def_tags);
        Metrics {
            client: match req.app_data::<Data<ServerState>>() {
                Some(v) => Some(*v.metrics.clone()),
                None => {
                    warn!("⚠️ metric error: No App State");
                    None
                }
            },
            tags: Some(tags.clone()),
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
            mtags.extend(t.tags)
        }

        trace!("⌚ Starting timer... {:?}", &label;
            "ua.os.family" => mtags.get("ua.os.family"),
            "ua.browser.family" => mtags.get("ua.browser.family"),
            "ua.name" => mtags.get("ua.name"),
            "ua.os.ver" => mtags.get("ua.os.ver"),
            "ua.browser.ver" => mtags.get("ua.browser.ver"));
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags: mtags,
        });
    }

    // increment a counter with no tags data.
    pub fn incr(self, label: &str) {
        self.incr_with_tags(label, None)
    }

    pub fn incr_with_tags(self, label: &str, tags: Option<Tags>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.incr_with_tags(label);
            let mut mtags = self.tags.clone().unwrap_or_default().tags;
            if let Some(t) = tags {
                mtags.extend(t.tags)
            }
            let tag_keys = mtags.keys();
            for key in tag_keys.clone() {
                // REALLY wants a static here, or at least a well defined ref.
                tagged = tagged.with_tag(&key, &mtags.get(key).unwrap());
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    warn!("⚠️ Metric {} error: {:?} ", label, e;
                        "ua.os.family" => mtags.get("ua.os.family"),
                        "ua.browser.family" => mtags.get("ua.browser.family"),
                        "ua.name" => mtags.get("ua.name"),
                        "ua.os.ver" => mtags.get("ua.os.ver"),
                        "ua.browser.ver" => mtags.get("ua.browser.ver")
                    );
                }
                Ok(v) => trace!("☑️ {:?}", v.as_metric_str()),
            }
        }
    }
}

pub fn metrics_from_req(req: &HttpRequest) -> Result<Box<StatsdClient>, Error> {
    Ok(req
        .app_data::<Data<ServerState>>()
        .ok_or_else(|| ErrorInternalServerError("Could not get state"))
        .expect("Could not get state in metrics_from_req")
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
            warn!("⚠️ Metric send error:  {:?}", err);
        })
        .build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags() {
        use actix_web::dev::RequestHead;
        use actix_web::http::{header, uri::Uri};
        use std::collections::HashMap;

        let mut rh = RequestHead::default();
        let path = "/1.5/42/storage/meta/global";
        rh.uri = Uri::from_static(path);
        rh.headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:72.0) Gecko/20100101 Firefox/72.0",
            ),
        );

        let tags = Tags::from_request_head(&rh);

        let mut result = HashMap::<String, String>::new();
        result.insert("ua.os.ver".to_owned(), "NT 10.0".to_owned());
        result.insert("ua.os.family".to_owned(), "Windows".to_owned());
        result.insert("ua.browser.ver".to_owned(), "72.0".to_owned());
        result.insert("ua.name".to_owned(), "Firefox".to_owned());
        result.insert("ua.browser.family".to_owned(), "Firefox".to_owned());
        result.insert("uri.method".to_owned(), "GET".to_owned());

        assert_eq!(tags.tags, result)
    }

    #[test]
    fn no_empty_tags() {
        use actix_web::dev::RequestHead;
        use actix_web::http::{header, uri::Uri};

        let mut rh = RequestHead::default();
        let path = "/1.5/42/storage/meta/global";
        rh.uri = Uri::from_static(path);
        rh.headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("Mozilla/5.0 (curl) Gecko/20100101 curl"),
        );

        let tags = Tags::from_request_head(&rh);
        assert!(!tags.tags.contains_key("ua.os.ver"));
        println!("{:?}", tags);
    }
}
