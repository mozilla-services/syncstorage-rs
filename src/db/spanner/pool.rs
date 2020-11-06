use std::{collections::HashMap, fmt, sync::Arc};

use async_trait::async_trait;
use bb8::ErrorSink;
use tokio::sync::RwLock;

use crate::{
    db::{error::DbError, results, Db, DbPool, STD_COLLS},
    error::ApiResult,
    server::metrics::Metrics,
    settings::Settings,
};

pub use super::manager::Conn;
use super::{
    manager::{SpannerSession, SpannerSessionManager},
    models::Result,
    models::SpannerDb,
};

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
    pub async fn put(&self, id: i32, name: String) {
        // XXX: should this emit a metric?
        // XXX: one RwLock might be sufficient?
        self.by_name.write().await.insert(name.clone(), id);
        self.by_id.write().await.insert(id, name);
    }

    pub async fn get_id(&self, name: &str) -> Option<i32> {
        self.by_name.read().await.get(name).cloned()
    }

    pub async fn get_name(&self, id: i32) -> Option<String> {
        self.by_id.read().await.get(&id).cloned()
    }

    /// Get multiple names, returning a tuple of both the mapping of
    /// ids to their names and a Vec of ids not found in the cache.
    pub async fn get_names(&self, ids: &[i32]) -> (HashMap<i32, String>, Vec<i32>) {
        let len = ids.len();
        // the ids array shouldn't be very large but avoid reallocating
        // while holding the lock
        let mut names = HashMap::with_capacity(len);
        let mut missing = Vec::with_capacity(len);
        let by_id = self.by_id.read().await;
        for &id in ids {
            if let Some(name) = by_id.get(&id) {
                names.insert(id, name.to_owned());
            } else {
                missing.push(id)
            }
        }
        (names, missing)
    }

    #[cfg(test)]
    pub async fn clear(&self) {
        self.by_name.write().await.clear();
        self.by_id.write().await.clear();
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
