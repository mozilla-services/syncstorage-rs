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
use futures::future::lazy;
use tokio_threadpool::ThreadPool;

use super::models::{MysqlDb, Result};
#[cfg(test)]
use super::test::TestTransactionCustomizer;
use db::{error::DbError, Db, DbFuture, DbPool, STD_COLLS};
use settings::Settings;

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(settings: &Settings) -> Result<()> {
    let conn = MysqlConnection::establish(&settings.database_url)?;
    Ok(embedded_migrations::run(&conn)?)
}

#[derive(Clone)]
pub struct MysqlDbPool {
    /// Pool of db connections
    pool: Pool<ConnectionManager<MysqlConnection>>,
    /// Thread Pool for running synchronous db calls
    thread_pool: Arc<ThreadPool>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,
}

impl MysqlDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Also initializes the Mysql db, ensuring all migrations are ran.
    pub fn new(settings: &Settings) -> Result<Self> {
        run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings)
    }

    pub fn new_without_migrations(settings: &Settings) -> Result<Self> {
        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
        let builder = Pool::builder().max_size(settings.database_pool_max_size.unwrap_or(10));

        #[cfg(test)]
        let builder = if settings.database_use_test_transactions {
            builder.connection_customizer(Box::new(TestTransactionCustomizer))
        } else {
            builder
        };

        // XXX: tokio_threadpool:ThreadPool probably not the best option: db
        // calls are longerish running/blocking, so should likely run on
        // ThreadPool's "backup threads", but it defaults scheduling to its
        // "worker threads"
        // XXX: allow configuring the ThreadPool size
        Ok(Self {
            pool: builder.build(manager)?,
            thread_pool: Arc::new(ThreadPool::new()),
            coll_cache: Default::default(),
        })
    }

    pub fn get_sync(&self) -> Result<MysqlDb> {
        Ok(MysqlDb::new(
            self.pool.get()?,
            Arc::clone(&self.thread_pool),
            Arc::clone(&self.coll_cache),
        ))
    }
}

impl DbPool for MysqlDbPool {
    fn get(&self) -> DbFuture<Box<dyn Db>> {
        let pool = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            pool.get_sync()
                .map(|db| Box::new(db) as Box<dyn Db>)
                .map_err(Into::into)
        })))
    }
}

impl fmt::Debug for MysqlDbPool {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MysqlDbPool {{ coll_cache: {:?} }}", self.coll_cache)
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
