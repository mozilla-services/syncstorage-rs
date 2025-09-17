use std::time::Duration;

use deadpool::managed::PoolError;
use diesel::Connection;
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncPgConnection,
};

use diesel_logger::LoggingConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use tokenserver_db_common::error::{DbError, DbResult};

use super::models::TokenserverDb;
use tokenserver_settings::Settings;

/// The `embed_migrations!` macro reads migrations at compile time.
/// This creates a constant that references a list of migrations.
/// See https://docs.rs/diesel_migrations/2.2.0/diesel_migrations/macro.embed_migrations.html
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Connection type defined as an AsyncPgConnection for purposes of abstraction.
pub(crate) type Conn = Object<AsyncPgConnection>;

#[allow(dead_code)]
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = AsyncConnectionWrapper::<AsyncPgConnection>::establish(database_url)?;
    LoggingConnection::new(conn).run_pending_migrations(MIGRATIONS)?;
    Ok(())
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct TokenserverPgPool {
    /// Pool of db connections.
    inner: Pool<AsyncPgConnection>,
    /// Metrics module from synserver-common.
    metrics: Metrics,
    /// This field is public so the service ID can be set after the pool is created.
    pub service_id: Option<i32>,
    /// Optional associated spanner node.
    spanner_node_id: Option<i32>,
    /// Optional pool timeout duration, defined as i32.
    pub timeout: Option<Duration>,
    /// Config setting flag to determine if migrations should run.
    run_migrations: bool,
    /// URL for associated Postgres database
    database_url: String,
}

#[allow(dead_code)]
impl TokenserverPgPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        let manager =
            AsyncDieselConnectionManager::<AsyncPgConnection>::new(&settings.database_url);

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

        let pool = builder.build().map_err(|e| {
            DbError::internal(format!("Couldn't build Tokenserver Postgres Db Pool: {e}"))
        })?;
        let timeout = settings
            .database_request_timeout
            .map(|v| Duration::from_secs(v as u64));

        Ok(Self {
            inner: pool,
            metrics: metrics.clone(),
            spanner_node_id: settings.spanner_node_id,
            service_id: None,
            timeout,
            run_migrations: settings.run_migrations,
            database_url: settings.database_url.clone(),
        })
    }

    async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        let conn = self.inner.get().await.map_err(|e| match e {
            PoolError::Backend(backend_err) => match backend_err {
                diesel_async::pooled_connection::PoolError::ConnectionError(conn_err) => {
                    conn_err.into()
                }
                diesel_async::pooled_connection::PoolError::QueryError(query_err) => {
                    query_err.into()
                }
            },
            PoolError::Timeout(timeout_type) => DbError::pool_timeout(timeout_type),
            _ => DbError::internal(format!("Deadpool PoolError: {e}")),
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
