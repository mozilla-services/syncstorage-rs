use actix_web::web::block;
use futures::future::TryFutureExt;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use diesel::r2d2;
use diesel::r2d2::Pool;

use scheduled_thread_pool::ScheduledThreadPool;

use super::models::Result;
#[cfg(test)]
use super::test_util::SpannerTestTransactionCustomizer;
use crate::db::{error::DbError, Db, DbFuture, DbPool, STD_COLLS};
use crate::server::metrics::Metrics;
use crate::settings::Settings;

use super::manager::SpannerConnectionManager;
use super::models::SpannerDb;

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
    pool: Pool<SpannerConnectionManager>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
}

impl SpannerDbPool {
    /// Creates a new pool of Mysql db connections.
    ///
    /// Also initializes the Mysql db, ensuring all migrations are ran.
    pub fn new(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        //run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings, metrics)
    }

    pub fn new_without_migrations(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        let manager = SpannerConnectionManager::new(settings)?;
        let max_size = settings.database_pool_max_size.unwrap_or(10);
        // r2d2 creates max_size count of db connections on creation via its
        // own thread_pool. increase its default size to quicken their
        // creation, accommodating large max_size values (otherwise it may
        // timeout)
        let r2d2_thread_pool_size = ((max_size as f32 * 0.05) as usize).max(3);
        let builder = r2d2::Pool::builder()
            .max_size(max_size)
            .thread_pool(Arc::new(ScheduledThreadPool::new(r2d2_thread_pool_size)));
        let mut metrics = metrics.clone();
        metrics.start_timer("storage.spanner.pool.get", None);

        #[cfg(test)]
        let builder = if settings.database_use_test_transactions {
            builder.connection_customizer(Box::new(SpannerTestTransactionCustomizer))
        } else {
            builder
        };

        Ok(Self {
            pool: builder.build(manager)?,
            coll_cache: Default::default(),
            metrics,
        })
    }

    pub fn get_sync(&self) -> Result<SpannerDb> {
        Ok(SpannerDb::new(
            self.pool.get()?,
            Arc::clone(&self.coll_cache),
            &self.metrics,
        ))
    }
}

impl DbPool for SpannerDbPool {
    fn get(&self) -> DbFuture<Box<dyn Db>> {
        let pool = self.clone();
        Box::pin(
            block(move || {
                pool.get_sync()
                    .map(|db| Box::new(db) as Box<dyn Db>)
                    .map_err(Into::into)
            })
            .map_err(Into::into),
        )
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

impl fmt::Debug for SpannerDbPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpannerDbPool {{ coll_cache: {:?} }}", self.coll_cache)
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
