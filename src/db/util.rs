#![allow(proc_macro_derive_resolution_fallback)]

use chrono::offset::Utc;
use diesel::{
    backend::Backend,
    deserialize::{self, FromSql},
    sql_types::BigInt,
};

use super::{DbError, DbErrorKind};

/// Get the time since the UNIX epoch in milliseconds
pub fn ms_since_epoch() -> i64 {
    Utc::now().timestamp_millis()
}

/// Sync Timestamp
///
/// Internally represents a Sync timestamp as a u64 representing milliseconds since the epoch.
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Deserialize, Serialize, FromSqlRow)]
pub struct SyncTimestamp(u64);

impl SyncTimestamp {
    /// Create a string value compatible with existing Sync Timestamp headers
    ///
    /// Represents the timestamp as second since epoch with two decimal places of precision.
    pub fn as_header(&self) -> String {
        format!("{:.*}", 2, &self.0)
    }

    /// Create a `SyncTimestamp` from a string header
    ///
    /// Assumes the string represents the seconds since epoch with two decimal places of precision.
    pub fn from_header(val: &str) -> Result<Self, &'static str> {
        val.parse::<f64>()
            .map_err(|_| "Invalid value")
            .and_then(|v| {
                if v < 0f64 {
                    Err("Invalid value")
                } else {
                    Ok(v)
                }
            }).map(|v: f64| (v * 1_000f64) as u64)
            .map(SyncTimestamp::from_milliseconds)
            .map_err(|_| "Invalid value")
    }

    /// Create a `SyncTimestamp` from an i64
    ///
    /// Only called from the db module
    pub(super) fn from_i64(val: i64) -> Result<Self, DbError> {
        if val < 0 {
            Err(DbErrorKind::Integrity(
                "Invalid modified i64 (< 0)".to_owned(),
            ))?;
        }
        Ok(SyncTimestamp::from_milliseconds(val as u64))
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

    /// Return the timestamp as an i64
    pub fn as_i64(&self) -> i64 {
        self.0 as i64
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
