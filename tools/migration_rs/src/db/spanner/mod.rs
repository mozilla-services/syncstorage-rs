use std::sync::Arc;
use std::ops::Deref;
use std::str::FromStr;

use googleapis_raw::spanner::v1::{
    result_set::ResultSet,
    spanner::{CreateSessionRequest, ExecuteSqlRequest, GetSessionRequest, Session, BeginTransactionRequest},
    transaction::{TransactionOptions, TransactionSelector},
    spanner_grpc::SpannerClient,
};
use grpcio::{
    CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, Environment, MetadataBuilder,
};


use crate::error::{ApiError, ApiErrorKind, ApiResult};
use crate::settings::Settings;
use crate::db::UserData;
use crate::db::collections::Collections;

const MAX_MESSAGE_LEN: i32 = 104_857_600;

#[derive(Clone)]
pub struct Spanner {
    pub client: SpannerClient,
}

fn get_path(raw: &str) -> ApiResult<String> {
    let url = match url::Url::parse(raw){
        Ok(v) => v,
        Err(e) => {
            return Err(ApiErrorKind::Internal(format!("Invalid Spanner DSN {}", e)).into())
        }
    };
    Ok(format!("{}{}", url.host_str().unwrap(), url.path()))
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

impl Spanner {
    pub fn new(settings: &Settings) -> ApiResult<Self> {
        if settings.dsns.spanner.is_none() ||
            settings.dsns.mysql.is_none() {
                return Err(ApiErrorKind::Internal("No DSNs set".to_owned()).into())
            }
        let spanner_path = &settings.dsns.spanner.clone().unwrap();
        let database_name = get_path(&spanner_path).unwrap();
        let env = Arc::new(EnvBuilder::new().build());
        let creds = ChannelCredentials::google_default_credentials().unwrap();
        let chan = ChannelBuilder::new(env.clone())
            .max_send_message_len(MAX_MESSAGE_LEN)
            .max_receive_message_len(MAX_MESSAGE_LEN)
            .secure_connect(&spanner_path, creds);
        let client = SpannerClient::new(chan);

        Ok(Self {client})
    }

    pub async fn transaction(mut self, sql: &str) -> ApiResult<ResultSet> {
        let mut opts = TransactionOptions::new();
        let mut req = BeginTransactionRequest::new();
        let sreq = CreateSessionRequest::new();
        let mut meta = MetadataBuilder::new();
        let sopt = CallOption::default().headers(meta.build());
        let session = self.client.create_session_opt(&sreq, sopt).unwrap();
        req.set_session(session.name.clone());
        req.set_options(opts);

        let mut txn = self.client.begin_transaction(&req).unwrap();

        let mut txns = TransactionSelector::new();
        txns.set_id(txn.take_id());

        let mut sreq = ExecuteSqlRequest::new();
        sreq.set_session(session.name.clone());
        sreq.set_transaction(txns);

        sreq.set_sql(sql.to_owned());
        match self.client.execute_sql(&sreq) {
            Ok(v) => Ok(v),
            Err(e) => {
                Err(ApiErrorKind::Internal(format!("spanner transaction failed: {}", e)).into())
            }
        }
    }

    pub async fn collections(&mut self) -> ApiResult<Collections> {
        let result = self.clone().transaction(
            "SELECT
                DISTINCT uc.collection, cc.name
            FROM
                user_collections as uc,
                collections as cc
            WHERE
                uc.collection = cc.collectionid
            ORDER BY
                uc.collection"
        ).await?;
        let mut collections = Collections::default();
        for row in result.get_rows() {
            let id: u8 = u8::from_str(row.values[0].get_string_value()).unwrap();
            let name:&str = row.values[1].get_string_value();
            if collections.get(name).is_none(){
                collections.set(name, id);
            }
        }
        Ok(collections)

    }

    pub async fn update_user(&self, user: UserData) -> ApiResult<u64> {
        Err(ApiErrorKind::Internal(format!("TODO: Incomplete")).into())
    }
}
