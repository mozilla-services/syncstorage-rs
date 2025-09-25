use std::time::Duration;

use async_trait::async_trait;
use diesel::Connection;
use diesel_async::{
    async_connection_wrapper::AsyncConnectionWrapper,
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncMysqlConnection,
};
use diesel_logger::LoggingConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_db_common::{params, Db, DbError, DbPool, DbResult};

use tokenserver_settings::Settings;
use tokio::task::spawn_blocking;

use super::models::TokenserverDb;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) type Conn = Object<AsyncMysqlConnection>;

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
///
/// Note that this runs as a plain diesel blocking method as diesel_async
/// doesn't support async migrations (but we utilize its connection via its
/// [AsyncConnectionWrapper])
fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = AsyncConnectionWrapper::<AsyncMysqlConnection>::establish(database_url)?;

    LoggingConnection::new(conn).run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

#[derive(Clone)]
pub struct TokenserverPool {
    /// Pool of db connections
    inner: Pool<AsyncMysqlConnection>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
    run_migrations: bool,
    database_url: String,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        let manager =
            AsyncDieselConnectionManager::<AsyncMysqlConnection>::new(&settings.database_url);

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
            run_migrations: settings.run_migrations,
            database_url: settings.database_url.clone(),
        })
    }

    pub async fn get_tokenserver_db(&self) -> Result<TokenserverDb, DbError> {
        Ok(TokenserverDb::new(
            self.inner.get().await?,
            &self.metrics,
            self.service_id,
            self.spanner_node_id,
            self.timeout,
        ))
    }

    /// Cache the common "sync-1.5" service_id
    async fn init_service_id(&mut self) -> Result<(), tokenserver_common::TokenserverError> {
        let service_id = self
            .get()
            .await?
            .get_service_id(params::GetServiceId {
                service: "sync-1.5".to_owned(),
            })
            .await?;
        self.service_id = Some(service_id.id);
        Ok(())
    }
}

#[async_trait(?Send)]
impl DbPool for TokenserverPool {
    async fn init(&mut self) -> Result<(), DbError> {
        if self.run_migrations {
            let database_url = self.database_url.clone();
            spawn_blocking(move || run_embedded_migrations(&database_url))
                .await
                .map_err(|e| DbError::internal(format!("Couldn't spawn migrations: {e}")))??;
        }

        // NOTE: Provided there's a "sync-1.5" service record in the database, it is highly
        // unlikely for this query to fail outside of network failures or other random errors
        let _ = self.init_service_id().await;
        Ok(())
    }

    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_pool", None);
        Ok(Box::new(self.get_tokenserver_db().await?) as Box<dyn Db>)
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

impl GetPoolState for TokenserverPool {
    fn state(&self) -> PoolState {
        self.inner.status().into()
    }
}
