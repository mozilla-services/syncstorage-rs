use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub database_url: String,
    pub database_pool_max_size: Option<u32>,
    // NOTE: Not supported by deadpool!
    pub database_pool_min_idle: Option<u32>,
    /// Pool timeout when waiting for a slot to become available, in seconds
    pub database_pool_connection_timeout: Option<u32>,
    pub fxa_metrics_hash_secret: String,

    /// The email domain for users' FxA accounts. This should be set according to the
    /// desired FxA environment (production or stage).
    pub fxa_email_domain: String,

    /// The URL of the FxA server used for verifying Tokenserver OAuth tokens.
    pub fxa_oauth_server_url: Option<String>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            database_url: "mysql://root@127.0.0.1/tokenserver_rs".to_owned(),
            database_pool_max_size: None,
            database_pool_min_idle: None,
            database_pool_connection_timeout: Some(30),
            fxa_email_domain: "api.accounts.firefox.com".to_owned(),
            fxa_metrics_hash_secret: "secret".to_owned(),
            fxa_oauth_server_url: None,
        }
    }
}
