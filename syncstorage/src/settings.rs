//! Application settings objects and initialization
use std::{
    cmp::min,
    env::{self, VarError},
};

use actix_cors::Cors;
use actix_web::http::header::{AUTHORIZATION, CONTENT_TYPE, USER_AGENT};
use config::{Config, ConfigError, Environment, File};
use http::method::Method;
use rand::{thread_rng, Rng};
use serde::{de::Deserializer, Deserialize, Serialize};
use url::Url;

use crate::db::spanner::models::MAX_SPANNER_LOAD_SIZE;
use crate::error::ApiError;
use crate::tokenserver::settings::Settings as TokenserverSettings;
use crate::web::auth::hkdf_expand_32;
use crate::web::{
    X_LAST_MODIFIED, X_VERIFY_CODE, X_WEAVE_BYTES, X_WEAVE_NEXT_OFFSET, X_WEAVE_RECORDS,
    X_WEAVE_TIMESTAMP, X_WEAVE_TOTAL_BYTES, X_WEAVE_TOTAL_RECORDS,
};

static DEFAULT_PORT: u16 = 8000;

static KILOBYTE: u32 = 1024;
static MEGABYTE: u32 = KILOBYTE * KILOBYTE;
static GIGABYTE: u32 = MEGABYTE * 1_000;
static DEFAULT_MAX_POST_BYTES: u32 = 2 * MEGABYTE;
static DEFAULT_MAX_POST_RECORDS: u32 = 100;
static DEFAULT_MAX_RECORD_PAYLOAD_BYTES: u32 = 2 * MEGABYTE;
static DEFAULT_MAX_REQUEST_BYTES: u32 = DEFAULT_MAX_POST_BYTES + 4 * KILOBYTE;
static DEFAULT_MAX_TOTAL_BYTES: u32 = 100 * DEFAULT_MAX_POST_BYTES;
// also used to determine the max number of records to return for MySQL.
pub static DEFAULT_MAX_TOTAL_RECORDS: u32 = 100 * DEFAULT_MAX_POST_RECORDS;
// Hard spanner limit is 4GB per split (items under a unique index).
// This gives us more than a bit of wiggle room.
static DEFAULT_MAX_QUOTA_LIMIT: u32 = 2 * GIGABYTE;
static PREFIX: &str = "sync";

#[derive(Clone, Debug, Default, Copy)]
pub struct Quota {
    pub size: usize,
    pub enabled: bool,
    pub enforced: bool,
}

#[derive(Copy, Clone, Default, Debug)]
/// Deadman configures how the `/__lbheartbeat__` health check endpoint fails
/// for special conditions.
///
/// We'll fail the check (usually temporarily) due to the db pool maxing out
/// connections, which notifies the orchestration system that it should back
/// off traffic to this instance until the check succeeds.
///
/// Optionally we can permanently fail the check after a set time period,
/// indicating that this instance should be evicted and replaced.
pub struct Deadman {
    pub max_size: u32,
    pub previous_count: usize,
    pub clock_start: Option<time::Instant>,
    pub expiry: Option<time::Instant>,
}

