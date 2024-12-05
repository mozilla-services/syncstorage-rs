use async_trait::async_trait;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
    time::Duration,
};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection,
};
#[cfg(debug_assertions)]
use diesel_logger::LoggingConnection;
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(debug_assertions)]
use syncserver_db_common::test::TestTransactionCustomizer;
use syncserver_db_common::{GetPoolState, PoolState};
use syncstorage_db_common::{Db, DbPool, STD_COLLS};
use syncstorage_settings::{Quota, Settings};

use super::{error::DbError, models::MysqlDb, DbResult};

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;
    #[cfg(debug_assertions)]
    // XXX: this doesn't show the DDL statements
    // https://github.com/shssoichiro/diesel-logger/issues/1
    embedded_migrations::run(&LoggingConnection::new(conn))?;
    #[cfg(not(debug_assertions))]
    embedded_migrations::run(&conn)?;
    Ok(())
}

#[derive(Clone)]
pub struct MysqlDbPool {
    /// Pool of db connections
    pool: Pool<ConnectionManager<MysqlConnection>>,
    /// Thread Pool for running synchronous db calls
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
    quota: Quota,
    blocking_threadpool: Arc<BlockingThreadpool>,
}

impl MysqlDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Also initializes the Mysql db, ensuring all migrations are ran.
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        run_embedded_migrations(&settings.database_url)?;
        Self::new_without_migrations(settings, metrics, blocking_threadpool)
    }

    pub fn new_without_migrations(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> DbResult<Self> {
        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
        let builder = Pool::builder()
            .max_size(settings.database_pool_max_size)
            .connection_timeout(Duration::from_secs(
                settings.database_pool_connection_timeout.unwrap_or(30) as u64,
            ))
            .min_idle(settings.database_pool_min_idle);

        #[cfg(debug_assertions)]
        let builder = if settings.database_use_test_transactions {
            builder.connection_customizer(Box::new(TestTransactionCustomizer))
        } else {
            builder
        };

        Ok(Self {
            pool: builder.build(manager)?,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
            quota: Quota {
                size: settings.limits.max_quota_limit as usize,
                enabled: settings.enable_quota,
                enforced: settings.enforce_quota,
            },
            blocking_threadpool,
        })
    }

    /// Spawn a task to periodically evict idle connections. Calls wrapper sweeper fn
    ///  to use pool.retain, retaining objects only if they are shorter in duration than
    ///  defined max_idle. Noop for mysql impl.
    pub fn spawn_sweeper(&self, _interval: Duration) {
        sweeper()
    }

    pub fn get_sync(&self) -> DbResult<MysqlDb> {
        Ok(MysqlDb::new(
            self.pool.get()?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            &self.quota,
            self.blocking_threadpool.clone(),
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

    async fn get<'a>(&'a self) -> DbResult<Box<dyn Db<Error = Self::Error>>> {
        let pool = self.clone();
        self.blocking_threadpool
            .spawn(move || pool.get_sync())
            .await
            .map(|db| Box::new(db) as Box<dyn Db<Error = Self::Error>>)
    }

    fn validate_batch_id(&self, id: String) -> DbResult<()> {
        super::batch::validate_batch_id(&id)
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
        self.pool.state().into()
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
