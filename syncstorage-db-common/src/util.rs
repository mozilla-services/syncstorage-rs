use std::{convert::TryInto, u64};

use chrono::{
    offset::{FixedOffset, TimeZone, Utc},
    DateTime, SecondsFormat,
};
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    sql_types::BigInt,
    FromSqlRow,
};
use serde::{ser, Deserialize, Deserializer, Serialize, Serializer};

use super::error::CommonDbError;

/// Get the time since the UNIX epoch in milliseconds
fn ms_since_epoch() -> i64 {
    Utc::now().timestamp_millis()
}

/// Sync Timestamp
///
/// Internally represents a Sync timestamp as a u64 representing milliseconds since the epoch.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Deserialize, Serialize, FromSqlRow)]
pub struct SyncTimestamp(
    #[serde(deserialize_with = "deserialize_ts", serialize_with = "serialize_ts")] u64,
);

impl SyncTimestamp {
    /// Create a string value compatible with existing Sync Timestamp headers
    ///
    /// Represents the timestamp as second since epoch with two decimal places of precision.
    pub fn as_header(self) -> String {
        format_ts(self.0)
    }

    /// Create a `SyncTimestamp` from a string header
    ///
    /// Assumes the string represents the seconds since epoch with two decimal places of precision.
    pub fn from_header(val: &str) -> Result<Self, &'static str> {
        val.parse::<f64>()
            .map_err(|_| "Invalid value")
            .and_then(|v| {
                if v < 0f64 || v > ((u64::MAX / 1_000u64) as f64) || v.is_nan() {
                    Err("Invalid value")
                } else {
                    Ok(v)
                }
            })
            .map(|v: f64| (v * 1_000f64) as u64)
            .map(SyncTimestamp::from_milliseconds)
    }

    /// Create a `SyncTimestamp` from an i64
    pub fn from_i64(val: i64) -> Result<Self, CommonDbError> {
        if val < 0 {
            return Err(CommonDbError::internal(
                "Invalid modified i64 (< 0)".to_owned(),
            ));
        }
        Ok(SyncTimestamp::from_milliseconds(val as u64))
    }

    /// Exposed separately for db tests
    pub fn _from_i64(val: i64) -> Result<Self, CommonDbError> {
        SyncTimestamp::from_i64(val)
    }

    /// Create a `SyncTimestamp` from the milliseconds since epoch
    pub fn from_milliseconds(val: u64) -> Self {
        SyncTimestamp(val - (val % 10))
    }

    /// Create a `SyncTimestamp` from seconds since epoch
    pub fn from_seconds(val: f64) -> Self {
        let val = (val * 1000f64) as u64;
        SyncTimestamp(val - (val % 10))
    }

    /// Create a `SyncTimestamp` from an RFC 3339 and ISO 8601 date and time
    /// string such as 1996-12-19T16:39:57-08:00
    pub fn from_rfc3339(val: &str) -> Result<Self, CommonDbError> {
        let dt = DateTime::parse_from_rfc3339(val)
            .map_err(|e| CommonDbError::internal(format!("Invalid TIMESTAMP {}", e)))?;
        Self::from_datetime(dt)
    }

    /// Create a `SyncTimestamp` from a chrono DateTime
    fn from_datetime(val: DateTime<FixedOffset>) -> Result<Self, CommonDbError> {
        let millis = val.timestamp_millis();
        if millis < 0 {
            return Err(CommonDbError::internal("Invalid DateTime (< 0)".to_owned()));
        }
        Ok(SyncTimestamp::from_milliseconds(millis as u64))
    }

    /// Return the timestamp as an i64 milliseconds since epoch
    pub fn as_i64(self) -> i64 {
        self.0 as i64
    }

    /// Return the timestamp as an f64 seconds since epoch
    pub fn as_seconds(self) -> f64 {
        self.0 as f64 / 1000.0
    }

    /// Return the timestamp as an RFC 3339 and ISO 8601 date and time string such as
    /// 1996-12-19T16:39:57-08:00
    pub fn as_rfc3339(self) -> Result<String, CommonDbError> {
        to_rfc3339(self.as_i64())
    }
}

impl Default for SyncTimestamp {
    fn default() -> Self {
        SyncTimestamp::from_milliseconds(ms_since_epoch() as u64)
    }
}

impl From<SyncTimestamp> for i64 {
    fn from(val: SyncTimestamp) -> i64 {
        val.0 as i64
    }
}

impl From<SyncTimestamp> for u64 {
    fn from(val: SyncTimestamp) -> u64 {
        val.0
    }
}

impl<DB> FromSql<BigInt, DB> for SyncTimestamp
where
    i64: FromSql<BigInt, DB>,
    DB: Backend,
{
    fn from_sql(value: Option<&<DB as Backend>::RawValue>) -> deserialize::Result<Self> {
        let i64_value = <i64 as FromSql<BigInt, DB>>::from_sql(value)?;
        SyncTimestamp::from_i64(i64_value)
            .map_err(|e| format!("Invalid SyncTimestamp i64 {}", e).into())
    }
}

/// Format a timestamp as second since epoch with two decimal places of precision.
fn format_ts(val: u64) -> String {
    format!("{:.*}", 2, val as f64 / 1000.0)
}

fn deserialize_ts<'de, D>(d: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    Deserialize::deserialize(d).map(|result: f64| (result * 1_000f64) as u64)
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn serialize_ts<S>(x: &u64, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // Using serde_json::Number w/ the arbitrary_precision feature enabled to
    // persist the two decimal places of precision (vs serialize_f64 which
    // renders e.g. 0.00 as 0.0)
    let precise: serde_json::Number =
        serde_json::from_str(&format_ts(*x)).map_err(ser::Error::custom)?;
    precise.serialize(s)
}

/// Render a timestamp (as an i64 milliseconds since epoch) as an RFC 3339 and ISO 8601
/// date and time string such as 1996-12-19T16:39:57-08:00
pub fn to_rfc3339(val: i64) -> Result<String, CommonDbError> {
    let secs = val / 1000;
    let nsecs = ((val % 1000) * 1_000_000).try_into().map_err(|e| {
        CommonDbError::internal(format!("Invalid timestamp (nanoseconds) {}: {}", val, e))
    })?;
    Ok(Utc
        .timestamp(secs, nsecs)
        .to_rfc3339_opts(SecondsFormat::Nanos, true))
}

#[macro_export]
macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result, Self::Error> {
            let db = self.clone();
            Box::pin(util::run_on_blocking_threadpool(
                move || db.$sync_name(params),
                Self::Error::internal,
            ))
        }
    };
}
