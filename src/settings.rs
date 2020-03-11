//! Application settings objects and initialization
use std::cmp::min;

use config::{Config, ConfigError, Environment, File};
use serde::{de::Deserializer, Deserialize, Serialize};
use url::Url;

use crate::db::spanner::models::MAX_SPANNER_LOAD_SIZE;
use crate::error::ApiError;
use crate::web::auth::hkdf_expand_32;

static DEFAULT_PORT: u16 = 8000;

static KILOBYTE: u32 = 1024;
static MEGABYTE: u32 = KILOBYTE * KILOBYTE;
static DEFAULT_MAX_POST_BYTES: u32 = 2 * MEGABYTE;
static DEFAULT_MAX_POST_RECORDS: u32 = 100;
static DEFAULT_MAX_RECORD_PAYLOAD_BYTES: u32 = 2 * MEGABYTE;
static DEFAULT_MAX_REQUEST_BYTES: u32 = DEFAULT_MAX_POST_BYTES + 4 * KILOBYTE;
static DEFAULT_MAX_TOTAL_BYTES: u32 = 100 * DEFAULT_MAX_POST_BYTES;
static DEFAULT_MAX_TOTAL_RECORDS: u32 = 100 * DEFAULT_MAX_POST_RECORDS;
static PREFIX: &str = "sync";

#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub port: u16,
    pub host: String,
    pub database_url: String,
    pub database_pool_max_size: Option<u32>,
    #[cfg(test)]
    pub database_use_test_transactions: bool,

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
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            debug: false,
            port: DEFAULT_PORT,
            host: "127.0.0.1".to_string(),
            database_url: "mysql://root@127.0.0.1/syncstorage".to_string(),
            database_pool_max_size: None,
            #[cfg(test)]
            database_use_test_transactions: false,
            limits: ServerLimits::default(),
            master_secret: Secrets::default(),
            statsd_host: None,
            statsd_port: 8125,
            statsd_label: "syncstorage".to_string(),
            human_logs: false,
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
        s.set_default("database_use_test_transactions", false)?;
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
        s.set_default("statsd_host", "localhost")?;
        s.set_default("statsd_port", 8125)?;
        s.set_default("statsd_label", "syncstorage")?;

        // Merge the config file if supplied
        if let Some(config_filename) = filename {
            s.merge(File::with_name(config_filename))?;
        }

        // Merge the environment overrides
        s.merge(Environment::with_prefix(PREFIX))?;

        Ok(match s.try_into::<Self>() {
            Ok(s) => {
                // Adjust the max values if required.
                if s.uses_spanner() {
                    let mut ms = s;
                    ms.limits.max_total_bytes =
                        min(ms.limits.max_total_bytes, MAX_SPANNER_LOAD_SIZE as u32);
                    return Ok(ms);
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
        self.database_url.as_str().starts_with("spanner")
    }

    /// A simple banner for display of certain settings at startup
    pub fn banner(&self) -> String {
        let db = Url::parse(&self.database_url)
            .map(|url| url.scheme().to_owned())
            .unwrap_or_else(|_| "<invalid db>".to_owned());
        format!("http://{}:{} ({})", self.host, self.port, db)
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
