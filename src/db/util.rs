use chrono::offset::Utc;

/// Get the time since the UNIX epoch in milliseconds
pub fn ms_since_epoch() -> i64 {
    Utc::now().timestamp_millis()
}
