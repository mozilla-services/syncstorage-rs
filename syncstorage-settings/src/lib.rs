//! Application settings objects and initialization

use std::cmp::min;

use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use syncserver_common::{self, MAX_SPANNER_LOAD_SIZE};

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
#[serde(default)]
pub struct Settings {
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
    #[cfg(debug_assertions)]
    pub database_use_test_transactions: bool,

    /// Server-enforced limits for request payloads.
    pub limits: ServerLimits,

    pub statsd_label: String,

    pub enable_quota: bool,
    pub enforce_quota: bool,

    pub spanner_emulator_host: Option<String>,
    pub enabled: bool,

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
            database_url: "mysql://root@127.0.0.1/syncstorage".to_string(),
            database_pool_max_size: 10,
            database_pool_min_idle: None,
            database_pool_connection_lifespan: None,
            database_pool_connection_max_idle: None,
            database_pool_connection_timeout: Some(30),
            #[cfg(debug_assertions)]
            database_use_test_transactions: false,
            limits: ServerLimits::default(),
            statsd_label: "syncstorage".to_string(),
            enable_quota: false,
            enforce_quota: false,
            spanner_emulator_host: None,
            enabled: true,
            lbheartbeat_ttl: None,
            lbheartbeat_ttl_jitter: 25,
        }
    }
}

impl Settings {
    pub fn normalize(&mut self) {
        // Adjust the max values if required.
        if self.uses_spanner() {
            self.limits.max_total_bytes =
                min(self.limits.max_total_bytes, MAX_SPANNER_LOAD_SIZE as u32);
        } else {
            // No quotas for stand alone servers
            self.limits.max_quota_limit = 0;
            self.enable_quota = false;
            self.enforce_quota = false;
        }
    }

    pub fn spanner_database_name(&self) -> Option<&str> {
        if !self.uses_spanner() {
            None
        } else {
            Some(&self.database_url["spanner://".len()..])
        }
    }

    pub fn uses_spanner(&self) -> bool {
        self.database_url.as_str().starts_with("spanner://")
    }
}

/// Server-enforced limits for request payloads.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
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
