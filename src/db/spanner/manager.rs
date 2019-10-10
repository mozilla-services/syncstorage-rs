use diesel::r2d2::ManageConnection;

use crate::{
    db::error::{DbError, DbErrorKind},
    settings::Settings,
};

use googleapis_raw::spanner::v1::spanner_grpc::SpannerClient;

use googleapis_raw::spanner::v1::spanner::{CreateSessionRequest, ExecuteSqlRequest, Session};

#[derive(Debug)]
pub struct SpannerConnectionManager {
    database_name: String,
}

impl SpannerConnectionManager {
    pub fn new(settings: &Settings) -> Result<Self, DbError> {
        let url = &settings.database_url;
        if !url.starts_with("spanner://") {
            Err(DbErrorKind::InvalidUrl(url.to_owned()))?;
        }
        let database_name = url["spanner://".len()..].to_owned();
        Ok(SpannerConnectionManager { database_name })
    }
}

pub struct SpannerSession {
    pub client: SpannerClient,
    pub session: Session,

    pub(super) use_test_transactions: bool,
}

impl ManageConnection for SpannerConnectionManager {
    type Connection = SpannerSession;
    type Error = grpcio::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};
        use std::sync::Arc;

        // Google Cloud configuration.
        let endpoint = "spanner.googleapis.com:443";

        // Set up the gRPC environment.
        let env = Arc::new(EnvBuilder::new().build());
        // Requires GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
        let creds = ChannelCredentials::google_default_credentials()?;

        // Create a Spanner client.
        let chan = ChannelBuilder::new(env.clone())
            .max_send_message_len(100 << 20)
            .max_receive_message_len(100 << 20)
            .secure_connect(&endpoint, creds);
        let client = SpannerClient::new(chan);

        // Connect to the instance and create a Spanner session.
        let mut req = CreateSessionRequest::new();
        req.database = self.database_name.clone();
        let mut meta = MetadataBuilder::new();
        meta.add_str("google-cloud-resource-prefix", &self.database_name)?;
        meta.add_str("x-goog-api-client", "gcp-grpc-rs")?;
        let opt = CallOption::default().headers(meta.build());
        let session = client.create_session_opt(&req, opt)?;

        Ok(SpannerSession {
            client,
            session,
            use_test_transactions: false,
        })
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let mut req = ExecuteSqlRequest::new();
        req.set_sql("SELECT 1".to_owned());
        req.set_session(conn.session.get_name().to_owned());
        conn.client.execute_sql(&req)?;
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
