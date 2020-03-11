use std::task::Context;
use std::{cell::RefCell, rc::Rc};

use crate::web::{
    extractors::{
        extrude_db, BsoParam, CollectionParam, PreConditionHeader, PreConditionHeaderOpt,
    },
    middleware::SyncServerRequest,
    tags::Tags,
    DOCKER_FLOW_ENDPOINTS, X_LAST_MODIFIED,
};
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::{header, StatusCode},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{self, Either, FutureExt, LocalBoxFuture, TryFutureExt};
use std::task::Poll;

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
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        future::ok(PreConditionCheckMiddleware {
            service: Rc::new(RefCell::new(service)),
        })
        .boxed_local()
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
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    // call super poll_ready()
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            let mut service = Rc::clone(&self.service);
            return Box::new(service.call(sreq)).boxed_local();
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
                warn!("⚠️ Precondition error {:?}", e);
                return Box::pin(future::ok(
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
                warn!("⚠️ Hawk header error {:?}", e);
                return Box::pin(future::ok(
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
                error!("⚠️ Database access error {:?}", e);
                return Box::pin(future::ok(
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
                warn!("⚠️ Collection Error:  {:?}", e);
                return Box::pin(future::ok(
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
        Box::pin(
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
                        return Either::Left(future::ok(
                            sreq.into_response(
                                HttpResponse::build(status)
                                    .content_type("application/json")
                                    .header(X_LAST_MODIFIED, resource_ts.as_header())
                                    .body("".to_owned())
                                    .into_body(),
                            ),
                        ));
                    };

                    // Make the call, then do all the post-processing steps.
                    Either::Right(service.call(sreq).map(move |resp| {
                        let mut resp =
                            resp.expect("Could not get resp in PreConditionCheckMiddleware::call");
                        if resp.headers().contains_key(X_LAST_MODIFIED) {
                            return Ok(resp);
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
                        Ok(resp)
                    }))
                }),
        )
    }
}
