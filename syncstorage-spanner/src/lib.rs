use chrono::Utc;

#[macro_use]
extern crate slog_scope;

#[macro_use]
mod macros;

mod db;
mod error;
mod manager;
mod metadata;
mod pool;

pub use db::SpannerDb;
pub use error::DbError;
pub use pool::SpannerDbPool;

type DbResult<T> = Result<T, error::DbError>;

/// Return a timestamp of the seconds since Epoch, repr as `i64`.
fn now() -> i64 {
    Utc::now().timestamp()
}
