#[cfg(target_os = "linux")]
use std::fs;

use std::{
    io::{self, IsTerminal},
    sync::{Arc, Mutex, Once, OnceLock, PoisonError, mpsc},
    thread,
    time::Instant,
};

#[cfg(target_os = "linux")]
use std::os::linux::fs::MetadataExt;

use crate::error::{ApiError, ApiResult};

use slog::{self, Drain};
use slog_mozlog_json::MozLogJson;
use tracing_appender::non_blocking::{ErrorCounter, NonBlocking, NonBlockingBuilder, WorkerGuard};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, filter::LevelFilter, fmt::MakeWriter, layer::Layered, prelude::*,
    registry::LookupSpan,
};

#[cfg(test)]
pub(crate) mod capture;
mod mozlog;

use mozlog::{HumanFields, MozLogFormatter};

/// Writes last-seen upload failures and shutdown drops synchronously (avoiding slog_async's
/// buffering to a bounded queue that drops on overflow) to the log output Set by `init_logging`
/// and used only by the dedicated row-log thread because writes may block.
static ROW_LOGGER: Mutex<Option<slog::Logger>> = Mutex::new(None);

/// Keeps the tracing writer thread running until `reset_logging` drops it.
static TRACING_GUARD: Mutex<Option<WorkerGuard>> = Mutex::new(None);

/// Lines the tracing writer dropped because its queue was full.
static DROPPED_LINES: OnceLock<ErrorCounter> = OnceLock::new();

type OutputDrain = Box<dyn Drain<Ok = (), Err = slog::Never> + Send>;
type OutputLayer = Box<dyn Layer<Layered<EnvFilter, Registry>> + Send + Sync>;

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

pub(crate) fn hostname() -> String {
    hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname")
}

/// The `RUST_LOG` filter, matching slog_envlogger: errors only when unset or empty.
fn env_filter() -> EnvFilter {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_default();
    EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .parse_lossy(directives(&rust_log))
}

/// The directives of a `RUST_LOG` value. slog_envlogger allowed a `/regex` message filter after
/// them, which tracing would read as part of a target and then match nothing, so it is dropped.
/// Like slog_envlogger, a value with more than one `/` is ignored.
fn directives(rust_log: &str) -> &str {
    let mut parts = rust_log.split('/');
    let directives = parts.next().unwrap_or_default();
    match (parts.next(), parts.next()) {
        (_, Some(_)) => "",
        _ => directives,
    }
}

/// A writer thread with a bounded queue that drops lines when full, like slog_async, so a
/// stalled output never blocks the caller.
fn non_blocking(out: impl io::Write + Send + 'static) -> (NonBlocking, WorkerGuard) {
    NonBlockingBuilder::default()
        .lossy(true)
        .thread_name("log-writer")
        .finish(out)
}

fn json_layer<S, W>(hostname: String, writer: W) -> impl Layer<S> + Send + Sync
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    tracing_subscriber::fmt::layer()
        .event_format(MozLogFormatter::new(hostname))
        .with_writer(writer)
}

fn human_layer<S, W>(writer: W, ansi: bool) -> impl Layer<S> + Send + Sync
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    tracing_subscriber::fmt::layer()
        .fmt_fields(HumanFields)
        .with_target(false)
        .with_ansi(ansi)
        .with_writer(writer)
}

/// A non-blocking writer's guard and its count of dropped lines.
type Writer = (WorkerGuard, ErrorCounter);

/// journald when stderr is connected to it, like slog_journald, otherwise human-readable lines
/// on stderr, where slog_term writes.
fn journal_or_stderr() -> (OutputLayer, Option<Writer>) {
    #[cfg(target_os = "linux")]
    if connected_to_journal()
        && let Ok(layer) = tracing_journald::layer()
    {
        return (layer.with_field_prefix(None).boxed(), None);
    }
    let (writer, guard) = non_blocking(io::stderr());
    let dropped = writer.error_counter();
    let layer = human_layer(writer, io::stderr().is_terminal()).boxed();
    (layer, Some((guard, dropped)))
}

/// Returns how many lines the tracing writer has dropped since it started.
pub fn dropped_lines() -> u64 {
    DROPPED_LINES
        .get()
        .map_or(0, |dropped| dropped.dropped_lines() as u64)
}

/// Installs the global tracing subscriber, with the same outputs and `RUST_LOG` handling as the
/// slog drains. Only the first call does anything (tests call `init_logging` repeatedly).
fn init_tracing(json: bool) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let (output, writer) = if json {
            let (writer, guard) = non_blocking(io::stdout());
            let dropped = writer.error_counter();
            (
                json_layer(hostname(), writer).boxed(),
                Some((guard, dropped)),
            )
        } else {
            journal_or_stderr()
        };
        let subscriber = tracing_subscriber::registry()
            .with(env_filter())
            .with(output);
        if tracing::subscriber::set_global_default(subscriber).is_ok()
            && let Some((guard, dropped)) = writer
        {
            *TRACING_GUARD.lock().unwrap_or_else(PoisonError::into_inner) = Some(guard);
            let _ = DROPPED_LINES.set(dropped);
        }
    });
}

