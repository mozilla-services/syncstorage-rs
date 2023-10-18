use std::{fmt, sync::Arc};

use async_trait::async_trait;
use deadpool::managed::{Manager, RecycleError, RecycleResult};
use grpcio::{EnvBuilder, Environment};
use syncserver_common::{BlockingThreadpool, Metrics};
use syncstorage_settings::Settings;

use super::session::{
    create_spanner_session, recycle_spanner_session, SpannerSession, SpannerSessionSettings,
};
use crate::error::DbError;

pub(crate) type Conn = deadpool::managed::Object<SpannerSession, DbError>;

pub(crate) struct SpannerSessionManager {
    settings: SpannerSessionSettings,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    blocking_threadpool: Arc<BlockingThreadpool>,
}

impl fmt::Debug for SpannerSessionManager {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("deadpool::SpannerSessionManager")
            .field("settings", &self.settings)
            .field("blocking_threadpool", &self.blocking_threadpool)
            .finish()
    }
}

impl SpannerSessionManager {
    pub fn new(
        settings: &Settings,
        metrics: &Metrics,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> Result<Self, DbError> {
        Ok(Self {
            settings: SpannerSessionSettings::from_settings(settings)?,
            env: Arc::new(EnvBuilder::new().build()),
            metrics: metrics.clone(),
            blocking_threadpool,
        })
    }
}

#[async_trait]
impl Manager<SpannerSession, DbError> for SpannerSessionManager {
    async fn create(&self) -> Result<SpannerSession, DbError> {
        let session = create_spanner_session(
            &self.settings,
            Arc::clone(&self.env),
            self.metrics.clone(),
            self.blocking_threadpool.clone(),
        )
        .await?;
        Ok(session)
    }

    async fn recycle(&self, conn: &mut SpannerSession) -> RecycleResult<DbError> {
        recycle_spanner_session(conn, &self.metrics)
            .await
            .map_err(RecycleError::Backend)
    }
}