impl From<&Settings> for Deadman {
    fn from(settings: &Settings) -> Self {
        let expiry = settings.lbheartbeat_ttl.map(|lbheartbeat_ttl| {
            // jitter's a range of percentage of ttl added to ttl. E.g. a 60s
            // ttl w/ a 10% jitter results in a random final ttl between 60-66s
            let ttl = lbheartbeat_ttl as f32;
            let max_jitter = ttl * (settings.lbheartbeat_ttl_jitter as f32 * 0.01);
            let ttl = thread_rng().gen_range(ttl..ttl + max_jitter);
            time::Instant::now() + time::Duration::seconds(ttl as i64)
        });
        Deadman {
            max_size: settings.database_pool_max_size,
            expiry,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub port: u16,
    pub host: String,
    pub database_url: String,
    pub database_pool_max_size: u32,
    // NOTE: Not supported by deadpool!
    pub database_pool_min_idle: Option<u32>,
    /// Pool timeout when waiting for a slot to become available, in seconds
    pub database_pool_connection_timeout: Option<u32>,
    /// Max age a given connection should live, in seconds
    pub database_pool_connection_lifespan: Option<u32>,
    /// Max time a connection should sit idle before being dropped.
    pub database_pool_connection_max_idle: Option<u32>,
    #[cfg(test)]
    pub database_use_test_transactions: bool,

    pub actix_keep_alive: Option<u32>,

    /// Server-enforced limits for request payloads.
    pub limits: ServerLimits,

    /// The master secret, from which are derived
    /// the signing secret and token secret
    /// that are used during Hawk authentication.
    pub master_secret: Secrets,

    pub human_logs: bool,

    pub statsd_host: Option<String>,
    pub statsd_port: u16,
    pub statsd_label: String,

    pub enable_quota: bool,
    pub enforce_quota: bool,

    pub spanner_emulator_host: Option<String>,

    /// Disable all of the endpoints related to syncstorage. To be used when running Tokenserver
    /// in isolation.
    pub disable_syncstorage: bool,

    /// Settings specific to Tokenserver
    pub tokenserver: TokenserverSettings,

    /// Cors Settings
    pub cors_allowed_origin: Option<String>,
    pub cors_max_age: Option<usize>,
    pub cors_allowed_methods: Option<Vec<String>>,
    pub cors_allowed_headers: Option<Vec<String>>,

    /// Fail the `/__lbheartbeat__` healthcheck after running for this duration
    /// of time (in seconds) + jitter
    pub lbheartbeat_ttl: Option<u32>,
    /// Percentage of `lbheartbeat_ttl` time to "jitter" (adds additional,
    /// randomized time)
    pub lbheartbeat_ttl_jitter: u32,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            debug: false,
            port: DEFAULT_PORT,
            host: "127.0.0.1".to_string(),
            database_url: "mysql://root@127.0.0.1/syncstorage".to_string(),
            database_pool_max_size: 10,
            database_pool_min_idle: None,
            database_pool_connection_lifespan: None,
            database_pool_connection_max_idle: None,
            database_pool_connection_timeout: Some(30),
            #[cfg(test)]
            database_use_test_transactions: false,
            actix_keep_alive: None,
            limits: ServerLimits::default(),
            master_secret: Secrets::default(),
            statsd_host: None,
            statsd_port: 8125,
            statsd_label: "syncstorage".to_string(),
            human_logs: false,
            enable_quota: false,
            enforce_quota: false,
            spanner_emulator_host: None,
            disable_syncstorage: false,
            tokenserver: TokenserverSettings::default(),
            cors_allowed_origin: Some("*".to_owned()),
            cors_allowed_methods: Some(vec![
                "DELETE".to_owned(),
                "GET".to_owned(),
                "POST".to_owned(),
                "PUT".to_owned(),
            ]),
            cors_allowed_headers: Some(vec![
                AUTHORIZATION.to_string(),
                CONTENT_TYPE.to_string(),
                USER_AGENT.to_string(),
                X_LAST_MODIFIED.to_owned(),
                X_WEAVE_TIMESTAMP.to_owned(),
                X_WEAVE_NEXT_OFFSET.to_owned(),
                X_WEAVE_RECORDS.to_owned(),
                X_WEAVE_BYTES.to_owned(),
                X_WEAVE_TOTAL_RECORDS.to_owned(),
                X_WEAVE_TOTAL_BYTES.to_owned(),
                X_VERIFY_CODE.to_owned(),
                "TEST_IDLES".to_owned(),
            ]),
            cors_max_age: Some(1728000),
            lbheartbeat_ttl: None,
            lbheartbeat_ttl_jitter: 25,
        }
    }
}

