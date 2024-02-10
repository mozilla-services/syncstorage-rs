mod mozlog;

use crate::error::ApiResult;
use slog::{self, slog_o};
use std::{io::stdout, sync::Once};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt::MakeWriter, EnvFilter};

/// Initialize logging for the main process
///
/// This sets the global tracing subscriber and must only be called once at startup time.
/// Subsequent calls will panic.
///
/// Returns a `tracing_appender::WorkerGuard` that keeps the logging thread alive.  The caller must
/// ensure this this value is not dropped while the server is running.
pub fn init_logging(json: bool) -> ApiResult<WorkerGuard> {
    let (writer, guard) = tracing_appender::non_blocking(stdout());
    if json {
        tracing::subscriber::set_global_default(json_subscriber(writer))?;
    } else {
        tracing::subscriber::set_global_default(human_subscriber(writer))?;
    };
    init_slog_drain();
    Ok(guard)
}

/// Initialize logging for the tests
///
/// Returns a DefaultGuard that must be kept alive for the duration of the test
pub fn init_test_logging() -> tracing::subscriber::DefaultGuard {
    init_slog_drain();
    tracing::subscriber::set_default(human_subscriber(stdout))
}

fn json_subscriber<W>(writer: W) -> impl tracing::Subscriber + Send + Sync + 'static
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .event_format(mozlog::EventFormatter::new())
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(writer)
        .finish()
}

fn human_subscriber<W>(writer: W) -> impl tracing::Subscriber + Send + Sync + 'static
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    tracing_subscriber::fmt()
        .pretty()
        .with_ansi(true)
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(writer)
        .finish()
}

fn init_slog_drain() {
    static ONCE: Once = Once::new();

    ONCE.call_once(|| {
        let drain = tracing_slog::TracingSlogDrain;
        let logger = slog::Logger::root(drain, slog_o!());
        // XXX: cancel slog_scope's NoGlobalLoggerSet for now, it's difficult to
        // prevent it from potentially panicing during tests. reset_logging resets
        // the global logger during shutdown anyway:
        // https://github.com/slog-rs/slog/issues/169
        slog_scope::set_global_logger(logger).cancel_reset();
    });
}

pub fn reset_logging() {
    let logger = slog::Logger::root(slog::Discard, slog_o!());
    slog_scope::set_global_logger(logger).cancel_reset();
}
