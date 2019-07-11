extern crate google_spanner1 as spanner1;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
use diesel::r2d2;
use oauth2::ServiceAccountAccess;
use spanner1::CreateSessionRequest;
use spanner1::Error;
use spanner1::Session;
use spanner1::Spanner;
use yup_oauth2::service_account_key_from_file;

use crate::{
    db::error::{DbError, DbErrorKind},
    settings::Settings,
};

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
    pub hub: Spanner<hyper::Client, ServiceAccountAccess<hyper::Client>>,
    pub session: Session,
}

impl r2d2::ManageConnection for SpannerConnectionManager {
    type Connection = SpannerSession;
    type Error = Error;

    fn connect(&self) -> std::result::Result<Self::Connection, Error> {
        let secret = service_account_key_from_file(&String::from("service-account.json")).unwrap();
        let client = hyper::Client::with_connector(hyper::net::HttpsConnector::new(
            hyper_rustls::TlsClient::new(),
        ));
        let mut access = ServiceAccountAccess::new(secret, client);
        use yup_oauth2::GetToken;
        let _token = access
            .token(&vec!["https://www.googleapis.com/auth/spanner.data"])
            .unwrap();
        // println!("{:?}", token);
        let client2 = hyper::Client::with_connector(hyper::net::HttpsConnector::new(
            hyper_rustls::TlsClient::new(),
        ));
        let hub = Spanner::new(client2, access);
        let req = CreateSessionRequest::default();
        let session = hub
            .projects()
            .instances_databases_sessions_create(req, &self.database_name)
            .doit()
            .unwrap()
            .1;
        Ok(SpannerSession { hub, session })
    }

    fn is_valid(&self, _conn: &mut Self::Connection) -> std::result::Result<(), Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
