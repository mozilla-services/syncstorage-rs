use crate::db::{params, Db, DbPool};
use crate::error::{ApiError, ApiErrorKind};
use crate::server::metrics::Metrics;
use crate::server::ServerState;
use crate::web::extractors::CollectionParam;
use crate::web::middleware::sentry::report;
use crate::web::middleware::SyncServerRequest;
use crate::web::tags::Tags;
use actix_http::http::{HeaderValue, Method};
use actix_http::Error;
use actix_web::dev::{Payload, PayloadStream};
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
}

impl DbTransactionPool {
    pub async fn transaction<'a, A, F>(&'a self, action: A) -> Result<HttpResponse, Error>
    where
        A: FnOnce(Box<dyn Db<'a>>) -> F,
        F: Future<Output = Result<HttpResponse, Error>> + 'a,
    {
        let db = self.pool.get().await?;
        let db2 = db.clone();

        let result = match (self.lock_collection.clone(), self.is_read) {
            (Some(lc), true) => db.lock_for_read(lc).await,
            (Some(lc), false) => db.lock_for_write(lc).await,
            _ => Ok(()),
        };

        if let Err(e) = result {
            db.rollback().await?;
            return Err(e.into());
        }

        let resp = action(db).await?;

        // XXX: lock_for_x usually begins transactions but Dbs
        // may also implicitly create them, so commit/rollback
        // are always called to finish them. They noop when no
        // implicit transaction was created (maybe rename them
        // to maybe_commit/rollback?)
        let result = match resp.error() {
            None => db2.commit().await,
            Some(_) => db2.rollback().await,
        };

        if let Err(apie) = result {
            // we can't queue_report here (no access to extensions)
            // so just report it immediately with tags on hand
            if apie.is_reportable() {
                report(
                    &self.tags,
                    sentry::integrations::failure::event_from_fail(&apie),
                );
            } else {
                debug!("Not reporting error: {:?}", apie);
            }
            return Err(apie.into());
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
            info!(">>> testing db middleware"; "user_agent" => useragent);

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
            let hawk_user_id = match req.get_hawk_id() {
                Ok(v) => v,
                Err(e) => {
                    warn!("⚠️ Bad Hawk Id: {:?}", e; "user_agent"=> useragent);
                    return Err(e);
                }
            };
            let (lc, is_read) = if let Some(collection) = collection {
                let lc = params::LockCollection {
                    user_id: hawk_user_id,
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

            let pool = Self {
                pool: state.db_pool.clone(),
                lock_collection: lc,
                is_read,
                tags,
            };

            req.extensions_mut().insert(pool.clone());
            Ok(pool)
        }
        .boxed_local()
    }
}