pub fn init_logging(json: bool) -> ApiResult<()> {
    init_tracing(json);
    let output: OutputDrain = if json {
        let hostname = hostname();

        let drain = MozLogJson::new(WholeLines::new(io::stdout()))
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
        let drain = slog_journald::JournaldDrain::default().fuse();
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

/// Writes each complete line to `inner` in one `write_all`, so lines from another writer on the
/// same stream (`io::Stdout` locks per call) never land in the middle of one. MozLogJson writes a
/// record and its newline in two calls.
struct WholeLines<W: io::Write> {
    inner: W,
    buf: Vec<u8>,
}

impl<W: io::Write> WholeLines<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            buf: Vec::new(),
        }
    }
}

impl<W: io::Write> io::Write for WholeLines<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(data);
        if self.buf.ends_with(b"\n") {
            let written = self.inner.write_all(&self.buf);
            self.buf.clear();
            written?;
        }
        Ok(data.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.buf.is_empty() {
            let written = self.inner.write_all(&self.buf);
            self.buf.clear();
            written?;
        }
        self.inner.flush()
    }
}

/// Returns the per-row logger, or an error if `init_logging` has not been called.
pub fn row_logger() -> ApiResult<slog::Logger> {
    ROW_LOGGER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
        .ok_or_else(|| ApiError::internal("Row logger requires init_logging"))
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
    use capture::Capture;

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

    /// Records each call to `write`.
    #[derive(Default)]
    struct Calls(Vec<Vec<u8>>);

    impl io::Write for Calls {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_whole_lines_writes_each_record_at_once() {
        use std::io::Write;

        let mut out = WholeLines::new(Calls::default());
        out.write_all(br#"{"a":1}"#).unwrap();
        assert!(
            out.inner.0.is_empty(),
            "a partial line waits for its newline"
        );
        out.write_all(b"\n").unwrap();
        out.write_all(br#"{"b":2}"#).unwrap();
        out.write_all(b"\n").unwrap();
        assert_eq!(
            out.inner.0,
            [b"{\"a\":1}\n".to_vec(), b"{\"b\":2}\n".to_vec()]
        );
    }

    #[test]
    fn test_whole_lines_returns_write_errors() {
        use std::io::Write;

        struct Broken;
        impl io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut out = WholeLines::new(Broken);
        out.write_all(b"record").unwrap();
        let err = out.write_all(b"\n").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::BrokenPipe);
        assert!(out.buf.is_empty(), "a failed line is not written again");
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

    const HOST: &str = "test-host";

    fn json_subscriber(directives: &str, capture: &Capture) -> impl tracing::Subscriber {
        tracing_subscriber::registry()
            .with(EnvFilter::new(directives))
            .with(json_layer(HOST.to_owned(), capture.clone()))
    }

    /// The fixed part of every line before `Severity`.
    fn prefix() -> String {
        format!(
            r#"{{"Logger":"syncserver-{}","Type":"syncserver:log","Hostname":"{HOST}","Pid":{}"#,
            env!("CARGO_PKG_VERSION"),
            std::process::id()
        )
    }

    #[test]
    fn test_mozlog_golden_lines() {
        let capture = Capture::default();
        tracing::subscriber::with_default(json_subscriber("trace", &capture), || {
            tracing::info!("native {}", "event");
            tracing::warn!(
                uid = "abc",
                attempt = 2_u64,
                ok = true,
                error = %"timed out",
                "with fields"
            );
            // The shape of a last-seen row log.
            tracing::error!(
                uid = "abc",
                client_id = "client",
                last_seen = "2026-10-30T00:00:00+00:00",
                reason = "rejected",
                "Last-seen upload failed; dropping observation"
            );
        });

        let prefix = prefix();
        assert_eq!(
            capture.normalized(),
            [
                format!(
                    r#"{prefix},"Severity":6,"Timestamp":0,"Fields":{{"msg":"native event"}}}}"#
                ),
                format!(
                    r#"{prefix},"Severity":4,"Timestamp":0,"Fields":{{"msg":"with fields","uid":"abc","attempt":2,"ok":true,"error":"timed out"}}}}"#
                ),
                format!(
                    r#"{prefix},"Severity":3,"Timestamp":0,"Fields":{{"msg":"Last-seen upload failed; dropping observation","uid":"abc","client_id":"client","last_seen":"2026-10-30T00:00:00+00:00","reason":"rejected"}}}}"#
                ),
            ]
        );
        for line in capture.json() {
            assert!(line.get("EnvVersion").is_none());
        }
    }

    /// Writes the same records through slog-mozlog-json and as tracing events, and expects
    /// identical lines apart from the timestamp. slog serializes logger values first, then the
    /// record's pairs last to first, so the tracing fields are listed in that order.
    #[test]
    fn test_mozlog_matches_slog_mozlog_json() {
        let old = Capture::default();
        let drain = MozLogJson::new(old.clone())
            .logger_name(format!("syncserver-{}", env!("CARGO_PKG_VERSION")))
            .msg_type("syncserver:log".to_owned())
            .hostname(HOST.to_owned())
            .build();
        let logger = slog::Logger::root(Mutex::new(drain).fuse(), slog::o!());
        let child = logger.new(slog::o!("request" => "r1"));
        slog::info!(logger, "plain");
        slog::error!(child, "with {} pairs", 4; "uid" => "abc", "count" => 3_u64,
            "neg" => -1_i64, "ratio" => 0.5_f64, "flag" => false);
        slog::warn!(logger, "quoted \"text\"\n"; "error" => format_args!("{}: {}", "a", 1));
        slog::debug!(logger, "debug");
        slog::trace!(logger, "trace");

        let new = Capture::default();
        tracing::subscriber::with_default(json_subscriber("trace", &new), || {
            tracing::info!("plain");
            tracing::error!(
                request = "r1",
                flag = false,
                ratio = 0.5_f64,
                neg = -1_i64,
                count = 3_u64,
                uid = "abc",
                "with {} pairs",
                4
            );
            tracing::warn!(error = %format_args!("{}: {}", "a", 1), "quoted \"text\"\n");
            tracing::debug!("debug");
            tracing::trace!("trace");
        });

        assert_eq!(new.lines().len(), 5);
        assert_eq!(new.normalized(), old.normalized());
    }

    #[test]
    fn test_mozlog_severity() {
        let capture = Capture::default();
        tracing::subscriber::with_default(json_subscriber("trace", &capture), || {
            tracing::error!("e");
            tracing::warn!("w");
            tracing::info!("i");
            tracing::debug!("d");
            tracing::trace!("t");
        });
        let severities: Vec<_> = capture
            .json()
            .iter()
            .map(|line| line["Severity"].as_u64().unwrap())
            .collect();
        assert_eq!(severities, [3, 4, 6, 7, 7]);
    }

    #[test]
    fn test_mozlog_field_named_msg_keeps_the_message() {
        let capture = Capture::default();
        tracing::subscriber::with_default(json_subscriber("info", &capture), || {
            tracing::info!(msg = "a field", "the message");
        });
        let line = &capture.lines()[0];
        assert!(
            line.ends_with(r#""Fields":{"msg":"the message","msg_field":"a field"}}"#),
            "{line}"
        );
    }

    #[test]
    fn test_directives_drop_the_regex_filter() {
        assert_eq!(directives(""), "");
        assert_eq!(directives("info"), "info");
        assert_eq!(directives("warn,syncserver=debug"), "warn,syncserver=debug");
        assert_eq!(directives("info/foo.*bar"), "info");
        assert_eq!(directives("/foo"), "");
        assert_eq!(directives("info/a/b"), "", "too many '/'s");
    }

    #[test]
    fn test_env_filter_defaults_to_errors() {
        let hint = |value: Option<&str>| {
            temp_env::with_var("RUST_LOG", value, || env_filter().max_level_hint())
        };
        assert_eq!(hint(None), Some(LevelFilter::ERROR));
        assert_eq!(hint(Some("")), Some(LevelFilter::ERROR));
        assert_eq!(hint(Some("info")), Some(LevelFilter::INFO));
        assert_eq!(hint(Some("info/regex")), Some(LevelFilter::INFO));
        assert_eq!(hint(Some("info/a/b")), Some(LevelFilter::ERROR));
        assert_eq!(
            hint(Some("warn,syncserver::tokenserver=debug")),
            Some(LevelFilter::DEBUG)
        );
    }

    #[test]
    fn test_env_filter_with_regex_filter_still_logs() {
        let capture = Capture::default();
        let filter = temp_env::with_var("RUST_LOG", Some("info/regex"), env_filter);
        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(json_layer(HOST.to_owned(), capture.clone()));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!("kept");
            tracing::debug!("filtered");
        });
        assert_eq!(capture.lines().len(), 1);
    }

    #[test]
    fn test_human_fields() {
        let capture = Capture::default();
        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::new("info"))
            .with(human_layer(capture.clone(), false));
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(uid = "abc", attempt = 2_u64, "native event");
        });
        let lines = capture.lines();
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].ends_with(" INFO native event uid=abc attempt=2"),
            "{}",
            lines[0]
        );
    }
}
