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
use crate::error::{ApiError, ApiErrorKind};
use crate::server::{metrics, ServerState};
use crate::web::middleware::sentry::{queue_report, report};
use crate::web::{
    extractors::CollectionParam, middleware::SyncServerRequest, tags::Tags, DOCKER_FLOW_ENDPOINTS,
};
use futures::FutureExt;

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
        async move {
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
                return service.call(sreq).await;
            }

            let tags = match sreq.extensions().get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(sreq.head()),
            };
            let col_result =
                CollectionParam::extrude(&sreq.uri(), &mut sreq.extensions_mut(), &tags);
            let state = match &sreq.app_data::<ServerState>() {
                Some(v) => v.clone(),
                None => {
                    let apie: ApiError = ApiErrorKind::NoServerState.into();
                    queue_report(sreq.extensions_mut(), &apie.into());
                    return Ok(sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: No State".to_owned())
                            .into_body(),
                    ));
                }
            };
            let collection = match col_result {
                Ok(v) => v,
                Err(e) => {
                    // Semi-example to show how to use metrics inside of middleware.
                    // Metrics::from(state.as_ref()).incr("sync.error.collectionParam");
                    warn!("⚠️ CollectionParam err: {:?}", e);
                    queue_report(sreq.extensions_mut(), &e);
                    return Ok(sreq.into_response(
                        HttpResponse::InternalServerError()
                            .content_type("application/json")
                            .body("Err: invalid collection".to_owned())
                            .into_body(),
                    ));
                }
            };
            let method = sreq.method().clone();
            let hawk_user_id = match sreq.get_hawk_id() {
                Ok(v) => v,
                Err(e) => {
                    warn!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                    queue_report(sreq.extensions_mut(), &e);
                    return Ok(sreq.into_response(
                        HttpResponse::Unauthorized()
                            .content_type("application/json")
                            .body("Err: Invalid Authorization".to_owned())
                            .into_body(),
                    ));
                }
            };
            let mut service = Rc::clone(&self.service);
            let db = state.db_pool.get().await?;
            let db2 = db.clone();

            let result = if let Some(collection) = collection {
                let lc = params::LockCollection {
                    user_id: hawk_user_id,
                    collection: collection.collection,
                };
                match method {
                    Method::GET | Method::HEAD => db.lock_for_read(lc).await,
                    _ => db.lock_for_write(lc).await,
                }
            } else {
                Ok(())
            };

            if let Err(e) = result {
                db.rollback().await?;
                return Err(e.into());
            }

            let resp = service.call(sreq).await?;

            // XXX: lock_for_x usually begins transactions but Dbs
            // may also implicitly create them, so commit/rollback
            // are always called to finish them. They noop when no
            // implicit transaction was created (maybe rename them
            // to maybe_commit/rollback?)
            let result = match resp.response().error() {
                None => db2.commit().await,
                Some(_) => db2.rollback().await,
            };

            if let Err(apie) = result {
                // we can't queue_report here (no access to extensions)
                // so just report it immediately with tags on hand
                if apie.is_reportable() {
                    report(&tags, sentry::integrations::failure::event_from_fail(&apie));
                } else {
                    if let Some(label) = apie.metric_label() {
                        state.metrics.incr_with_tags(label, tags);
                    }
                    debug!("Not reporting error: {:?}", apie);
                }
                return Err(apie.into());
            }

            Ok(resp)
        }
        .boxed_local()
    }
}
