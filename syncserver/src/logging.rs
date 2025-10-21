use std::io;

use crate::error::ApiResult;

use slog::{self, slog_o, Drain};
use slog_mozlog_json::MozLogJson;
use std::os::fd::AsFd;

fn connected_to_journal() -> bool {
    rustix::fs::fstat(std::io::stderr().as_fd())
        .map(|stat| format!("{}:{}", stat.st_dev, stat.st_ino))
        .ok()
        .and_then(|stderr| {
            std::env::var_os("JOURNAL_STREAM").map(|s| s.to_string_lossy() == stderr.as_str())
        })
        .unwrap_or(false)
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
            let drain = slog_journald::JournaldDrain.fuse();
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
