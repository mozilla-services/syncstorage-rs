use std::time::Duration;

use async_trait::async_trait;
use diesel_async::{
    AsyncMysqlConnection,
    pooled_connection::{
        AsyncDieselConnectionManager,
        deadpool::{Object, Pool},
    },
};
use diesel_migrations::{EmbeddedMigrations, embed_migrations};
use syncserver_common::Metrics;
#[cfg(debug_assertions)]
use syncserver_db_common::test::test_transaction_hook;
use syncserver_db_common::{
    GetPoolState, PoolState, establish_connection_with_logging, manager_config_with_logging,
    run_embedded_migrations,
};
use tokenserver_db_common::{Db, DbError, DbPool, DbResult, params};

use tokenserver_settings::Settings;

use crate::db::TokenserverDb;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub(crate) type Conn = Object<AsyncMysqlConnection>;

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
    init_node_url: Option<String>,
    init_node_capacity: i32,
}

impl TokenserverPool {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        _use_test_transactions: bool,
    ) -> DbResult<Self> {
        let manager = AsyncDieselConnectionManager::<AsyncMysqlConnection>::new_with_config(
            &settings.database_url,
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
            database_url: settings.database_url.clone(),
            init_node_url: settings.init_node_url.clone(),
            init_node_capacity: settings.init_node_capacity,
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

    /// Bootstrap the initial Sync 1.5 node record if init_node_url is set.
    async fn init_sync15_node(&mut self, node_url: String, capacity: i32) -> Result<(), DbError> {
        let _ = self
            .get()
            .await?
            .insert_sync15_node(params::Sync15Node {
                node: node_url,
                capacity,
            })
            .await;
        Ok(())
    }
}

#[async_trait(?Send)]
impl DbPool for TokenserverPool {
    async fn init(&mut self) -> Result<(), DbError> {
        if self.run_migrations {
            // Mysql DDL statements implicitly commit which could disrupt
            // MysqlPool's begin_test_transaction during tests. So this runs on
            // its own separate conn
            let conn =
                establish_connection_with_logging::<AsyncMysqlConnection>(&self.database_url)
                    .await?;
            run_embedded_migrations(conn, MIGRATIONS).await?;
        }

        // NOTE: Provided there's a "sync-1.5" service record in the database, it is highly
        // unlikely for this query to fail outside of network failures or other random errors
        let _ = self.init_service_id().await;

        // Init the Sync 1.5 node record if init_node_url is set
        if let Some(node_url) = self.init_node_url.clone() {
            let _ = self
                .init_sync15_node(node_url, self.init_node_capacity)
                .await;
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

impl GetPoolState for TokenserverPool {
    fn state(&self) -> PoolState {
        self.inner.status().into()
    }
}
