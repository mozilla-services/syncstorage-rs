use std::time::SystemTime;

#[macro_use]
extern crate slog_scope;

#[macro_use]
mod macros;

mod batch;
mod error;
mod manager;
mod models;
mod pool;
mod support;

pub use error::DbError;
pub use models::SpannerDb;
pub use pool::SpannerDbPool;

type DbResult<T> = Result<T, error::DbError>;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
