use serde::Deserialize;

use super::NodeType;

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    /// The URL of the Tokenserver MySQL database.
    pub database_url: String,
    /// The max size of the database connection pool.
    pub database_pool_max_size: Option<u32>,
    // NOTE: Not supported by deadpool!
    /// The minimum number of database connections to be maintained at any given time.
    pub database_pool_min_idle: Option<u32>,
    /// Pool timeout when waiting for a slot to become available, in seconds
    pub database_pool_connection_timeout: Option<u32>,
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
    /// performance benefits.
    pub fxa_oauth_jwk: Option<Jwk>,
    /// The issuer expected in the BrowserID verification response.
    pub fxa_browserid_issuer: String,
    /// The audience to be sent to the FxA BrowserID verification server.
    pub fxa_browserid_audience: String,
    /// The URL of the FxA server used for verifying BrowserID assertions.
    pub fxa_browserid_server_url: String,
    /// The timeout to be used when making requests to the FxA BrowserID verification server. This
    /// timeout applies to the duration of the entire request lifecycle, from when the client
    /// begins connecting to when the response body has been received.
    pub fxa_browserid_request_timeout: u64,
    /// The timeout to be used when connecting to the FxA BrowserID verification server. This
    /// timeout applies only to the connect portion of the request lifecycle.
    pub fxa_browserid_connect_timeout: u64,
    /// The rate at which capacity should be released from nodes that are at capacity.
    pub node_capacity_release_rate: Option<f32>,
    /// The type of the storage nodes used by this instance of Tokenserver.
    pub node_type: NodeType,
    /// The label to be used when reporting Metrics.
    pub statsd_label: String,
    /// Whether or not to run the Tokenserver migrations upon startup.
    pub run_migrations: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Jwk {
    pub kty: String,
    pub alg: String,
    pub kid: String,
    pub fxa_created_at: u64,
    #[serde(rename = "use")]
    pub use_of_key: String,
    pub n: String,
    pub e: String,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            database_url: "mysql://root@127.0.0.1/tokenserver_rs".to_owned(),
            database_pool_max_size: None,
            database_pool_min_idle: None,
            database_pool_connection_timeout: Some(30),
            enabled: false,
            fxa_email_domain: "api.accounts.firefox.com".to_owned(),
            fxa_metrics_hash_secret: "secret".to_owned(),
            fxa_oauth_server_url: "https://oauth.stage.mozaws.net".to_owned(),
            fxa_oauth_request_timeout: 10,
            fxa_oauth_jwk: None,
            fxa_browserid_audience: "https://token.stage.mozaws.net".to_owned(),
            fxa_browserid_issuer: "api-accounts.stage.mozaws.net".to_owned(),
            fxa_browserid_server_url: "https://verifier.stage.mozaws.net/v2".to_owned(),
            fxa_browserid_request_timeout: 10,
            fxa_browserid_connect_timeout: 5,
            node_capacity_release_rate: None,
            node_type: NodeType::Spanner,
            statsd_label: "syncstorage.tokenserver".to_owned(),
            run_migrations: cfg!(test),
        }
    }
}
