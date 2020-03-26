//! Spanner client.

use std::sync::Arc;

use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};

pub struct Client {
    database: String,
    client: SpannerClient,
    session: Session,
}

#[allow(dead_code)]
impl Client {
    /// Creates a new Spanner client.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use googleapis::spanner;
    ///
    /// let db = "projects/my_project/instances/my_instance/databases/my_database";
    /// let client = spanner::Client::new(db);
    /// ```
    pub fn new(database: &str) -> crate::Result<Client> {
        let database = database.to_string();
        let endpoint = "spanner.googleapis.com:443";

        // Set up the gRPC environment.
        let env = Arc::new(EnvBuilder::new().build());
        let creds = ChannelCredentials::google_default_credentials()?;

        // Create a Spanner client.
        let chan = ChannelBuilder::new(env.clone())
            .max_send_message_len(100 << 20)
            .max_receive_message_len(100 << 20)
            .secure_connect(&endpoint, creds);
        let client = SpannerClient::new(chan);

        let mut req = CreateSessionRequest::new();
        req.set_database(database.to_string());
        let mut meta = MetadataBuilder::new();
        meta.add_str("google-cloud-resource-prefix", &database)?;
        meta.add_str("x-goog-api-client", "googleapis-rs")?;
        let opt = CallOption::default().headers(meta.build());
        let session = client.create_session_opt(&req, opt)?;

        Ok(Client {
            database,
            client,
            session,
        })
    }
}
