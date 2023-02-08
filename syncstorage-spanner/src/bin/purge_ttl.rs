use std::env;
use std::error::Error;
use std::net::UdpSocket;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cadence::{
    BufferedUdpMetricSink, Metric, QueuingMetricSink, StatsdClient, Timed, DEFAULT_PORT,
};
use google_cloud_rust_raw::spanner::v1::{
    spanner::{
        BeginTransactionRequest, CommitRequest, CreateSessionRequest, ExecuteSqlRequest, Session,
    },
    spanner_grpc::SpannerClient,
    transaction::{
        TransactionOptions, TransactionOptions_PartitionedDml, TransactionOptions_ReadOnly,
        TransactionOptions_ReadWrite, TransactionSelector,
    },
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};
use log::{info, trace, warn};
use url::{Host, Url};

const SPANNER_ADDRESS: &str = "spanner.googleapis.com:443";
const RETRY_ENV_VAR: &str = "PURGE_TTL_RETRY_COUNT"; // Default value = 10
const SLEEP_ENV_VAR: &str = "PURGE_TTL_RETRY_SLEEP_MILLIS"; // Default value = 0

use protobuf::well_known_types::Value;

pub struct MetricTimer {
    pub client: StatsdClient,
    pub label: String,
    pub start: Instant,
}

impl Drop for MetricTimer {
    fn drop(&mut self) {
        let lapse = (Instant::now() - self.start).as_millis() as u64;
        match self.client.time(&self.label, lapse) {
            Err(e) => {
                warn!("⚠️ Metric {} error: {:?}", self.label, e);
            }
            Ok(v) => {
                trace!("⌚ {:?}", v.as_metric_str());
            }
        }
    }
}

pub fn start_timer(client: &StatsdClient, label: &str) -> MetricTimer {
    trace!("⌚ Starting timer... {:?}", label);
    MetricTimer {
        start: Instant::now(),
        label: label.to_owned(),
        client: client.clone(),
    }
}

pub fn statsd_from_env() -> Result<StatsdClient, Box<dyn Error>> {
    let statsd_host = env::var("STATSD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let statsd_port = match env::var("STATSD_PORT") {
        Ok(port) => port.parse::<u16>()?,
        Err(_) => DEFAULT_PORT,
    };

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.set_nonblocking(true)?;
    let host = (statsd_host.as_str(), statsd_port);
    let udp_sink = BufferedUdpMetricSink::from(host, socket)?;
    let sink = QueuingMetricSink::from(udp_sink);
    let builder = StatsdClient::builder("syncstorage", sink);

    Ok(builder
        .with_error_handler(|err| {
            warn!("Metric send error: {:?}", err);
        })
        .build())
}

pub enum RequestType {
    ReadOnly,
    ReadWrite,
    PartitionedDml,
}

fn begin_transaction(
    client: &SpannerClient,
    session: &Session,
    request_type: RequestType,
) -> Result<(ExecuteSqlRequest, Vec<u8>), Box<grpcio::Error>> {
    // Create a transaction
    let mut opt = TransactionOptions::new();
    match request_type {
        RequestType::ReadWrite => {
            opt.set_read_write(TransactionOptions_ReadWrite::new());
        }
        RequestType::ReadOnly => {
            opt.set_read_only(TransactionOptions_ReadOnly::new());
        }
        RequestType::PartitionedDml => {
            opt.set_partitioned_dml(TransactionOptions_PartitionedDml::new());
        }
    }

    let mut req = BeginTransactionRequest::new();
    req.set_session(session.get_name().to_owned());
    req.set_options(opt);
    let mut txn = client.begin_transaction(&req)?;

    let id = txn.take_id();
    let mut ts = TransactionSelector::new();
    ts.set_id(id.clone());

    let mut req = ExecuteSqlRequest::new();
    req.set_session(session.get_name().to_string());
    req.set_transaction(ts);

    Ok((req, id))
}

fn continue_transaction(session: &Session, transaction_id: Vec<u8>) -> ExecuteSqlRequest {
    let mut ts = TransactionSelector::new();
    ts.set_id(transaction_id);
    let mut req = ExecuteSqlRequest::new();
    req.set_session(session.get_name().to_string());
    req.set_transaction(ts);
    req
}

