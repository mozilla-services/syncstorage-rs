use std::time::Duration;

use async_trait::async_trait;
use diesel::{Connection, sqlite::SqliteConnection};
use diesel_async::{
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
    sync_connection_wrapper::SyncConnectionWrapper,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{GetPoolStatus, manager_config_with_logging};
use tokenserver_db_common::{Db, DbError, DbPool, DbResult, params};
use tokio::task::spawn_blocking;

use tokenserver_settings::Settings;

use crate::db::TokenserverSqliteDb;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// SQLite has no native async driver (unlike MySQL/Postgres), so the
/// underlying synchronous connection is wrapped to present the same
/// `AsyncConnection` interface the rest of the workspace relies on.
pub(crate) type SqliteAsyncConnection = SyncConnectionWrapper<SqliteConnection>;

pub(crate) type Conn = Object<SqliteAsyncConnection>;

/// SQLite doesn't recognize a `sqlite://` scheme prefix; diesel expects a
/// plain path (or `:memory:`).
fn normalize_sqlite_url(url: &str) -> String {
    url.strip_prefix("sqlite://").unwrap_or(url).to_owned()
}

#[derive(Clone)]
pub struct TokenserverSqlitePool {
    /// Pool of db connections
    inner: Pool<SqliteAsyncConnection>,
    metrics: Metrics,
    // This field is public so the service ID can be set after the pool is created
    pub service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
    run_migrations: bool,
    database_url: String,
    init_node_url: Option<String>,
    init_node_capacity: i32,
}

impl TokenserverSqlitePool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        let database_url = normalize_sqlite_url(&settings.database_url);
        let manager = AsyncDieselConnectionManager::<SqliteAsyncConnection>::new_with_config(
            &database_url,
            manager_config_with_logging(),
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
            run_migrations: settings.run_migrations,
            database_url,
            init_node_url: settings.init_node_url.clone(),
            init_node_capacity: settings.init_node_capacity,
        })
    }

    pub async fn get_tokenserver_db(&self) -> Result<TokenserverSqliteDb, DbError> {
        Ok(TokenserverSqliteDb::new(
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

    /// Bootstrap the initial Sync 1.5 node record if init_node_url is set.
    async fn init_sync15_node(&mut self, node_url: String, capacity: i32) -> Result<(), DbError> {
        let node_added = self
            .get()
            .await?
            .insert_sync15_node(params::Sync15Node {
                node: node_url.clone(),
                capacity,
            })
            .await?;
        if node_added {
            info!("Initialized syncstorage node entry, node: {node_url:?} capacity: {capacity}");
        }
        Ok(())
    }
}

#[async_trait(?Send)]
impl DbPool for TokenserverSqlitePool {
    async fn init(&mut self) -> Result<(), DbError> {
        if self.run_migrations {
            // SQLite DDL statements implicitly commit, which could disrupt
            // begin_test_transaction during tests, and diesel's migration
            // harness is inherently synchronous, so this runs on its own
            // plain (non-pooled) blocking connection.
            let database_url = self.database_url.clone();
            spawn_blocking(move || -> DbResult<()> {
                let mut conn = SqliteConnection::establish(&database_url)
                    .map_err(|e| DbError::internal(format!("Couldn't open sqlite db: {e}")))?;
                conn.run_pending_migrations(MIGRATIONS)
                    .map_err(|e| DbError::internal(format!("Couldn't run migrations: {e}")))?;
                Ok(())
            })
            .await
            .map_err(|e| DbError::internal(format!("Migration task panicked: {e}")))??;
        }

        // NOTE: Provided there's a "sync-1.5" service record in the database, it is highly
        // unlikely for this query to fail outside of network failures or other random errors
        let _ = self.init_service_id().await;

        // Init the Sync 1.5 node record if init_node_url is set
        if let Some(node_url) = self.init_node_url.clone() {
            self.init_sync15_node(node_url, self.init_node_capacity)
                .await?;
        }

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

impl GetPoolStatus for TokenserverSqlitePool {
    fn status(&self) -> deadpool::Status {
        self.inner.status()
    }
}
