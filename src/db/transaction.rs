use crate::db::{params, Db, DbPool};
use crate::error::{ApiError, ApiErrorKind};
use crate::server::metrics::Metrics;
use crate::server::ServerState;
use crate::web::extractors::{
    BsoParam, CollectionParam, HawkIdentifier, PreConditionHeader, PreConditionHeaderOpt,
};
use crate::web::middleware::sentry::report;
use crate::web::middleware::SyncServerRequest;
use crate::web::tags::Tags;
use crate::web::X_LAST_MODIFIED;
use actix_http::http::{HeaderValue, Method, StatusCode};
use actix_http::Error;
use actix_web::dev::{Payload, PayloadStream};
use actix_web::http::header;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use std::future::Future;

#[derive(Clone)]
pub struct DbTransactionPool {
    pool: Box<dyn DbPool>,
    lock_collection: Option<params::LockCollection>,
    is_read: bool,
    tags: Tags,
    user_id: HawkIdentifier,
    collection: Option<String>,
    bso_opt: Option<String>,
    precondition: PreConditionHeaderOpt,
}

impl DbTransactionPool {
    /// Perform an action inside of a DB transaction. If the action fails, the
    /// transaction is rolled back. If the action succeeds, the transaction is
    /// NOT committed. Further processing is required before we are sure the
    /// action has succeeded (ex. check HTTP response for internal error).
    async fn transaction_internal<'a, A, R, F>(
        &'a self,
        db: Box<dyn Db<'a>>,
        action: A,
    ) -> Result<(R, Box<dyn Db<'a>>), Error>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<R, Error>> + 'a,
    {
        let db2 = db.clone();

        // Lock for transaction
        let result = match (self.lock_collection.clone(), self.is_read) {
            (Some(lc), true) => db.lock_for_read(lc).await,
            (Some(lc), false) => db.lock_for_write(lc).await,
            _ => Ok(()),
        };

        // Handle lock error
        if let Err(e) = result {
            db.rollback().await?;
            return Err(e.into());
        }

        // Perform the action
        let resp = action(db).await;

        // XXX: lock_for_x usually begins transactions but Dbs
        // may also implicitly create them, so commit/rollback
        // are always called to finish them. They noop when no
        // implicit transaction was created (maybe rename them
        // to maybe_commit/rollback?)
        match resp {
            Ok(resp) => Ok((resp, db2)),
            Err(e) => {
                let result = db2.rollback().await;

                // Handle rollback error
                if let Err(e) = result {
                    self.report_error(&e);
                    Err(e.into())
                } else {
                    Err(e)
                }
            }
        }
    }

    pub fn get_pool(&self) -> Result<Box<dyn DbPool>, Error> {
        Ok(self.pool.clone())
    }

    /// Perform an action inside of a DB transaction.
    pub async fn transaction<'a, A, R, F>(&'a self, action: A) -> Result<R, Error>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<R, Error>> + 'a,
    {
        // Get connection from pool
        let db = self.pool.get().await?;
        let (resp, db) = self.transaction_internal(db, action).await?;

        // No further processing before commit is possible
        let result = db.commit().await;

        // Handle commit error
        if let Err(e) = result {
            self.report_error(&e);
            return Err(e.into());
        }

        Ok(resp)
    }

    /// Report an error to sentry, if possible
    fn report_error(&self, error: &ApiError) {
        // we can't queue_report here (no access to extensions)
        // so just report it immediately with tags on hand
        if error.is_reportable() {
            report(
                &self.tags,
                sentry::integrations::failure::event_from_fail(error),
            );
        } else {
            debug!("Not reporting error: {:?}", error);
        }
    }

    /// Perform an action inside of a DB transaction. This method will rollback
    /// if the HTTP response is an error.
    pub async fn transaction_http<'a, A, F>(&'a self, action: A) -> Result<HttpResponse, Error>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<HttpResponse, Error>> + 'a,
    {
        // Get connection from pool
        let db = self.pool.get().await?;
        let resource_ts = db
            .extract_resource(
                self.user_id.clone(),
                self.collection.clone(),
                self.bso_opt.clone(),
            )
            .await?;

        if let Some(precondition) = &self.precondition.opt {
            let status = match precondition {
                PreConditionHeader::IfModifiedSince(header_ts) if resource_ts <= *header_ts => {
                    StatusCode::NOT_MODIFIED
                }
                PreConditionHeader::IfUnmodifiedSince(header_ts) if resource_ts > *header_ts => {
                    StatusCode::PRECONDITION_FAILED
                }
                _ => StatusCode::OK,
            };
            if status != StatusCode::OK {
                return Ok(HttpResponse::build(status)
                    .content_type("application/json")
                    .header(X_LAST_MODIFIED, resource_ts.as_header())
                    .body("".to_owned())
                    .into_body());
            };
        }
        let (mut resp, db) = self.transaction_internal(db, action).await?;

        // HttpResponse can contain an internal error
        let result = match resp.error() {
            None => db.commit().await,
            Some(_) => db.rollback().await,
        };

        // Handle commit/rollback error
        if let Err(e) = result {
            self.report_error(&e);
            return Err(e.into());
        }

        if resp.headers().contains_key(X_LAST_MODIFIED) {
            return Ok(resp);
        }

        // See if we already extracted one and use that if possible
        if let Ok(ts_header) = header::HeaderValue::from_str(&resource_ts.as_header()) {
            debug!("📝 Setting X-Last-Modfied {:?}", ts_header);
            resp.headers_mut()
                .insert(header::HeaderName::from_static(X_LAST_MODIFIED), ts_header);
        }
        Ok(resp)
    }
}

