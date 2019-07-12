//! # Web Middleware
//!
//! Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use actix_web::{
    http::{header, Method},
    web::Data,
    Error, HttpMessage, HttpResponse,
};

use actix_http::Response;
use actix_web::dev::{MessageBody, ServiceRequest, ServiceResponse};
// use actix_router::PathDeserializer;
use actix_service::{Service, Transform};

use futures::{
    future::{self, Either, FutureResult},
    Future, Poll,
};

use crate::db::{params, util::SyncTimestamp, Db};
use crate::server::ServerState;
use crate::settings::Secrets;
use crate::web::extractors::{CollectionParam, HawkIdentifier};

/// Default Timestamp used for WeaveTimestamp middleware.
#[derive(Default)]
struct DefaultWeaveTimestamp(SyncTimestamp);

pub struct WeaveTimestampMiddleware<S> {
    service: S,
}

impl<S, B> Service for WeaveTimestampMiddleware<S>
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    // call super poll_ready()
    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        Box::new(self.service.call(req).map(move |mut resp| {
            // let (req, _payload) = req.into_parts();
            let req = resp.request().clone();
            if let Some(ts) = req.extensions().get::<DefaultWeaveTimestamp>() {
                let weave_ts = if let Some(val) = resp.headers().get("X-Last-Modified") {
                    let resp_ts = val
                        .to_str()
                        // .map_err(|e| ApiErrorKind::Internal(format!("Invalid X-Last-Modfied response header: {}", e)).into())
                        .unwrap()
                        .parse::<f64>()
                        // .map_err(|e| ApiErrorKind::Internal(format!("Invalid X-Last-Modified response header: {}", e)).into())
                        .unwrap();
                    if resp_ts > ts.0.into() {
                        resp_ts
                    } else {
                        ts.0.into()
                    }
                } else {
                    ts.0.into()
                };
                resp.headers_mut().insert(
                    header::HeaderName::from_static("x-weave-timestamp"),
                    header::HeaderValue::from_str(&format!("{:.*}", 2, &weave_ts))
                        // .map_err(|e|{ ApiErrorKind::Internal(format!("Invalid X-Weave-Timestamp response header: {}", e)).into()})
                        .unwrap(),
                )
            };
            resp
        }))
    }
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
    B: MessageBody,
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
/*
#[derive(Debug)]
pub struct DbTransactionMiddleware<S> {
    service: S,
}

impl<S, B> Service for DbTransactionMiddleware<S>
where
    B: 'static,
    S: Service<Request = ServiceRequest, Response = Response<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = HttpResponse<B>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        // `into_parts()` consumes the service request.
        // that's bad.
        // let items = sreq.match_info();
        println!("### >> DbTransactionMiddleware wrapper");
        let method = sreq.method();
        let mut exts = sreq.extensions_mut();
        let data: Data<ServerState> = sreq.app_data().unwrap();
        let collection = CollectionParam::extrude(&sreq.uri())
            .map(|param| param.collection.clone())
            .ok();
        let secrets = sreq.app_data::<Secrets>().unwrap();
        let hawk_user_id = HawkIdentifier::extrude(
            &secrets,
            sreq.method().as_str(),
            sreq.headers()
                .get("authentication")
                .unwrap()
                .to_str()
                .unwrap(),
            &sreq.connection_info(),
            &sreq.uri(),
        )
        .unwrap();
        exts.insert(hawk_user_id.clone());
        let in_transaction = collection.is_some();

        // TODO: actually make this future used async.
        data.db_pool
            .get()
            .and_then(move |db| {
                let db2 = db.clone();
                let fut = if let Some(collection) = collection {
                    let lc = params::LockCollection {
                        user_id: hawk_user_id,
                        collection,
                    };
                    Either::A(
                        match *method {
                            Method::GET | Method::HEAD => db.lock_for_read(lc),
                            _ => db.lock_for_write(lc),
                        }
                        .or_else(move |e| db2.rollback().and_then(|_| future::err(e))),
                    )
                } else {
                    Either::B(future::ok(()))
                };
                fut.and_then(move |_| {
                    // track whether a transaction was started above via the
                    // lock methods
                    exts.insert((db, in_transaction));
                    future::ok(())
                })
            })
            .wait()
            .unwrap();
        Box::new(self.service.call(sreq).map(move |resp| {
            if let Some((db, in_transaction)) = resp.extensions().get::<(Box<dyn Db>, bool)>() {
                if *in_transaction {
                    match resp.error() {
                        None => db.commit(),
                        Some(_) => db.rollback(),
                    };
                }
            };
            resp
        }))
       }
}
*/

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

