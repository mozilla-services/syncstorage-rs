use std::time::Duration;

use async_trait::async_trait;
use diesel::{mysql::MysqlConnection, Connection};
use diesel_async::{
    pooled_connection::{deadpool::Pool, AsyncDieselConnectionManager},
    AsyncMysqlConnection,
};
use diesel_logger::LoggingConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_settings::Settings;

use super::{
    error::{DbError, DbResult},
    models::{Db, TokenserverDb},
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) type Conn = diesel_async::pooled_connection::deadpool::Object<AsyncMysqlConnection>;

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;

    LoggingConnection::new(conn).run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: diesel_async::pooled_connection::deadpool::Pool<AsyncMysqlConnection>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        if settings.run_migrations {
            // NOTE: this is blocking
            run_embedded_migrations(&settings.database_url)?;
        }

        let manager = AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(
            settings.database_url.clone(),
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
        let builder = if _use_test_transactions {
            builder.post_create(deadpool::managed::Hook::async_fn(|conn, _| {
                Box::pin(async { test_transaction_hook(conn).await })
            }))
        } else {
            builder
        };
        let pool = builder
            .build()
            .map_err(|e| DbError::internal(format!("Couldn't build Db Pool: {e}")))?;

        let timeout = settings
            .database_request_timeout
            .map(|v| Duration::from_secs(v as u64));

        Ok(Self {
            inner: pool,
            metrics: metrics.clone(),
            spanner_node_id: settings.spanner_node_id,
            service_id: None,
            timeout,
        })
    }

    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().await.map_err(|e| match e {
            deadpool::managed::PoolError::Backend(poole) => match poole {
                diesel_async::pooled_connection::PoolError::ConnectionError(ce) => ce.into(),
                diesel_async::pooled_connection::PoolError::QueryError(dbe) => dbe.into(),
            },
            deadpool::managed::PoolError::Timeout(timeout_type) => {
                DbError::pool_timeout(timeout_type)
            }
            _ => DbError::internal(format!("deadpool PoolError: {e}")),
        })?;

        Ok(TokenserverDb::new(
            conn,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.timeout,
        ))
    }
}

#[async_trait]
impl DbPool for TokenserverPool {
    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_pool", None);
        Ok(Box::new(self.get_tokenserver_db().await?) as Box<dyn Db>)
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
        self.inner.status().into()
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
