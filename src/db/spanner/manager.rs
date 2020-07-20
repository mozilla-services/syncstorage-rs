use std::marker::PhantomData;
use std::{fmt, sync::Arc};

use async_trait::async_trait;
use bb8::ManageConnection;
use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, GetSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{
    CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, Environment, MetadataBuilder,
};

use crate::{
    db::error::{DbError, DbErrorKind},
    settings::Settings,
};

const SPANNER_ADDRESS: &str = "spanner.googleapis.com:443";

pub struct SpannerConnectionManager<T> {
    database_name: String,
    /// The gRPC environment
    env: Arc<Environment>,
    test_transactions: bool,
    phantom: PhantomData<T>,
}

impl<_T> fmt::Debug for SpannerConnectionManager<_T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("SpannerConnectionManager")
            .field("database_name", &self.database_name)
            .finish()
    }
}

impl<T> SpannerConnectionManager<T> {
    pub fn new(settings: &Settings) -> Result<Self, DbError> {
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

        Ok(SpannerConnectionManager::<T> {
            database_name,
            env,
            test_transactions,
            phantom: PhantomData,
        })
    }
}

pub struct SpannerSession {
    pub client: SpannerClient,
    pub session: Session,

    pub(super) use_test_transactions: bool,
}

#[async_trait]
impl<T: std::marker::Send + std::marker::Sync + 'static> ManageConnection
    for SpannerConnectionManager<T>
{
    type Connection = SpannerSession;
    type Error = grpcio::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let chan = {
            // Requires
            // GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
            // XXX: issue732: Could google_default_credentials (or
            // ChannelBuilder::secure_connect) block?!
            let creds = ChannelCredentials::google_default_credentials()?;

            // Create a Spanner client.
            ChannelBuilder::new(self.env.clone())
                .max_send_message_len(100 << 20)
                .max_receive_message_len(100 << 20)
                .secure_connect(SPANNER_ADDRESS, creds)
        };
        let client = SpannerClient::new(chan);

        // Connect to the instance and create a Spanner session.
        let session = create_session(&client, &self.database_name).await?;

        Ok(SpannerSession {
            client,
            session,
            use_test_transactions: self.test_transactions,
        })
    }

    async fn is_valid(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        let mut req = GetSessionRequest::new();
        req.set_name(conn.session.get_name().to_owned());
        if let Err(e) = conn.client.get_session_async(&req)?.await {
            match e {
                grpcio::Error::RpcFailure(ref status)
                    if status.status == grpcio::RpcStatusCode::NOT_FOUND =>
                {
                    conn.session = create_session(&conn.client, &self.database_name).await?;
                }
                _ => return Err(e),
            }
        }
        Ok(conn)
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

async fn create_session(
    client: &SpannerClient,
    database_name: &str,
) -> Result<Session, grpcio::Error> {
    let mut req = CreateSessionRequest::new();
    req.database = database_name.to_owned();
    let mut meta = MetadataBuilder::new();
    meta.add_str("google-cloud-resource-prefix", database_name)?;
    meta.add_str("x-goog-api-client", "gcp-grpc-rs")?;
    let opt = CallOption::default().headers(meta.build());
    client.create_session_async_opt(&req, opt)?.await
}
