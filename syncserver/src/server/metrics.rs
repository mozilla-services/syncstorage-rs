use std::net::UdpSocket;
use std::time::Instant;

use actix_web::{
    dev::Payload, error::ErrorInternalServerError, web::Data, Error, FromRequest, HttpRequest,
};
use cadence::{
    BufferedUdpMetricSink, Counted, Metric, NopMetricSink, QueuingMetricSink, StatsdClient, Timed,
};
use futures::future;
use futures::future::Ready;

use crate::error::ApiError;
use crate::server::ServerState;
use crate::web::tags::Tags;

#[derive(Debug, Clone)]
pub struct MetricTimer {
    pub label: String,
    pub start: Instant,
    pub tags: Tags,
}

#[derive(Debug, Clone)]
pub struct Metrics {
    pub client: Option<StatsdClient>,
    pub tags: Option<Tags>,
    pub timer: Option<MetricTimer>,
}

impl Drop for Metrics {
    fn drop(&mut self) {
        let tags = self.tags.clone().unwrap_or_default();
        if let Some(client) = self.client.as_ref() {
            if let Some(timer) = self.timer.as_ref() {
                let lapse = (Instant::now() - timer.start).as_millis() as u64;
                trace!("⌚ Ending timer at nanos: {:?} : {:?}", &timer.label, lapse; &tags);
                let mut tagged = client.time_with_tags(&timer.label, lapse);
                // Include any "hard coded" tags.
                // tagged = tagged.with_tag("version", env!("CARGO_PKG_VERSION"));
                let tags = timer.tags.tags.clone();
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
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        future::ok(metrics_from_request(
            req,
            req.app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone()),
        ))
    }
}

pub fn metrics_from_request(req: &HttpRequest, client: Option<Box<StatsdClient>>) -> Metrics {
    let exts = req.extensions();
    let def_tags = Tags::from(req.head());
    let tags = exts.get::<Tags>().unwrap_or(&def_tags);

    if client.is_none() {
        warn!("⚠️ metric error: No App State");
    }

    Metrics {
        client: client.as_deref().cloned(),
        tags: Some(tags.clone()),
        timer: None,
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

impl From<&ServerState> for Metrics {
    fn from(state: &ServerState) -> Self {
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
            mtags.extend(t)
        }

        trace!("⌚ Starting timer... {:?}", &label; &mtags);
        self.timer = Some(MetricTimer {
            label: label.to_owned(),
            start: Instant::now(),
            tags: mtags,
        });
    }

    // increment a counter with no tags data.
    pub fn incr(&self, label: &str) {
        self.incr_with_tags(label, None)
    }

    pub fn incr_with_tags(&self, label: &str, tags: Option<Tags>) {
        self.count_with_tags(label, 1, tags)
    }

    pub fn incr_with_tag(&self, label: &str, key: &str, value: &str) {
        self.incr_with_tags(label, Some(Tags::with_tag(key, value)))
    }

    pub fn count(&self, label: &str, count: i64) {
        self.count_with_tags(label, count, None)
    }

    pub fn count_with_tags(&self, label: &str, count: i64, tags: Option<Tags>) {
        if let Some(client) = self.client.as_ref() {
            let mut tagged = client.count_with_tags(label, count);
            let mut mtags = self.tags.clone().unwrap_or_default();
            if let Some(tags) = tags {
                mtags.extend(tags);
            }
            for key in mtags.tags.keys().clone() {
                if let Some(val) = mtags.tags.get(key) {
                    tagged = tagged.with_tag(key, val.as_ref());
                }
            }
            // Include any "hard coded" tags.
            // incr = incr.with_tag("version", env!("CARGO_PKG_VERSION"));
            match tagged.try_send() {
                Err(e) => {
                    // eat the metric, but log the error
                    warn!("⚠️ Metric {} error: {:?} ", label, e; mtags);
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

        let tags = Tags::from(&rh);

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

        let tags = Tags::from(&rh);
        assert!(!tags.tags.contains_key("ua.os.ver"));
        println!("{:?}", tags);
    }
}
