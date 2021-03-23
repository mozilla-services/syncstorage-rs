use std::{fmt, sync::Arc};

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleError, RecycleResult};
use grpcio::{EnvBuilder, Environment};

use crate::{
    db::{
        error::{DbError, DbErrorKind},
        results::PoolState,
    },
    server::metrics::Metrics,
    settings::Settings,
};

use super::session::{create_spanner_session, recycle_spanner_session, SpannerSession};

pub type Conn = deadpool::managed::Object<SpannerSession, DbError>;

pub struct SpannerSessionManager {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    test_transactions: bool,
    max_lifespan: Option<u32>,
    max_idle: Option<u32>,
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
            .ok_or_else(|| DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?
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

impl From<deadpool::Status> for PoolState {
    fn from(status: deadpool::Status) -> PoolState {
        PoolState {
            connections: status.size as u32,
            idle_connections: status.available.max(0) as u32,
        }
    }
}
