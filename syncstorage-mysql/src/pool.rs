use async_trait::async_trait;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
    time::Duration,
};

use deadpool::managed::PoolError;
use diesel_async::{
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncMysqlConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{
    establish_connection_with_logging, manager_config_with_logging, run_embedded_migrations,
    GetPoolState, PoolState,
};
use syncstorage_db_common::{Db, DbPool, STD_COLLS};
use syncstorage_settings::{Quota, Settings};

use super::{db::MysqlDb, DbError, DbResult};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) type Conn = Object<AsyncMysqlConnection>;

#[derive(Clone)]
pub struct MysqlDbPool {
    /// Pool of db connections
    pool: Pool<AsyncMysqlConnection>,
    /// Thread Pool for running synchronous db calls
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
    quota: Quota,
    database_url: String,
}

impl MysqlDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Doesn't initialize the db (does not run migrations).
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        let manager = AsyncDieselConnectionManager::<AsyncMysqlConnection>::new_with_config(
            &settings.database_url,
            manager_config_with_logging(),
        );

        let wait = settings
            .database_pool_connection_timeout
            .map(|seconds| Duration::from_secs(seconds as u64));
        let timeouts = deadpool::managed::Timeouts {
            wait,
            ..Default::default()
        };
        let config = deadpool::managed::PoolConfig {
            max_size: settings.database_pool_max_size as usize,
            timeouts,
            ..Default::default()
        };

        let builder = Pool::builder(manager)
            .config(config)
            .runtime(deadpool::Runtime::Tokio1);
        #[cfg(debug_assertions)]
        let builder = if settings.database_use_test_transactions {
            builder.post_create(deadpool::managed::Hook::async_fn(|conn, _| {
                Box::pin(async { test_transaction_hook(conn).await })
            }))
        } else {
            builder
        };
        let pool = builder
            .build()
            .map_err(|e| DbError::internal(format!("Couldn't build Db Pool: {e}")))?;

        Ok(Self {
            pool,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
            quota: Quota {
                size: settings.limits.max_quota_limit as usize,
                enabled: settings.enable_quota,
                enforced: settings.enforce_quota,
            },
            database_url: settings.database_url.clone(),
        })
    }

    /// Spawn a task to periodically evict idle connections. Calls wrapper sweeper fn
    ///  to use pool.retain, retaining objects only if they are shorter in duration than
    ///  defined max_idle. Noop for mysql impl.
    pub fn spawn_sweeper(&self, _interval: Duration) {
        sweeper()
    }

    async fn get_conn(&self) -> DbResult<Conn> {
        self.pool.get().await.map_err(|e| match e {
            PoolError::Backend(be) => match be {
                diesel_async::pooled_connection::PoolError::ConnectionError(ce) => ce.into(),
                diesel_async::pooled_connection::PoolError::QueryError(dbe) => dbe.into(),
            },
            PoolError::Timeout(timeout_type) => DbError::pool_timeout(timeout_type),
            _ => DbError::internal(format!("deadpool PoolError: {e}")),
        })
    }

    pub async fn get_mysql_db(&self) -> DbResult<MysqlDb> {
        Ok(MysqlDb::new(
            self.get_conn().await?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            &self.quota,
        ))
    }
}

/// Sweeper to retain only the objects specified within the closure.
/// In this context, if a Spanner connection is unutilized, we want it
/// to release the given connections.
/// See: https://docs.rs/deadpool/latest/deadpool/managed/struct.Pool.html#method.retain
/// Noop for mysql impl
fn sweeper() {}

#[async_trait]
impl DbPool for MysqlDbPool {
    type Error = DbError;

    async fn init(&mut self) -> Result<(), Self::Error> {
        // Mysql DDL statements implicitly commit which could disrupt
        // MysqlPool's begin_test_transaction during tests. So this runs on
        // its own separate conn
        let conn =
            establish_connection_with_logging::<AsyncMysqlConnection>(&self.database_url).await?;
        run_embedded_migrations(conn, MIGRATIONS).await?;
        Ok(())
    }

    async fn get<'a>(&'a self) -> DbResult<Box<dyn Db<Error = Self::Error>>> {
        Ok(Box::new(self.get_mysql_db().await?) as Box<dyn Db<Error = Self::Error>>)
    }

    fn validate_batch_id(&self, id: String) -> DbResult<()> {
        super::db::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool<Error = Self::Error>> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for MysqlDbPool {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("MysqlDbPool")
            .field("coll_cache", &self.coll_cache)
            .finish()
    }
}

impl GetPoolState for MysqlDbPool {
    fn state(&self) -> PoolState {
        self.pool.status().into()
    }
}

#[derive(Debug)]
pub(super) struct CollectionCache {
    pub by_name: RwLock<HashMap<String, i32>>,
    pub by_id: RwLock<HashMap<i32, String>>,
}

impl CollectionCache {
    pub fn put(&self, id: i32, name: String) -> DbResult<()> {
        // XXX: should this emit a metric?
        // XXX: should probably either lock both simultaneously during
        // writes or use an RwLock alternative
        self.by_name
            .write()
            .map_err(|_| DbError::internal("by_name write".to_owned()))?
            .insert(name.clone(), id);
        self.by_id
            .write()
            .map_err(|_| DbError::internal("by_id write".to_owned()))?
            .insert(id, name);
        Ok(())
    }

    pub fn get_id(&self, name: &str) -> DbResult<Option<i32>> {
        Ok(self
            .by_name
            .read()
            .map_err(|_| DbError::internal("by_name read".to_owned()))?
            .get(name)
            .cloned())
    }

    pub fn get_name(&self, id: i32) -> DbResult<Option<String>> {
        Ok(self
            .by_id
            .read()
            .map_err(|_| DbError::internal("by_id read".to_owned()))?
            .get(&id)
            .cloned())
    }

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
