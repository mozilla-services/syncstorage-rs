use std::time::SystemTime;

#[macro_use]
mod macros;

mod batch;
pub mod manager;
pub mod models;
pub mod pool;
mod support;

pub use self::pool::SpannerDbPool;

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