fn commit_transaction(
    client: &SpannerClient,
    session: &Session,
    txn: Vec<u8>,
) -> Result<(), Box<grpcio::Error>> {
    let mut req = CommitRequest::new();
    req.set_session(session.get_name().to_owned());
    req.set_transaction_id(txn);
    client.commit(&req)?;
    Ok(())
}

pub struct SyncResultSet {
    result: google_cloud_rust_raw::spanner::v1::result_set::ResultSet,
}

impl Iterator for SyncResultSet {
    type Item = Vec<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let rows = &mut self.result.rows;
        if rows.is_empty() {
            None
        } else {
            let row = rows.remove(0);
            Some(row.get_values().to_vec())
        }
    }
}

fn delete_incremental(
    client: &SpannerClient,
    session: &Session,
    table: String,
    column: String,
    chunk_size: u64,
    max_to_delete: u64,
) -> Result<(), Box<grpcio::Error>> {
    let mut total: u64 = 0;
    let (mut req, mut txn) = begin_transaction(client, session, RequestType::ReadWrite)?;
    loop {
        let select_sql = format!("SELECT fxa_uid, fxa_kid, collection_id, {} FROM {} WHERE expiry < CURRENT_TIMESTAMP() LIMIT {}", column, table, chunk_size);
        trace!(
            "Delete: Selecting incremental rows to delete: {}",
            select_sql
        );
        req.set_sql(select_sql.clone());
        let mut result = SyncResultSet {
            result: client.execute_sql(&req)?,
        };

        if result.result.rows.is_empty() || total >= max_to_delete {
            trace!("Delete: {}: done", table);
            break;
        }
        let mut delete_sql = format!(
            "DELETE FROM {} WHERE (fxa_uid, fxa_kid, collection_id, {}) IN (",
            table, column,
        );
        for row in &mut result {
            // Count starting at 1 so that i % chunk_size is false when on the first row
            let fxa_uid = row[0].get_string_value().to_owned();
            let fxa_kid = row[1].get_string_value().to_owned();
            let collection_id = row[2].get_string_value().parse::<i32>().unwrap();
            let id = row[3].get_string_value().to_owned();
            trace!(
                "Delete: Selected row for delete: i={} fxa_uid={} fxa_kid={} collection_id={} {}={}",
                total,
                fxa_uid,
                fxa_kid,
                collection_id,
                column,
                id
            );
            delete_sql = format!(
                "{}('{}', '{}', {}, '{}'), ",
                delete_sql, fxa_uid, fxa_kid, collection_id, id
            );

            total += 1;
        }
        delete_sql = format!("{})", delete_sql.trim_end_matches(&", ".to_string()));
        trace!("Deleting chunk with: {}", delete_sql);
        let mut delete_req = continue_transaction(session, txn.clone());
        delete_req.set_sql(delete_sql);
        client.execute_sql(&delete_req)?;
        info!("{}: removed {} rows", table, total);
        commit_transaction(client, session, txn.clone())?;
        let (newreq, newtxn) = begin_transaction(client, session, RequestType::ReadWrite)?;
        req = newreq;
        txn = newtxn;
    }

    Ok(())
}

fn delete_all(
    client: &SpannerClient,
    session: &Session,
    table: String,
) -> Result<(), Box<grpcio::Error>> {
    let (mut req, _txn) = begin_transaction(client, session, RequestType::PartitionedDml)?;
    req.set_sql(format!(
        "DELETE FROM {} WHERE expiry < CURRENT_TIMESTAMP()",
        table
    ));
    let result = client.execute_sql(&req)?;
    info!(
        "DeleteAll: {}: removed {} rows",
        table,
        result.get_stats().get_row_count_lower_bound()
    );
    Ok(())
}

