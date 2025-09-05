use jsonwebtoken::jwk::Jwk;
use serde::Deserialize;
use tokenserver_common::NodeType;

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The URL of the Tokenserver MySQL database.
    pub database_url: String,
    /// The max size of the database connection pool.
    pub database_pool_max_size: u32,
    /// Pool timeout when waiting for a slot to become available, in seconds
    pub database_pool_connection_timeout: Option<u32>,
    /// Database request timeout, in seconds
    pub database_request_timeout: Option<u32>,
    // XXX: This is a temporary setting used to enable Tokenserver-related features. In
    // the future, Tokenserver will always be enabled, and this setting will be
    // removed.
    /// Whether or not to enable Tokenserver.
    pub enabled: bool,
    /// The secret to be used when computing the hash for a Tokenserver user's metrics UID.
    pub fxa_metrics_hash_secret: String,
    /// The email domain for users' FxA accounts. This should be set according to the
    /// desired FxA environment (production or stage).
    pub fxa_email_domain: String,
    /// The URL of the FxA server used for verifying OAuth tokens.
    pub fxa_oauth_server_url: String,
    /// The timeout to be used when making requests to the FxA OAuth verification server.
    pub fxa_oauth_request_timeout: u64,
    /// The JWK to be used to verify OAuth tokens. Passing a JWK to the PyFxA Python library
    /// prevents it from making an external API call to FxA to get the JWK, yielding substantial
    /// performance benefits. This value should match that on the `/v1/jwks` endpoint on the FxA
    /// Auth Server.
    pub fxa_oauth_primary_jwk: Option<Jwk>,
    /// A secondary JWK to be used to verify OAuth tokens. This is intended to be used to enable
    /// seamless key rotations on FxA.
    pub fxa_oauth_secondary_jwk: Option<Jwk>,
    /// The rate at which capacity should be released from nodes that are at capacity.
    pub node_capacity_release_rate: Option<f32>,
    /// The type of the storage nodes used by this instance of Tokenserver.
    #[serde(default = "NodeType::spanner")]
    pub node_type: NodeType,
    /// The label to be used when reporting Metrics.
    pub statsd_label: String,
    /// Whether or not to run the Tokenserver migrations upon startup.
    pub run_migrations: bool,
    /// The database ID of the Spanner node.
    pub spanner_node_id: Option<i32>,
    /// The number of additional blocking threads to add to the blocking threadpool to handle
    /// OAuth verification requests to FxA. This value is added to the worker_max_blocking_threads
    /// config var.
    /// Note that this setting only applies if the OAuth public JWK is not cached, since OAuth
    /// verifications do not require requests to FXA if the JWK is set on Tokenserver. The server
    /// will return an error at startup if the JWK is not cached and this setting is `None`.
    pub additional_blocking_threads_for_fxa_requests: Option<u32>,
    /// The amount of time in seconds before a token provided by Tokenserver expires.
    pub token_duration: u64,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            database_url: "mysql://root@127.0.0.1/tokenserver".to_owned(),
            database_pool_max_size: 10,
            database_pool_connection_timeout: Some(30),
            database_request_timeout: None,
            enabled: false,
            fxa_email_domain: "api-accounts.stage.mozaws.net".to_owned(),
            fxa_metrics_hash_secret: "secret".to_owned(),
            fxa_oauth_server_url: "https://oauth.stage.mozaws.net".to_owned(),
            fxa_oauth_request_timeout: 10,
            fxa_oauth_primary_jwk: None,
            fxa_oauth_secondary_jwk: None,
            node_capacity_release_rate: None,
            node_type: NodeType::Spanner,
            statsd_label: "syncstorage.tokenserver".to_owned(),
            run_migrations: cfg!(test),
            spanner_node_id: None,
            additional_blocking_threads_for_fxa_requests: Some(1),
            token_duration: 3600,
        }
    }
}
