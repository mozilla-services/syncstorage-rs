//! Optional offload of BSO payloads to Google Cloud Storage.
//!
//! When `gcs_payload_bucket` is set and the request's collection appears in
//! `gcs_payload_offload_collections` (both in syncstorage settings), the BSO
//! write handlers upload the incoming payload to GCS prior to opening the
//! database transaction. The returned object URL is written to the BSO
//! `payload_link` column, the payload's byte length to `payload_size`, and the
//! inline `payload` field is cleared.
//!
//! On the read path, BSOs with a `payload_link` set have their payload
//! resolved by downloading from GCS after the database transaction commits,
//! and `payload_link` is cleared before the response is rendered.
//!
//! Objects are written with the custom metadata `committed=false` and a
//! `customTime` set to upload time; a later step flips `committed` to `true`
//! once the database row is durably visible.

use std::{collections::HashMap, time::SystemTime};

use google_cloud_auth::credentials::anonymous;
use google_cloud_storage::client::{Storage, StorageControl};
use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiErrorKind},
    server::ServerState,
};

const COMMITTED_METADATA_KEY: &str = "committed";

/// Counter for the best-effort GCS cleanup that runs when the database
/// transaction of an offloading write fails. Tagged with the `handler` that
/// issued it, a `result` of `success` or `failure`, and on failure a `reason`.
const CLEANUP_METRIC: &str = "storage.gcs.payload.cleanup";

/// Return the GCS bucket name if `collection` is opted into payload off-load
/// and a bucket is configured. `None` disables off-load for this request.
pub fn offload_bucket<'a>(state: &'a ServerState, collection: &str) -> Option<&'a str> {
    let bucket = state.gcs_payload_bucket.as_deref()?;
    state
        .gcs_payload_offload_collections
        .iter()
        .any(|c| c == collection)
        .then_some(bucket)
}

/// Build a GCS client honoring the `endpoint` override. When the override is
/// set we additionally use anonymous credentials so the SDK does not attempt
/// to acquire Application Default Credentials against a mock server. This is
/// opt-in via the `SYNC_SYNCSTORAGE__GCS_ENDPOINT` setting (unset in prod
/// deployments); setting it to the wrong value in prod would immediately break
/// offload, so the opt-in is self-defeating as a stealth-security-degradation
/// vector.
pub async fn build_client(endpoint: Option<&str>) -> Result<Storage, ApiError> {
    let mut builder = Storage::builder();
    if let Some(endpoint) = endpoint {
        builder = builder
            .with_endpoint(endpoint)
            .with_credentials(anonymous::Builder::new().build());
    }
    builder
        .build()
        .await
        .map_err(|e| ApiErrorKind::Internal(format!("GCS builder error: {e}")).into())
}

/// Upload `payload` to `bucket` under the key `{fxa_uid}/{collection}/{bso_id}`
/// and return the resulting `gs://` URL.
///
/// The object is written with custom metadata `committed=false` and a
/// `customTime` of the upload moment.
pub async fn upload_payload(
    client: &Storage,
    bucket: &str,
    user_id: &UserIdentifier,
    collection: &str,
    bso_id: &str,
    payload: String,
) -> Result<String, ApiError> {
    let object_name = format!(
        "{}/{}/{}/{}",
        user_id.fxa_uid,
        collection,
        bso_id,
        Uuid::new_v4().hyphenated()
    );

    let custom_time: wkt::Timestamp = SystemTime::now()
        .try_into()
        .map_err(|e| ApiErrorKind::Internal(format!("custom_time: {e}")))?;

    client
        .write_object(bucket_path(bucket), object_name.clone(), payload)
        .set_metadata([(COMMITTED_METADATA_KEY.to_string(), "false".to_string())])
        .set_custom_time(custom_time)
        .send_buffered()
        .await?;

    Ok(format!("gs://{bucket}/{object_name}"))
}

/// Download payload bytes from a `gs://{bucket}/{object}` URL produced by
/// [`upload_payload`] and return them as a UTF-8 string.
pub async fn download_payload(client: &Storage, gs_url: &str) -> Result<String, ApiError> {
    let (bucket, object) = parse_gs_url(gs_url)?;

    let mut response = client
        .read_object(bucket_path(bucket), object.to_string())
        .send()
        .await?;

    let mut bytes = Vec::new();
    while let Some(chunk) = response.next().await.transpose()? {
        bytes.extend_from_slice(&chunk);
    }

    String::from_utf8(bytes)
        .map_err(|e| ApiErrorKind::Internal(format!("invalid utf-8 in GCS payload: {e}")).into())
}

pub async fn build_control_client(endpoint: Option<&str>) -> Result<StorageControl, ApiError> {
    let mut builder = StorageControl::builder();
    if let Some(endpoint) = endpoint {
        builder = builder
            .with_endpoint(endpoint)
            .with_credentials(anonymous::Builder::new().build());
    }
    builder
        .build()
        .await
        .map_err(|e| ApiErrorKind::Internal(format!("GCS builder error: {e}")).into())
}

