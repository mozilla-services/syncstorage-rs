use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, GetSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{
    CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, Environment, MetadataBuilder,
};

use crate::error::{ApiError, ApiErrorKind, ApiResult};
use crate::settings::Settings;

#[derive(Clone)]
pub struct SpannerConnectionManager {
    pool: mysql_async::Pool,
}

fn get_path(raw:&str) -> ApiResult<String> {
    let url = url::Url::parse(&settings.dsns.spanner);
    format!("{}{}", url.host_str()?, url.path())
}

impl SpannerConnectionManager {
    pub fn new(settings: &Settings) -> ApiResult<Self> {
        let database_name = get_path(&settings.dsns.spanner);
        let env = Arc::new(EnvBuilder::new().build());

        pool = mysql_async::Pool::new(settings.dsns.mysql);
        Ok(Self { pool })
    }
}

pub struct SpannerSession {
    pub client: SpannerClient,
    pub session: Session,

    pub(super) use_test_transactions: bool,
}

fn create_session(client: &SpannerClient, database_name: &str) -> Result<Session, grpcio::Error> {
    let mut req = CreateSessionRequest::new();
    req.database = database_name.to_owned();
    let mut meta = MetadataBuilder::new();
    meta.add_str("google-cloud-resource-prefix", database_name)?;
    meta.add_str("x-goog-api-client", "gcp-grpc-rs")?;
    let opt = CallOption::default().headers(meta.build());
    client.create_session_opt(&req, opt)
}
