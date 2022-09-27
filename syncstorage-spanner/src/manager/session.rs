use std::sync::Arc;

use google_cloud_rust_raw::spanner::v1::{
    spanner::{CreateSessionRequest, GetSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, Environment, MetadataBuilder};
use syncserver_common::Metrics;

use crate::error::DbError;

const SPANNER_ADDRESS: &str = "spanner.googleapis.com:443";

/// Represents a communication channel w/ Spanner
///
/// Session creation is expensive in Spanner so sessions should be long-lived
/// and cached for reuse.
pub struct SpannerSession {
    /// This is a reference copy of the Session info.
    /// It is used for meta info, not for actual connections.
    pub session: Session,
    /// The underlying client (Connection/Channel) for interacting with spanner
    pub client: SpannerClient,
    pub(crate) use_test_transactions: bool,
    /// A second based UTC for SpannerSession creation.
    /// Session has a similar `create_time` value that is managed by protobuf,
    /// but some clock skew issues are possible.
    pub(crate) create_time: i64,
    /// Whether we are using the Spanner emulator
    pub using_spanner_emulator: bool,
}

/// Create a Session (and the underlying gRPC Channel)
pub async fn create_spanner_session(
    env: Arc<Environment>,
    mut metrics: Metrics,
    database_name: &str,
    use_test_transactions: bool,
    emulator_host: Option<String>,
) -> Result<SpannerSession, DbError> {
    let using_spanner_emulator = emulator_host.is_some();
    let chan = syncserver_db_common::run_on_blocking_threadpool(
        move || -> Result<grpcio::Channel, DbError> {
            if let Some(spanner_emulator_address) = emulator_host {
                Ok(ChannelBuilder::new(env)
                    .max_send_message_len(100 << 20)
                    .max_receive_message_len(100 << 20)
                    .connect(&spanner_emulator_address))
            } else {
                // Requires
                // GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
                metrics.start_timer("storage.pool.grpc_auth", None);

                // XXX: issue732: Could google_default_credentials (or
                // ChannelBuilder::secure_connect) block?!
                let creds = ChannelCredentials::google_default_credentials()?;
                Ok(ChannelBuilder::new(env)
                    .max_send_message_len(100 << 20)
                    .max_receive_message_len(100 << 20)
                    .secure_connect(SPANNER_ADDRESS, creds))
            }
        },
        DbError::internal,
    )
    .await?;
    let client = SpannerClient::new(chan);

    // Connect to the instance and create a Spanner session.
    let session = create_session(&client, database_name).await?;

    Ok(SpannerSession {
        session,
        client,
        use_test_transactions,
        create_time: crate::now(),
        using_spanner_emulator,
    })
}

/// Recycle a cached Session for reuse
pub async fn recycle_spanner_session(
    conn: &mut SpannerSession,
    database_name: &str,
    metrics: &Metrics,
    max_lifetime: Option<u32>,
    max_idle: Option<u32>,
) -> Result<(), DbError> {
    let now = crate::now();
    let mut req = GetSessionRequest::new();
    req.set_name(conn.session.get_name().to_owned());
    /*
    Connections can sometimes produce GOAWAY errors. GOAWAYs are HTTP2 frame
    errors that are (usually) sent before a given connection is shut down. It
    appears that GRPC passes these up the chain. The problem is that since the
    connection is being closed, further retries will (probably?) also fail. The
    best course of action is to spin up a new session.

    In theory, UNAVAILABLE-GOAWAY messages are retryable. How we retry them,
    however, is not so clear. There are a few places in spanner functions where
    we could possibly do this, but they get complicated quickly. (e.g. pass a
    `&mut SpannerDb` to `db.execute_async`, but that gets REALLY messy, REALLY
    fast.)

    For now, we try a slightly different tactic here. Connections can age out
    both from overall age and from lack of use. We can try to pre-emptively
    kill off connections before we get the GOAWAY messages. Any additional
    GOAWAY messages would be returned to the client as a 500 which will
    result in the client re-trying.

     */
    match conn.client.get_session_async(&req)?.await {
        Ok(this_session) => {
            // Remember, this_session may not be related to
            // the SpannerSession.session, so you may need
            // to reflect changes if you want a more permanent
            // data reference.
            if this_session.get_name() != conn.session.get_name() {
                warn!(
                    "This session may not be the session you want {} != {}",
                    this_session.get_name(),
                    conn.session.get_name()
                );
            }
            if let Some(max_life) = max_lifetime {
                // use our create time. (this_session has it's own
                // `create_time` timestamp, but clock drift could
                // be an issue.)
                let age = now - conn.create_time;
                if age > max_life as i64 {
                    metrics.incr("db.connection.max_life");
                    dbg!("### aging out", this_session.get_name());
                    return Err(DbError::expired());
                }
            }
            // check how long that this has been idle...
            if let Some(max_idle) = max_idle {
                // use the Protobuf last use time from the saved
                // reference Session. It's not perfect, but it's good enough.
                let idle = conn
                    .session
                    .approximate_last_use_time
                    .clone()
                    .into_option()
                    .map(|time| now - time.seconds)
                    .unwrap_or_default();
                if idle > max_idle as i64 {
                    metrics.incr("db.connection.max_idle");
                    dbg!("### idling out", this_session.get_name());
                    return Err(DbError::expired());
                }
                // and update the connection's reference session info
                conn.session = this_session;
            }
            Ok(())
        }
        Err(e) => match e {
            grpcio::Error::RpcFailure(ref status)
                if status.code() == grpcio::RpcStatusCode::NOT_FOUND =>
            {
                conn.session = create_session(&conn.client, database_name).await?;
                Ok(())
            }
            _ => Err(e.into()),
        },
    }
}

async fn create_session(
    client: &SpannerClient,
    database_name: &str,
) -> Result<Session, grpcio::Error> {
    let mut req = CreateSessionRequest::new();
    req.database = database_name.to_owned();
    let mut meta = MetadataBuilder::new();
    meta.add_str("google-cloud-resource-prefix", database_name)?;
    meta.add_str("x-goog-api-client", "gcp-grpc-rs")?;
    let opt = CallOption::default().headers(meta.build());
    client.create_session_async_opt(&req, opt)?.await
}
