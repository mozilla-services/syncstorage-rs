use std::fmt::Display;
use std::task::Context;

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{self, HeaderMap},
    Error,
};

use futures::future::{self, LocalBoxFuture, TryFutureExt};
use std::task::Poll;

use crate::db::util::SyncTimestamp;
use crate::error::{ApiError, ApiErrorKind};
use crate::web::{DOCKER_FLOW_ENDPOINTS, X_LAST_MODIFIED, X_WEAVE_TIMESTAMP};

pub struct WeaveTimestampMiddleware<S> {
    service: S,
}

impl<S, B> Service for WeaveTimestampMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            return Box::pin(self.service.call(sreq));
        }

        let ts = SyncTimestamp::default().as_seconds();
        Box::pin(self.service.call(sreq).and_then(move |mut resp| {
            future::ready(
                set_weave_timestamp(resp.headers_mut(), ts)
                    .map_err(Into::into)
                    .map(|_| resp),
            )
        }))
    }
}

/// Set a X-Weave-Timestamp header on all responses (depending on the
/// response's X-Last-Modified header)
fn set_weave_timestamp(headers: &mut HeaderMap, ts: f64) -> Result<(), ApiError> {
    fn invalid_xlm<E>(e: E) -> ApiError
    where
        E: Display,
    {
        ApiErrorKind::Internal(format!("Invalid X-Last-Modified response header: {}", e)).into()
    }

    let weave_ts = if let Some(val) = headers.get(X_LAST_MODIFIED) {
        let resp_ts = val
            .to_str()
            .map_err(invalid_xlm)?
            .parse::<f64>()
            .map_err(invalid_xlm)?;
        if resp_ts > ts {
            resp_ts
        } else {
            ts
        }
    } else {
        ts
    };
    headers.insert(
        header::HeaderName::from_static(X_WEAVE_TIMESTAMP),
        header::HeaderValue::from_str(&format!("{:.2}", &weave_ts)).map_err(invalid_xlm)?,
    );
    Ok(())
}

/// Middleware to set the X-Weave-Timestamp header on all responses.
pub struct WeaveTimestamp;

impl WeaveTimestamp {
    pub fn new() -> Self {
        WeaveTimestamp::default()
    }
}

impl Default for WeaveTimestamp {
    fn default() -> Self {
        Self
    }
}

impl<S: 'static, B> Transform<S> for WeaveTimestamp
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = WeaveTimestampMiddleware<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(future::ok(WeaveTimestampMiddleware { service }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http, HttpResponse};
    use chrono::Utc;

    #[test]
    fn test_no_modified_header() {
        let mut resp = HttpResponse::build(http::StatusCode::OK).finish();
        set_weave_timestamp(resp.headers_mut(), SyncTimestamp::default().as_seconds()).unwrap();
        let weave_hdr = resp
            .headers()
            .get(X_WEAVE_TIMESTAMP)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let uts = Utc::now().timestamp_millis() as u64;
        let weave_hdr = (weave_hdr * 1000.0) as u64;
        // Add 10 to compensate for how fast Rust can run these
        // tests (Due to 2-digit rounding for the sync ts).
        assert!(weave_hdr < uts + 10);
        assert!(weave_hdr > uts - 2000);
    }

    #[test]
    fn test_older_timestamp() {
        let ts = (Utc::now().timestamp_millis() as u64) - 1000;
        let hts = format!("{:.*}", 2, ts as f64 / 1_000.0);
        let mut resp = HttpResponse::build(http::StatusCode::OK)
            .header(X_LAST_MODIFIED, hts.clone())
            .finish();
        set_weave_timestamp(resp.headers_mut(), ts as f64).unwrap();
        let weave_hdr = resp
            .headers()
            .get(X_WEAVE_TIMESTAMP)
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let hts = hts.parse::<f64>().unwrap();
        assert!(weave_hdr > hts);
    }

    #[test]
    fn test_newer_timestamp() {
        let ts = (Utc::now().timestamp_millis() as u64) + 4000;
        let hts = format!("{:.2}", ts as f64 / 1_000.0);
        let mut resp = HttpResponse::build(http::StatusCode::OK)
            .header(X_LAST_MODIFIED, hts.clone())
            .finish();
        set_weave_timestamp(resp.headers_mut(), ts as f64 / 1_000.0).unwrap();
        let weave_hdr = resp
            .headers()
            .get(X_WEAVE_TIMESTAMP)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(weave_hdr, hts);
    }
}
