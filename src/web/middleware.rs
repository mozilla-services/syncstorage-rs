//! # Web Middleware
//!
//! Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use std::{cell::RefCell, fmt::Display, rc::Rc};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{
    http::{
        header::{self, HeaderMap, HeaderValue},
        Method, StatusCode,
    },
    Error, HttpMessage, HttpResponse,
};
use futures::{
    future::{self, Either, FutureResult},
    Future, Poll,
};

use crate::db::{params, util::SyncTimestamp};
use crate::error::{ApiError, ApiErrorKind};
use crate::server::{metrics, ServerState};
use crate::web::{
    extractors::{
        extrude_db, BsoParam, CollectionParam, HawkIdentifier, PreConditionHeader,
        PreConditionHeaderOpt,
    },
    tags::Tags,
};
use crate::web::{X_LAST_MODIFIED, X_WEAVE_TIMESTAMP};

pub struct WeaveTimestampMiddleware<S> {
    service: S,
}

// Known DockerFlow commands for Ops callbacks
const DOCKER_FLOW_ENDPOINTS: [&str; 4] = [
    "/__heartbeat__",
    "/__lbheartbeat__",
    "/__version__",
    "/__error__",
];

impl<S, B> Service for WeaveTimestampMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            return Box::new(self.service.call(sreq));
        }

        let ts = SyncTimestamp::default().as_seconds();
        Box::new(self.service.call(sreq).and_then(move |mut resp| {
            future::result(
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

impl<S, B> Transform<S> for WeaveTimestamp
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
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(WeaveTimestampMiddleware { service })
    }
}
//*
pub struct DbTransaction;

impl DbTransaction {
    pub fn new() -> Self {
        DbTransaction::default()
    }
}

impl Default for DbTransaction {
    fn default() -> Self {
        Self
    }
}

impl<S, B> Transform<S> for DbTransaction
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = DbTransactionMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(DbTransactionMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

#[derive(Debug)]
pub struct DbTransactionMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for DbTransactionMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        let no_agent = HeaderValue::from_str("NONE").unwrap();
        let useragent = sreq
            .headers()
            .get("user-agent")
            .unwrap_or(&no_agent)
            .to_str()
            .unwrap_or("NONE");
        info!(">>> testing db middleware"; "user_agent" => useragent);
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            let mut service = Rc::clone(&self.service);
            return Box::new(service.call(sreq));
        }

        let tags = match sreq.extensions().get::<Tags>() {
            Some(t) => t.clone(),
            None => Tags::from_request_head(sreq.head()),
        };
        let col_result = CollectionParam::extrude(&sreq.uri(), &mut sreq.extensions_mut(), &tags);
        let state = match &sreq.app_data::<ServerState>() {
            Some(v) => v.clone(),
            None => {
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: No State".to_owned())
                            .into_body(),
                    ),
                ))
            }
        };
        let collection = match col_result {
            Ok(v) => v,
            Err(e) => {
                // Semi-example to show how to use metrics inside of middleware.
                metrics::Metrics::from(&state).incr("sync.error.collectionParam");
                debug!("⚠️ CollectionParam err: {:?}", e);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: invalid collection".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let method = sreq.method().clone();
        let hawk_user_id = match sreq.get_hawk_id() {
            Ok(v) => v,
            Err(e) => {
                debug!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::Unauthorized()
                            .content_type("application/json")
                            .body("Err: Invalid Authorization".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let mut service = Rc::clone(&self.service);
        let fut = state.db_pool.get().map_err(Into::into).and_then(move |db| {
            sreq.extensions_mut().insert(db.clone());
            let db2 = db.clone();

            if let Some(collection) = collection {
                let lc = params::LockCollection {
                    user_id: hawk_user_id,
                    collection: collection.collection,
                };
                Either::A(match method {
                    Method::GET | Method::HEAD => db.lock_for_read(lc),
                    _ => db.lock_for_write(lc),
                })
            } else {
                Either::B(future::ok(()))
            }
            .or_else(move |e| db.rollback().and_then(|_| future::err(e)))
            .map_err(Into::into)
            .and_then(move |_| {
                service.call(sreq).and_then(move |resp| {
                    // XXX: lock_for_x usually begins transactions but Dbs
                    // may also implicitly create them, so commit/rollback
                    // are always called to finish them. They noop when no
                    // implicit transaction was created (maybe rename them
                    // to maybe_commit/rollback?)
                    match resp.response().error() {
                        None => db2.commit(),
                        Some(_) => db2.rollback(),
                    }
                    .map_err(Into::into)
                    .and_then(|_| resp)
                })
            })
        });
        Box::new(fut)
    }
}

pub struct SentryWrapper;

impl SentryWrapper {
    pub fn new() -> Self {
        SentryWrapper::default()
    }
}

impl Default for SentryWrapper {
    fn default() -> Self {
        Self
    }
}

impl<S, B> Transform<S> for SentryWrapper
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SentryWrapperMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(SentryWrapperMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

#[derive(Debug)]
pub struct SentryWrapperMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for SentryWrapperMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        let mut tags = Tags::from_request_head(sreq.head());
        sreq.extensions_mut().insert(tags.clone());

        Box::new(self.service.call(sreq).and_then(move |sresp| {
            // handed an actix_error::error::Error;
            // Fetch out the tags (in case any have been added.)
            match sresp.response().error() {
                None => {}
                Some(e) => {
                    // The extensions defined in the request do not get populated
                    // into the response. There can be two different, and depending
                    // on where a tag may be set, only one set may be available.
                    // Base off of the request, then overwrite/suppliment with the
                    // response.
                    if let Some(t) = sresp.request().extensions().get::<Tags>() {
                        debug!("Found request tags: {:?}", &t.tags);
                        for (k, v) in t.tags.clone() {
                            tags.tags.insert(k, v);
                        }
                    };
                    if let Some(t) = sresp.response().extensions().get::<Tags>() {
                        debug!("Found response tags: {:?}", &t.tags);
                        for (k, v) in t.tags.clone() {
                            tags.tags.insert(k, v);
                        }
                    };
                    // deriving the sentry event from a fail directly from the error
                    // is not currently thread safe. Downcasting the error to an
                    // ApiError resolves this.
                    let apie: Option<&ApiError> = e.as_error();
                    if let Some(apie) = apie {
                        let mut event = sentry::integrations::failure::event_from_fail(apie);
                        event.tags = tags.as_btree();
                        sentry::capture_event(event);
                    }
                }
            }
            sresp
        }))
    }
}

/// The resource in question's Timestamp
pub struct ResourceTimestamp(SyncTimestamp);

#[derive(Debug)]
pub struct PreConditionCheck;

impl PreConditionCheck {
    pub fn new() -> Self {
        PreConditionCheck::default()
    }
}

impl Default for PreConditionCheck {
    fn default() -> Self {
        Self
    }
}

impl<S, B> Transform<S> for PreConditionCheck
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PreConditionCheckMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(PreConditionCheckMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
    }
}

pub struct PreConditionCheckMiddleware<S> {
    service: Rc<RefCell<S>>,
}

impl<S, B> Service for PreConditionCheckMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<dyn Future<Item = Self::Response, Error = Self::Error>>;

    // call super poll_ready()
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            let mut service = Rc::clone(&self.service);
            return Box::new(service.call(sreq));
        }

        // Pre check
        let tags = {
            let exts = sreq.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(sreq.head()),
            }
        };
        let precondition = match PreConditionHeaderOpt::extrude(&sreq.headers(), Some(tags.clone()))
        {
            Ok(precond) => match precond.opt {
                Some(p) => p,
                None => PreConditionHeader::NoHeader,
            },
            Err(e) => {
                debug!("⚠️ Precondition error {:?}", e);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .body("An error occurred in preprocessing".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let user_id = match sreq.get_hawk_id() {
            Ok(v) => v,
            Err(e) => {
                debug!("⚠️ Hawk header error {:?}", e);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::Unauthorized()
                            .content_type("application/json")
                            .body("Invalid Authorization".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let edb = extrude_db(&sreq.extensions());
        let db = match edb {
            Ok(v) => v,
            Err(e) => {
                debug!("⚠️ Database access error {:?}", e);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: database access error".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let uri = &sreq.uri();
        let col_result = CollectionParam::extrude(&uri, &mut sreq.extensions_mut(), &tags);
        let collection = match col_result {
            Ok(v) => v.map(|c| c.collection),
            Err(e) => {
                debug!("⚠️ Collection Error:  {:?}", e);
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: bad collection".to_owned())
                            .into_body(),
                    ),
                ));
            }
        };
        let bso = BsoParam::extrude(sreq.head(), &mut sreq.extensions_mut()).ok();
        let bso_opt = bso.map(|b| b.bso);

        let mut service = Rc::clone(&self.service);
        Box::new(
            db.extract_resource(user_id, collection, bso_opt)
                .map_err(Into::into)
                .and_then(move |resource_ts| {
                    let status = match precondition {
                        PreConditionHeader::IfModifiedSince(header_ts)
                            if resource_ts <= header_ts =>
                        {
                            StatusCode::NOT_MODIFIED
                        }
                        PreConditionHeader::IfUnmodifiedSince(header_ts)
                            if resource_ts > header_ts =>
                        {
                            StatusCode::PRECONDITION_FAILED
                        }
                        _ => StatusCode::OK,
                    };
                    if status != StatusCode::OK {
                        return Either::A(future::ok(
                            sreq.into_response(
                                HttpResponse::Ok()
                                    .content_type("application/json")
                                    .header(X_LAST_MODIFIED, resource_ts.as_header())
                                    .status(status)
                                    .body("".to_owned())
                                    .into_body(),
                            ),
                        ));
                    };

                    // Make the call, then do all the post-processing steps.
                    Either::B(service.call(sreq).map(move |mut resp| {
                        if resp.headers().contains_key(X_LAST_MODIFIED) {
                            return resp;
                        }

                        // See if we already extracted one and use that if possible
                        if let Ok(ts_header) =
                            header::HeaderValue::from_str(&resource_ts.as_header())
                        {
                            debug!("📝 Setting X-Last-Modfied {:?}", ts_header);
                            resp.headers_mut().insert(
                                header::HeaderName::from_static(X_LAST_MODIFIED),
                                ts_header,
                            );
                        }
                        resp
                    }))
                }),
        )
    }
}

trait SyncServerRequest {
    fn get_hawk_id(&self) -> Result<HawkIdentifier, Error>;
}

impl SyncServerRequest for ServiceRequest {
    fn get_hawk_id(&self) -> Result<HawkIdentifier, Error> {
        if DOCKER_FLOW_ENDPOINTS.contains(&self.uri().path().to_lowercase().as_str()) {
            return Ok(HawkIdentifier::cmd_dummy());
        }
        let method = self.method().clone();
        // NOTE: `connection_info()` gets a mutable reference lock on `extensions()`, so
        // it must be cloned
        let ci = &self.connection_info().clone();
        let state = &self.app_data::<ServerState>().ok_or_else(|| -> ApiError {
            ApiErrorKind::Internal("No app_data ServerState".to_owned()).into()
        })?;
        let tags = Tags::from_request_head(self.head());
        HawkIdentifier::extrude(self, &method.as_str(), &self.uri(), &ci, &state, Some(tags))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http;
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
