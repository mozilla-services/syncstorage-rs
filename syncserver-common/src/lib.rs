#[macro_use]
extern crate slog_scope;

mod metrics;

use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use actix_web::web;
use hkdf::Hkdf;
use sha2::Sha256;

pub use metrics::{metrics_from_opts, MetricError, Metrics};

// header statics must be lower case, numbers and symbols per the RFC spec. This reduces chance of error.
pub static X_LAST_MODIFIED: &str = "x-last-modified";
pub static X_WEAVE_TIMESTAMP: &str = "x-weave-timestamp";
pub static X_WEAVE_NEXT_OFFSET: &str = "x-weave-next-offset";
pub static X_WEAVE_RECORDS: &str = "x-weave-records";
pub static X_WEAVE_BYTES: &str = "x-weave-bytes";
pub static X_WEAVE_TOTAL_RECORDS: &str = "x-weave-total-records";
pub static X_WEAVE_TOTAL_BYTES: &str = "x-weave-total-bytes";
pub static X_VERIFY_CODE: &str = "x-verify-code";

// max load size in bytes
pub const MAX_SPANNER_LOAD_SIZE: usize = 100_000_000;

/// Helper function for [HKDF](https://tools.ietf.org/html/rfc5869) expansion to 32 bytes.
pub fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> Result<[u8; 32], String> {
    let mut result = [0u8; 32];
    let hkdf = Hkdf::<Sha256>::new(salt, key);
    hkdf.expand(info, &mut result)
        .map_err(|e| format!("HKDF Error: {:?}", e))?;
    Ok(result)
}

#[macro_export]
macro_rules! from_error {
    ($from:ty, $to:ty, $to_kind:expr) => {
        impl From<$from> for $to {
            fn from(inner: $from) -> $to {
                $to_kind(inner).into()
            }
        }
    };
}

#[macro_export]
macro_rules! impl_fmt_display {
    ($error:ty, $kind:ty) => {
        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.kind, formatter)
            }
        }
    };
}

pub trait ReportableError {
    fn error_backtrace(&self) -> String;
    fn is_sentry_event(&self) -> bool;
    fn metric_label(&self) -> Option<String>;
}

/// Types that implement this trait can represent internal errors.
pub trait InternalError {
    /// Constructs an internal error with the given error message.
    fn internal_error(message: String) -> Self;
}

/// A threadpool on which callers can spawn non-CPU-bound tasks that block their thread (this is
/// mostly useful for running I/O tasks). `BlockingThreadpool` intentionally does not implement
/// `Clone`: `Arc`s are not used internally, so a `BlockingThreadpool` should be instantiated once
/// and shared by passing around `Arc<BlockingThreadpool>`s.
#[derive(Debug, Default)]
pub struct BlockingThreadpool {
    spawned_tasks: AtomicU64,
}

impl BlockingThreadpool {
    /// Runs a function as a task on the blocking threadpool.
    ///
    /// WARNING: Spawning a blocking task through means other than calling this method will
    /// result in inaccurate threadpool metrics being reported. If you want to spawn a task on
    /// the blocking threadpool, you **must** use this function.
    pub async fn spawn<F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Result<T, E> + Send + 'static,
        T: Send + 'static,
        E: fmt::Debug + Send + InternalError + 'static,
    {
        self.spawned_tasks.fetch_add(1, Ordering::Relaxed);

        let result = web::block(f).await.unwrap_or_else(|_| {
            Err(E::internal_error(
                "Blocking threadpool operation canceled".to_owned(),
            ))
        });

        self.spawned_tasks.fetch_sub(1, Ordering::Relaxed);

        result
    }

    pub fn active_threads(&self) -> u64 {
        self.spawned_tasks.load(Ordering::Relaxed)
    }
}
