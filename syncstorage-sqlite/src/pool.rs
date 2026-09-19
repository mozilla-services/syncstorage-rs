use async_trait::async_trait;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
    time::Duration,
};

use deadpool::managed::PoolError;
use diesel::{Connection, sqlite::SqliteConnection};
use diesel_async::{
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
    sync_connection_wrapper::SyncConnectionWrapper,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{GetPoolStatus, manager_config_with_logging};
use syncstorage_db_common::{Db, DbPool, STD_COLLS};
use syncstorage_settings::{Quota, Settings};
use tokio::task::spawn_blocking;

use super::{DbError, DbResult, db::SqliteDb};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// SQLite has no native async driver (unlike MySQL/Postgres), so the
/// underlying synchronous connection is wrapped to present the same
/// `AsyncConnection` interface the rest of the workspace relies on. Queries
/// run on diesel-async's blocking threadpool under the hood.
pub(crate) type SqliteAsyncConnection = SyncConnectionWrapper<SqliteConnection>;

pub(crate) type Conn = Object<SqliteAsyncConnection>;

/// SQLite doesn't recognize a `sqlite://` scheme prefix the way MySQL/Postgres
/// URLs do; diesel expects a plain path (or `:memory:`).
fn normalize_sqlite_url(url: &str) -> String {
    url.strip_prefix("sqlite://").unwrap_or(url).to_owned()
}

#[derive(Clone)]
pub struct SqliteDbPool {
    /// Pool of db connections
    pool: Pool<SqliteAsyncConnection>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
    quota: Quota,
    /// Connection string, normalized to a bare filesystem path (or
    /// `:memory:`), used to run migrations on a dedicated connection.
    database_url: String,
}

impl SqliteDbPool {
    /// Creates a new pool of Sqlite db connections.
    ///
    /// Doesn't initialize the db (does not run migrations).
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        let database_url = normalize_sqlite_url(&settings.database_url);
        let manager = AsyncDieselConnectionManager::<SqliteAsyncConnection>::new_with_config(
            &database_url,
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
            .map_err(|e| DbError::internal(format!("Couldn't build Sqlite Db Pool: {e}")))?;

        Ok(Self {
            pool,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
            quota: Quota {
                size: settings.limits.max_quota_limit as usize,
                enabled: settings.enable_quota,
                enforced: settings.enforce_quota,
            },
            database_url,
        })
    }

    /// Spawn a task to periodically evict idle connections. Noop for the
    /// sqlite impl (single-file db, nothing useful to evict).
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

    pub async fn get_sqlite_db(&self) -> DbResult<SqliteDb> {
        Ok(SqliteDb::new(
            self.get_conn().await?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            &self.quota,
        ))
    }
}

fn sweeper() {}

#[async_trait]
impl DbPool for SqliteDbPool {
    type Error = DbError;

    async fn init(&mut self) -> Result<(), Self::Error> {
        // SQLite DDL statements implicitly commit, which could disrupt
        // SqliteDbPool's begin_test_transaction during tests, and diesel's
        // migration harness is inherently synchronous, so this runs on its
        // own plain (non-pooled) blocking connection rather than reusing
        // `run_embedded_migrations` (which assumes a genuinely async conn).
        let database_url = self.database_url.clone();
        spawn_blocking(move || -> DbResult<()> {
            let mut conn = SqliteConnection::establish(&database_url)
                .map_err(|e| DbError::internal(format!("Couldn't open sqlite db: {e}")))?;
            conn.run_pending_migrations(MIGRATIONS)
                .map_err(|e| DbError::internal(format!("Couldn't run migrations: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| DbError::internal(format!("Migration task panicked: {e}")))?
    }

    async fn get<'a>(&'a self) -> DbResult<Box<dyn Db<Error = Self::Error>>> {
        Ok(Box::new(self.get_sqlite_db().await?) as Box<dyn Db<Error = Self::Error>>)
    }

    fn validate_batch_id(&self, id: String) -> DbResult<()> {
        super::db::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool<Error = Self::Error>> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for SqliteDbPool {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SqliteDbPool")
            .field("coll_cache", &self.coll_cache)
            .finish()
    }
}

impl GetPoolStatus for SqliteDbPool {
    fn status(&self) -> deadpool::Status {
        self.pool.status()
    }
}

#[derive(Debug)]
pub(super) struct CollectionCache {
    pub by_name: RwLock<HashMap<String, i32>>,
    pub by_id: RwLock<HashMap<i32, String>>,
}

impl CollectionCache {
    pub fn put(&self, id: i32, name: String) -> DbResult<()> {
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

    #[allow(dead_code)]
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
