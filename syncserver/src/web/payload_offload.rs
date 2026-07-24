//! Optional offload of BSO payloads to Google Cloud Storage.
//!
//! When `gcs_payload_bucket` is set and the request's collection appears in
//! `gcs_payload_offload_collections` (both in syncstorage settings), the BSO
//! write handlers upload the incoming payload to GCS prior to opening the
//! database transaction. The returned object URL is written to the BSO
//! `payload_link` column and the inline `payload` field is cleared.
//!
//! On the read path, BSOs with a `payload_link` set have their payload
//! resolved by downloading from GCS after the database transaction commits,
//! and `payload_link` is cleared before the response is rendered.
//!
//! Objects are written with the custom metadata `committed=false` and a
//! `customTime` set to upload time; a later step flips `committed` to `true`
//! once the database row is durably visible.

use std::time::SystemTime;

use google_cloud_auth::credentials::anonymous;
use google_cloud_storage::client::{Storage, StorageControl};
use syncstorage_db::UserIdentifier;
use uuid::Uuid;

use crate::{
    error::{ApiError, ApiErrorKind},
    server::ServerState,
};

const COMMITTED_METADATA_KEY: &str = "committed";

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

/// Build a GCS client honoring the endpoint override stored on
/// [`ServerState`]. When the override is set we additionally use anonymous
/// credentials so the SDK does not attempt to acquire Application Default
/// Credentials against a mock server. This is opt-in via the
/// `SYNC_SYNCSTORAGE__GCS_ENDPOINT` setting (unset in prod deployments);
/// setting it to the wrong value in prod would immediately break offload,
/// so the opt-in is self-defeating as a stealth-security-degradation vector.
async fn gcs_client(state: &ServerState) -> Result<Storage, ApiError> {
    let mut builder = Storage::builder();
    if let Some(endpoint) = state.gcs_endpoint.as_deref() {
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
    state: &ServerState,
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
    let client = gcs_client(state).await?;

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
pub async fn download_payload(state: &ServerState, gs_url: &str) -> Result<String, ApiError> {
    let (bucket, object) = parse_gs_url(gs_url)?;
    let client = gcs_client(state).await?;

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

async fn gcs_control_client() -> Result<StorageControl, ApiError> {
    StorageControl::builder()
        .build()
        .await
        .map_err(|e| ApiErrorKind::Internal(format!("GCS builder error: {e}")).into())
}

pub async fn delete_payload(gs_url: &str) -> Result<(), ApiError> {
    let client = gcs_control_client().await?;
    delete_payload_with(&client, gs_url).await
}

/// Allowed a provided control client so a stub can be injected for testing.
async fn delete_payload_with(client: &StorageControl, gs_url: &str) -> Result<(), ApiError> {
    let (bucket, object) = parse_gs_url(gs_url)?;
    client
        .delete_object()
        .set_bucket(bucket_path(bucket))
        .set_object(object)
        .send()
        .await
        .inspect_err(|e| warn!("gcs payload cleanup failed for {gs_url}: {e}"))
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

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{Arc, Mutex},
    };

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

        delete_payload_with(&client, "gs://test-bucket/uid/bookmarks/bid/uuid")
            .await
            .expect("delete_payload_with should succeed");

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
    async fn delete_payload_surfaces_delete_error() {
        let client = StorageControl::from_stub(FailingStub);

        let result = delete_payload_with(&client, "gs://test-bucket/uid/bookmarks/bid/uuid").await;

        assert!(
            result.is_err(),
            "a failed GCS delete should surface as an error"
        );
    }
}