/// The write handler a [`CLEANUP_METRIC`] emission came from.
#[derive(Clone, Copy, Debug)]
pub enum CleanupHandler {
    PutBso,
    PostCollection,
}

impl CleanupHandler {
    fn as_str(self) -> &'static str {
        match self {
            Self::PutBso => "put_bso",
            Self::PostCollection => "post_collection",
        }
    }
}

/// The outcome of a cleanup attempt, as the `result` and `reason` tags.
#[derive(Clone, Copy, Debug)]
enum CleanupResult {
    Success,
    /// The `gs://` URL did not parse, so no delete was issued.
    InvalidUrl,
    /// GCS rejected the delete.
    GcsError,
}

impl CleanupResult {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::InvalidUrl | Self::GcsError => "failure",
        }
    }

    /// The `reason` tag, set for the failure variants only.
    fn reason(self) -> Option<&'static str> {
        match self {
            Self::Success => None,
            Self::InvalidUrl => Some("invalid_url"),
            Self::GcsError => Some("gcs_error"),
        }
    }
}

/// Tags for a [`CLEANUP_METRIC`] emission from `handler` with `result`.
fn cleanup_tags(handler: CleanupHandler, result: CleanupResult) -> HashMap<String, String> {
    let mut tags = HashMap::from([
        ("handler".to_owned(), handler.as_str().to_owned()),
        ("result".to_owned(), result.as_str().to_owned()),
    ]);
    if let Some(reason) = result.reason() {
        tags.insert("reason".to_owned(), reason.to_owned());
    }
    tags
}

pub async fn delete_payload(
    client: &StorageControl,
    gs_url: &str,
    metrics: &Metrics,
    handler: CleanupHandler,
) -> Result<(), ApiError> {
    let (bucket, object) = parse_gs_url(gs_url).inspect_err(|_| {
        metrics.incr_with_tags(
            CLEANUP_METRIC,
            cleanup_tags(handler, CleanupResult::InvalidUrl),
        )
    })?;

    client
        .delete_object()
        .set_bucket(bucket_path(bucket))
        .set_object(object)
        .send()
        .await
        .inspect(|_| {
            metrics.incr_with_tags(
                CLEANUP_METRIC,
                cleanup_tags(handler, CleanupResult::Success),
            )
        })
        .inspect_err(|e| {
            warn!("gcs payload cleanup failed for {gs_url}: {e}");
            metrics.incr_with_tags(
                CLEANUP_METRIC,
                cleanup_tags(handler, CleanupResult::GcsError),
            );
        })
        .map_err(|e| ApiErrorKind::Internal(format!("cannot delete GCS object: {e}")).into())
}

fn bucket_path(bucket: &str) -> String {
    format!("projects/_/buckets/{bucket}")
}

fn parse_gs_url(url: &str) -> Result<(&str, &str), ApiError> {
    url.strip_prefix("gs://")
        .and_then(|p| p.split_once('/'))
        .ok_or_else(|| ApiErrorKind::Internal(format!("invalid GCS URL: {url}")).into())
}

