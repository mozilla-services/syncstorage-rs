//! Buffers `(fxa_uid, client_id, timestamp)` from successful token exchanges for upload to a
//! BigQuery staging table.

use std::{
    collections::{HashMap, hash_map::Entry},
    future::Future,
    mem,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use cadence::{Gauged, StatsdClient};
use chrono::{DateTime, SecondsFormat, Utc};
use google_cloud_auth::credentials::{self, CacheableResource, Credentials};
use http::Extensions;
use reqwest::{StatusCode, Url, header::CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;
use tokio::{
    sync::{Notify, OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    task::JoinSet,
    time::{Instant, sleep_until, timeout_at},
};

use syncserver_common::Metrics;
use tokenserver_settings::parse_bigquery_table_id;

use crate::error::{ApiError, ApiErrorKind};

pub const FLUSH_THRESHOLD: usize = 500;
pub const BUFFER_CAPACITY: usize = 5_000;

const BIGQUERY_ENDPOINT: &str = "https://bigquery.googleapis.com";
const INSERT_DATA_SCOPE: &str = "https://www.googleapis.com/auth/bigquery.insertdata";
const MAX_ATTEMPTS: usize = 3;
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(10);
const RETRY_DELAY: Duration = Duration::from_secs(1);

/// Time since the previous flush, after which a nonempty buffer is due for flushing.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(60);
/// Actix's 30-second request drain plus one 10-second upload attempt.
pub const SHUTDOWN_BUDGET: Duration = Duration::from_secs(40);
/// Additional time after the shutdown deadline to finish writing row logs. The combined
/// 50-second budget fits within the pods' 60-second termination grace period.
pub const ROW_LOG_BUDGET: Duration = Duration::from_secs(10);
/// Maximum number of queued log batches. Senders wait asynchronously when the queue is full.
const ROW_LOG_QUEUE: usize = 4;

const METRIC_RECORDED: &str = "last_seen.recorded";
const METRIC_DROPPED: &str = "last_seen.dropped";
const METRIC_UPLOAD_ERROR: &str = "last_seen.upload.error";
const METRIC_FLUSH: &str = "last_seen.flush";
const METRIC_BUFFERED: &str = "last_seen.buffered";
const METRIC_IN_FLIGHT: &str = "last_seen.in_flight";

/// An `(fxa_uid, client_id)` pair.
pub type Key = (String, String);
/// Rows removed from the buffer for one flush.
pub type FlushBatch = HashMap<Key, DateTime<Utc>>;

#[derive(Default)]
struct LastSeenBuffer {
    map: HashMap<Key, DateTime<Utc>>,
    /// Limits Sentry reports to one per period at capacity; cleared when a flush empties the
    /// buffer.
    saturated: bool,
    /// Set by `begin_draining` once shutdown starts; every later record wakes the coordinator.
    draining: bool,
    /// Set by `close` at the end of shutdown; later records are logged and counted as dropped.
    closed: bool,
    /// Set by the first record after `close`, so only one Sentry event is sent.
    close_reported: bool,
}

pub struct Collector {
    buffer: Mutex<LastSeenBuffer>,
    pub wake: Notify,
    metrics: Arc<StatsdClient>,
}

impl Collector {
    pub fn new(metrics: Arc<StatsdClient>) -> Self {
        Self {
            buffer: Mutex::new(LastSeenBuffer::default()),
            wake: Notify::new(),
            metrics,
        }
    }

    /// Locks the buffer. A poisoned lock is recovered; a panic here would fail token exchanges.
    fn buffer(&self) -> MutexGuard<'_, LastSeenBuffer> {
        self.buffer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn record(&self, fxa_uid: &str, client_id: &str, seen: DateTime<Utc>) {
        let key = (fxa_uid.to_owned(), client_id.to_owned());
        let metrics = Metrics::from(&self.metrics);

        let (len, newly_saturated, draining) = {
            let mut buffer = self.buffer();

            if buffer.closed {
                let report = !mem::replace(&mut buffer.close_reported, true);

                // Release the buffer lock before logging and reporting the rejection.
                drop(buffer);

                if report {
                    sentry::capture_message(
                        "Last-seen record after close; dropping observations",
                        sentry::Level::Error,
                    );
                }
                error!(
                    "Last-seen collector closed; dropping observation";
                    "uid" => fxa_uid,
                    "client_id" => client_id,
                    "last_seen" => seen.to_rfc3339(),
                );
                metrics.incr_with_tag(METRIC_DROPPED, "reason", "shutdown");

                return;
            }

            let draining = buffer.draining;
            let len = buffer.map.len();
            match buffer.map.entry(key) {
                Entry::Occupied(mut entry) => {
                    if seen > *entry.get() {
                        entry.insert(seen);
                    }
                    (Some(len), false, draining)
                }
                Entry::Vacant(entry) if len < BUFFER_CAPACITY => {
                    entry.insert(seen);
                    (Some(len + 1), false, draining)
                }
                Entry::Vacant(_) => (None, !mem::replace(&mut buffer.saturated, true), draining),
            }
        };

        match len {
            Some(len) => {
                metrics.incr(METRIC_RECORDED);
                // While draining, every row is due for flushing and the coordinator has no
                // other signal that one arrived.
                if len >= FLUSH_THRESHOLD || draining {
                    self.wake.notify_one();
                }
            }
            None => {
                metrics.incr_with_tag(METRIC_DROPPED, "reason", "buffer_full");
                error!(
                    "Last-seen buffer full; dropping observation";
                    "uid" => fxa_uid,
                    "client_id" => client_id,
                    "last_seen" => seen.to_rfc3339(),
                );
                if newly_saturated {
                    sentry::with_scope(
                        |scope| scope.set_extra("capacity", BUFFER_CAPACITY.into()),
                        || {
                            sentry::capture_message(
                                "Last-seen buffer full; dropping observations",
                                sentry::Level::Error,
                            )
                        },
                    );
                }
            }
        }
    }

    /// Makes every later record wake the coordinator, which is otherwise only woken by a full
    /// buffer or the flush timer.
    pub fn begin_draining(&self) {
        self.buffer().draining = true;
    }

    pub fn take_all(&self) -> HashMap<Key, DateTime<Utc>> {
        let mut buffer = self.buffer();
        buffer.saturated = false;
        mem::take(&mut buffer.map)
    }

    pub fn close(&self) -> FlushBatch {
        let mut buffer = self.buffer();
        buffer.closed = true;
        mem::take(&mut buffer.map)
    }

    pub fn len(&self) -> usize {
        self.buffer().map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A row dropped by the uploader, with its failure reason.
pub type Dropped = (Key, DateTime<Utc>, &'static str);

enum AttemptError {
    Retryable(String),
    Permanent(String),
}

#[derive(Clone, Copy)]
enum RowFailure {
    /// Invalid row data that should not be retried.
    Invalid,
    /// A transient error, or a row `stopped` because another row was invalid.
    Retry,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InsertAllResponse {
    #[serde(default)]
    insert_errors: Vec<InsertError>,
}

#[derive(Deserialize)]
struct InsertError {
    index: usize,
    #[serde(default)]
    errors: Vec<ErrorProto>,
}

#[derive(Deserialize)]
struct ErrorProto {
    reason: Option<String>,
}

/// The body of a failed Google API request:
/// `{"error": {"message": ..., "errors": [{"reason": ...}]}}`.
#[derive(Deserialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: Option<String>,
    #[serde(default)]
    errors: Vec<ErrorProto>,
}

/// BigQuery rate and quota errors that make an HTTP 403 retryable.
const RETRYABLE_403_REASONS: [&str; 2] = ["rateLimitExceeded", "quotaExceeded"];

pub struct Uploader {
    http: reqwest::Client,
    credentials: Credentials,
    url: Url,
    retry_delay: Duration,
    row_log: RowLog,
    metrics: Arc<StatsdClient>,
}

impl Uploader {
    pub fn new(
        staging_table: &str,
        row_log: RowLog,
        metrics: Arc<StatsdClient>,
    ) -> Result<Self, ApiError> {
        let credentials = credentials::Builder::default()
            .with_scopes([INSERT_DATA_SCOPE])
            .build()
            .map_err(|e| internal(format!("Last-seen credentials error: {e}")))?;
        Self::with_endpoint(
            BIGQUERY_ENDPOINT,
            staging_table,
            credentials,
            RETRY_DELAY,
            row_log,
            metrics,
        )
    }

    fn with_endpoint(
        endpoint: &str,
        staging_table: &str,
        credentials: Credentials,
        retry_delay: Duration,
        row_log: RowLog,
        metrics: Arc<StatsdClient>,
    ) -> Result<Self, ApiError> {
        let (project, dataset, table) = parse_bigquery_table_id(staging_table)
            .ok_or_else(|| internal(format!("Invalid last-seen staging table: {staging_table}")))?;
        let url = Url::parse(&format!(
            "{endpoint}/bigquery/v2/projects/{project}/datasets/{dataset}/tables/{table}/insertAll"
        ))
        .map_err(|e| internal(format!("Invalid last-seen insertAll URL: {e}")))?;
        let http = reqwest::Client::builder()
            .timeout(ATTEMPT_TIMEOUT)
            .build()
            .map_err(|e| internal(format!("Last-seen HTTP client error: {e}")))?;

        Ok(Self {
            http,
            credentials,
            url,
            retry_delay,
            row_log,
            metrics,
        })
    }

    pub async fn upload(&self, rows: HashMap<Key, DateTime<Utc>>) -> Vec<Dropped> {
        self.upload_tracked(rows, None).await
    }

    /// Uploads rows while tracking pending rows and dropped rows awaiting log-queue space in
    /// `entry`, if provided. Shutdown uses these tracked rows to report an aborted upload.
    async fn upload_tracked(
        &self,
        rows: HashMap<Key, DateTime<Utc>>,
        entry: Option<&InFlightEntry>,
    ) -> Vec<Dropped> {
        let mut metrics = Metrics::from(&self.metrics);
        metrics.start_timer(METRIC_FLUSH, None);

        let mut pending: Vec<_> = rows.into_iter().collect();
        let mut dropped = Vec::new();
        let mut last_error = None;

        for attempt in 1..=MAX_ATTEMPTS {
            if attempt > 1 {
                tokio::time::sleep(self.retry_delay).await;
            }

            match self.insert_all(&pending).await {
                Ok(row_errors) => {
                    if !row_errors.is_empty() {
                        // Count once per failed attempt, regardless of how many rows failed.
                        metrics.incr(METRIC_UPLOAD_ERROR);
                        last_error = Some("insertAll row errors".to_owned());
                    }

                    let mut retry = Vec::new();
                    for (index, row) in pending.into_iter().enumerate() {
                        match row_errors.get(&index) {
                            None => {}
                            Some(RowFailure::Invalid) => {
                                dropped.push((row.0, row.1, "invalid row"))
                            }
                            // Includes rows `stopped` because another row was invalid.
                            Some(RowFailure::Retry) => retry.push(row),
                        }
                    }

                    pending = retry;
                    // stop tracking rows inserted by this `insert_all` by excluding them
                    if let Some(entry) = entry {
                        entry.update(
                            pending
                                .iter()
                                .cloned()
                                .chain(dropped.iter().map(|(key, seen, _)| (key.clone(), *seen))),
                        );
                    }
                    if pending.is_empty() {
                        break;
                    }
                }
                Err(AttemptError::Retryable(e)) => {
                    metrics.incr(METRIC_UPLOAD_ERROR);
                    warn!("Last-seen upload attempt failed"; "attempt" => attempt, "error" => &e);
                    last_error = Some(e);
                }
                Err(AttemptError::Permanent(e)) => {
                    metrics.incr(METRIC_UPLOAD_ERROR);
                    warn!("Last-seen upload rejected"; "error" => &e);
                    last_error = Some(e);
                    dropped.extend(pending.drain(..).map(|(key, seen)| (key, seen, "rejected")));
                    break;
                }
            }
        }

        // Drop and log any rows still pending after the last attempt. If no response arrived,
        // BigQuery may have inserted some of them.
        dropped.extend(
            pending
                .into_iter()
                .map(|(key, seen)| (key, seen, "retries exhausted")),
        );

        if !dropped.is_empty() {
            // Keep rows in `entry` while waiting for log-queue space so `abandon` can report
            // them if shutdown aborts this upload. If the writer has stopped, `send` reports
            // the unwritten logs; the dropped rows are still counted and reported below.
            self.row_log
                .send(RowLogBatch::UploadFailed(dropped.clone()))
                .await;
            if let Some(entry) = entry {
                entry.update(std::iter::empty());
            }
            metrics.count_with_tags(
                METRIC_DROPPED,
                dropped.len() as i64,
                HashMap::from([("reason".to_owned(), "upload_failed".to_owned())]),
            );
            report_dropped(
                "Last-seen upload dropped observations",
                dropped.len(),
                last_error.as_deref(),
            );
        }
        dropped
    }

    async fn insert_all(
        &self,
        rows: &[(Key, DateTime<Utc>)],
    ) -> Result<HashMap<usize, RowFailure>, AttemptError> {
        let headers = match self.credentials.headers(Extensions::new()).await {
            Ok(CacheableResource::New { data, .. }) => data,
            // No cache validator was supplied, so `NotModified` is unexpected.
            Ok(CacheableResource::NotModified) => {
                return Err(AttemptError::Retryable(
                    "no auth headers (unexpected NotModified)".to_owned(),
                ));
            }
            Err(e) => return Err(AttemptError::Retryable(format!("credentials: {e}"))),
        };
        let response = self
            .http
            .post(self.url.clone())
            .headers(headers)
            .header(CONTENT_TYPE, "application/json")
            .body(insert_all_body(rows))
            .send()
            .await
            .map_err(|e| AttemptError::Retryable(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.bytes().await.map_err(|e| {
                // Retry when an incomplete response body leaves the failure reason unknown.
                AttemptError::Retryable(format!("HTTP {status}, reading body: {e}"))
            })?;

            let detail = serde_json::from_slice::<ErrorResponse>(&body)
                .ok()
                .map(|body| body.error);
            let reason = detail
                .as_ref()
                .and_then(|detail| detail.errors.iter().find_map(|e| e.reason.clone()));
            let mut error = format!("HTTP {status}");
            if let Some(reason) = &reason {
                error.push_str(&format!(" {reason}"));
            }
            if let Some(message) = detail.and_then(|detail| detail.message) {
                error.push_str(&format!(": {message}"));
            }
            let retryable = status == StatusCode::REQUEST_TIMEOUT
                || status == StatusCode::TOO_MANY_REQUESTS
                || status.is_server_error()
                || (status == StatusCode::FORBIDDEN
                    && reason.is_some_and(|r| RETRYABLE_403_REASONS.contains(&r.as_str())));
            return Err(if retryable {
                AttemptError::Retryable(error)
            } else {
                AttemptError::Permanent(error)
            });
        }

        let body = response
            .bytes()
            .await
            .map_err(|e| AttemptError::Retryable(e.to_string()))?;
        let response: InsertAllResponse = serde_json::from_slice(&body)
            .map_err(|e| AttemptError::Retryable(format!("insertAll response: {e}")))?;

        Ok(response
            .insert_errors
            .into_iter()
            .map(|error| {
                let invalid = error
                    .errors
                    .iter()
                    .any(|e| e.reason.as_deref() == Some("invalid"));
                let failure = if invalid {
                    RowFailure::Invalid
                } else {
                    RowFailure::Retry
                };
                (error.index, failure)
            })
            .collect())
    }
}

fn insert_all_body(rows: &[(Key, DateTime<Utc>)]) -> Vec<u8> {
    let rows: Vec<_> = rows
        .iter()
        .map(|((uid, client_id), seen)| {
            json!({ "json": {
                "uid": uid,
                "client_id": client_id,
                "last_seen": seen.to_rfc3339_opts(SecondsFormat::Micros, true),
            }})
        })
        .collect();
    serde_json::to_vec(&json!({ "rows": rows })).expect("insertAll body is valid JSON")
}

/// Send one Sentry error event for a batch of dropped rows.
fn report_dropped(message: &str, dropped: usize, error: Option<&str>) {
    sentry::with_scope(
        |scope| {
            scope.set_extra("dropped", dropped.into());
            if let Some(error) = error {
                scope.set_extra("error", error.into());
            }
        },
        || sentry::capture_message(message, sentry::Level::Error),
    );
}

fn internal(message: String) -> ApiError {
    ApiErrorKind::Internal(message).into()
}

/// Required per-row error logs, queued as one batch.
enum RowLogBatch {
    UploadFailed(Vec<Dropped>),
    Abandoned(Vec<(Key, DateTime<Utc>)>),
    /// Acknowledged after all earlier batches have been written.
    Flush(oneshot::Sender<()>),
}

/// Writes per-row error logs on a dedicated thread. Senders wait asynchronously for queue
/// space, keeping the runtime responsive when log output stalls.
#[derive(Clone)]
pub struct RowLog {
    queue: mpsc::Sender<RowLogBatch>,
    /// Set by the first report of unwritten row logs, so only one Sentry event is sent.
    unwritten_reported: Arc<AtomicBool>,
}

impl RowLog {
    /// Starts the writer thread, which exits after all senders are dropped and the queue is drained.
    pub fn spawn(logger: slog::Logger) -> Result<Self, ApiError> {
        let (queue, mut batches) = mpsc::channel(ROW_LOG_QUEUE);
        std::thread::Builder::new()
            .name("last-seen-row-log".to_owned())
            .spawn(move || {
                while let Some(batch) = batches.blocking_recv() {
                    write_rows(&logger, batch);
                }
            })
            .map_err(|e| internal(format!("Last-seen row log thread: {e}")))?;
        Ok(Self {
            queue,
            unwritten_reported: Arc::default(),
        })
    }

    /// Queues `batch`, waiting asynchronously for space. Returns `false` and reports unwritten
    /// logs if the writer has stopped. Cancelling the wait leaves the batch unqueued.
    async fn send(&self, batch: RowLogBatch) -> bool {
        let queued = self.queue.send(batch).await.is_ok();
        if !queued {
            self.report_unwritten();
        }
        queued
    }

    /// Waits until every batch queued so far is written, or until `deadline`. Returns `false`
    /// and reports unwritten logs if the deadline passes or the writer stops.
    pub async fn flush_until(&self, deadline: Instant) -> bool {
        let written = timeout_at(deadline, async {
            let (done, flushed) = oneshot::channel();
            // A stopped writer drops `done` with the queue, so `flushed` fails.
            self.send(RowLogBatch::Flush(done)).await && flushed.await.is_ok()
        })
        .await
        .unwrap_or(false);
        if !written {
            self.report_unwritten();
        }
        written
    }

    /// Reports unwritten row logs to Sentry once.
    fn report_unwritten(&self) {
        if !self.unwritten_reported.swap(true, Ordering::Relaxed) {
            sentry::capture_message("Last-seen row logs not written", sentry::Level::Error);
        }
    }
}

/// Waits for the coordinator after the server stops, then drains row logs until `ROW_LOG_BUDGET`
/// after the shutdown deadline. Returns the same deadline for final logger
/// cleanup in `main`.
pub async fn finish_shutdown(
    shutdown: &Shutdown,
    coordinator: tokio::task::JoinHandle<usize>,
    row_log: &RowLog,
) -> std::time::Instant {
    shutdown.finish_requests();
    match coordinator.await {
        Ok(abandoned) => info!("Last-seen shutdown finished"; "abandoned" => abandoned),
        Err(e) => error!("Last-seen coordinator failed"; "error" => e.to_string()),
    }
    // `finish_requests` set the deadline.  `begin` returns it.
    let limit = shutdown.begin() + ROW_LOG_BUDGET;
    row_log.flush_until(limit).await;
    limit.into_std()
}

fn write_rows(log: &slog::Logger, batch: RowLogBatch) {
    match batch {
        RowLogBatch::UploadFailed(rows) => {
            for ((uid, client_id), seen, reason) in rows {
                slog::error!(
                    log,
                    "Last-seen upload failed; dropping observation";
                    "uid" => uid,
                    "client_id" => client_id,
                    "last_seen" => seen.to_rfc3339(),
                    "reason" => reason,
                );
            }
        }
        RowLogBatch::Abandoned(rows) => {
            for ((uid, client_id), seen) in rows {
                slog::error!(
                    log,
                    "Last-seen shutdown deadline reached; dropping observation";
                    "uid" => uid,
                    "client_id" => client_id,
                    "last_seen" => seen.to_rfc3339(),
                );
            }
        }
        RowLogBatch::Flush(done) => {
            let _ = done.send(());
        }
    }
}

/// Sends one flush. `Uploader` implements it; the coordinator tests use a fake.
pub trait Upload: Send + Sync + 'static {
    /// Uploads `rows`, keeping `entry` set to rows awaiting acknowledgment or drop reporting.
    fn upload(
        &self,
        rows: FlushBatch,
        entry: InFlightEntry,
    ) -> impl Future<Output = Vec<Dropped>> + Send;
}

impl Upload for Uploader {
    async fn upload(&self, rows: FlushBatch, entry: InFlightEntry) -> Vec<Dropped> {
        self.upload_tracked(rows, Some(&entry)).await
    }
}

pub fn flush_due(buffered: usize, since_last_flush: Duration, shutting_down: bool) -> bool {
    buffered > 0
        && (buffered >= FLUSH_THRESHOLD || since_last_flush >= FLUSH_INTERVAL || shutting_down)
}

#[derive(Default)]
pub struct InFlight {
    state: Mutex<InFlightState>,
}

#[derive(Default)]
struct InFlightState {
    next_id: u64,
    batches: HashMap<u64, FlushBatch>,
}

impl InFlight {
    fn state(&self) -> MutexGuard<'_, InFlightState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Registers an upload and copies its rows for shutdown accounting.
    pub fn register(&self, rows: &FlushBatch) -> u64 {
        let mut state = self.state();
        let id = state.next_id;
        state.next_id += 1;
        state.batches.insert(id, rows.clone());
        id
    }

    /// Removes a completed upload from the registry.
    pub fn finish(&self, id: u64) {
        self.state().batches.remove(&id);
    }

    /// Replaces the rows still tracked for a running upload.
    pub fn update(&self, id: u64, rows: FlushBatch) {
        if let Some(batch) = self.state().batches.get_mut(&id) {
            *batch = rows;
        }
    }

    /// Removes and returns all rows still tracked for shutdown.
    pub fn take_remaining(&self) -> Vec<(Key, DateTime<Utc>)> {
        mem::take(&mut self.state().batches)
            .into_values()
            .flatten()
            .collect()
    }

    pub fn len(&self) -> usize {
        self.state().batches.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One running upload's entry in `InFlight`, handed to the uploader.
pub struct InFlightEntry {
    in_flight: Arc<InFlight>,
    id: u64,
}

impl InFlightEntry {
    /// Updates the rows still awaiting acknowledgment or drop reporting.
    fn update(&self, rows: impl Iterator<Item = (Key, DateTime<Utc>)>) {
        self.in_flight.update(self.id, rows.collect());
    }
}

/// What the coordinator knows about shutdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShutdownState {
    /// Fixed by the first signal or `finish_requests`; later calls do not extend it.
    pub deadline: Option<Instant>,
    /// The HTTP server has stopped: no more exchanges can record.
    pub requests_finished: bool,
}

pub struct Shutdown {
    state: watch::Sender<ShutdownState>,
}

impl Default for Shutdown {
    fn default() -> Self {
        Self {
            state: watch::Sender::new(ShutdownState::default()),
        }
    }
}

impl Shutdown {
    /// Starts the shutdown budget on the first call and returns the fixed deadline.
    pub fn begin(&self) -> Instant {
        self.state.send_if_modified(|s| {
            if s.deadline.is_none() {
                s.deadline = Some(Instant::now() + SHUTDOWN_BUDGET);
                true
            } else {
                false
            }
        });
        self.state().deadline.unwrap()
    }

    pub fn finish_requests(&self) {
        self.begin();
        self.state
            .send_if_modified(|s| !mem::replace(&mut s.requests_finished, true));
    }

    pub fn state(&self) -> ShutdownState {
        *self.state.borrow()
    }

    pub fn subscribe(&self) -> watch::Receiver<ShutdownState> {
        self.state.subscribe()
    }
}

pub struct Coordinator<U> {
    collector: Arc<Collector>,
    uploader: Arc<U>,
    row_log: RowLog,
    in_flight: Arc<InFlight>,
    slots: Arc<Semaphore>,
    uploads: JoinSet<()>,
    shutdown: Arc<Shutdown>,
    metrics: Arc<StatsdClient>,
}

impl<U: Upload> Coordinator<U> {
    pub fn new(
        collector: Arc<Collector>,
        uploader: Arc<U>,
        row_log: RowLog,
        max_concurrent_flushes: usize,
        shutdown: Arc<Shutdown>,
        metrics: Arc<StatsdClient>,
    ) -> Self {
        Self {
            collector,
            uploader,
            row_log,
            in_flight: Arc::new(InFlight::default()),
            slots: Arc::new(Semaphore::new(max_concurrent_flushes)),
            uploads: JoinSet::new(),
            shutdown,
            metrics,
        }
    }

    pub async fn run(mut self) -> usize {
        let mut last_flush = Instant::now();
        let mut shutdown = self.shutdown.subscribe();
        let mut draining = false;

        loop {
            let shutdown_state = *shutdown.borrow_and_update();

            while self.uploads.try_join_next().is_some() {}

            if let Some(deadline) = shutdown_state.deadline {
                let drained = shutdown_state.requests_finished
                    && self.collector.is_empty()
                    && self.uploads.is_empty();
                if drained || Instant::now() >= deadline {
                    return self.abandon(deadline).await;
                }
            }

            let shutting_down = shutdown_state.deadline.is_some();
            if shutting_down && !draining {
                // Rows recorded by requests still finishing are due immediately. Switched on
                // before `buffered` is read below, so a concurrent record either lands in this
                // pass's buffer or wakes the next one.
                self.collector.begin_draining();
                draining = true;
            }
            let buffered = self.collector.len();
            self.report_gauges(buffered);
            let mut waiting_for_slot = false;

            if flush_due(buffered, last_flush.elapsed(), shutting_down) {
                match Arc::clone(&self.slots).try_acquire_owned() {
                    Ok(permit) => {
                        let rows = self.collector.take_all();
                        self.spawn_upload(rows, permit);
                        last_flush = Instant::now();
                        continue;
                    }
                    Err(_) => waiting_for_slot = true,
                }
            } else if buffered == 0 && last_flush.elapsed() >= FLUSH_INTERVAL {
                // Reset the expired timer to avoid repeatedly waking with an empty buffer.
                last_flush = Instant::now();
            }

            let deadline = shutdown_state.deadline.unwrap_or_else(Instant::now);

            tokio::select! {
                _ = self.collector.wake.notified(), if !waiting_for_slot => {}
                _ = sleep_until(last_flush + FLUSH_INTERVAL), if !waiting_for_slot => {}
                _ = shutdown.changed() => {}
                Some(_) = self.uploads.join_next(), if !self.uploads.is_empty() => {}
                _ = sleep_until(deadline), if shutting_down => {}
            }
        }
    }

    /// Reports the buffered pairs and the running uploads. Called on every pass, after
    /// finished uploads are reaped.
    fn report_gauges(&self, buffered: usize) {
        let _ = self.metrics.gauge(METRIC_BUFFERED, buffered as u64);
        let _ = self
            .metrics
            .gauge(METRIC_IN_FLIGHT, self.uploads.len() as u64);
    }

    fn spawn_upload(&mut self, rows: FlushBatch, permit: OwnedSemaphorePermit) {
        let in_flight_id = self.in_flight.register(&rows);
        let uploader = Arc::clone(&self.uploader);
        let in_flight = Arc::clone(&self.in_flight);
        let entry = InFlightEntry {
            in_flight: Arc::clone(&self.in_flight),
            id: in_flight_id,
        };
        self.uploads.spawn(async move {
            uploader.upload(rows, entry).await;
            in_flight.finish(in_flight_id);
            drop(permit);
        });
    }

    /// Aborts running uploads and queues logs for their remaining rows and buffered rows.
    /// Waits for queue space until `deadline + ROW_LOG_BUDGET`; the caller uses
    /// `RowLog::flush_until` to wait for the writer.
    async fn abandon(&mut self, deadline: Instant) -> usize {
        self.uploads.abort_all();

        let mut abandoned_rows = self.in_flight.take_remaining();
        abandoned_rows.extend(self.collector.close());
        let abandoned = abandoned_rows.len();

        if abandoned > 0 {
            let metrics = Metrics::from(&self.metrics);
            metrics.count_with_tags(
                METRIC_DROPPED,
                abandoned as i64,
                HashMap::from([("reason".to_owned(), "shutdown".to_owned())]),
            );
            report_dropped("Last-seen shutdown dropped observations", abandoned, None);
            // A full queue past the budget leaves these rows unlogged; `finish_shutdown`'s
            // flush reports it, and `send` reports a stopped writer.
            let _ = timeout_at(
                deadline + ROW_LOG_BUDGET,
                self.row_log.send(RowLogBatch::Abandoned(abandoned_rows)),
            )
            .await;
        }

        abandoned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;
    use tokio::time::{sleep, timeout};

    fn collector() -> Collector {
        Collector::new(Arc::new(
            StatsdClient::builder("", cadence::NopMetricSink).build(),
        ))
    }

    #[test]
    fn test_record_keeps_newest_timestamp() {
        let collector = collector();
        let now = Utc::now();
        collector.record("uid", "client", now);
        collector.record("uid", "client", now - TimeDelta::seconds(5));
        collector.record("uid", "other", now);

        assert_eq!(
            collector.take_all(),
            HashMap::from([
                (("uid".to_owned(), "client".to_owned()), now),
                (("uid".to_owned(), "other".to_owned()), now),
            ])
        );
        assert!(collector.is_empty());
    }

    #[test]
    fn test_record_drops_new_pairs_at_capacity() {
        let collector = collector();
        let now = Utc::now();
        for i in 0..BUFFER_CAPACITY {
            collector.record(&format!("uid{i}"), "client", now);
        }
        collector.record("new", "client", now);
        assert_eq!(collector.len(), BUFFER_CAPACITY);

        // existing pairs still update at capacity
        let later = now + TimeDelta::seconds(1);
        collector.record("uid0", "client", later);
        let batch = collector.take_all();
        assert_eq!(batch.len(), BUFFER_CAPACITY);
        assert!(!batch.contains_key(&("new".to_owned(), "client".to_owned())));
        assert_eq!(batch[&("uid0".to_owned(), "client".to_owned())], later);
    }

    #[test]
    fn test_take_all_empties_buffer() {
        let collector = collector();
        let now = Utc::now();
        for i in 0..FLUSH_THRESHOLD + 1 {
            collector.record(&format!("uid{i}"), "client", now);
        }
        assert_eq!(collector.take_all().len(), FLUSH_THRESHOLD + 1);
        assert!(collector.is_empty());
        assert!(collector.take_all().is_empty());
    }

    #[tokio::test]
    async fn test_record_wakes_at_flush_threshold() {
        let collector = collector();
        let now = Utc::now();
        for i in 0..FLUSH_THRESHOLD {
            collector.record(&format!("uid{i}"), "client", now);
        }
        tokio::time::timeout(std::time::Duration::from_secs(1), collector.wake.notified())
            .await
            .expect("reaching FLUSH_THRESHOLD should wake the coordinator");
    }

    #[tokio::test]
    async fn test_record_wakes_every_time_while_draining() {
        let collector = collector();
        collector.record("early", "client", Utc::now());
        assert!(
            timeout(Duration::from_millis(50), collector.wake.notified())
                .await
                .is_err(),
            "a single row below the threshold must not wake the coordinator"
        );

        collector.begin_draining();
        for uid in ["late", "later"] {
            collector.record(uid, "client", Utc::now());
            timeout(Duration::from_secs(1), collector.wake.notified())
                .await
                .expect("each row recorded while draining wakes the coordinator");
        }
    }

    #[test]
    fn test_one_sentry_event_per_saturation() {
        let collector = collector();
        let now = Utc::now();
        let fill = |collector: &Collector| {
            for i in 0..BUFFER_CAPACITY {
                collector.record(&format!("uid{i}"), "client", now);
            }
        };
        let events = sentry::test::with_captured_events(|| {
            fill(&collector);
            for i in 0..3 {
                collector.record(&format!("new{i}"), "client", now);
            }
            // flush to reset
            collector.take_all();
            fill(&collector);
            collector.record("new", "client", now);
        });

        assert_eq!(events.len(), 2);
        for event in &events {
            assert_eq!(event.level, sentry::Level::Error);
            assert_eq!(
                event.message.as_deref(),
                Some("Last-seen buffer full; dropping observations")
            );
            assert_eq!(event.extra["capacity"], BUFFER_CAPACITY);
        }
    }

    struct CaptureDrain(Arc<Mutex<Vec<String>>>);

    impl slog::Drain for CaptureDrain {
        type Ok = ();
        type Err = slog::Never;

        fn log(
            &self,
            record: &slog::Record<'_>,
            _values: &slog::OwnedKVList,
        ) -> Result<(), slog::Never> {
            struct Fields(String);
            impl slog::Serializer for Fields {
                fn emit_arguments(
                    &mut self,
                    key: slog::Key,
                    val: &std::fmt::Arguments<'_>,
                ) -> slog::Result {
                    use std::fmt::Write;
                    write!(self.0, " {key}={val}").map_err(|_| slog::Error::Other)
                }
            }

            if record.level() != slog::Level::Error {
                return Ok(());
            }
            let mut fields = Fields(record.msg().to_string());
            let _ = slog::KV::serialize(&record.kv(), record, &mut fields);
            self.0.lock().unwrap().push(fields.0);
            Ok(())
        }
    }

    #[test]
    fn test_each_rejection_logs_original_observation() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logger = slog::Logger::root(CaptureDrain(logs.clone()), slog::o!());

        let now = Utc::now();
        let first = now - TimeDelta::seconds(10);
        let second = now - TimeDelta::seconds(5);

        slog_scope::scope(&logger, || {
            let collector = collector();
            for i in 0..BUFFER_CAPACITY {
                collector.record(&format!("uid{i}"), "client", now);
            }
            collector.record("new", "client", first);
            collector.record("new", "client", second);
            // existing pair at capacity updates
            collector.record("uid0", "client", now + TimeDelta::seconds(1));
        });

        let logs = logs.lock().unwrap();
        let expected: Vec<String> = [first, second]
            .iter()
            .map(|seen| {
                format!(
                    "Last-seen buffer full; dropping observation \
                     last_seen={} client_id=client uid=new",
                    seen.to_rfc3339()
                )
            })
            .collect();
        assert_eq!(*logs, expected);
    }

    const INSERT_ALL_PATH: &str = "/bigquery/v2/projects/my-project/datasets/account_last_seen/tables/last_seen_staging/insertAll";

    fn discard_row_log() -> RowLog {
        RowLog::spawn(slog::Logger::root(slog::Discard, slog::o!())).unwrap()
    }

    fn row_log_to(logs: &Arc<Mutex<Vec<String>>>) -> RowLog {
        RowLog::spawn(slog::Logger::root(
            CaptureDrain(Arc::clone(logs)),
            slog::o!(),
        ))
        .unwrap()
    }

    /// Waits, outside any runtime, until `row_log` has written everything queued so far.
    fn wait_written(row_log: &RowLog) {
        let written = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(row_log.flush_until(Instant::now() + Duration::from_secs(5)));
        assert!(written, "the row logs are written");
    }

    fn uploader(server: &httptest::Server) -> Uploader {
        uploader_with(server, discard_row_log())
    }

    fn uploader_with(server: &httptest::Server, row_log: RowLog) -> Uploader {
        Uploader::with_endpoint(
            server.url_str("").trim_end_matches('/'),
            "my-project.account_last_seen.last_seen_staging",
            credentials::anonymous::Builder::new().build(),
            Duration::ZERO,
            row_log,
            Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build()),
        )
        .unwrap()
    }

    fn rows(uids: &[&str], seen: DateTime<Utc>) -> HashMap<Key, DateTime<Utc>> {
        uids.iter()
            .map(|uid| ((uid.to_string(), "client".to_owned()), seen))
            .collect()
    }

    #[tokio::test]
    async fn test_upload_success() {
        use httptest::{Expectation, matchers::*, responders::*};
        let server = httptest::Server::run();
        let seen = "2026-10-09T12:00:00.123456Z".parse().unwrap();
        server.expect(
            Expectation::matching(all_of![
                request::method_path("POST", INSERT_ALL_PATH),
                request::body(matches(r#""uid":"uid0""#)),
                request::body(matches(r#""client_id":"client""#)),
                request::body(matches(r#""last_seen":"2026-10-09T12:00:00.123456Z""#)),
            ])
            .times(1)
            .respond_with(
                status_code(200).body(r#"{"kind":"bigquery#tableDataInsertAllResponse"}"#),
            ),
        );

        assert!(
            uploader(&server)
                .upload(rows(&["uid0"], seen))
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_upload_retries_stopped_rows_and_drops_invalid_rows() {
        use httptest::{Expectation, cycle, matchers::*, responders::*};
        let server = httptest::Server::run();
        // one row is invalid; BQ stops the other, which succeeds on the retry.
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(2)
                .respond_with(cycle![
                    status_code(200).body(
                        r#"{"insertErrors":[
                            {"index":0,"errors":[{"reason":"invalid"}]},
                            {"index":1,"errors":[{"reason":"stopped"}]}]}"#
                    ),
                    status_code(200).body("{}"),
                ]),
        );

        let dropped = uploader(&server)
            .upload(rows(&["a", "b"], Utc::now()))
            .await;
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].2, "invalid row");
    }

    #[test]
    fn test_upload_counts_row_errors_even_if_the_retry_succeeds() {
        use httptest::{Expectation, cycle, matchers::*, responders::*};
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(2)
                .respond_with(cycle![
                    status_code(200).body(
                        r#"{"insertErrors":[
                            {"index":0,"errors":[{"reason":"backendError"}]},
                            {"index":1,"errors":[{"reason":"backendError"}]}]}"#
                    ),
                    status_code(200).body("{}"),
                ]),
        );
        let (recorded, sink) = cadence::SpyMetricSink::new();
        let mut uploader = uploader(&server);
        uploader.metrics = Arc::new(StatsdClient::builder("", sink).build());

        let mut dropped = Vec::new();
        let events = sentry::test::with_captured_events(|| {
            dropped = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(uploader.upload(rows(&["a", "b"], Utc::now())));
        });

        assert!(dropped.is_empty(), "the retry inserts both rows");
        assert!(events.is_empty(), "nothing was dropped, so no Sentry event");
        let lines: Vec<String> = recorded
            .try_iter()
            .map(|line| String::from_utf8(line).unwrap())
            .collect();
        let errors: Vec<_> = lines
            .iter()
            .filter(|line| line.starts_with("last_seen.upload.error:"))
            .collect();
        assert_eq!(
            errors,
            ["last_seen.upload.error:1|c"],
            "one failed attempt, whatever the number of failed rows: {lines:?}"
        );
        assert!(
            !lines
                .iter()
                .any(|line| line.starts_with("last_seen.dropped:"))
        );
    }

    #[test]
    fn test_upload_drops_and_logs_rows_after_retries() {
        use httptest::{Expectation, matchers::*, responders::*};
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(MAX_ATTEMPTS)
                .respond_with(status_code(503)),
        );
        let logs = Arc::new(Mutex::new(Vec::new()));
        let row_log = row_log_to(&logs);
        let seen = Utc::now();

        let mut dropped = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(uploader_with(&server, row_log.clone()).upload(rows(&["a", "b"], seen)));
        wait_written(&row_log);

        dropped.sort();
        let key = |uid: &str| (uid.to_owned(), "client".to_owned());
        assert_eq!(
            dropped,
            vec![
                (key("a"), seen, "retries exhausted"),
                (key("b"), seen, "retries exhausted")
            ]
        );
        let mut logs = logs.lock().unwrap().clone();
        logs.sort();
        let expected: Vec<String> = ["a", "b"]
            .iter()
            .map(|uid| {
                format!(
                    "Last-seen upload failed; dropping observation \
                     reason=retries exhausted last_seen={} client_id=client uid={uid}",
                    seen.to_rfc3339()
                )
            })
            .collect();
        assert_eq!(logs, expected);
    }

    fn upload_capturing_events(
        server: &httptest::Server,
        rows: HashMap<Key, DateTime<Utc>>,
    ) -> (Vec<Dropped>, Vec<sentry::protocol::Event<'static>>) {
        let mut dropped = Vec::new();
        let events = sentry::test::with_captured_events(|| {
            dropped = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(uploader(server).upload(rows));
        });
        (dropped, events)
    }

    #[test]
    fn test_upload_does_not_retry_permanent_errors() {
        use httptest::{Expectation, matchers::*, responders::*};
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(1)
                .respond_with(status_code(400)),
        );

        let (dropped, events) = upload_capturing_events(&server, rows(&["a", "b"], Utc::now()));
        assert_eq!(dropped.len(), 2);
        assert!(dropped.iter().all(|(_, _, reason)| *reason == "rejected"));

        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.level, sentry::Level::Error);
        assert_eq!(
            event.message.as_deref(),
            Some("Last-seen upload dropped observations")
        );
        assert_eq!(event.extra["dropped"], 2);
        assert_eq!(event.extra["error"], "HTTP 400 Bad Request");
        let serialized = serde_json::to_string(event).unwrap();
        assert!(!serialized.contains("\"a\"") && !serialized.contains("client"));
    }

    fn forbidden(reason: &str) -> httptest::responders::ResponseBuilder<String> {
        httptest::responders::status_code(403).body(format!(
            r#"{{"error":{{"code":403,"message":"{reason} message","errors":[{{"reason":"{reason}"}}]}}}}"#
        ))
    }

    #[tokio::test]
    async fn test_upload_retries_rate_limited_403() {
        use httptest::{Expectation, cycle, matchers::*, responders::*};
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(3)
                .respond_with(cycle![
                    forbidden("rateLimitExceeded"),
                    forbidden("quotaExceeded"),
                    status_code(200).body("{}"),
                ]),
        );

        let dropped = uploader(&server).upload(rows(&["a"], Utc::now())).await;
        assert!(dropped.is_empty());
    }

    #[test]
    fn test_upload_does_not_retry_access_denied_403() {
        use httptest::{Expectation, matchers::*};
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(1)
                .respond_with(forbidden("accessDenied")),
        );

        let (dropped, events) = upload_capturing_events(&server, rows(&["a"], Utc::now()));
        assert_eq!(dropped.len(), 1);
        assert_eq!(dropped[0].2, "rejected");
        assert_eq!(
            events[0].extra["error"],
            "HTTP 403 Forbidden accessDenied: accessDenied message"
        );
    }

    /// Reads one HTTP/1.1 request (headers and `Content-Length` body) so the client finishes
    /// sending before the test server replies.
    fn read_request(stream: &mut std::net::TcpStream) {
        use std::io::Read;
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        let header_end = loop {
            let n = stream.read(&mut buf).unwrap();
            data.extend_from_slice(&buf[..n]);
            if let Some(pos) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
        };
        let headers = String::from_utf8_lossy(&data[..header_end]).to_lowercase();
        let length: usize = headers
            .lines()
            .find_map(|line| line.strip_prefix("content-length:"))
            .map_or(0, |value| value.trim().parse().unwrap());
        while data.len() < header_end + length {
            let n = stream.read(&mut buf).unwrap();
            data.extend_from_slice(&buf[..n]);
        }
    }

    #[tokio::test]
    async fn test_upload_retries_truncated_error_body() {
        use std::{
            io::Write,
            net::TcpListener,
            sync::atomic::{AtomicUsize, Ordering},
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let served = Arc::clone(&requests);

        std::thread::spawn(move || {
            for (i, stream) in listener.incoming().enumerate() {
                let mut stream = stream.unwrap();
                read_request(&mut stream);
                served.fetch_add(1, Ordering::SeqCst);
                let response: &[u8] = if i == 0 {
                    // Promises 200 bytes, sends a fragment, then closes the connection.
                    b"HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\n\
                      Content-Length: 200\r\nConnection: close\r\n\r\n{\"error\""
                } else {
                    b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}"
                };
                stream.write_all(response).unwrap();
            }
        });

        let uploader = Uploader::with_endpoint(
            &endpoint,
            "my-project.account_last_seen.last_seen_staging",
            credentials::anonymous::Builder::new().build(),
            Duration::ZERO,
            discard_row_log(),
            Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build()),
        )
        .unwrap();
        let dropped = uploader.upload(rows(&["a"], Utc::now())).await;

        assert!(dropped.is_empty());
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }

    type UploadCall = (FlushBatch, tokio::sync::oneshot::Sender<()>);

    /// An uploader whose uploads wait until the test completes them.
    struct FakeUploader {
        calls: tokio::sync::mpsc::UnboundedSender<UploadCall>,
    }

    impl Upload for FakeUploader {
        fn upload(
            &self,
            rows: FlushBatch,
            _entry: InFlightEntry,
        ) -> impl Future<Output = Vec<Dropped>> + Send {
            let (done, wait) = tokio::sync::oneshot::channel();
            let _ = self.calls.send((rows, done));
            async move {
                let _ = wait.await;
                Vec::new()
            }
        }
    }

    struct Harness {
        collector: Arc<Collector>,
        shutdown: Arc<Shutdown>,
        uploads: tokio::sync::mpsc::UnboundedReceiver<UploadCall>,
        run: tokio::task::JoinHandle<usize>,
    }

    /// Lets spawned tasks run without moving the paused clock.
    async fn settle() {
        for _ in 0..100 {
            tokio::task::yield_now().await;
        }
    }

    /// Starts a coordinator with a fake uploader and lets it reach its first wait.
    async fn start(max_concurrent_flushes: usize) -> Harness {
        let metrics = Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build());
        start_with(max_concurrent_flushes, metrics, discard_row_log()).await
    }

    async fn start_with(
        max_concurrent_flushes: usize,
        metrics: Arc<StatsdClient>,
        row_log: RowLog,
    ) -> Harness {
        let collector = Arc::new(Collector::new(Arc::clone(&metrics)));
        let shutdown = Arc::new(Shutdown::default());
        let (calls, uploads) = tokio::sync::mpsc::unbounded_channel();
        let coordinator = Coordinator::new(
            Arc::clone(&collector),
            Arc::new(FakeUploader { calls }),
            row_log,
            max_concurrent_flushes,
            Arc::clone(&shutdown),
            metrics,
        );
        let run = tokio::spawn(coordinator.run());
        settle().await;
        Harness {
            collector,
            shutdown,
            uploads,
            run,
        }
    }

    impl Harness {
        /// The next upload started so far, if any.
        async fn next_upload(&mut self) -> Option<UploadCall> {
            settle().await;
            self.uploads.try_recv().ok()
        }

        /// `run`'s result if it has returned.
        async fn finished(&mut self) -> Option<usize> {
            settle().await;
            if self.run.is_finished() {
                Some((&mut self.run).await.unwrap())
            } else {
                None
            }
        }
    }

    fn record_n(collector: &Collector, prefix: &str, n: usize) {
        let now = Utc::now();
        for i in 0..n {
            collector.record(&format!("{prefix}{i}"), "client", now);
        }
    }

    #[test]
    fn test_flush_due() {
        let zero = Duration::ZERO;
        assert!(
            !flush_due(0, FLUSH_INTERVAL, true),
            "never for an empty buffer"
        );
        assert!(flush_due(FLUSH_THRESHOLD, zero, false));
        assert!(!flush_due(
            FLUSH_THRESHOLD - 1,
            FLUSH_INTERVAL - Duration::from_millis(1),
            false
        ));
        assert!(flush_due(1, FLUSH_INTERVAL, false));
        assert!(flush_due(1, zero, true), "shutdown forces partial flushes");
    }

    #[test]
    fn test_close_ends_collection() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        let logger = slog::Logger::root(CaptureDrain(logs.clone()), slog::o!());
        let collector = collector();
        let late = Utc::now();
        let events = sentry::test::with_captured_events(|| {
            slog_scope::scope(&logger, || {
                collector.record("a", "client", Utc::now());
                assert_eq!(
                    collector.close().len(),
                    1,
                    "close returns the buffered rows"
                );
                collector.record("b", "client", late);
                collector.record("c", "client", late);
            })
        });

        assert!(collector.is_empty(), "records after close are not stored");
        let line = |uid: &str| {
            format!(
                "Last-seen collector closed; dropping observation \
                 last_seen={} client_id=client uid={uid}",
                late.to_rfc3339()
            )
        };
        assert_eq!(
            *logs.lock().unwrap(),
            vec![line("b"), line("c")],
            "every record after close is logged like other drops"
        );
        assert_eq!(
            events.len(),
            1,
            "one Sentry event, on the first record after close"
        );
        assert_eq!(events[0].level, sentry::Level::Error);
        assert_eq!(
            events[0].message.as_deref(),
            Some("Last-seen record after close; dropping observations")
        );
    }

    #[test]
    fn test_in_flight_tracks_each_upload() {
        let in_flight = InFlight::default();
        let first = rows(&["a"], Utc::now());
        let second = rows(&["b"], Utc::now());
        let first_id = in_flight.register(&first);
        let second_id = in_flight.register(&second);
        assert_eq!(in_flight.len(), 2);

        in_flight.finish(first_id);
        assert_eq!(
            in_flight.take_remaining(),
            second.into_iter().collect::<Vec<_>>()
        );
        in_flight.finish(second_id);
        assert!(in_flight.is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn test_repeated_signal_keeps_the_deadline() {
        let shutdown = Shutdown::default();
        assert_eq!(shutdown.state(), ShutdownState::default());
        let deadline = shutdown.begin();
        assert_eq!(deadline, Instant::now() + SHUTDOWN_BUDGET);

        tokio::time::advance(Duration::from_secs(20)).await;
        assert_eq!(
            shutdown.begin(),
            deadline,
            "a later signal must not grant a fresh budget"
        );
        assert_eq!(shutdown.state().deadline, Some(deadline));
    }

    #[tokio::test(start_paused = true)]
    async fn test_finish_requests_starts_the_budget() {
        let shutdown = Shutdown::default();
        let watcher = shutdown.subscribe();
        shutdown.finish_requests();
        assert!(watcher.has_changed().unwrap(), "watchers are notified");
        let state = shutdown.state();
        assert!(state.requests_finished);
        assert_eq!(state.deadline, Some(Instant::now() + SHUTDOWN_BUDGET));
    }

    #[tokio::test(start_paused = true)]
    async fn test_size_trigger_flushes_whole_buffer() {
        let mut h = start(4).await;
        record_n(&h.collector, "uid", FLUSH_THRESHOLD);
        let (batch, _done) = h.next_upload().await.expect("500 rows trigger a flush");
        assert_eq!(batch.len(), FLUSH_THRESHOLD);
        assert!(h.collector.is_empty(), "a flush takes the whole buffer");
    }

    #[tokio::test(start_paused = true)]
    async fn test_timer_flushes_below_threshold() {
        let mut h = start(4).await;
        record_n(&h.collector, "uid", 1);
        tokio::time::advance(FLUSH_INTERVAL - Duration::from_secs(1)).await;
        assert!(h.next_upload().await.is_none(), "too early");
        tokio::time::advance(Duration::from_secs(1)).await;
        let (batch, _done) = h.next_upload().await.expect("the timer flushes one row");
        assert_eq!(batch.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_each_flush_restarts_the_timer() {
        let mut h = start(4).await;
        tokio::time::advance(Duration::from_secs(30)).await;
        record_n(&h.collector, "big", FLUSH_THRESHOLD);
        let (_, done) = h.next_upload().await.expect("size flush at 30 s");
        done.send(()).unwrap();

        record_n(&h.collector, "small", 1);
        tokio::time::advance(Duration::from_secs(30)).await;
        assert!(
            h.next_upload().await.is_none(),
            "60 s since start, but only 30 s since the last flush"
        );
        tokio::time::advance(Duration::from_secs(30)).await;
        let (batch, _done) = h.next_upload().await.expect("60 s after the last flush");
        assert_eq!(batch.len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn test_full_slots_delay_due_flushes() {
        let mut h = start(2).await;
        let mut held = Vec::new();
        for prefix in ["a", "b"] {
            record_n(&h.collector, prefix, FLUSH_THRESHOLD);
            held.push(h.next_upload().await.expect("a slot is free").1);
        }
        record_n(&h.collector, "c", FLUSH_THRESHOLD);
        assert!(h.next_upload().await.is_none(), "both slots are busy");
        assert_eq!(
            h.collector.len(),
            FLUSH_THRESHOLD,
            "due work waits in the buffer"
        );

        held.remove(0).send(()).unwrap();
        let (batch, _done) = h
            .next_upload()
            .await
            .expect("a freed slot takes the due flush");
        assert_eq!(batch.len(), FLUSH_THRESHOLD);
    }

    #[tokio::test(start_paused = true)]
    async fn test_shutdown_flushes_partial_buffer_and_finishes() {
        let mut h = start(4).await;
        record_n(&h.collector, "uid", 3);
        h.shutdown.begin();
        let (batch, done) = h
            .next_upload()
            .await
            .expect("shutdown forces a partial flush");
        assert_eq!(batch.len(), 3);
        done.send(()).unwrap();

        assert_eq!(
            h.finished().await,
            None,
            "the HTTP server has not stopped yet"
        );
        h.shutdown.finish_requests();
        assert_eq!(h.finished().await, Some(0));
    }

    #[tokio::test(start_paused = true)]
    async fn test_rows_recorded_after_the_signal_are_flushed() {
        let mut h = start(4).await;
        h.shutdown.begin();
        settle().await;
        record_n(&h.collector, "late", 1);
        let (batch, done) = h
            .next_upload()
            .await
            .expect("a finishing exchange's row is flushed without waiting for the timer");
        assert_eq!(batch.len(), 1);
        done.send(()).unwrap();

        h.shutdown.finish_requests();
        assert_eq!(h.finished().await, Some(0));
    }

    #[tokio::test(start_paused = true)]
    async fn test_drain_waits_for_running_uploads() {
        let mut h = start(4).await;
        record_n(&h.collector, "uid", FLUSH_THRESHOLD);
        let (_, done) = h.next_upload().await.expect("size flush");
        h.shutdown.finish_requests();
        assert_eq!(h.finished().await, None, "an upload is still running");
        done.send(()).unwrap();
        assert_eq!(h.finished().await, Some(0));
    }

    #[tokio::test(start_paused = true)]
    async fn test_coordinator_reports_gauges() {
        let (recorded, sink) = cadence::SpyMetricSink::new();
        let metrics = Arc::new(StatsdClient::builder("", sink).build());
        let mut h = start_with(4, metrics, discard_row_log()).await;
        record_n(&h.collector, "uid", FLUSH_THRESHOLD);
        let (_, _done) = h.next_upload().await.expect("size flush");

        let lines: Vec<String> = recorded
            .try_iter()
            .map(|line| String::from_utf8(line).unwrap())
            .collect();
        // The pass after the flush: the buffer is empty and one upload is running.
        assert!(
            lines.contains(&"last_seen.buffered:0|g".to_owned()),
            "{lines:?}"
        );
        assert!(
            lines.contains(&"last_seen.in_flight:1|g".to_owned()),
            "{lines:?}"
        );
    }

    // Exercise 6

    #[test]
    fn test_deadline_abandons_logs_and_reports_every_row() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        // The row log writes the abandoned rows; the scoped logger gets the record after close.
        let logger = slog::Logger::root(CaptureDrain(logs.clone()), slog::o!());
        let row_log = row_log_to(&logs);
        let mut abandoned = 0;
        // A paused current-thread runtime on this thread: the scoped logger and the test
        // Sentry hub see everything the coordinator does.
        let events = sentry::test::with_captured_events(|| {
            slog_scope::scope(&logger, || {
                abandoned = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .start_paused(true)
                    .build()
                    .unwrap()
                    .block_on(async {
                        let metrics =
                            Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build());
                        let mut h = start_with(1, metrics, row_log.clone()).await;
                        record_n(&h.collector, "flight", FLUSH_THRESHOLD);
                        let (_batch, _never_done) = h.next_upload().await.expect("first flush");
                        record_n(&h.collector, "buffered", 2);

                        h.shutdown.begin();
                        tokio::time::advance(SHUTDOWN_BUDGET - Duration::from_secs(1)).await;
                        assert_eq!(h.finished().await, None, "the deadline has not passed");
                        tokio::time::advance(Duration::from_secs(1)).await;
                        let abandoned = h.finished().await.expect("the deadline ends shutdown");

                        record_n(&h.collector, "late", 1);
                        assert!(h.collector.is_empty(), "collection is closed");
                        abandoned
                    })
            })
        });

        wait_written(&row_log);
        assert_eq!(abandoned, FLUSH_THRESHOLD + 2);
        let logs = logs.lock().unwrap();
        let (deadline_logs, other_logs): (Vec<_>, Vec<_>) = logs.iter().partition(|line| {
            line.starts_with("Last-seen shutdown deadline reached; dropping observation")
        });
        assert_eq!(
            deadline_logs.len(),
            FLUSH_THRESHOLD + 2,
            "one error per abandoned row"
        );
        assert!(
            deadline_logs
                .iter()
                .all(|line| line.contains("client_id=client"))
        );
        // The record after close is logged too (exercise 2).
        assert_eq!(other_logs.len(), 1);
        assert!(other_logs[0].starts_with("Last-seen collector closed; dropping observation"));
        assert!(other_logs[0].contains("uid=late0"));
        assert!(
            deadline_logs
                .iter()
                .any(|line| line.contains("uid=flight0"))
        );
        assert!(
            deadline_logs
                .iter()
                .any(|line| line.contains("uid=buffered0"))
        );
        // One event for the whole shutdown, and one for the first record after close.
        assert_eq!(events.len(), 2);
        assert_eq!(
            events[0].message.as_deref(),
            Some("Last-seen shutdown dropped observations")
        );
        assert_eq!(events[0].extra["dropped"], FLUSH_THRESHOLD + 2);
        assert_eq!(
            events[1].message.as_deref(),
            Some("Last-seen record after close; dropping observations")
        );
    }

    #[tokio::test]
    async fn test_shutdown_counts_only_unacknowledged_rows() {
        use httptest::{Expectation, matchers::*, responders::*};
        let mut server = httptest::Server::run();
        let rows_in_request = |count: usize| {
            request::body(json_decoded(move |body: &serde_json::Value| {
                body["rows"].as_array().unwrap().len() == count
            }))
        };
        // Row 1 fails transiently; row 0 is acknowledged.
        server.expect(
            Expectation::matching(all_of![
                request::method_path("POST", INSERT_ALL_PATH),
                rows_in_request(2),
            ])
            .times(1)
            .respond_with(
                status_code(200)
                    .body(r#"{"insertErrors":[{"index":1,"errors":[{"reason":"backendError"}]}]}"#),
            ),
        );
        // The retry carries only the unacknowledged row and outlasts the deadline.
        server.expect(
            Expectation::matching(all_of![
                request::method_path("POST", INSERT_ALL_PATH),
                rows_in_request(1),
            ])
            .times(1)
            .respond_with(delay_and_then(
                Duration::from_secs(3),
                status_code(200).body("{}"),
            )),
        );
        let metrics = Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build());
        let collector = Arc::new(Collector::new(Arc::clone(&metrics)));
        let mut uploader = uploader(&server);
        uploader.retry_delay = RETRY_DELAY;
        let shutdown = Arc::new(Shutdown::default());
        // A forced flush that starts with 1.5 s of the budget left.
        shutdown.state.send_modify(|state| {
            state.deadline = Some(Instant::now() + Duration::from_millis(1500));
            state.requests_finished = true;
        });
        collector.record("a", "client", Utc::now());
        collector.record("b", "client", Utc::now());

        let coordinator = Coordinator::new(
            collector,
            Arc::new(uploader),
            discard_row_log(),
            1,
            shutdown,
            metrics,
        );
        let abandoned = coordinator.run().await;

        server.verify_and_clear();
        assert_eq!(
            abandoned, 1,
            "the acknowledged row is not reported as dropped"
        );
    }

    #[test]
    fn test_row_log_waits_without_stalling_the_runtime() {
        use httptest::{Expectation, matchers::*, responders::*};
        use std::sync::mpsc;

        /// Stalls on its first record until released, like a slow log output.
        struct StalledDrain {
            capture: CaptureDrain,
            entered: mpsc::Sender<()>,
            release: Mutex<Option<mpsc::Receiver<()>>>,
        }
        impl slog::Drain for StalledDrain {
            type Ok = ();
            type Err = slog::Never;
            fn log(
                &self,
                record: &slog::Record<'_>,
                values: &slog::OwnedKVList,
            ) -> Result<(), slog::Never> {
                if let Some(release) = self.release.lock().unwrap().take() {
                    self.entered.send(()).unwrap();
                    release.recv().unwrap();
                }
                self.capture.log(record, values)
            }
        }

        let logs = Arc::new(Mutex::new(Vec::new()));
        let (entered, stalled) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let row_log = RowLog::spawn(slog::Logger::root(
            StalledDrain {
                capture: CaptureDrain(Arc::clone(&logs)),
                entered,
                release: Mutex::new(Some(wait)),
            },
            slog::o!(),
        ))
        .unwrap();
        row_log
            .queue
            .blocking_send(RowLogBatch::Abandoned(
                rows(&["stall"], Utc::now()).into_iter().collect(),
            ))
            .unwrap();
        stalled.recv_timeout(Duration::from_secs(5)).unwrap();

        // One more failed upload than the queue holds: the last one waits for space.
        let uploads = ROW_LOG_QUEUE + 1;
        let rows_per_upload = 1_000;
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(request::method_path("POST", INSERT_ALL_PATH))
                .times(uploads)
                .respond_with(status_code(400)),
        );
        let uploader = Arc::new(uploader_with(&server, row_log.clone()));
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            release.send(()).unwrap();
        });

        // The current-thread runtime that #[actix_web::main] uses.
        let flushing = row_log.clone();
        let (timer, flush, gave_up) = actix_web::rt::System::new().block_on(async move {
            let mut tasks = JoinSet::new();
            for u in 0..uploads {
                let uploader = Arc::clone(&uploader);
                let batch: FlushBatch = (0..rows_per_upload)
                    .map(|i| ((format!("u{u}-{i}"), "client".to_owned()), Utc::now()))
                    .collect();
                tasks.spawn(async move { uploader.upload(batch).await.len() });
            }
            let full = Instant::now() + Duration::from_secs(1);
            while flushing.queue.capacity() > 0 {
                assert!(Instant::now() < full, "the queue fills");
                sleep(Duration::from_millis(5)).await;
            }

            let began = std::time::Instant::now();
            sleep(Duration::from_millis(50)).await;
            let timer = began.elapsed();
            let began = std::time::Instant::now();
            let gave_up = !flushing
                .flush_until(Instant::now() + Duration::from_millis(100))
                .await;
            let flush = began.elapsed();

            while let Some(dropped) = tasks.join_next().await {
                assert_eq!(dropped.unwrap(), rows_per_upload);
            }
            (timer, flush, gave_up)
        });
        releaser.join().unwrap();
        wait_written(&row_log);

        assert!(
            timer < Duration::from_millis(250),
            "a 50 ms timer took {timer:?} while the row logs waited"
        );
        assert!(gave_up, "flushing a stalled row log stops at its deadline");
        assert!(flush < Duration::from_secs(1), "the flush took {flush:?}");
        let written = logs
            .lock()
            .unwrap()
            .iter()
            .filter(|line| line.starts_with("Last-seen upload failed"))
            .count();
        assert_eq!(written, uploads * rows_per_upload, "no row log is dropped");
    }

    #[test]
    fn test_row_logs_reach_the_test_logger_after_server_logging_starts() {
        // server::test::init_app! starts the process-wide loggers.
        crate::logging::init_logging(false).unwrap();
        test_upload_drops_and_logs_rows_after_retries();
        test_deadline_abandons_logs_and_reports_every_row();
    }

    #[test]
    fn test_stopped_row_log_writer_reports_unwritten_rows() {
        use slog::Drain;
        /// Fails every write; the production `.fuse()` then panics the writer thread.
        struct BrokenOutput;
        impl slog::Drain for BrokenOutput {
            type Ok = ();
            type Err = std::io::Error;
            fn log(&self, _: &slog::Record<'_>, _: &slog::OwnedKVList) -> Result<(), Self::Err> {
                Err(std::io::ErrorKind::BrokenPipe.into())
            }
        }

        let row_log = RowLog::spawn(slog::Logger::root(BrokenOutput.fuse(), slog::o!())).unwrap();
        let mut results = (false, false);
        let events = sentry::test::with_captured_events(|| {
            results = actix_web::rt::System::new().block_on(async {
                let lost =
                    RowLogBatch::Abandoned(rows(&["lost"], Utc::now()).into_iter().collect());
                assert!(row_log.send(lost).await, "queued before the writer stops");
                let until = Instant::now() + Duration::from_secs(2);
                while !row_log.queue.is_closed() {
                    assert!(Instant::now() < until, "the writer thread stops");
                    sleep(Duration::from_millis(1)).await;
                }
                let late =
                    RowLogBatch::Abandoned(rows(&["late"], Utc::now()).into_iter().collect());
                let queued = row_log.send(late).await;
                let written = row_log
                    .flush_until(Instant::now() + Duration::from_secs(1))
                    .await;
                (queued, written)
            });
        });

        assert_eq!(
            results,
            (false, false),
            "a stopped writer neither queues nor flushes"
        );
        assert_eq!(events.len(), 1, "one event for the unwritten row logs");
        assert_eq!(
            events[0].message.as_deref(),
            Some("Last-seen row logs not written")
        );
    }

    #[test]
    fn test_drain_and_logging_reset_end_by_the_exit_limit() {
        use slog::Drain;
        use std::sync::mpsc;

        /// Stalls on its first record until released, like a stuck log output.
        struct StalledOutput {
            entered: mpsc::Sender<()>,
            release: Mutex<Option<mpsc::Receiver<()>>>,
        }
        impl slog::Drain for StalledOutput {
            type Ok = ();
            type Err = slog::Never;
            fn log(&self, _: &slog::Record<'_>, _: &slog::OwnedKVList) -> Result<(), Self::Err> {
                if let Some(release) = self.release.lock().unwrap().take() {
                    self.entered.send(()).unwrap();
                    release.recv().unwrap();
                }
                Ok(())
            }
        }

        let (entered, stalled) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        // The same shared output and async main logger as `init_logging`.
        let output = Arc::new(
            Mutex::new(StalledOutput {
                entered,
                release: Mutex::new(Some(wait)),
            })
            .fuse(),
        );
        slog_scope::set_global_logger(slog::Logger::root(
            slog_async::Async::new(Arc::clone(&output)).build().fuse(),
            slog::o!(),
        ))
        .cancel_reset();
        let row_log = RowLog::spawn(slog::Logger::root(output, slog::o!())).unwrap();
        row_log
            .queue
            .blocking_send(RowLogBatch::Abandoned(
                rows(&["stall"], Utc::now()).into_iter().collect(),
            ))
            .unwrap();
        stalled.recv_timeout(Duration::from_secs(5)).unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            release.send(()).unwrap();
        });

        let metrics = Arc::new(StatsdClient::builder("", cadence::NopMetricSink).build());
        let collector = Arc::new(Collector::new(Arc::clone(&metrics)));
        collector.record("buffered", "client", Utc::now());
        let shutdown = Arc::new(Shutdown::default());
        let (calls, _uploads) = tokio::sync::mpsc::unbounded_channel();
        let mut limit = None;
        let events = sentry::test::with_captured_events(|| {
            limit = Some(actix_web::rt::System::new().block_on(async {
                // A deadline that has passed, leaving 100 ms of the row-log budget.
                shutdown.state.send_modify(|state| {
                    state.deadline =
                        Some(Instant::now() + Duration::from_millis(100) - ROW_LOG_BUDGET);
                    state.requests_finished = true;
                });
                let coordinator = Coordinator::new(
                    collector,
                    Arc::new(FakeUploader { calls }),
                    row_log.clone(),
                    1,
                    Arc::clone(&shutdown),
                    metrics,
                );
                let run = tokio::spawn(coordinator.run());
                finish_shutdown(&shutdown, run, &row_log).await
            }));
        });
        let limit = limit.unwrap();
        // The rest of `main` after the drain.
        slog_scope::info!("Server closing");
        crate::logging::reset_logging(Some(limit));
        let exited = std::time::Instant::now();
        releaser.join().unwrap();

        assert!(
            exited < limit + Duration::from_millis(250),
            "the exit ended {:?} after its limit",
            exited.saturating_duration_since(limit)
        );
        assert!(
            events
                .iter()
                .any(|e| e.message.as_deref() == Some("Last-seen row logs not written")),
            "the unwritten row logs are reported"
        );
    }
}
