use std::io;

use crate::error::{ApiErrorKind, ApiResult};

use mozsvc_common::{aws::get_ec2_instance_id, get_hostname};
use slog::{self, slog_o, Drain};
use slog_async;
use slog_mozlog_json::MozLogJson;
use slog_scope;
use slog_stdlog;
use slog_term;

pub fn init_logging(json: bool) -> ApiResult<()> {
    let logger = if json {
        let hostname = get_ec2_instance_id()
            .map(&str::to_owned)
            .or_else(get_hostname)
            .ok_or_else(|| "Couldn't get_hostname")
            .map_err(|e| ApiErrorKind::Internal(e.to_owned()))?;

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
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_envlogger::new(drain);
        let drain = slog_async::Async::new(drain).build().fuse();
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
