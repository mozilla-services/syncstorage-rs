use diesel::r2d2::ManageConnection;
use google_spanner1::{CreateSessionRequest, Error, Session, Spanner};
use hyper::{net::HttpsConnector, Client};
use hyper_rustls::TlsClient;
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
    pub hub: Spanner<Client, ServiceAccountAccess<Client>>,
    pub session: Session,
    pub(super) use_test_transactions: bool,
}

impl ManageConnection for SpannerConnectionManager {
    type Connection = SpannerSession;
    type Error = Error;

    fn connect(&self) -> Result<Self::Connection, Error> {
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

    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Error> {
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
