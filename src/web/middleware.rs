//! # Web Middleware
//!
//! Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use std::{cell::RefCell, fmt::Display, rc::Rc};

use actix_service::{Service, Transform};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{
    http::{
        header::{self, HeaderMap},
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
use crate::server::ServerState;
use crate::web::extractors::{
    extrude_db, BsoParam, CollectionParam, HawkIdentifier, PreConditionHeader,
    PreConditionHeaderOpt,
};
use crate::web::{X_LAST_MODIFIED, X_WEAVE_TIMESTAMP};

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
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        let ts = SyncTimestamp::default().as_seconds();
        Box::new(self.service.call(sreq).and_then(move |mut resp| {
            match set_weave_timestamp(resp.headers_mut(), ts) {
                Ok(_) => future::ok(resp),
                Err(e) => future::err(e.into()),
            }
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
    //type Transform = WeaveTimestampMiddleware<S>;

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
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S: 'static,
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
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S: 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        // `into_parts()` consumes the service request.
        let method = sreq.method().clone();
        let col_result = CollectionParam::extrude(&sreq.uri(), &mut sreq.extensions_mut());
        let collection = match col_result {
            Ok(v) => v,
            Err(_e) => {
                // pending circleci update to 1.36
                // dbg!("!!! CollectionParam err: {:?}", e);
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
        let ci = &sreq.connection_info().clone();
        let headers = &sreq.headers();
        let auth = match headers.get("authorization") {
            Some(a) => a.to_str().unwrap(),
            None => {
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: missing auth header".to_owned())
                            .into_body(),
                    ),
                ))
            }
        };
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
        let secrets = &state.secrets.clone();
        let uri = &sreq.uri();
        let hawk_user_id =
            HawkIdentifier::generate(&secrets, &method.as_str(), &auth, &ci, &uri).unwrap();
        {
            let mut exts = sreq.extensions_mut();
            exts.insert(hawk_user_id.clone());
        }
        let in_transaction = collection.is_some();

        let mut service = Rc::clone(&self.service);
        let fut = state.db_pool.get().map_err(Into::into).and_then(move |db| {
            let db2 = db.clone();

            sreq.extensions_mut().insert((db, in_transaction));
            if let Some(collection) = collection {
                let db3 = db2.clone();
                let mut service2 = Rc::clone(&service);

                let lc = params::LockCollection {
                    user_id: hawk_user_id,
                    collection: collection.collection,
                };
                Either::A(
                    match method {
                        Method::GET | Method::HEAD => db2.lock_for_read(lc),
                        _ => db2.lock_for_write(lc),
                    }
                    .or_else(move |e| db2.rollback().and_then(|_| future::err(e)))
                    .map_err(Into::into)
                    .and_then(move |_| {
                        service2.call(sreq).and_then(move |resp| {
                            match resp.response().error() {
                                None => db3.commit(),
                                Some(_) => db3.rollback(),
                            }
                            .map_err(Into::into)
                            .and_then(|_| resp)
                        })
                    }),
                )
            } else {
                Either::B(service.call(sreq).map_err(Into::into).map(|resp| resp))
            }
        });
        Box::new(fut)
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
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S: 'static,
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
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S: 'static,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    // call super poll_ready()
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        // Pre check
        let precondition = match PreConditionHeaderOpt::extrude(&sreq.headers()) {
            Ok(precond) => match precond.opt {
                Some(p) => p,
                None => PreConditionHeader::NoHeader,
            },
            Err(e) => {
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::BadRequest()
                            .content_type("application/json")
                            .body(format!("Err: {:?}", e))
                            .into_body(),
                    ),
                ))
            }
        };

        let secrets = match &sreq.app_data::<ServerState>() {
            Some(v) => v,
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
        }
        .secrets
        .clone();

        let ci = &sreq.connection_info().clone();
        let headers = &sreq.headers();
        let auth = match headers.get("authorization") {
            Some(a) => a.to_str().unwrap(),
            None => {
                return Box::new(future::ok(
                    sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: missing auth".to_owned())
                            .into_body(),
                    ),
                ))
            }
        };
        let uri = &sreq.uri();
        let user_id =
            HawkIdentifier::generate(&secrets, &sreq.method().as_str(), &auth, &ci, &uri).unwrap();
        let db = extrude_db(&sreq.extensions()).unwrap();
        let col_result = CollectionParam::extrude(&uri, &mut sreq.extensions_mut());
        let collection = match col_result {
            Ok(v) => v.map(|c| c.collection),
            Err(_e) => {
                // pending circleci update to 1.36
                // dbg!("!!! Collection Error: ", e);
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
        let bso = BsoParam::extrude(&sreq.uri(), &mut sreq.extensions_mut()).ok();
        let bso_opt = bso.clone().map(|b| b.bso);

        let mut service = self.service.clone();
        Box::new(
            db.extract_resource(&user_id.clone(), collection, bso_opt)
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
                            // dbg!(format!("XXX Setting X-Last-Modfied {:?}", ts_header));
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
// */
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
        assert_eq!(weave_hdr < uts + 10, true);
        assert_eq!(weave_hdr > uts - 2000, true);
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
