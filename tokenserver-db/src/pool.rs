use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use diesel::r2d2::{ConnectionManager, Pool};
use syncserver_common::{BlockingThreadpool, Metrics};
#[cfg(debug_assertions)]
use syncserver_db_common::test::TestTransactionCustomizer;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_db_common::error::{DbError, DbResult};
#[cfg(feature = "mysql")]
use tokenserver_db_mysql::pool::run_embedded_migrations;
#[cfg(feature = "sqlite")]
use tokenserver_db_sqlite::pool::run_embedded_migrations;
use tokenserver_settings::Settings;

use super::{
    models::{Db, TokenserverDb},
    Conn,
};

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: Pool<ConnectionManager<Conn>>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    blocking_threadpool: Arc<BlockingThreadpool>,
    pub timeout: Option<Duration>,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        if settings.run_migrations {
            run_embedded_migrations(&settings.database_url)?;
        }

        // SQLite can't handle its uri prefix
        let database_url = settings.database_url
            .strip_prefix("sqlite://")
            .unwrap_or(&settings.database_url);

        let manager = ConnectionManager::<Conn>::new(database_url);
        let builder = Pool::builder()
            .max_size(settings.database_pool_max_size)
            .connection_timeout(Duration::from_secs(
                settings.database_pool_connection_timeout.unwrap_or(30) as u64,
            ))
            .min_idle(settings.database_pool_min_idle);

        #[cfg(debug_assertions)]
        let builder = if _use_test_transactions {
            builder.connection_customizer(Box::new(TestTransactionCustomizer))
        } else {
            builder
        };
        let timeout = settings
            .database_request_timeout
            .map(|v| Duration::from_secs(v as u64));

        Ok(Self {
            inner: builder.build(manager)?,
            metrics: metrics.clone(),
            spanner_node_id: settings.spanner_node_id,
            service_id: None,
            blocking_threadpool,
            timeout,
        })
    }

    pub fn get_sync(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().map_err(DbError::from)?;

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.blocking_threadpool.clone(),
            self.timeout,
        ))
    }

    #[cfg(test)]
    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        let pool = self.clone();
        let conn = self
            .blocking_threadpool
            .spawn(move || pool.inner.get().map_err(DbError::from))
            .await?;

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.blocking_threadpool.clone(),
            self.timeout,
        ))
    }
}

#[async_trait]
impl DbPool for TokenserverPool {
    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_pool", None);

        let pool = self.clone();
        let conn = self
            .blocking_threadpool
            .spawn(move || pool.inner.get().map_err(DbError::from))
            .await?;

        Ok(Box::new(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.blocking_threadpool.clone(),
            self.timeout,
        )) as Box<dyn Db>)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

#[async_trait]
pub trait DbPool: Sync + Send + GetPoolState {
    async fn get(&self) -> Result<Box<dyn Db>, DbError>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl GetPoolState for TokenserverPool {
    fn state(&self) -> PoolState {
        self.inner.state().into()
    }
}

impl GetPoolState for Box<dyn DbPool> {
    fn state(&self) -> PoolState {
        (**self).state()
    }
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}