fn retryable(err: &grpcio::Error) -> bool {
    // if it is NOT an ABORT, we should not retry this function.
    match err {
        grpcio::Error::RpcFailure(ref status)
            if status.code() == grpcio::RpcStatusCode::ABORTED =>
        {
            true
        }
        grpcio::Error::RpcFinished(Some(ref status))
            if status.code() == grpcio::RpcStatusCode::ABORTED =>
        {
            true
        }
        _ => false,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::try_init()?;

    let chunk_size: u64 = env::var("PURGE_TTL_CHUNK_SIZE")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap();
    let max_to_delete: u64 = env::var("PURGE_TTL_MAX_TO_DELETE")
        .unwrap_or_else(|_| "1000".to_string())
        .parse()
        .unwrap();

    const INCREMENTAL_ENV: &str = "PURGE_TTL_INCREMENTAL";
    let incremental = env::var(INCREMENTAL_ENV)
        .map(|x| x == "1" || x.to_lowercase() == "true")
        .unwrap_or(false);
    info!("INCREMENTAL: {:?}", incremental);

    const DB_ENV: &str = "SYNC_SYNCSTORAGE__DATABASE_URL";
    let db_url = env::var(DB_ENV).map_err(|_| format!("Invalid or undefined {}", DB_ENV))?;
    let url = Url::parse(&db_url).map_err(|e| format!("Invalid {}: {}", DB_ENV, e))?;
    if url.scheme() != "spanner" || url.host() != Some(Host::Domain("projects")) {
        return Err(format!("Invalid {}", DB_ENV).into());
    }
    let retries: u64 =
        str::parse::<u64>(&env::var(RETRY_ENV_VAR).unwrap_or_else(|_| "10".to_owned()))
            .unwrap_or(10);
    let nap_time: Duration = Duration::from_millis(
        str::parse::<u64>(&env::var(SLEEP_ENV_VAR).unwrap_or_else(|_| "0".to_owned())).unwrap_or(0),
    );

    let database = db_url["spanner://".len()..].to_owned();
    info!("Retries: {}, sleep: {}ms", retries, nap_time.as_millis());
    info!("For {}", database);

    // Set up the gRPC environment.
    let env = Arc::new(EnvBuilder::new().build());
    let creds = ChannelCredentials::google_default_credentials()?;

    // Create a Spanner client.
    let chan = ChannelBuilder::new(env)
        .max_send_message_len(100 << 20)
        .max_receive_message_len(100 << 20)
        .secure_connect(SPANNER_ADDRESS, creds);
    let client = SpannerClient::new(chan);

    // Create a session
    let mut req = CreateSessionRequest::new();
    req.set_database(database.to_string());
    let mut meta = MetadataBuilder::new();
    meta.add_str("google-cloud-resource-prefix", &database)?;
    meta.add_str("x-goog-api-client", "googleapis-rs")?;
    let opt = CallOption::default().headers(meta.build());
    let session = client.create_session_opt(&req, opt)?;

    let statsd = statsd_from_env()?;

    {
        let _timer_total = start_timer(&statsd, "purge_ttl.total_duration");
        {
            let _timer_batches = start_timer(&statsd, "purge_ttl.batches_duration");
            let mut success = false;
            for i in 0..retries {
                match if incremental {
                    delete_incremental(
                        &client,
                        &session,
                        "batches".to_owned(),
                        "batch_id".to_owned(),
                        chunk_size,
                        max_to_delete,
                    )
                } else {
                    delete_all(&client, &session, "batches".to_owned())
                } {
                    Ok(_) => {
                        success = true;
                        break;
                    }
                    Err(e) => {
                        warn!("Batch transaction error: {}: {:?}", i, e);
                        if !retryable(&e) {
                            return Err(e);
                        }
                        if nap_time.as_millis() > 0 {
                            thread::sleep(nap_time);
                        }
                    }
                }
            }
            if !success {
                panic!(
                    "Could not delete expired batches after {} attempts",
                    retries
                );
            }
        }
        {
            let _timer_bso = start_timer(&statsd, "purge_ttl.bso_duration");
            let mut success = false;
            for i in 0..retries {
                match if incremental {
                    delete_incremental(
                        &client,
                        &session,
                        "bsos".to_owned(),
                        "bso_id".to_owned(),
                        chunk_size,
                        max_to_delete,
                    )
                } else {
                    delete_all(&client, &session, "bsos".to_owned())
                } {
                    Ok(_) => {
                        success = true;
                        break;
                    }
                    Err(e) => {
                        warn!("BSO transaction error {}: {:?}", i, e);
                        if nap_time.as_millis() > 0 {
                            thread::sleep(nap_time);
                        }
                    }
                }
            }
            if !success {
                panic!("Could not delete expired bsos after {} attempts", retries);
            }
        }
        info!("Completed purge_ttl");
    }

    Ok(())
}
