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
#[cfg(test)]
use diesel_logger::LoggingConnection;
use syncstorage_db_common::{error::DbError, Db, DbPool, GetPoolState, PoolState, STD_COLLS};

use super::models::{MysqlDb, Result};
#[cfg(test)]
use super::test::TestTransactionCustomizer;
use crate::db;
use crate::server::metrics::Metrics;
use crate::settings::{Quota, Settings};

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(database_url: &str) -> Result<()> {
    let conn = MysqlConnection::establish(database_url)?;
    #[cfg(test)]
    // XXX: this doesn't show the DDL statements
    // https://github.com/shssoichiro/diesel-logger/issues/1
    embedded_migrations::run(&LoggingConnection::new(conn))?;
    #[cfg(not(test))]
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
}

impl MysqlDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Also initializes the Mysql db, ensuring all migrations are ran.
    pub fn new(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        run_embedded_migrations(&settings.database_url)?;
        Self::new_without_migrations(settings, metrics)
    }

    pub fn new_without_migrations(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
        let builder = Pool::builder()
            .max_size(settings.database_pool_max_size)
            .connection_timeout(Duration::from_secs(
                settings.database_pool_connection_timeout.unwrap_or(30) as u64,
            ))
            .min_idle(settings.database_pool_min_idle);

        #[cfg(test)]
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
        })
    }

    pub fn get_sync(&self) -> Result<MysqlDb> {
        Ok(MysqlDb::new(
            self.pool.get()?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            &self.quota,
        ))
    }
}

#[async_trait]
impl DbPool for MysqlDbPool {
    async fn get<'a>(&'a self) -> Result<Box<dyn Db<'a>>> {
        let pool = self.clone();
        let db = db::run_on_blocking_threadpool(move || pool.get_sync()).await?;

        Ok(Box::new(db) as Box<dyn Db<'a>>)
    }

    fn validate_batch_id(&self, id: String) -> Result<()> {
        super::batch::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

impl GetPoolState for MysqlDbPool {
    fn state(&self) -> PoolState {
        self.pool.state().into()
    }
}

impl fmt::Debug for MysqlDbPool {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("MysqlDbPool")
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
