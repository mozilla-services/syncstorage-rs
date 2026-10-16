use jsonwebtoken::jwk::Jwk;
use regex::Regex;
use serde::Deserialize;

use tokenserver_common::NodeType;

const PROJECT_ID_REGEX: &str = r"^[a-z][a-z0-9-]{4,28}[a-z0-9]$";
const DATASET_ID_REGEX: &str = r"^[A-Za-z0-9_]{1,1024}$";
const TABLE_ID_REGEX: &str = r"^[A-Za-z0-9_-]{1,1024}$";

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The URL of the Tokenserver MySQL database.
    ///
    /// Has no usable default: when unset (and Tokenserver is enabled) this is
    /// left empty so the server fails fast at startup (see
    /// `syncserver_settings::Settings::validate`) rather than silently
    /// connecting to a default host.
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
    #[serde(default)]
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
    /// The storage node URL to insert into the `nodes` table on startup.  The insert is performed
    /// only when this value is set.  On db record conflict it will do nothing.
    pub init_node_url: Option<String>,
    /// The capacity value for the node record created for `init_node_url`.  Only used if
    /// `init_node_url` is set.
    pub init_node_capacity: i32,
    /// Whether to enable the FxA webhook endpoint.
    /// Defaults to false.
    pub fxa_webhook_enabled: bool,
    /// Whether the FxA webhook handler runs in metrics-only mode. When enabled, received events
    /// are counted but not processed.
    /// Defaults to false.
    pub fxa_webhook_metrics_only: bool,
    /// The `aud` of Security Event Tokens received on the account events webhook endpoint.
    pub fxa_webhook_set_client_id: Option<String>,
    /// The `iss` of Security Event Tokens received on the account events webhook endpoint.
    pub fxa_webhook_set_issuer: Option<String>,
    /// Whether new users may register to the Tokenserver.
    /// Defaults to true.
    pub allow_new_users: bool,
    /// Whether to record account last-seen times in BigQuery. Requires syncserver's `last_seen`
    /// feature.
    /// Defaults to false.
    pub last_seen_enabled: bool,
    /// The fully qualified BigQuery staging table for last-seen rows, as
    /// `PROJECT_ID.DATASET_ID.TABLE_ID`. Required when `last_seen_enabled` is true.
    pub last_seen_staging_table: Option<String>,
    /// The maximum number of concurrent last-seen uploads.
    /// Defaults to 4.
    pub last_seen_max_concurrent_flushes: usize,
}

/// Splits a fully qualified BigQuery table ID into `(project, dataset, table)`.
pub fn parse_bigquery_table_id(table_id: &str) -> Option<(&str, &str, &str)> {
    let parts: Vec<&str> = table_id.split('.').collect();
    let [project, dataset, table] = parts[..] else {
        return None;
    };
    let matches = |regex: &str, id: &str| Regex::new(regex).unwrap().is_match(id);
    let valid = matches(PROJECT_ID_REGEX, project)
        && matches(DATASET_ID_REGEX, dataset)
        && matches(TABLE_ID_REGEX, table);
    valid.then_some((project, dataset, table))
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            // Intentionally empty: a missing DATABASE_URL must fail fast at
            // startup rather than fall back to a default connection string.
            database_url: String::new(),
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
            init_node_url: None,
            init_node_capacity: 100000,
            fxa_webhook_enabled: false,
            fxa_webhook_metrics_only: false,
            fxa_webhook_set_client_id: None,
            fxa_webhook_set_issuer: None,
            allow_new_users: true,
            last_seen_enabled: false,
            last_seen_staging_table: None,
            last_seen_max_concurrent_flushes: 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_bigquery_table_id;

    #[test]
    fn test_parse_bigquery_table_id() {
        assert_eq!(
            parse_bigquery_table_id("moz-fx-sync-prod.account_last_seen.last_seen_staging"),
            Some(("moz-fx-sync-prod", "account_last_seen", "last_seen_staging"))
        );
        assert_eq!(
            parse_bigquery_table_id("my-project.Data_1.last-seen_staging"),
            Some(("my-project", "Data_1", "last-seen_staging"))
        );
        let long_table = format!("project.dataset.{}", "x".repeat(1025));
        for invalid in [
            "",
            "table",
            "dataset.table",
            ".dataset.table",
            "project..table",
            "project.dataset.",
            "project.dataset.table.columnmaybe",
            "project:dataset.table",
            "My-project.dataset.table",
            "proj.dataset.table",
            "a-project-id-that-is-over-thirty.dataset.table",
            "1project.dataset.table",
            "project-.dataset.table",
            "project.data-set.table",
            "project.dataset.ta/ble",
            "project.dataset.last seen",
            &long_table,
        ] {
            assert_eq!(parse_bigquery_table_id(invalid), None, "{invalid:?}");
        }
    }
}