impl Settings {
    /// Load the settings from the config file if supplied, then the environment.
    pub fn with_env_and_config_file(filename: &Option<String>) -> Result<Self, ConfigError> {
        let mut s = Config::default();
        // Set our defaults, this can be fixed up drastically later after:
        // https://github.com/mehcode/config-rs/issues/60
        s.set_default("debug", false)?;
        s.set_default("port", i64::from(DEFAULT_PORT))?;
        s.set_default("host", "127.0.0.1")?;
        s.set_default("human_logs", false)?;
        #[cfg(test)]
        s.set_default("database_pool_connection_timeout", Some(30))?;
        s.set_default("database_pool_max_size", 10)?;
        // Max lifespan a connection should have.
        s.set_default::<Option<String>>("database_connection_lifespan", None)?;
        // Max time a connection should be idle before dropping.
        s.set_default::<Option<String>>("database_connection_max_idle", None)?;
        s.set_default("database_use_test_transactions", false)?;
        s.set_default("master_secret", "")?;
        // Each backend does their own default process, so specifying a "universal" value
        // for database_pool_max_size doesn't quite work. Generally the max pool size is
        // 10.
        s.set_default("master_secret", "")?;
        s.set_default("limits.max_post_bytes", i64::from(DEFAULT_MAX_POST_BYTES))?;
        s.set_default(
            "limits.max_post_records",
            i64::from(DEFAULT_MAX_POST_RECORDS),
        )?;
        s.set_default(
            "limits.max_record_payload_bytes",
            i64::from(DEFAULT_MAX_RECORD_PAYLOAD_BYTES),
        )?;
        s.set_default(
            "limits.max_request_bytes",
            i64::from(DEFAULT_MAX_REQUEST_BYTES),
        )?;
        s.set_default("limits.max_total_bytes", i64::from(DEFAULT_MAX_TOTAL_BYTES))?;
        s.set_default(
            "limits.max_total_records",
            i64::from(DEFAULT_MAX_TOTAL_RECORDS),
        )?;
        s.set_default("limits.max_quota_limit", i64::from(DEFAULT_MAX_QUOTA_LIMIT))?;

        s.set_default("statsd_host", "localhost")?;
        s.set_default("statsd_port", 8125)?;
        s.set_default("statsd_label", "syncstorage")?;
        s.set_default("enable_quota", false)?;
        s.set_default("enforce_quota", false)?;
        s.set_default("disable_syncstorage", false)?;

        // Set Tokenserver defaults
        s.set_default(
            "tokenserver.database_url",
            "mysql://root@127.0.0.1/tokenserver",
        )?;
        s.set_default("tokenserver.database_pool_max_size", 10)?;
        s.set_default("tokenserver.enabled", false)?;
        s.set_default(
            "tokenserver.fxa_browserid_audience",
            "https://token.stage.mozaws.net",
        )?;
        s.set_default(
            "tokenserver.fxa_browserid_issuer",
            "api-accounts.stage.mozaws.net",
        )?;
        s.set_default(
            "tokenserver.fxa_browserid_server_url",
            "https://verifier.stage.mozaws.net/v2",
        )?;
        s.set_default("tokenserver.fxa_browserid_request_timeout", 10)?;
        s.set_default("tokenserver.fxa_browserid_connect_timeout", 5)?;
        s.set_default(
            "tokenserver.fxa_email_domain",
            "api-accounts.stage.mozaws.net",
        )?;
        s.set_default("tokenserver.fxa_metrics_hash_secret", "secret")?;
        s.set_default(
            "tokenserver.fxa_oauth_server_url",
            "https://oauth.stage.mozaws.net",
        )?;
        s.set_default("tokenserver.fxa_oauth_request_timeout", 10)?;

        // The type parameter for None::<bool> below would more appropriately be `Jwk`, but due
        // to constraints imposed by version 0.11 of the config crate, it is not possible to
        // implement `ValueKind: From<Jwk>`. The next best thing would be to use `ValueKind`,
        // but `ValueKind` is private in this version of config. We use `bool` as a placeholder,
        // since `ValueKind: From<bool>` is implemented, and None::<T> for all T is simply
        // converted to ValueKind::Nil (see below link).
        // https://github.com/mehcode/config-rs/blob/0.11.0/src/value.rs#L35
        s.set_default("tokenserver.fxa_oauth_primary_jwk", None::<bool>)?;
        s.set_default("tokenserver.fxa_oauth_secondary_jwk", None::<bool>)?;

        s.set_default("tokenserver.node_type", "spanner")?;
        s.set_default("tokenserver.statsd_label", "syncstorage.tokenserver")?;
        s.set_default("tokenserver.run_migrations", cfg!(test))?;

        // Set Cors defaults
        s.set_default(
            "cors_allowed_headers",
            Some(vec![
                AUTHORIZATION.to_string().as_str(),
                CONTENT_TYPE.to_string().as_str(),
                USER_AGENT.to_string().as_str(),
                X_LAST_MODIFIED,
                X_WEAVE_TIMESTAMP,
                X_WEAVE_NEXT_OFFSET,
                X_WEAVE_RECORDS,
                X_WEAVE_BYTES,
                X_WEAVE_TOTAL_RECORDS,
                X_WEAVE_TOTAL_BYTES,
                X_VERIFY_CODE,
                "TEST_IDLES",
            ]),
        )?;
        s.set_default(
            "cors_allowed_methods",
            Some(vec!["DELETE", "GET", "POST", "PUT"]),
        )?;
        s.set_default("cors_allowed_origin", Some("*"))?;
        s.set_default("lbheartbeat_ttl_jitter", 25)?;

        // Merge the config file if supplied
        if let Some(config_filename) = filename {
            s.merge(File::with_name(config_filename))?;
        }

        // Merge the environment overrides
        // While the prefix is currently case insensitive, it's traditional that
        // environment vars be UPPERCASE, this ensures that will continue should
        // Environment ever change their policy about case insensitivity.
        // This will accept environment variables specified as
        // `SYNC_FOO__BAR_VALUE="gorp"` as `foo.bar_value = "gorp"`
        s.merge(Environment::with_prefix(&PREFIX.to_uppercase()).separator("__"))?;

        Ok(match s.try_into::<Self>() {
            Ok(mut s) => {
                // Adjust the max values if required.
                if s.uses_spanner() {
                    let mut ms = s;
                    ms.limits.max_total_bytes =
                        min(ms.limits.max_total_bytes, MAX_SPANNER_LOAD_SIZE as u32);
                    return Ok(ms);
                } else {
                    // No quotas for stand alone servers
                    s.limits.max_quota_limit = 0;
                    s.enable_quota = false;
                    s.enforce_quota = false;
                }
                if s.limits.max_quota_limit == 0 {
                    s.enable_quota = false
                }
                if s.enforce_quota {
                    s.enable_quota = true
                }

                if matches!(
                    env::var("ACTIX_THREADPOOL").unwrap_err(),
                    Err(VarError::NotPresent)
                ) {
                    // Db backends w/ blocking calls block via
                    // actix-threadpool: grow its size to accommodate the
                    // full number of connections
                    let total_db_pool_size = {
                        let syncstorage_pool_max_size = if s.uses_spanner() || s.disable_syncstorage
                        {
                            0
                        } else {
                            s.database_pool_max_size
                        };

                        let tokenserver_pool_max_size = if s.tokenserver.enabled {
                            s.tokenserver.database_pool_max_size
                        } else {
                            0
                        };

                        syncstorage_pool_max_size + tokenserver_pool_max_size
                    };

                    let fxa_threads = if s.tokenserver.enabled
                        && s.tokenserver.fxa_oauth_primary_jwk.is_none()
                        && s.tokenserver.fxa_oauth_secondary_jwk.is_none()
                    {
                        s.tokenserver
                            .additional_blocking_threads_for_fxa_requests
                            .ok_or_else(|| {
                                println!(
                                    "If the Tokenserver OAuth JWK is not cached, additional blocking \
                                     threads must be used to handle the requests to FxA."
                                );

                                let setting_name =
                                    "tokenserver.additional_blocking_threads_for_fxa_requests";
                                ConfigError::NotFound(String::from(setting_name))
                            })?
                    } else {
                        0
                    };

                    env::set_var(
                        "ACTIX_THREADPOOL",
                        ((total_db_pool_size + fxa_threads) as usize)
                            .max(num_cpus::get() * 5)
                            .to_string(),
                    );
                }

                s
            }
            Err(e) => match e {
                // Configuration errors are not very sysop friendly, Try to make them
                // a bit more 3AM useful.
                ConfigError::Message(v) => {
                    println!("Bad configuration: {:?}", &v);
                    println!("Please set in config file or use environment variable.");
                    println!(
                        "For example to set `database_url` use env var `{}_DATABASE_URL`\n",
                        PREFIX.to_uppercase()
                    );
                    error!("Configuration error: Value undefined {:?}", &v);
                    return Err(ConfigError::NotFound(v));
                }
                _ => {
                    error!("Configuration error: Other: {:?}", &e);
                    return Err(e);
                }
            },
        })
    }

