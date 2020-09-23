use actix_web::web::block;

use async_trait::async_trait;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;

use super::models::{MysqlDb, Result};
#[cfg(test)]
use super::test::TestTransactionCustomizer;
use crate::db::{
    error::DbError,
    results::{self, PoolState},
    Db, DbPool, STD_COLLS,
};
use crate::error::{ApiError, ApiResult};
use crate::server::metrics::Metrics;
use crate::settings::Settings;

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(settings: &Settings) -> Result<()> {
    let conn = MysqlConnection::establish(&settings.database_url)?;
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
    quota: usize,
    quota_enabled: bool,
}

impl MysqlDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Also initializes the Mysql db, ensuring all migrations are ran.
    pub fn new(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings, metrics)
    }

    pub fn new_without_migrations(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
        let builder = Pool::builder()
            .max_size(settings.database_pool_max_size.unwrap_or(10))
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
            quota: settings.limits.max_quota_limit as usize,
            quota_enabled: settings.enable_quota,
        })
    }

    pub fn get_sync(&self) -> Result<MysqlDb> {
        Ok(MysqlDb::new(
            self.pool.get()?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
            &self.quota,
            self.quota_enabled,
        ))
    }
}

#[async_trait(?Send)]
impl DbPool for MysqlDbPool {
    async fn get<'a>(&'a self) -> ApiResult<Box<dyn Db<'a>>> {
        let pool = self.clone();
        let db = block(move || pool.get_sync().map_err(ApiError::from)).await?;

        Ok(Box::new(db) as Box<dyn Db<'a>>)
    }

    fn state(&self) -> results::PoolState {
        self.pool.state().into()
    }

    fn validate_batch_id(&self, id: String) -> Result<()> {
        super::batch::validate_batch_id(&id)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
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

impl From<diesel::r2d2::State> for PoolState {
    fn from(state: diesel::r2d2::State) -> PoolState {
        PoolState {
            connections: state.connections,
            idle_connections: state.idle_connections,
        }
    }
}
