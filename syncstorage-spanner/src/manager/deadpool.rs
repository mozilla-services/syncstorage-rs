use std::{fmt, sync::Arc};

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleError, RecycleResult};
use grpcio::{EnvBuilder, Environment};
use syncserver_common::Metrics;
use syncstorage_settings::Settings;

use super::session::{create_spanner_session, recycle_spanner_session, SpannerSession};
use crate::error::DbError;

pub type Conn = deadpool::managed::Object<SpannerSession, DbError>;

pub struct SpannerSessionManager {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    test_transactions: bool,
    max_lifespan: Option<u32>,
    max_idle: Option<u32>,
    emulator_host: Option<String>,
}

impl fmt::Debug for SpannerSessionManager {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("deadpool::SpannerSessionManager")
            .field("database_name", &self.database_name)
            .field("test_transactions", &self.test_transactions)
            .finish()
    }
}

impl SpannerSessionManager {
    pub fn new(settings: &Settings, metrics: &Metrics) -> Result<Self, DbError> {
        let database_name = settings
            .spanner_database_name()
            .ok_or_else(|| {
                DbError::internal(format!("invalid database url: {}", settings.database_url))
            })?
            .to_owned();
        let env = Arc::new(EnvBuilder::new().build());

        #[cfg(not(test))]
        let test_transactions = false;
        #[cfg(test)]
        let test_transactions = settings.database_use_test_transactions;

        Ok(Self {
            database_name,
            env,
            metrics: metrics.clone(),
            test_transactions,
            max_lifespan: settings.database_pool_connection_lifespan,
            max_idle: settings.database_pool_connection_max_idle,
            emulator_host: settings.spanner_emulator_host.clone(),
        })
    }
}

#[async_trait]
impl Manager<SpannerSession, DbError> for SpannerSessionManager {
    async fn create(&self) -> Result<SpannerSession, DbError> {
        let session = create_spanner_session(
            Arc::clone(&self.env),
            self.metrics.clone(),
            &self.database_name,
            self.test_transactions,
            self.emulator_host.clone(),
        )
        .await?;
        Ok(session)
    }

    async fn recycle(&self, conn: &mut SpannerSession) -> RecycleResult<DbError> {
        recycle_spanner_session(
            conn,
            &self.database_name,
            &self.metrics,
            self.max_lifespan,
            self.max_idle,
        )
        .await
        .map_err(RecycleError::Backend)
    }
}
