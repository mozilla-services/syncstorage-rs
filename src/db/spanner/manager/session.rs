use actix_web::web::block;
use googleapis_raw::spanner::v1::{
    spanner::{CreateSessionRequest, GetSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, Environment, MetadataBuilder};
use std::sync::Arc;
use std::time::SystemTime;

use crate::{db::error::DbError, server::metrics::Metrics};

const SPANNER_ADDRESS: &str = "spanner.googleapis.com:443";

/// Represents a communication channel w/ Spanner
///
/// Session creation is expensive in Spanner so sessions should be long-lived
/// and cached for reuse.
pub struct SpannerSession {
    pub session: Session,
    /// The underlying client (Connection/Channel) for interacting with spanner
    pub client: SpannerClient,
    pub(in crate::db::spanner) use_test_transactions: bool,
}

/// Create a Session (and the underlying gRPC Channel)
pub async fn create_spanner_session(
    env: Arc<Environment>,
    mut metrics: Metrics,
    database_name: &str,
    use_test_transactions: bool,
) -> Result<SpannerSession, DbError> {
    // XXX: issue732: Could google_default_credentials (or
    // ChannelBuilder::secure_connect) block?!
    let chan = block(move || -> Result<grpcio::Channel, grpcio::Error> {
        metrics.start_timer("storage.pool.grpc_auth", None);
        // Requires
        // GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
        let creds = ChannelCredentials::google_default_credentials()?;
        Ok(ChannelBuilder::new(env)
            .max_send_message_len(100 << 20)
            .max_receive_message_len(100 << 20)
            .secure_connect(SPANNER_ADDRESS, creds))
    })
    .await
    .map_err(|e| match e {
        actix_web::error::BlockingError::Error(e) => e.into(),
        actix_web::error::BlockingError::Canceled => {
            DbError::internal("web::block Manager operation canceled")
        }
    })?;
    let client = SpannerClient::new(chan);

    // Connect to the instance and create a Spanner session.
    let session = create_session(&client, database_name).await?;

    Ok(SpannerSession {
        session,
        client,
        use_test_transactions,
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
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
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
        Ok(session) => {
            if let Some(max_life) = max_lifetime {
                let create_time = session.get_create_time().seconds;
                let age = now - create_time;
                if age > max_life as i64 {
                    metrics.incr("db.connection.max_life");
                    dbg!("### aging out", conn.session.get_name());
                    conn.session = create_session(&conn.client, database_name).await?;
                }
            }
            // check how long that this has been idle...
            if let Some(max_idle) = max_idle {
                let last_use = session.get_approximate_last_use_time().seconds;
                let idle = std::cmp::max(0, now - last_use);
                if idle > max_idle as i64 {
                    metrics.incr("db.connection.max_idle");
                    dbg!("### idling out", session.get_name());
                    conn.session = create_session(&conn.client, database_name).await?;
                }
            }
            Ok(())
        }
        Err(e) => match e {
            grpcio::Error::RpcFailure(ref status)
                if status.status == grpcio::RpcStatusCode::NOT_FOUND =>
            {
                conn.session = create_session(&conn.client, database_name).await?;
                Ok(())
            }
            _ => return Err(e.into()),
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