/// Reattach GCS results to their corresponding entries in `items` by index.
pub fn reattach_by_index<T, V>(
    items: &mut [T],
    results: impl IntoIterator<Item = (usize, V)>,
    mut apply: impl FnMut(&mut T, V),
) {
    for (i, value) in results {
        apply(&mut items[i], value);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{Arc, Mutex},
    };

    use cadence::{SpyMetricSink, StatsdClient};
    use crossbeam_channel::Receiver;
    use google_cloud_gax::{
        error::{
            Error,
            rpc::{Code, Status},
        },
        options::RequestOptions,
        response::Response,
    };
    use google_cloud_storage::{Result as GcsResult, client::StorageControl, model};

    use super::*;

    /// A [`Metrics`] that records every statsd line, alongside the receiver
    /// those lines arrive on.
    fn recording_metrics() -> (Metrics, Receiver<Vec<u8>>) {
        let (recorded, sink) = SpyMetricSink::new();
        let client = StatsdClient::builder("", sink).build();
        (
            Metrics {
                client: Some(Arc::new(client)),
                tags: HashMap::default(),
                timer: None,
            },
            recorded,
        )
    }

    /// Every statsd line emitted so far, joined for substring assertions.
    fn emitted(recorded: &Receiver<Vec<u8>>) -> String {
        recorded
            .try_iter()
            .map(|line| String::from_utf8(line).expect("statsd line was not utf-8"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Stub to record delete_object
    #[derive(Debug, Default)]
    struct RecordingStub {
        deletes: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl google_cloud_storage::stub::StorageControl for RecordingStub {
        fn delete_object(
            &self,
            req: model::DeleteObjectRequest,
            _options: RequestOptions,
        ) -> impl Future<Output = GcsResult<Response<()>>> + Send {
            self.deletes
                .lock()
                .expect("deletes lock poisoned")
                .push((req.bucket.clone(), req.object.clone()));
            async move { Ok(Response::from(())) }
        }
    }

    #[derive(Debug, Default)]
    struct FailingStub;

    impl google_cloud_storage::stub::StorageControl for FailingStub {
        async fn delete_object(
            &self,
            _req: model::DeleteObjectRequest,
            _options: RequestOptions,
        ) -> GcsResult<Response<()>> {
            Err(Error::service(Status::default().set_code(Code::Internal)))
        }
    }

    #[actix_rt::test]
    async fn delete_payload_issues_delete_for_parsed_url() {
        let deletes = Arc::new(Mutex::new(Vec::new()));
        let client = StorageControl::from_stub(RecordingStub {
            deletes: deletes.clone(),
        });

        delete_payload(
            &client,
            "gs://test-bucket/uid/bookmarks/bid/uuid",
            &Metrics::noop(),
            CleanupHandler::PutBso,
        )
        .await
        .expect("delete_payload should succeed");

        let recorded = deletes.lock().unwrap();
        assert_eq!(
            &*recorded,
            &[(
                "projects/_/buckets/test-bucket".to_owned(),
                "uid/bookmarks/bid/uuid".to_owned(),
            )],
        );
    }

    #[actix_rt::test]
    async fn delete_payload_counts_a_success() {
        let client = StorageControl::from_stub(RecordingStub::default());
        let (metrics, recorded) = recording_metrics();

        delete_payload(
            &client,
            "gs://test-bucket/uid/bookmarks/bid/uuid",
            &metrics,
            CleanupHandler::PutBso,
        )
        .await
        .expect("delete_payload should succeed");

        let emitted = emitted(&recorded);
        assert!(
            emitted.contains(CLEANUP_METRIC)
                && emitted.contains("result:success")
                && emitted.contains("handler:put_bso"),
            "unexpected cleanup metric: {emitted}"
        );
    }

    #[actix_rt::test]
    async fn delete_payload_surfaces_delete_error() {
        let client = StorageControl::from_stub(FailingStub);

        let result = delete_payload(
            &client,
            "gs://test-bucket/uid/bookmarks/bid/uuid",
            &Metrics::noop(),
            CleanupHandler::PutBso,
        )
        .await;

        assert!(
            result.is_err(),
            "a failed GCS delete should surface as an error"
        );
    }

    #[actix_rt::test]
    async fn delete_payload_counts_a_gcs_failure() {
        let client = StorageControl::from_stub(FailingStub);
        let (metrics, recorded) = recording_metrics();

        delete_payload(
            &client,
            "gs://test-bucket/uid/bookmarks/bid/uuid",
            &metrics,
            CleanupHandler::PostCollection,
        )
        .await
        .expect_err("a failed GCS delete should surface as an error");

        let emitted = emitted(&recorded);
        assert!(
            emitted.contains(CLEANUP_METRIC)
                && emitted.contains("result:failure")
                && emitted.contains("reason:gcs_error")
                && emitted.contains("handler:post_collection"),
            "unexpected cleanup metric: {emitted}"
        );
    }

    #[actix_rt::test]
    async fn delete_payload_counts_an_unparseable_url() {
        let deletes = Arc::new(Mutex::new(Vec::new()));
        let client = StorageControl::from_stub(RecordingStub {
            deletes: deletes.clone(),
        });
        let (metrics, recorded) = recording_metrics();

        delete_payload(&client, "not-a-gs-url", &metrics, CleanupHandler::PutBso)
            .await
            .expect_err("an unparseable URL should surface as an error");

        assert!(
            deletes.lock().unwrap().is_empty(),
            "no delete should be issued for an unparseable URL"
        );
        let emitted = emitted(&recorded);
        assert!(
            emitted.contains(CLEANUP_METRIC)
                && emitted.contains("result:failure")
                && emitted.contains("reason:invalid_url"),
            "unexpected cleanup metric: {emitted}"
        );
    }

    #[test]
    fn reattach_results_to_correct_slots() {
        let mut items: Vec<Option<String>> = vec![None, None, None, None];
        let results = vec![
            (2, "two".to_owned()),
            (0, "zero".to_owned()),
            (3, "three".to_owned()),
            (1, "one".to_owned()),
        ];
        reattach_by_index(&mut items, results, |slot, payload| *slot = Some(payload));
        assert_eq!(
            items,
            vec![
                Some("zero".to_owned()),
                Some("one".to_owned()),
                Some("two".to_owned()),
                Some("three".to_owned()),
            ]
        );
    }

    #[test]
    fn reattach_only_indexed_slots() {
        let mut items = vec![
            "keep-0".to_owned(),
            "keep-1".to_owned(),
            "keep-2".to_owned(),
        ];
        let results = vec![(1, "replaced-1".to_owned())];
        reattach_by_index(&mut items, results, |slot, v| *slot = v);
        assert_eq!(
            items,
            vec![
                "keep-0".to_owned(),
                "replaced-1".to_owned(),
                "keep-2".to_owned(),
            ]
        );
    }

    #[test]
    fn reattach_empty_results_is_noop() {
        let mut items = vec![1, 2, 3];
        reattach_by_index(&mut items, Vec::<(usize, i32)>::new(), |slot, v| *slot = v);
        assert_eq!(items, vec![1, 2, 3]);
    }
}
