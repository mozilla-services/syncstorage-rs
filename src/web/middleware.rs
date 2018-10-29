//! # Web Middleware
//!
//! Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).
use actix_web::{
    http::{header, Method},
    middleware::{Middleware, Response, Started},
    FromRequest, HttpRequest, HttpResponse, Result,
};
use chrono::Utc;
use futures::{future, Future};

use db::{params, Db};
use error::{ApiError, ApiErrorKind};
use server::ServerState;
use web::extractors::{CollectionParam, HawkIdentifier};

/// Default Timestamp used for WeaveTimestamp middleware.
struct DefaultWeaveTimestamp(f64);

/// Middleware to set the X-Weave-Timestamp header on all responses.
pub struct WeaveTimestamp;

impl<S> Middleware<S> for WeaveTimestamp {
    /// Set the `DefaultWeaveTimestamp` and attach to the `HttpRequest`
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        // Get millisecond resolution and convert to seconds
        let ts = Utc::now().timestamp_millis() as f64 / 1_000.0;
        req.extensions_mut().insert(DefaultWeaveTimestamp(ts));
        Ok(Started::Done)
    }

    /// Method is called when handler returns response,
    /// but before sending http message to peer.
    fn response(&self, req: &HttpRequest<S>, mut resp: HttpResponse) -> Result<Response> {
        let extensions = req.extensions();
        let ts = match extensions.get::<DefaultWeaveTimestamp>() {
            Some(ts) => ts,
            None => return Ok(Response::Done(resp)),
        };

        let weave_ts = if let Some(val) = resp.headers().get("X-Last-Modified") {
            let resp_ts = val
                .to_str()
                .map_err(|e| {
                    let error: ApiError = ApiErrorKind::Internal(format!(
                        "Invalid X-Last-Modified response header: {}",
                        e
                    )).into();
                    error
                })?.parse::<f64>()
                .map_err(|e| {
                    let error: ApiError = ApiErrorKind::Internal(format!(
                        "Invalid X-Last-Modified response header: {}",
                        e
                    )).into();
                    error
                })?;
            if resp_ts > ts.0 {
                resp_ts
            } else {
                ts.0
            }
        } else {
            ts.0
        };
        resp.headers_mut().insert(
            "x-weave-timestamp",
            header::HeaderValue::from_str(&format!("{:.*}", 2, &weave_ts)).map_err(|e| {
                let error: ApiError = ApiErrorKind::Internal(format!(
                    "Invalid X-Weave-Timestamp response header: {}",
                    e
                )).into();
                error
            })?,
        );
        Ok(Response::Done(resp))
    }
}

#[derive(Debug)]
pub struct DbTransaction;

impl Middleware<ServerState> for DbTransaction {
    /// Initialize the database
    fn start(&self, req: &HttpRequest<ServerState>) -> Result<Started> {
        let req = req.clone();
        // We may or may not be operating on a collection
        let collection = CollectionParam::from_request(&req, &())
            .map(|param| param.collection.clone())
            .ok();
        let user_id = HawkIdentifier::from_request(&req, &())?;

        let fut = req
            .state()
            .db_pool
            .get()
            .and_then(move |db| {
                let fut = if let Some(collection) = collection {
                    // Take a read or write lock depending on request method
                    let lc = params::LockCollection {
                        user_id,
                        collection,
                    };
                    match *req.method() {
                        Method::GET | Method::HEAD => db.lock_for_read(lc),
                        _ => db.lock_for_write(lc),
                    }
                } else {
                    // If we're not operating on a collection, don't take a lock
                    Box::new(future::ok(()))
                };
                fut.and_then(move |_| {
                    req.extensions_mut().insert(db);
                    future::ok(None)
                })
            }).map_err(Into::into);
        Ok(Started::Future(Box::new(fut)))
    }

    fn response(&self, req: &HttpRequest<ServerState>, resp: HttpResponse) -> Result<Response> {
        if let Some(db) = req.extensions().get::<Box<dyn Db>>() {
            let fut = match resp.error() {
                None => db.commit(),
                Some(_) => db.rollback(),
            };
            let fut = fut.and_then(|_| Ok(resp)).map_err(Into::into);
            Ok(Response::Future(Box::new(fut)))
        } else {
            Ok(Response::Done(resp))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http;
    use actix_web::test::TestRequest;

    #[test]
    fn test_no_modified_header() {
        let weave_timestamp = WeaveTimestamp {};
        let req = TestRequest::default().finish();
        let resp = HttpResponse::build(http::StatusCode::OK).finish();
        match weave_timestamp.start(&req) {
            Ok(Started::Done) => (),
            _ => panic!(),
        };
        let resp = match weave_timestamp.response(&req, resp) {
            Ok(Response::Done(resp)) => resp,
            _ => panic!(),
        };
        let weave_hdr = resp
            .headers()
            .get("X-Weave-Timestamp")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let weave_hdr = (weave_hdr * 1000.0) as u64;
        // Add 10 to compensate for how fast Rust can run these
        // tests (Due to 2-digit rounding for the sync ts).
        let ts = (Utc::now().timestamp_millis() as u64) + 10;
        assert_eq!(weave_hdr < ts, true);
        let ts = ts - 2000;
        assert_eq!(weave_hdr > ts, true);
    }

    #[test]
    fn test_older_timestamp() {
        let weave_timestamp = WeaveTimestamp {};
        let ts = (Utc::now().timestamp_millis() as u64) - 1000;
        let hts = format!("{:.*}", 2, ts as f64 / 1_000.0);
        let req = TestRequest::default().finish();
        let resp = HttpResponse::build(http::StatusCode::OK)
            .header("X-Last-Modified", hts.clone())
            .finish();
        match weave_timestamp.start(&req) {
            Ok(Started::Done) => (),
            _ => panic!(),
        };
        let resp = match weave_timestamp.response(&req, resp) {
            Ok(Response::Done(resp)) => resp,
            _ => panic!(),
        };
        let weave_hdr = resp
            .headers()
            .get("X-Weave-Timestamp")
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
        let weave_timestamp = WeaveTimestamp {};
        let ts = (Utc::now().timestamp_millis() as u64) + 4000;
        let hts = format!("{:.*}", 2, ts as f64 / 1_000.0);
        let req = TestRequest::default().finish();
        let resp = HttpResponse::build(http::StatusCode::OK)
            .header("X-Last-Modified", hts.clone())
            .finish();
        match weave_timestamp.start(&req) {
            Ok(Started::Done) => (),
            _ => panic!(),
        };
        let resp = match weave_timestamp.response(&req, resp) {
            Ok(Response::Done(resp)) => resp,
            _ => panic!(),
        };
        let weave_hdr = resp
            .headers()
            .get("X-Weave-Timestamp")
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(weave_hdr, hts);
    }
}
