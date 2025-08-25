use std::sync::Arc;

use google_cloud_rust_raw::spanner::v1::{
    spanner::{CreateSessionRequest, GetSessionRequest, Session},
    spanner_grpc::SpannerClient,
};
use grpcio::{CallOption, ChannelBuilder, ChannelCredentials, Environment};
use syncserver_common::{BlockingThreadpool, Metrics};
use syncstorage_settings::Settings;

use crate::{error::DbError, metadata::MetadataBuilder};

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
    /// Session Settings
    pub settings: SpannerSessionSettings,
    /// A second based UTC for SpannerSession creation.
    /// Session has a similar `create_time` value that is managed by protobuf,
    /// but some clock skew issues are possible.
    pub(crate) create_time: i64,
}

impl SpannerSession {
    /// Return [CallOption] including a Dynamic Routing header for this
    /// [SpannerSession] in its [grpcio::Metdata]
    ///
    /// The more common form of [grpcio::Metdata] used by Spanner operations
    /// such as `begin`, `commit`, `execute_sql`, etc.
    pub fn session_opt(&self) -> Result<CallOption, grpcio::Error> {
        // NOTE: this could also be cached (then cloned)
        let meta = self
            .settings
            .metadata_builder()
            .routing_param("session", self.session.get_name())
            .build()?;
        Ok(CallOption::default().headers(meta))
    }
}

#[derive(Clone, Debug)]
pub struct SpannerSessionSettings {
    /// The database name
    pub database: String,

    /// Whether [SpannerDb] uses mutations, which should be more efficient for
    /// some of its bulk operations
    pub use_mutations: bool,

    /// Whether the Leader Aware Routing header should be included in gRPC
    /// metdata
    pub route_to_leader: bool,

    /// Max age of a Session
    pub max_lifespan: Option<u32>,
    /// Max idle time of a Session
    pub max_idle: Option<u32>,

    /// For tests: disables transactions from committing
    pub(crate) use_test_transactions: bool,
    /// Spanner emulator hostname when set to Spanner emulator mode
    pub emulator_host: Option<String>,
}

impl SpannerSessionSettings {
    pub fn from_settings(settings: &Settings) -> Result<Self, DbError> {
        let database = settings
            .spanner_database_name()
            .ok_or_else(|| {
                DbError::internal(format!("Invalid database url: {}", settings.database_url))
            })?
            .to_owned();

        #[cfg(not(debug_assertions))]
        let (use_test_transactions, use_mutations) = (false, true);
        #[cfg(debug_assertions)]
        let (use_test_transactions, use_mutations) = (
            settings.database_use_test_transactions,
            settings.database_spanner_use_mutations,
        );

        Ok(Self {
            database,
            use_mutations,
            route_to_leader: settings.database_spanner_route_to_leader,
            max_lifespan: settings.database_pool_connection_lifespan,
            max_idle: settings.database_pool_connection_max_idle,
            use_test_transactions,
            emulator_host: settings.spanner_emulator_host.clone(),
        })
    }

    /// Whether the Spanner emulator's in use
    pub fn using_spanner_emulator(&self) -> bool {
        self.emulator_host.is_some()
    }

    /// Build [grpcio::Metadata] with a Resource prefix and other applicable
    /// settings
    pub fn metadata_builder(&self) -> MetadataBuilder<'_> {
        MetadataBuilder::with_prefix(&self.database).route_to_leader(self.route_to_leader)
    }
}

/// Create a Session (and the underlying gRPC Channel)
pub async fn create_spanner_session(
    settings: &SpannerSessionSettings,
    env: Arc<Environment>,
    mut metrics: Metrics,
    blocking_threadpool: Arc<BlockingThreadpool>,
) -> Result<SpannerSession, DbError> {
    let emulator_host = settings.emulator_host.clone();
    let chan = blocking_threadpool
        .spawn(move || -> Result<grpcio::Channel, DbError> {
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
                    .set_credentials(creds)
                    .connect(SPANNER_ADDRESS))
            }
        })
        .await?;
    let client = SpannerClient::new(chan);

    // Connect to the instance and create a Spanner session.
    let session = create_session(&client, settings).await?;

    Ok(SpannerSession {
        session,
        client,
        // NOTE: later versions of deadpool provide an Object::pool method
        // where we could get Settings from (via Manager) instead of cloning
        settings: settings.clone(),
        create_time: crate::now(),
    })
}

/// Recycle a cached Session for reuse
pub async fn recycle_spanner_session(
    conn: &mut SpannerSession,
    metrics: &Metrics,
) -> Result<(), DbError> {
    let settings = &conn.settings;
    let session = conn.session.get_name();
    let now = crate::now();
    let mut req = GetSessionRequest::new();
    req.set_name(session.to_owned());
    let meta = settings
        .metadata_builder()
        .routing_param("name", session)
        .build()?;
    let opt = CallOption::default().headers(meta);

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
    match conn.client.get_session_async_opt(&req, opt)?.await {
        Ok(this_session) => {
            // Remember, this_session may not be related to
            // the SpannerSession.session, so you may need
            // to reflect changes if you want a more permanent
            // data reference.
            if this_session.get_name() != session {
                warn!(
                    "This session may not be the session you want {} != {}",
                    this_session.get_name(),
                    conn.session.get_name()
                );
            }
            if let Some(max_life) = settings.max_lifespan {
                // use our create time. (this_session has it's own
                // `create_time` timestamp, but clock drift could
                // be an issue.)
                let age = now - conn.create_time;
                if age > max_life as i64 {
                    metrics.incr("db.connection.max_life");
                    return Err(DbError::expired());
                }
            }
            // check how long that this has been idle...
            if let Some(max_idle) = settings.max_idle {
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
                conn.session = create_session(&conn.client, settings).await?;
                Ok(())
            }
            _ => Err(e.into()),
        },
    }
}

async fn create_session(
    client: &SpannerClient,
    settings: &SpannerSessionSettings,
) -> Result<Session, grpcio::Error> {
    let mut req = CreateSessionRequest::new();
    req.database.clone_from(&settings.database);
    let meta = settings
        .metadata_builder()
        .routing_param("database", &settings.database)
        .build()?;
    let opt = CallOption::default().headers(meta);
    client.create_session_async_opt(&req, opt)?.await
}
