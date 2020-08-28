use std::marker::PhantomData;
use std::{fmt, sync::Arc};

use async_trait::async_trait;
use bb8::{ManageConnection, PooledConnection};
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

#[allow(dead_code)]
pub type Conn<'a> = PooledConnection<'a, SpannerSessionManager<SpannerSession>>;

pub struct SpannerSessionManager<T> {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    test_transactions: bool,
    phantom: PhantomData<T>,
}

impl<_T> fmt::Debug for SpannerSessionManager<_T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("bb8::SpannerSessionManager")
            .field("database_name", &self.database_name)
            .field("test_transactions", &self.test_transactions)
            .finish()
    }
}

impl<T> SpannerSessionManager<T> {
    #[allow(dead_code)]
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

        Ok(SpannerSessionManager::<T> {
            database_name,
            env,
            metrics: metrics.clone(),
            test_transactions,
            phantom: PhantomData,
        })
    }
}

#[async_trait]
impl<T: Send + Sync + 'static> ManageConnection for SpannerSessionManager<T> {
    type Connection = SpannerSession;
    type Error = DbError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        create_spanner_session(
            Arc::clone(&self.env),
            self.metrics.clone(),
            &self.database_name,
            self.test_transactions,
        )
        .await
    }

    async fn is_valid(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        recycle_spanner_session(&mut conn, &self.database_name).await?;
        Ok(conn)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

impl From<bb8::State> for PoolState {
    fn from(state: bb8::State) -> PoolState {
        PoolState {
            connections: state.connections,
            idle_connections: state.idle_connections,
        }
    }
}
