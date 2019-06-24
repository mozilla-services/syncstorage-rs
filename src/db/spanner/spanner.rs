
extern crate google_spanner1 as spanner1;
extern crate hyper;
extern crate hyper_rustls;
extern crate yup_oauth2 as oauth2;
use diesel::r2d2;
use oauth2::ServiceAccountAccess;
use yup_oauth2::service_account_key_from_file;
use spanner1::Error;
use spanner1::Spanner;


const DATABASE_INSTANCE: &'static str = "projects/lustrous-center-242019/instances/testing1";

pub struct SpannerConnectionManager;

impl r2d2::ManageConnection for SpannerConnectionManager {
    type Connection = Spanner<hyper::Client, ServiceAccountAccess<hyper::Client>>;
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
        Ok(Spanner::new(client2, access))
    }

    fn is_valid(&self, _conn: &mut Self::Connection) -> std::result::Result<(), Error> {
        Ok(())
    }

    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}
