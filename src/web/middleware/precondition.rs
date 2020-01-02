use std::{cell::RefCell, rc::Rc};

use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    http::{header, StatusCode},
    Error, HttpMessage, HttpResponse,
};
use futures::{
    future::{self, Either, FutureResult},
    Future, Poll,
};

use crate::web::{
    extractors::{
        extrude_db, BsoParam, CollectionParam, PreConditionHeader, PreConditionHeaderOpt,
    },
    middleware::SyncServerRequest,
    tags::Tags,
    DOCKER_FLOW_ENDPOINTS, X_LAST_MODIFIED,
};

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
