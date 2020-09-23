use async_trait::async_trait;
use bb8::ErrorSink;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use super::models::Result;
use crate::db::{error::DbError, results, Db, DbPool, STD_COLLS};
use crate::server::metrics::Metrics;
use crate::settings::Settings;

use super::manager::{SpannerSession, SpannerSessionManager};
use super::models::SpannerDb;
use crate::error::ApiResult;

pub use super::manager::Conn;

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
//pub fn run_embedded_migrations(settings: &Settings) -> Result<()> {
//    let conn = MysqlConnection::establish(&settings.database_url)?;
//    Ok(embedded_migrations::run(&conn)?)
//}

#[derive(Clone)]
pub struct SpannerDbPool {
    /// Pool of db connections
    pool: deadpool::managed::Pool<SpannerSession, DbError>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
    quota: usize,
    quota_enabled: bool,
}

impl SpannerDbPool {
    /// Creates a new pool of Spanner db connections.
    pub async fn new(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        //run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings, metrics).await
    }

    pub async fn new_without_migrations(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        let max_size = settings.database_pool_max_size.unwrap_or(10);
        let manager = SpannerSessionManager::new(settings, metrics)?;
        let config = deadpool::managed::PoolConfig::new(max_size as usize);
        let pool = deadpool::managed::Pool::from_config(manager, config);

        Ok(Self {
            pool,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
            quota: settings.limits.max_quota_limit as usize,
            quota_enabled: settings.enable_quota,
        })
    }

    pub async fn get_async(&self) -> Result<SpannerDb> {
        let conn = self.pool.get().await.map_err(|e| match e {
            deadpool::managed::PoolError::Backend(dbe) => dbe,
            deadpool::managed::PoolError::Timeout(timeout_type) => {
                DbError::internal(&format!("deadpool Timeout: {:?}", timeout_type))
            }
        })?;
        Ok(SpannerDb::new(
            conn,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            self.quota,
            self.quota_enabled,
        ))
    }
}

#[async_trait(?Send)]
impl DbPool for SpannerDbPool {
    async fn get<'a>(&'a self) -> ApiResult<Box<dyn Db<'a>>> {
        self.get_async()
            .await
            .map(|db| Box::new(db) as Box<dyn Db<'a>>)
            .map_err(Into::into)
    }

    fn state(&self) -> results::PoolState {
        self.pool.status().into()
    }

    fn validate_batch_id(&self, id: String) -> Result<()> {
        super::batch::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for SpannerDbPool {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SpannerDbPool")
            .field("coll_cache", &self.coll_cache)
            .finish()
    }
}

#[derive(Debug)]
pub struct CollectionCache {
    pub by_name: RwLock<HashMap<String, i32>>,
    pub by_id: RwLock<HashMap<i32, String>>,
}

impl CollectionCache {
    pub fn put(&self, id: i32, name: String) -> Result<()> {
        // XXX: should this emit a metric?
        // XXX: should probably either lock both simultaneously during
        // writes or use an RwLock alternative
        self.by_name
            .write()
            .map_err(|_| DbError::internal("by_name write"))?
            .insert(name.clone(), id);
        self.by_id
            .write()
            .map_err(|_| DbError::internal("by_id write"))?
            .insert(id, name);
        Ok(())
    }

    pub fn get_id(&self, name: &str) -> Result<Option<i32>> {
        Ok(self
            .by_name
            .read()
            .map_err(|_| DbError::internal("by_name read"))?
            .get(name)
            .cloned())
    }

    pub fn get_name(&self, id: i32) -> Result<Option<String>> {
        Ok(self
            .by_id
            .read()
            .map_err(|_| DbError::internal("by_id read"))?
            .get(&id)
            .cloned())
    }

    #[cfg(test)]
    pub fn clear(&self) {
        self.by_name.write().expect("by_name write").clear();
        self.by_id.write().expect("by_id write").clear();
    }
}

impl Default for CollectionCache {
    fn default() -> Self {
        Self {
            by_name: RwLock::new(
                STD_COLLS
                    .iter()
                    .map(|(k, v)| ((*v).to_owned(), *k))
                    .collect(),
            ),
            by_id: RwLock::new(
                STD_COLLS
                    .iter()
                    .map(|(k, v)| (*k, (*v).to_owned()))
                    .collect(),
            ),
        }
    }
}

/// Logs internal bb8 errors
#[derive(Debug, Clone, Copy)]
pub struct LoggingErrorSink;

impl<E: failure::Fail> ErrorSink<E> for LoggingErrorSink {
    fn sink(&self, e: E) {
        error!("bb8 Error: {}", e);
        let event = sentry::integrations::failure::event_from_fail(&e);
        sentry::capture_event(event);
    }

    fn boxed_clone(&self) -> Box<dyn ErrorSink<E>> {
        Box::new(*self)
    }
}
