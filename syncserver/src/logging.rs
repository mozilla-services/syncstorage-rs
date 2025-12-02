#[cfg(target_os = "linux")]
use std::fs;

use std::io;

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

use crate::error::ApiResult;

use slog::{self, slog_o, Drain};
use slog_mozlog_json::MozLogJson;

#[cfg(target_os = "linux")]
fn connected_to_journal() -> bool {
    fs::metadata("/dev/stderr")
        .map(|meta| format!("{}:{}", meta.st_dev(), meta.st_ino()))
        .ok()
        .and_then(|stderr| std::env::var_os("JOURNAL_STREAM").map(|s| s == stderr.as_str()))
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn connected_to_journal() -> bool {
    false
}

pub fn init_logging(json: bool) -> ApiResult<()> {
    let logger = if json {
        let hostname = hostname::get()
            .expect("Couldn't get hostname")
            .into_string()
            .expect("Couldn't get hostname");

        let drain = MozLogJson::new(io::stdout())
            .logger_name(format!(
                "{}-{}",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            ))
            .msg_type(format!("{}:log", env!("CARGO_PKG_NAME")))
            .hostname(hostname)
            .build()
            .fuse();
        let drain = slog_envlogger::new(drain);
        let drain = slog_async::Async::new(drain).build().fuse();
        slog::Logger::root(drain, slog_o!())
    } else {
        let drain = if connected_to_journal() {
            #[cfg(target_os = "linux")]
            let drain = slog_journald::JournaldDrain.fuse();
            #[cfg(not(target_os = "linux"))]
            let drain = slog::Discard;
            let drain = slog_envlogger::new(drain);
            slog_async::Async::new(drain).build().fuse()
        } else {
            let decorator = slog_term::TermDecorator::new().build();
            let drain = slog_term::FullFormat::new(decorator).build().fuse();
            let drain = slog_envlogger::new(drain);
            slog_async::Async::new(drain).build().fuse()
        };
        slog::Logger::root(drain, slog_o!())
    };
    // XXX: cancel slog_scope's NoGlobalLoggerSet for now, it's difficult to
    // prevent it from potentially panicing during tests. reset_logging resets
    // the global logger during shutdown anyway:
    // https://github.com/slog-rs/slog/issues/169
    slog_scope::set_global_logger(logger).cancel_reset();
    slog_stdlog::init().ok();
    Ok(())
}

pub fn reset_logging() {
    let logger = slog::Logger::root(slog::Discard, slog_o!());
    slog_scope::set_global_logger(logger).cancel_reset();
}
