use std::time::Duration;

use async_trait::async_trait;
use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
    Connection,
};
use diesel_logger::LoggingConnection;
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::TestTransactionCustomizer;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_settings::Settings;

use super::{
    error::{DbError, DbResult},
    models::{DbTrait, TokenserverDb},
};

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;

    embedded_migrations::run(&LoggingConnection::new(conn))?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: Pool<ConnectionManager<MysqlConnection>>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        if settings.run_migrations {
            run_embedded_migrations(&settings.database_url)?;
        }

        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.clone());
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

        Ok(Self {
            inner: builder.build(manager)?,
            metrics: metrics.clone(),
            spanner_node_id: settings.spanner_node_id,
            service_id: None,
        })
    }

    pub fn get_sync(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().map_err(DbError::from)?;

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
        ))
    }

    #[cfg(test)]
    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        let pool = self.clone();
        let conn = syncserver_db_common::run_on_blocking_threadpool(
            move || pool.inner.get().map_err(DbError::from),
            DbError::internal,
        )
        .await?;

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
        ))
    }
}

#[async_trait]
impl DbPoolTrait for TokenserverPool {
    async fn get(&self) -> Result<Box<dyn DbTrait>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_pool", None);

        let pool = self.clone();
        let conn = syncserver_db_common::run_on_blocking_threadpool(
            move || pool.inner.get().map_err(DbError::from),
            DbError::internal,
        )
        .await?;

        Ok(Box::new(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
        )) as Box<dyn DbTrait>)
    }

    fn box_clone(&self) -> Box<dyn DbPoolTrait> {
        Box::new(self.clone())
    }
}

#[async_trait]
pub trait DbPoolTrait: Sync + Send + GetPoolState {
    async fn get(&self) -> Result<Box<dyn DbTrait>, DbError>;

    fn box_clone(&self) -> Box<dyn DbPoolTrait>;
}

impl GetPoolState for TokenserverPool {
    fn state(&self) -> PoolState {
        self.inner.state().into()
    }
}

impl GetPoolState for Box<dyn DbPoolTrait> {
    fn state(&self) -> PoolState {
        (**self).state()
    }
}

impl Clone for Box<dyn DbPoolTrait> {
    fn clone(&self) -> Box<dyn DbPoolTrait> {
        self.box_clone()
    }
}
