use async_trait::async_trait;
use bb8;
use bb8::Pool;
use futures::future::TryFutureExt;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, RwLock},
};

use super::models::Result;
use crate::db::{error::DbError, results, Db, DbFuture, DbPool, STD_COLLS};
use crate::server::metrics::Metrics;
use crate::settings::Settings;

use super::manager::{SpannerConnectionManager, SpannerSession};
use super::models::SpannerDb;
use crate::error::ApiResult;

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
    pool: Pool<SpannerConnectionManager<SpannerSession>>,
    /// In-memory cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    metrics: Metrics,
}

impl SpannerDbPool {
    /// Creates a new pool of Spanner db connections.
    pub async fn new(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        //run_embedded_migrations(settings)?;
        Self::new_without_migrations(settings, metrics).await
    }

    pub async fn new_without_migrations(settings: &Settings, metrics: &Metrics) -> Result<Self> {
        let manager = SpannerConnectionManager::<SpannerSession>::new(settings)?;
        let max_size = settings.database_pool_max_size.unwrap_or(10);
        let builder = bb8::Pool::builder().max_size(max_size);

        Ok(Self {
            pool: builder.build(manager).await?,
            coll_cache: Default::default(),
            metrics: metrics.clone(),
        })
    }

    pub async fn get_async(&self) -> Result<SpannerDb<'_>> {
        let conn = self.pool.get().await?;
        Ok(SpannerDb::new(
            conn,
            Arc::clone(&self.coll_cache),
            &self.metrics,
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
        self.pool.state().into()
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
