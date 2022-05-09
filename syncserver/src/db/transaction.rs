use std::cell::RefMut;
use std::future::Future;

use actix_http::http::{HeaderValue, Method, StatusCode};
use actix_http::{Error, Extensions};
use actix_web::dev::{Payload, PayloadStream};
use actix_web::http::header;
use actix_web::web::Data;
use actix_web::{FromRequest, HttpRequest, HttpResponse};
use futures::future::LocalBoxFuture;
use futures::FutureExt;
use syncstorage_common::X_LAST_MODIFIED;
use syncstorage_db_common::{params, Db, DbPool, UserIdentifier};

use crate::db::results::ConnectionInfo;
use crate::error::{ApiError, ApiErrorKind};
use crate::server::metrics::Metrics;
use crate::server::ServerState;
use crate::web::extractors::{
    BsoParam, CollectionParam, HawkIdentifier, PreConditionHeader, PreConditionHeaderOpt,
};
use crate::web::tags::Tags;

#[derive(Clone)]
pub struct DbTransactionPool {
    pool: Box<dyn DbPool>,
    is_read: bool,
    user_id: UserIdentifier,
    collection: Option<String>,
    bso_opt: Option<String>,
    precondition: PreConditionHeaderOpt,
}

fn set_extra(exts: &mut RefMut<'_, Extensions>, connection_info: ConnectionInfo) {
    let mut tags = Tags::default();
    tags.add_extra("connection_age", &connection_info.age.to_string());
    tags.add_extra(
        "spanner_connection_age",
        &connection_info.spanner_age.to_string(),
    );
    tags.add_extra(
        "spanner_connection_idle",
        &connection_info.spanner_idle.to_string(),
    );
    tags.commit(exts);
}

impl DbTransactionPool {
    /// Perform an action inside of a DB transaction. If the action fails, the
    /// transaction is rolled back. If the action succeeds, the transaction is
    /// NOT committed. Further processing is required before we are sure the
    /// action has succeeded (ex. check HTTP response for internal error).
    async fn transaction_internal<'a, A: 'a, R, F>(
        &'a self,
        request: HttpRequest,
        action: A,
    ) -> Result<(R, Box<dyn Db<'a>>), ApiError>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<R, ApiError>> + 'a,
    {
        // Get connection from pool
        let db = self.pool.get().await?;
        let db2 = db.clone();

        // Lock for transaction
        let result = match (self.get_lock_collection(), self.is_read) {
            (Some(lc), true) => db.lock_for_read(lc).await,
            (Some(lc), false) => db.lock_for_write(lc).await,
            (None, is_read) => db.begin(!is_read).await,
        };

        // Handle lock error
        if let Err(e) = result {
            // Update the extra info fields.
            set_extra(&mut request.extensions_mut(), db.get_connection_info());
            db.rollback().await?;
            return Err(e.into());
        }

        // XXX: lock_for_x usually begins transactions but Dbs may also
        // implicitly create them, so commit/rollback are always called to
        // finish them. They noop when no implicit transaction was created
        // (maybe rename them to maybe_commit/rollback?)
        match action(db).await {
            Ok(resp) => Ok((resp, db2)),
            Err(e) => {
                db2.rollback().await?;
                Err(e)
            }
        }
    }

    pub fn get_pool(&self) -> Result<Box<dyn DbPool>, Error> {
        Ok(self.pool.clone())
    }

    /// Perform an action inside of a DB transaction.
    pub async fn transaction<'a, A: 'a, R, F>(
        &'a self,
        request: HttpRequest,
        action: A,
    ) -> Result<R, ApiError>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<R, ApiError>> + 'a,
    {
        let (resp, db) = self.transaction_internal(request, action).await?;

        // No further processing before commit is possible
        db.commit().await?;
        Ok(resp)
    }

    /// Perform an action inside of a DB transaction. This method will rollback
    /// if the HTTP response is an error.
    pub async fn transaction_http<'a, A: 'a, F>(
        &'a self,
        request: HttpRequest,
        action: A,
    ) -> Result<HttpResponse, ApiError>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<HttpResponse, ApiError>> + 'a,
    {
        let mreq = request.clone();
        let check_precondition = move |db: Box<dyn Db<'a>>| {
            async move {
                // set the extra information for all requests so we capture default err handlers.
                set_extra(&mut mreq.extensions_mut(), db.get_connection_info());
                let resource_ts = db
                    .extract_resource(
                        self.user_id.clone(),
                        self.collection.clone(),
                        self.bso_opt.clone(),
                    )
                    .await
                    .map_err(ApiError::from)?;

                if let Some(precondition) = &self.precondition.opt {
                    let status = match precondition {
                        PreConditionHeader::IfModifiedSince(header_ts)
                            if resource_ts <= *header_ts =>
                        {
                            StatusCode::NOT_MODIFIED
                        }
                        PreConditionHeader::IfUnmodifiedSince(header_ts)
                            if resource_ts > *header_ts =>
                        {
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

                let mut resp = action(db).await?;

                if resp.headers().contains_key(X_LAST_MODIFIED) {
                    return Ok(resp);
                }

                // See if we already extracted one and use that if possible
                if let Ok(ts_header) = header::HeaderValue::from_str(&resource_ts.as_header()) {
                    trace!("📝 Setting X-Last-Modfied {:?}", ts_header);
                    resp.headers_mut()
                        .insert(header::HeaderName::from_static(X_LAST_MODIFIED), ts_header);
                }

                Ok(resp)
            }
        };

        let (resp, db) = self
            .transaction_internal(request.clone(), check_precondition)
            .await?;
        // match on error and return a composed HttpResponse (so we can use the tags?)

        // HttpResponse can contain an internal error
        match resp.error() {
            None => db.commit().await?,
            Some(_) => db.rollback().await?,
        };
        Ok(resp)
    }

    /// Create a lock collection if there is a collection to lock
    fn get_lock_collection(&self) -> Option<params::LockCollection> {
        self.collection
            .clone()
            .map(|collection| params::LockCollection {
                collection,
                user_id: self.user_id.clone(),
            })
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

            let col_result = CollectionParam::extrude(req.uri(), &mut req.extensions_mut());
            let state = match req.app_data::<Data<ServerState>>() {
                Some(v) => v,
                None => {
                    let apie: ApiError = ApiErrorKind::NoServerState.into();
                    return Err(apie.into());
                }
            };
            let collection = match col_result {
                Ok(v) => v.map(|collection| collection.collection),
                Err(e) => {
                    // Semi-example to show how to use metrics inside of middleware.
                    Metrics::from(state.as_ref()).incr("sync.error.collectionParam");
                    warn!("⚠️ CollectionParam err: {:?}", e);
                    return Err(e);
                }
            };
            let method = req.method().clone();
            let user_id = HawkIdentifier::extract(&req).await.map_err(|e| {
                warn!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                e
            })?;
            let bso = BsoParam::extrude(req.head(), &mut req.extensions_mut()).ok();
            let bso_opt = bso.map(|b| b.bso);

            let is_read = matches!(method, Method::GET | Method::HEAD);
            let precondition = PreConditionHeaderOpt::extrude(req.headers())?;
            let pool = Self {
                pool: state.db_pool.clone(),
                is_read,
                user_id: user_id.into(),
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
