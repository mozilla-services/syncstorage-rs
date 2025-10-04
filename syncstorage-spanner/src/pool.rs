use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use actix_web::rt;
use async_trait::async_trait;
use syncserver_common::{BlockingThreadpool, Metrics};
use syncserver_db_common::{GetPoolState, PoolState};
use syncstorage_db_common::{Db, DbPool, STD_COLLS};
use syncstorage_settings::{Quota, Settings};
use tokio::sync::RwLock;

pub(super) use super::manager::Conn;
use super::{db::SpannerDb, error::DbError, manager::SpannerSessionManager, DbResult};

#[derive(Clone)]
pub struct SpannerDbPool {
    /// Pool of db connections
    pool: deadpool::managed::Pool<SpannerSessionManager>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
    quota: Quota,
}

impl SpannerDbPool {
    /// Creates a new pool of Spanner db connections.
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        //run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings, metrics, blocking_threadpool)
    }

    pub fn new_without_migrations(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        let max_size = settings.database_pool_max_size as usize;
        let wait = settings
            .database_pool_connection_timeout
            .map(|seconds| Duration::from_secs(seconds as u64));
        let manager = SpannerSessionManager::new(settings, metrics, blocking_threadpool)?;
        let timeouts = deadpool::managed::Timeouts {
            wait,
            ..Default::default()
        };
        let config = deadpool::managed::PoolConfig {
            max_size,
            timeouts,
            // Prefer LIFO to allow the sweeper task to evict least frequently
            // used connections.
            queue_mode: deadpool::managed::QueueMode::Lifo,
        };
        let pool = deadpool::managed::Pool::builder(manager)
            .config(config)
            .runtime(deadpool::Runtime::Tokio1)
            .build()
            .map_err(|e| DbError::internal(format!("Couldn't build Db Pool: {}", e)))?;

        Ok(Self {
            pool,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
            quota: Quota {
                size: settings.limits.max_quota_limit as usize,
                enabled: settings.enable_quota,
                enforced: settings.enforce_quota,
            },
        })
    }

    pub async fn get_spanner_db(&self) -> DbResult<SpannerDb> {
        let conn = self.pool.get().await.map_err(|e| match e {
            deadpool::managed::PoolError::Backend(dbe) => dbe,
            deadpool::managed::PoolError::Timeout(timeout_type) => {
                DbError::pool_timeout(timeout_type)
            }
            _ => DbError::internal(format!("deadpool PoolError: {}", e)),
        })?;
        Ok(SpannerDb::new(
            conn,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            self.quota,
        ))
    }

    /// Spawn a task to periodically evict idle connections. Calls wrapper sweeper fn
    ///  to use pool.retain, retaining objects only if they are shorter in duration than
    ///  defined max_idle.
    pub fn spawn_sweeper(&self, interval: Duration) {
        let Some(max_idle) = self.pool.manager().settings.max_idle else {
            return;
        };
        let pool = self.pool.clone();
        rt::spawn(async move {
            loop {
                sweeper(&pool, Duration::from_secs(max_idle.into()));
                rt::time::sleep(interval).await;
            }
        });
    }
}

/// Sweeper to retain only the objects specified within the closure.
/// In this context, if a Spanner connection is unutilized, we want it
/// to release the given connection.
/// See: https://docs.rs/deadpool/latest/deadpool/managed/struct.Pool.html#method.retain
fn sweeper(pool: &deadpool::managed::Pool<SpannerSessionManager>, max_idle: Duration) {
    pool.retain(|_, metrics| metrics.last_used() < max_idle);
}

#[async_trait]
impl DbPool for SpannerDbPool {
    type Error = DbError;

    async fn get(&self) -> DbResult<Box<dyn Db<Error = Self::Error>>> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.spanner.get_pool", None);

        self.get_spanner_db()
            .await
            .map(|db| Box::new(db) as Box<dyn Db<Error = Self::Error>>)
    }

    fn validate_batch_id(&self, id: String) -> DbResult<()> {
        super::db::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool<Error = Self::Error>> {
        Box::new(self.clone())
    }
}

impl GetPoolState for SpannerDbPool {
    fn state(&self) -> PoolState {
        self.pool.status().into()
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
pub(super) struct CollectionCache {
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