/*

// TODO: 
// Move this to extractors and make it part of MetaRequest or the various Request handlers. (e.g. BsoPutRequest)?

pub struct PreConditionCheckMiddleware<S> {
    service: S,
}

// TODO: Extract this to it's own function (if it's actually needed?)
impl<S, B> Service for PreConditionCheckMiddleware<S>
where
B: 'static,
S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error= Error>,
S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    // call super poll_ready()
    fn poll_ready(&mut self) -> Poll<(), Self::Error>{
        self.service.poll_ready()
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        // let (req, mut payload) = sreq.into_parts();
        // Pre check
        let precondition = match PreConditionHeaderOpt::extrude(&sreq) {
            Ok(precond) =>
                match precond.opt {
                    Some(p) => p,
                    None => {
                        return Box::new(future::ok(sreq.into_response(HttpResponse::Ok().finish().into_body())))
                    }
                },
            Err(e) => {
                return Box::new(future::ok(sreq.into_response(HttpResponse::InternalServerError().body(format!("Err: {:?}", e)).into_body())))
            }
        };

        let user_id = HawkIdentifier::extrude(&sreq).unwrap();
        //let db = <Box<dyn Db>>::from_request(&sreq, &mut payload).unwrap();
        let db =  extrude_db(&sreq).unwrap();
        let collection = CollectionParam::extrude(&sreq).ok().map(|v| v.collection);
        let bso = BsoParam::extrude(&sreq).ok();

        let resource_ts = db
            .extract_resource(user_id.clone(), collection.clone(), Some(bso.clone().unwrap().bso))
            //.map_err(Into::into)
            .wait().unwrap();

        sreq.extensions_mut().insert(ResourceTimestamp(resource_ts));
        let status = match precondition {
            PreConditionHeader::IfModifiedSince(header_ts) if resource_ts <= header_ts => {
                StatusCode::NOT_MODIFIED
            }
            PreConditionHeader::IfUnmodifiedSince(header_ts) if resource_ts > header_ts => {
                StatusCode::PRECONDITION_FAILED
            }
            _ => StatusCode::OK,
        };
        if status != StatusCode::OK {
            return Box::new(
                future::ok(
                        sreq.into_response(
                        HttpResponse::Ok()
                        .header("X-Last-Modified", resource_ts.as_header())
                        .body("".to_owned())
                        .into_body())
                )
            );
        };
        let rs_ts = sreq.extensions().get::<ResourceTimestamp>().clone();

        // Make the call, then do all the post-processing steps.
        Box::new(self.service.call(sreq).map(move |mut resp| {
            if resp.headers().contains_key("X-Last-Modified") {
                //return ServiceResponse::new(req, HttpResponse::build(StatusCode::OK).body("".to_owned()).into_body());
                return resp;
            }

            // See if we already extracted one and use that if possible
            if let Some(resource_ts) = rs_ts {
                let ts = resource_ts.0;
                if let Ok(ts_header) = header::HeaderValue::from_str(&ts.as_header()) {
                    resp.headers_mut().insert(header::HeaderName::from_static("X-Last-Modified"), ts_header);
                    //headers.insert(header::HeaderName::from_static("X-Last-Modified"), ts_header);
                }
            } else {
                // Do the work needed to generate a timestamp otherwise
                let resource_ts = db
                    .extract_resource(user_id, collection, Some(bso.unwrap().bso))
                    .wait()
                    .unwrap();
                if let Ok(ts_header) = header::HeaderValue::from_str(&resource_ts.as_header()) {

                    resp.headers_mut().insert(header::HeaderName::from_static("X-Last-Modified"), ts_header);
                }
            }
            return resp
        }))
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http;
    use actix_web::test::TestRequest;
    use chrono::Utc;

    #[test]
    fn test_no_modified_header() {
        let weave_timestamp = WeaveTimestamp {};
        let req = TestRequest::default().to_http_request();
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
