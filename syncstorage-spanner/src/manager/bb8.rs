use std::marker::PhantomData;
use std::{fmt, sync::Arc};

use async_trait::async_trait;
use bb8::{ManageConnection, PooledConnection};
use grpcio::{EnvBuilder, Environment};

use crate::{
    db::{
        error::{DbError, DbErrorKind},
        PoolState,
    },
    server::Metrics,
    settings::Settings,
};

use super::session::{create_spanner_session, recycle_spanner_session, SpannerSession};

#[allow(dead_code)]
pub type Conn<'a> = PooledConnection<'a, SpannerSessionManager<SpannerSession>>;

pub struct SpannerSessionManager {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    test_transactions: bool,
    phantom: PhantomData<T>,
}

impl fmt::Debug for SpannerSessionManager {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("bb8::SpannerSessionManager")
            .field("database_name", &self.database_name)
            .field("test_transactions", &self.test_transactions)
            .finish()
    }
}

#[async_trait]
impl ManageConnection for SpannerSessionManager {
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
