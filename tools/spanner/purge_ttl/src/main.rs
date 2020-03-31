use std::env;
use std::error::Error;
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Instant;

use cadence::{
    BufferedUdpMetricSink, Metric, QueuingMetricSink, StatsdClient, Timed, DEFAULT_PORT,
};
use env_logger;
use googleapis_raw::spanner::v1::{
    spanner::{BeginTransactionRequest, CreateSessionRequest, ExecuteSqlRequest, Session},
    spanner_grpc::SpannerClient,
    transaction::{TransactionOptions, TransactionOptions_PartitionedDml, TransactionSelector},
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, EnvBuilder, MetadataBuilder};
use log::{info, trace, warn};
use url::{Host, Url};

const SPANNER_ADDRESS: &str = "spanner.googleapis.com:443";

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
                info!("⌚ {:?}", v.as_metric_str());
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

fn prepare_request(
    client: &SpannerClient,
    session: &Session,
) -> Result<ExecuteSqlRequest, Box<dyn Error>> {
    // Create a transaction
    let mut opt = TransactionOptions::new();
    opt.set_partitioned_dml(TransactionOptions_PartitionedDml::new());
    let mut req = BeginTransactionRequest::new();
    req.set_session(session.get_name().to_owned());
    req.set_options(opt);
    let mut txn = client.begin_transaction(&req)?;

    let mut ts = TransactionSelector::new();
    ts.set_id(txn.take_id());

    // Create an SQL request
    let mut req = ExecuteSqlRequest::new();
    req.set_session(session.get_name().to_string());
    req.set_transaction(ts);

    Ok(req)
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::try_init()?;

    const DB_ENV: &str = "SYNC_DATABASE_URL";
    let db_url = env::var(DB_ENV).map_err(|_| format!("Invalid or undefined {}", DB_ENV))?;
    let url = Url::parse(&db_url).map_err(|e| format!("Invalid {}: {}", DB_ENV, e))?;
    if url.scheme() != "spanner" || url.host() != Some(Host::Domain("projects")) {
        return Err(format!("Invalid {}", DB_ENV).into());
    }

    let database = db_url["spanner://".len()..].to_owned();
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
            let mut req = prepare_request(&client, &session)?;
            req.set_sql("DELETE FROM batches WHERE expiry < CURRENT_TIMESTAMP()".to_string());
            let result = client.execute_sql(&req)?;
            info!(
                "batches: removed {} rows",
                result.get_stats().get_row_count_lower_bound()
            )
        }
        {
            let _timer_bso = start_timer(&statsd, "purge_ttl.bso_duration");
            let mut req = prepare_request(&client, &session)?;
            req.set_sql("DELETE FROM bsos WHERE expiry < CURRENT_TIMESTAMP()".to_string());
            let result = client.execute_sql(&req)?;
            info!(
                "bso: removed {} rows",
                result.get_stats().get_row_count_lower_bound()
            )
        }
        info!("Completed purge_ttl")
    }

    Ok(())
}
