#[cfg(target_os = "linux")]
use std::fs;

use std::{
    io,
    sync::{Arc, Mutex, PoisonError, mpsc},
    thread,
    time::Instant,
};

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

use crate::error::ApiResult;

use slog::{self, Drain};
use slog_mozlog_json::MozLogJson;

/// Writes last-seen upload failures and shutdown drops synchronously to the log output.
/// Set by `init_logging` and used only by the dedicated row-log thread because writes may block.
static ROW_LOGGER: Mutex<Option<slog::Logger>> = Mutex::new(None);

type OutputDrain = Box<dyn Drain<Ok = (), Err = slog::Never> + Send>;

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
    let output: OutputDrain = if json {
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
        Box::new(slog_envlogger::new(drain).fuse())
    } else if connected_to_journal() {
        #[cfg(target_os = "linux")]
        let drain = slog_journald::JournaldDrain.fuse();
        #[cfg(not(target_os = "linux"))]
        let drain = slog::Discard;
        Box::new(slog_envlogger::new(drain).fuse())
    } else {
        let decorator = slog_term::TermDecorator::new().build();
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        Box::new(slog_envlogger::new(drain).fuse())
    };
    // Both loggers write through one locked output, so their records never interleave.
    let output = Arc::new(Mutex::new(output).fuse());
    let logger = slog::Logger::root(
        slog_async::Async::new(Arc::clone(&output)).build().fuse(),
        slog::o!(),
    );
    *ROW_LOGGER.lock().unwrap_or_else(PoisonError::into_inner) =
        Some(slog::Logger::root(output, slog::o!()));
    // XXX: cancel slog_scope's NoGlobalLoggerSet for now, it's difficult to
    // prevent it from potentially panicing during tests. reset_logging resets
    // the global logger during shutdown anyway:
    // https://github.com/slog-rs/slog/issues/169
    slog_scope::set_global_logger(logger).cancel_reset();
    slog_stdlog::init().ok();
    Ok(())
}

/// Returns the per-row logger, falling back to the current scoped or global logger before
/// `init_logging`.
pub fn row_logger() -> slog::Logger {
    ROW_LOGGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .unwrap_or_else(slog_scope::logger)
}

/// Replaces the global logger with one that discards records. Dropping the old logger flushes
/// its queue and joins its thread. With `limit`, cleanup runs on a separate thread and the caller
/// waits only until the deadline, so stalled log output cannot block shutdown indefinitely.
pub fn reset_logging(limit: Option<Instant>) {
    let old = slog_scope::logger();
    let logger = slog::Logger::root(slog::Discard, slog::o!());
    slog_scope::set_global_logger(logger).cancel_reset();
    if let Some(limit) = limit {
        drop_until(old, limit, |cleanup| {
            thread::Builder::new()
                .name("log-reset".to_owned())
                .spawn(cleanup)
                .map(drop)
        });
    }
}

/// Drops `old` on a separate thread, waiting at most until `limit`. Transfer the logger only
/// after `spawn` succeeds: on failure, Rust drops the closure on the calling thread. If spawning
/// fails, keep the logger alive until process exit to avoid blocking cleanup.
fn drop_until(
    old: slog::Logger,
    limit: Instant,
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
) {
    let (hand_over, take) = mpsc::channel::<slog::Logger>();
    let (dropped, wait) = mpsc::channel();
    let started = spawn(Box::new(move || {
        drop(take.recv());
        let _ = dropped.send(());
    }));
    match started {
        Ok(()) => {
            let _ = hand_over.send(old);
            let _ = wait.recv_timeout(limit.saturating_duration_since(Instant::now()));
        }
        Err(_) => std::mem::forget(old),
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::mpsc, time::Duration};

    use super::*;

    /// Stalls every record until released, like a stuck log output.
    struct StalledOutput {
        entered: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl Drain for StalledOutput {
        type Ok = ();
        type Err = slog::Never;
        fn log(&self, _: &slog::Record<'_>, _: &slog::OwnedKVList) -> Result<(), slog::Never> {
            let _ = self.entered.send(());
            let _ = self.release.lock().unwrap().recv();
            Ok(())
        }
    }

    /// An async logger whose worker thread is stalled for 2 s, so dropping it blocks.
    fn stalled_logger() -> slog::Logger {
        let (entered, stalled) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let output = StalledOutput {
            entered,
            release: Mutex::new(wait),
        };
        let logger = slog::Logger::root(slog_async::Async::new(output).build().fuse(), slog::o!());
        slog::info!(logger, "stall the output");
        stalled.recv_timeout(Duration::from_secs(5)).unwrap();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(2));
            drop(release);
        });
        logger
    }

    #[test]
    fn test_drop_until_ends_by_the_limit() {
        let logger = stalled_logger();
        let limit = Instant::now() + Duration::from_millis(100);
        drop_until(logger, limit, |cleanup| {
            thread::Builder::new().spawn(cleanup).map(drop)
        });
        let late = Instant::now().saturating_duration_since(limit);
        assert!(
            late < Duration::from_millis(250),
            "ended {late:?} after the limit"
        );
    }

    #[test]
    fn test_drop_until_keeps_the_logger_if_no_thread_starts() {
        let logger = stalled_logger();
        let limit = Instant::now() + Duration::from_millis(100);
        drop_until(logger, limit, |_cleanup| {
            Err(io::Error::from(io::ErrorKind::WouldBlock))
        });
        let late = Instant::now().saturating_duration_since(limit);
        assert!(
            late < Duration::from_millis(250),
            "ended {late:?} after the limit"
        );
    }
}
