//! The layer for last-seen row logs (`ROW_LOG_TARGET`), which the row-log thread writes
//! synchronously so none are dropped.

use std::{
    io::{self, IsTerminal},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use tracing_subscriber::{
    Layer,
    filter::{FilterExt, LevelFilter, Targets},
    fmt::MakeWriter,
    registry::LookupSpan,
};

use super::{OutputLayer, ROW_LOG_TARGET, human_layer, json_layer};

/// The write-failure flag of the row layer `init_logging` installed.
static ROW_WRITE_FAILED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Returns the flag the installed row layer sets when a write fails. Before `init_logging`, or
/// with journald, it is a new flag that is never set.
pub fn row_write_failures() -> Arc<AtomicBool> {
    ROW_WRITE_FAILED.get().cloned().unwrap_or_default()
}

/// A writer, or a `MakeWriter`, that sets `failed` when a write fails. The fmt layer ignores
/// write errors.
struct RecordFailures<W> {
    inner: W,
    failed: Arc<AtomicBool>,
}

impl<W> RecordFailures<W> {
    fn record<T>(&self, result: io::Result<T>) -> io::Result<T> {
        result.inspect_err(|_| self.failed.store(true, Ordering::Relaxed))
    }
}

impl<W: io::Write> io::Write for RecordFailures<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let result = self.inner.write(buf);
        self.record(result)
    }

    /// One call, so `io::Stdout` holds its lock for the whole line.
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        let result = self.inner.write_all(buf);
        self.record(result)
    }

    fn flush(&mut self) -> io::Result<()> {
        let result = self.inner.flush();
        self.record(result)
    }
}

impl<'a, M: MakeWriter<'a>> MakeWriter<'a> for RecordFailures<M> {
    type Writer = RecordFailures<M::Writer>;

    fn make_writer(&'a self) -> Self::Writer {
        RecordFailures {
            inner: self.inner.make_writer(),
            failed: Arc::clone(&self.failed),
        }
    }
}

/// Writes events to `writer` synchronously, MozLog JSON with a hostname, otherwise human.
/// Returns the flag it sets when a write fails, for `RowLog::spawn`.
pub(crate) fn row_layer<S, W>(
    hostname: Option<String>,
    writer: W,
) -> (Box<dyn Layer<S> + Send + Sync>, Arc<AtomicBool>)
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    let failed = Arc::<AtomicBool>::default();
    let writer = RecordFailures {
        inner: writer,
        failed: Arc::clone(&failed),
    };
    let layer = match hostname {
        Some(hostname) => json_layer(hostname, writer).boxed(),
        None => human_layer(writer, io::stderr().is_terminal()).boxed(),
    };
    (layer, failed)
}

/// Sends row logs only to `rows`, and everything else only to `output`. `Targets` (and its
/// `not()`) decide per callsite, so tracing caches the result.
fn split_rows(output: OutputLayer, rows: OutputLayer) -> OutputLayer {
    let row_target = Targets::new().with_target(ROW_LOG_TARGET, LevelFilter::TRACE);
    vec![
        output.with_filter(row_target.clone().not()).boxed(),
        rows.with_filter(row_target).boxed(),
    ]
    .boxed()
}

/// Adds the row layer next to `output`: JSON on stdout with a hostname, otherwise human on
/// stderr. journald already writes synchronously, so it keeps the row logs (with their
/// priority and fields), but its write failures can't be seen.
pub(super) fn with_row_layer(
    output: OutputLayer,
    hostname: Option<String>,
    journal: bool,
) -> OutputLayer {
    if journal {
        return output;
    }
    let (rows, failed) = match hostname {
        Some(hostname) => row_layer(Some(hostname), io::stdout),
        None => row_layer(None, io::stderr),
    };
    let _ = ROW_WRITE_FAILED.set(failed);
    split_rows(output, rows)
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::{EnvFilter, prelude::*};

    use super::*;
    use crate::logging::capture::Capture;

    #[test]
    fn test_row_logs_go_only_to_the_row_layer() {
        let output = Capture::default();
        let rows = Capture::default();
        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::new("error"))
            .with(split_rows(
                json_layer("host".to_owned(), output.clone()).boxed(),
                json_layer("host".to_owned(), rows.clone()).boxed(),
            ));
        tracing::subscriber::with_default(subscriber, || {
            tracing::error!("not a row");
            tracing::error!(target: ROW_LOG_TARGET, uid = "abc", "a row");
        });
        let msgs = |capture: &Capture| -> Vec<_> {
            capture
                .json()
                .iter()
                .map(|line| line["Fields"]["msg"].as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(msgs(&output), ["not a row"]);
        assert_eq!(msgs(&rows), ["a row"]);
    }

    #[test]
    fn test_row_layer_records_write_failures() {
        struct Broken;
        impl io::Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let (layer, ok) = row_layer(None, io::sink);
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || tracing::error!("written"));
        assert!(!ok.load(Ordering::Relaxed));

        let (layer, failed) = row_layer(Some("host".to_owned()), || Broken);
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || tracing::error!("lost"));
        assert!(failed.load(Ordering::Relaxed));
    }
}
