use std::task::Context;
use std::{cell::RefCell, rc::Rc};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    http::{header::HeaderValue, Method},
    Error, HttpMessage, HttpResponse,
};
use futures::future::{self, Either, LocalBoxFuture, Ready, TryFutureExt};
use std::task::Poll;

use crate::db::params;
use crate::server::{metrics, ServerState};
use crate::web::{
    extractors::CollectionParam, middleware::SyncServerRequest, tags::Tags, DOCKER_FLOW_ENDPOINTS,
};

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
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

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
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, sreq: ServiceRequest) -> Self::Future {
        let no_agent = HeaderValue::from_str("NONE")
            .expect("Could not get no_agent in DbTransactionMiddleware::call");
        let useragent = sreq
            .headers()
            .get("user-agent")
            .unwrap_or(&no_agent)
            .to_str()
            .unwrap_or("NONE");
        info!(">>> testing db middleware"; "user_agent" => useragent);
        if DOCKER_FLOW_ENDPOINTS.contains(&sreq.uri().path().to_lowercase().as_str()) {
            let mut service = Rc::clone(&self.service);
            return Box::pin(service.call(sreq));
        }

        let tags = match sreq.extensions().get::<Tags>() {
            Some(t) => t.clone(),
            None => Tags::from_request_head(sreq.head()),
        };
        let col_result = CollectionParam::extrude(&sreq.uri(), &mut sreq.extensions_mut(), &tags);
        let state = match &sreq.app_data::<ServerState>() {
            Some(v) => v.clone(),
            None => {
                return Box::pin(future::ok(
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
                warn!("⚠️ CollectionParam err: {:?}", e);
                return Box::pin(future::ok(
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
                warn!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                return Box::pin(future::ok(
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
                Either::Left(match method {
                    Method::GET | Method::HEAD => db.lock_for_read(lc),
                    _ => db.lock_for_write(lc),
                })
            } else {
                Either::Right(future::ok(()))
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
                    .and_then(|_| future::ok(resp))
                })
            })
        });
        Box::pin(fut)
    }
}
