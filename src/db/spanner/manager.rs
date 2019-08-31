use diesel::r2d2::ManageConnection;
#[cfg(not(feature = "google_grpc"))]
use google_spanner1::{CreateSessionRequest, Session, Spanner};
#[cfg(not(feature = "google_grpc"))]
use hyper::{net::HttpsConnector, Client};
#[cfg(not(feature = "google_grpc"))]
use hyper_rustls::TlsClient;
#[cfg(not(feature = "google_grpc"))]
use yup_oauth2::{service_account_key_from_file, GetToken, ServiceAccountAccess};

use crate::{
    db::error::{DbError, DbErrorKind},
    settings::Settings,
};

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
    #[cfg(feature = "google_grpc")]
    pub client: googleapis_raw::spanner::v1::spanner_grpc::SpannerClient,
    #[cfg(feature = "google_grpc")]
    pub session: googleapis_raw::spanner::v1::spanner::Session,

    #[cfg(not(feature = "google_grpc"))]
    pub hub: Spanner<Client, ServiceAccountAccess<Client>>,
    #[cfg(not(feature = "google_grpc"))]
    pub session: Session,

    pub(super) use_test_transactions: bool,
}

#[cfg(feature = "google_grpc")]
impl ManageConnection for SpannerConnectionManager {
    type Connection = SpannerSession;
    type Error = grpcio::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        use googleapis_raw::spanner::v1::{
            spanner::CreateSessionRequest, spanner_grpc::SpannerClient,
        };
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
        meta.add_str("x-goog-api-client", "googleapis-rs")?;
        let opt = CallOption::default().headers(meta.build());
        let session = client.create_session_opt(&req, opt)?;

        Ok(SpannerSession {
            client,
            session,
            use_test_transactions: false,
        })
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        let mut req = googleapis_raw::spanner::v1::spanner::ExecuteSqlRequest::new();
        req.set_sql("SELECT 1".to_owned());
        req.set_session(conn.session.get_name().to_owned());
        conn.client.execute_sql(&req)?;
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

#[cfg(not(feature = "google_grpc"))]
impl ManageConnection for SpannerConnectionManager {
    type Connection = SpannerSession;
    type Error = google_spanner1::Error;

    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let secret = service_account_key_from_file(&String::from("service-account.json")).unwrap();
        let client = Client::with_connector(HttpsConnector::new(TlsClient::new()));
        let mut access = ServiceAccountAccess::new(secret, client);
        let _token = access
            .token(&vec!["https://www.googleapis.com/auth/spanner.data"])
            .unwrap();
        // println!("{:?}", token);
        let client2 = Client::with_connector(HttpsConnector::new(TlsClient::new()));
        let hub = Spanner::new(client2, access);
        let req = CreateSessionRequest::default();
        let session = hub
            .projects()
            .instances_databases_sessions_create(req, &self.database_name)
            .doit()?
            .1;
        Ok(SpannerSession {
            hub,
            session,
            use_test_transactions: false,
        })
    }

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        use google_spanner1::ExecuteSqlRequest;
        let mut request = ExecuteSqlRequest::default();
        request.sql = Some("SELECT 1".to_owned());
        let session = conn.session.name.as_ref().unwrap();
        conn.hub
            .projects()
            .instances_databases_sessions_execute_sql(request, session)
            .doit()?;
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