    pub fn uses_spanner(&self) -> bool {
        self.database_url.as_str().starts_with("spanner://")
    }

    pub fn spanner_database_name(&self) -> Option<&str> {
        if !self.uses_spanner() {
            None
        } else {
            Some(&self.database_url["spanner://".len()..])
        }
    }

    /// A simple banner for display of certain settings at startup
    pub fn banner(&self) -> String {
        let quota = if self.enable_quota {
            format!(
                "Quota: {} bytes ({}enforced)",
                self.limits.max_quota_limit,
                if !self.enforce_quota { "un" } else { "" }
            )
        } else {
            "No quota".to_owned()
        };
        let db = Url::parse(&self.database_url)
            .map(|url| url.scheme().to_owned())
            .unwrap_or_else(|_| "<invalid db>".to_owned());
        format!("http://{}:{} ({}) {}", self.host, self.port, db, quota)
    }

    pub fn build_cors(&self) -> Cors {
        // Followed by the "official middleware" so they run first.
        // actix is getting increasingly tighter about CORS headers. Our server is
        // not a huge risk but does deliver XHR JSON content.
        // For now, let's be permissive and use NGINX (the wrapping server)
        // for finer grained specification.
        let mut cors = Cors::default();

        if let Some(allowed_methods) = &self.cors_allowed_methods {
            let mut methods = vec![];
            for method_string in allowed_methods {
                let method = Method::from_bytes(method_string.as_bytes()).unwrap();
                methods.push(method);
            }
            cors = cors.allowed_methods(methods);
        }
        if let Some(allowed_headers) = &self.cors_allowed_headers {
            cors = cors.allowed_headers(allowed_headers);
        }

        if let Some(max_age) = &self.cors_max_age {
            cors = cors.max_age(*max_age);
        }
        // explicitly set the CORS allow origin, since Default does not
        // appear to set the `allow-origins: *` header.
        if let Some(origin) = &self.cors_allowed_origin {
            if origin == "*" {
                cors = cors.allow_any_origin();
            } else {
                cors = cors.allowed_origin(origin);
            }
        }

        cors
    }
}