impl FromRequest for DbTransactionPool {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload<PayloadStream>) -> Self::Future {
        // Cache in extensions to avoid parsing for the lock info multiple times
        if let Some(pool) = req.extensions().get::<Self>() {
            return futures::future::ok(pool.clone()).boxed_local();
        }

        let req = req.clone();
        async move {
            let no_agent = HeaderValue::from_str("NONE")
                .expect("Could not get no_agent in DbTransactionPool::from_request");
            let useragent = req
                .headers()
                .get("user-agent")
                .unwrap_or(&no_agent)
                .to_str()
                .unwrap_or("NONE");

            let tags = match req.extensions().get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            };
            let col_result = CollectionParam::extrude(&req.uri(), &mut req.extensions_mut(), &tags);
            let state = match req.app_data::<Data<ServerState>>() {
                Some(v) => v,
                None => {
                    let apie: ApiError = ApiErrorKind::NoServerState.into();
                    return Err(apie.into());
                }
            };
            let collection = match col_result {
                Ok(v) => v,
                Err(e) => {
                    // Semi-example to show how to use metrics inside of middleware.
                    Metrics::from(state.as_ref()).incr("sync.error.collectionParam");
                    warn!("⚠️ CollectionParam err: {:?}", e);
                    return Err(e);
                }
            };
            let method = req.method().clone();
            let user_id = match req.get_hawk_id() {
                Ok(v) => v,
                Err(e) => {
                    warn!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                    return Err(e);
                }
            };
            let bso = BsoParam::extrude(req.head(), &mut req.extensions_mut()).ok();
            let bso_opt = bso.map(|b| b.bso);

            let (lc, is_read) = if let Some(collection) = collection {
                let lc = params::LockCollection {
                    user_id: user_id.clone(),
                    collection: collection.collection,
                };
                let is_read = match method {
                    Method::GET | Method::HEAD => true,
                    _ => false,
                };

                (Some(lc), is_read)
            } else {
                (None, true)
            };
            let collection = lc.clone().map(|c| c.collection);
            let precondition = PreConditionHeaderOpt::extrude(&req.headers(), Some(tags.clone()))?;
            let pool = Self {
                pool: state.db_pool.clone(),
                lock_collection: lc,
                is_read,
                tags,
                user_id,
                collection,
                bso_opt,
                precondition,
            };

            req.extensions_mut().insert(pool.clone());
            Ok(pool)
        }
        .boxed_local()
    }
}
