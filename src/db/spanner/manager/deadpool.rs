use std::{fmt, sync::Arc};

use actix_web::web::block;
use async_trait::async_trait;
use deadpool::managed::{RecycleError, RecycleResult};
use googleapis_raw::spanner::v1::{spanner::GetSessionRequest, spanner_grpc::SpannerClient};
use grpcio::{ChannelBuilder, ChannelCredentials, EnvBuilder, Environment};

use crate::{
    db::error::{DbError, DbErrorKind},
    server::metrics::Metrics,
    settings::Settings,
};

use super::bb8::{create_session, SpannerSession, SPANNER_ADDRESS};

pub struct Manager {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    metrics: Metrics,
    test_transactions: bool,
}

impl fmt::Debug for Manager {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Manager")
            .field("database_name", &self.database_name)
            .field("test_transactions", &self.test_transactions)
            .finish()
    }
}

impl Manager {
    pub fn new(settings: &Settings, metrics: &Metrics) -> Result<Self, DbError> {
        let url = &settings.database_url;
        if !url.starts_with("spanner://") {
            Err(DbErrorKind::InvalidUrl(url.to_owned()))?;
        }
        let database_name = url["spanner://".len()..].to_owned();
        let env = Arc::new(EnvBuilder::new().build());

        #[cfg(not(test))]
        let test_transactions = false;
        #[cfg(test)]
        let test_transactions = settings.database_use_test_transactions;

        Ok(Manager {
            database_name,
            env,
            metrics: metrics.clone(),
            test_transactions,
        })
    }
}

#[async_trait]
impl deadpool::managed::Manager<SpannerSession, DbError> for Manager {
    async fn create(&self) -> Result<SpannerSession, DbError> {
        let env = self.env.clone();
        let mut metrics = self.metrics.clone();
        // XXX: issue732: Could google_default_credentials (or
        // ChannelBuilder::secure_connect) block?!
        let chan = block(move || -> Result<grpcio::Channel, grpcio::Error> {
            metrics.start_timer("storage.pool.grpc_auth", None);
            // Requires
            // GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
            let creds = ChannelCredentials::google_default_credentials()?;
            Ok(ChannelBuilder::new(env)
                .max_send_message_len(100 << 20)
                .max_receive_message_len(100 << 20)
                .secure_connect(SPANNER_ADDRESS, creds))
        })
        .await
        .map_err(|e| match e {
            actix_web::error::BlockingError::Error(e) => e.into(),
            actix_web::error::BlockingError::Canceled => {
                DbError::internal("web::block Manager operation canceled")
            }
        })?;
        let client = SpannerClient::new(chan);

        // Connect to the instance and create a Spanner session.
        let session = create_session(&client, &self.database_name).await?;

        Ok(SpannerSession {
            client,
            session,
            use_test_transactions: self.test_transactions,
        })
    }

    async fn recycle(&self, conn: &mut SpannerSession) -> RecycleResult<DbError> {
        let mut req = GetSessionRequest::new();
        req.set_name(conn.session.get_name().to_owned());
        if let Err(e) = conn
            .client
            .get_session_async(&req)
            .map_err(|e| RecycleError::Backend(e.into()))?
            .await
        {
            match e {
                grpcio::Error::RpcFailure(ref status)
                    if status.status == grpcio::RpcStatusCode::NOT_FOUND =>
                {
                    conn.session = create_session(&conn.client, &self.database_name)
                        .await
                        .map_err(|e| RecycleError::Backend(e.into()))?;
                }
                _ => return Err(RecycleError::Backend(e.into())),
            }
        }
        Ok(())
    }
}