/// Server-enforced limits for request payloads.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerLimits {
    /// Maximum combined size of BSO payloads for a single request, in bytes.
    pub max_post_bytes: u32,

    /// Maximum BSO count for a single request.
    pub max_post_records: u32,

    /// Maximum size of an individual BSO payload, in bytes.
    pub max_record_payload_bytes: u32,

    /// Maximum `Content-Length` for all incoming requests, in bytes.
    ///
    /// Enforced externally to this repo, at the web server level.
    /// It's important that nginx (or whatever)
    /// really is configured to enforce exactly this limit,
    /// otherwise client requests may fail with a 413
    /// before even reaching the API.
    pub max_request_bytes: u32,

    /// Maximum combined size of BSO payloads across a batch upload, in bytes.
    pub max_total_bytes: u32,

    /// Maximum BSO count across a batch upload.
    pub max_total_records: u32,
    pub max_quota_limit: u32,
}

impl Default for ServerLimits {
    /// Create a default `ServerLimits` instance.
    fn default() -> Self {
        Self {
            max_post_bytes: DEFAULT_MAX_POST_BYTES,
            max_post_records: DEFAULT_MAX_POST_RECORDS,
            max_record_payload_bytes: DEFAULT_MAX_RECORD_PAYLOAD_BYTES,
            max_request_bytes: DEFAULT_MAX_REQUEST_BYTES,
            max_total_bytes: DEFAULT_MAX_TOTAL_BYTES,
            max_total_records: DEFAULT_MAX_TOTAL_RECORDS,
            max_quota_limit: DEFAULT_MAX_QUOTA_LIMIT,
        }
    }
}

/// Secrets used during Hawk authentication.
#[derive(Clone, Debug)]
pub struct Secrets {
    /// The master secret in byte array form.
    ///
    /// The signing secret and token secret are derived from this.
    pub master_secret: Vec<u8>,

    /// The signing secret used during Hawk authentication.
    pub signing_secret: [u8; 32],
}

impl Secrets {
    /// Decode the master secret to a byte array
    /// and derive the signing secret from it.
    pub fn new(master_secret: &str) -> Result<Self, ApiError> {
        let master_secret = master_secret.as_bytes().to_vec();
        let signing_secret = hkdf_expand_32(
            b"services.mozilla.com/tokenlib/v1/signing",
            None,
            &master_secret,
        )?;
        Ok(Self {
            master_secret,
            signing_secret,
        })
    }
}

impl Default for Secrets {
    /// Create a (useless) default `Secrets` instance.
    fn default() -> Self {
        Self {
            master_secret: vec![],
            signing_secret: [0u8; 32],
        }
    }
}

impl<'d> Deserialize<'d> for Secrets {
    /// Deserialize the master secret and signing secret byte arrays
    /// from a single master secret string.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let master_secret: String = Deserialize::deserialize(deserializer)?;
        Secrets::new(&master_secret)
            .map_err(|e| serde::de::Error::custom(format!("error: {:?}", e)))
    }
}

#[cfg(test)]
pub fn test_settings() -> Settings {
    let mut settings = Settings::with_env_and_config_file(&None)
        .expect("Could not get Settings in get_test_settings");
    settings.debug = true;
    settings.port = 8000;
    settings.database_pool_max_size = 1;
    settings.database_use_test_transactions = true;
    settings.database_pool_connection_max_idle = Some(300);
    settings.database_pool_connection_lifespan = Some(300);
    settings
}
