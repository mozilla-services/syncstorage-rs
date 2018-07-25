use chrono::offset::Utc;
use diesel::sql_types::Bigint;

/// Get the time since the UNIX epoch in milliseconds
pub fn ms_since_epoch() -> i64 {
    Utc::now().timestamp_millis()
}

no_arg_sql_function!(last_insert_rowid, Bigint);
