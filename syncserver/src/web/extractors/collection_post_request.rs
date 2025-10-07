use actix_web::{dev::Payload, web::Data, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;

use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use super::{
    BatchRequest, BatchRequestOpt, BsoBodies, BsoQueryParams, CollectionParam, HawkIdentifier,
    RequestErrorLocation, KNOWN_BAD_PAYLOAD_REGEX,
};
use crate::{
    server::{MetricsWrapper, ServerState},
    web::error::ValidationErrorKind,
};

/// Collection Request Post extractor
///
/// Iterates over a list of BSOs in the request body and PUTs them into the
/// database with the same timestamp.
/// Extracts/validates information needed for batch collection POST requests.
pub struct CollectionPostRequest {
    pub collection: String,
    pub user_id: UserIdentifier,
    pub tokenserver_origin: TokenserverOrigin,
    pub query: BsoQueryParams,
    pub bsos: BsoBodies,
    pub batch: Option<BatchRequest>,
    pub metrics: Metrics,
    pub quota_enabled: bool,
}

impl FromRequest for CollectionPostRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extractor for Collection Posts (Batch BSO upload)
    ///
    /// Utilizes the `BsoBodies` for parsing, and add's two validation steps not
    /// done previously:
    ///   - If the collection is 'crypto', known bad payloads are checked for
    ///   - Any valid BSO's beyond `BATCH_MAX_RECORDS` are moved to invalid
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();
        Box::pin(async move {
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                    .into());
                }
            };

            let max_post_records = i64::from(state.limits.max_post_records);

            let (user_id, collection, query, mut bsos) =
                <(HawkIdentifier, CollectionParam, BsoQueryParams, BsoBodies)>::from_request(
                    &req,
                    &mut payload,
                )
                .await?;

            let collection = collection.collection;
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                for bso in &bsos.valid {
                    if let Some(ref data) = bso.payload {
                        if KNOWN_BAD_PAYLOAD_REGEX.is_match(data) {
                            return Err(ValidationErrorKind::FromDetails(
                                "Known-bad BSO payload".to_owned(),
                                RequestErrorLocation::Body,
                                Some("bsos".to_owned()),
                                Some("request.process.known_bad_bso"),
                            )
                            .into());
                        }
                    }
                }
            }

            // Trim the excess BSO's to be under the batch size
            let overage: i64 = (bsos.valid.len() as i64) - max_post_records;
            if overage > 0 {
                for _ in 1..=overage {
                    if let Some(last) = bsos.valid.pop() {
                        bsos.invalid.insert(last.id, "retry bso".to_string());
                    }
                }
            }

            // XXX: let's not use extract here (maybe convert to extrude?)
            let batch = BatchRequestOpt::extract(&req).await?;
            Ok(CollectionPostRequest {
                collection,
                tokenserver_origin: user_id.tokenserver_origin,
                user_id: user_id.into(),
                query,
                bsos,
                batch: batch.opt,
                metrics: MetricsWrapper::extract(&req).await?.0,
                quota_enabled: state.quota_enabled,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{dev::ServiceResponse, http::Method, test::TestRequest, HttpResponse};
    use serde_json::json;

    use crate::web::extractors::test_utils::{
        extract_body_as_str, make_state, post_collection, USER_ID,
    };

    #[actix_rt::test]
    async fn test_valid_collection_post_request() {
        // Batch requests require id's on each BSO
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        assert!(result.batch.is_none());
    }

    #[actix_rt::test]
    async fn test_invalid_collection_post_request() {
        // Add extra fields, these will be invalid
        let bso_body = json!([
            {"id": "1", "sortindex": 23, "jump": 1},
            {"id": "2", "sortindex": -99, "hop": "low"}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_invalid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.invalid.len(), 2);
    }

    #[actix_rt::test]
    async fn test_valid_collection_batch_post_request() {
        // If the "batch" parameter is has no value or has a value of "true"
        // then a new batch will be created.
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let result = post_collection("batch=True", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_batch_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        let batch = result
            .batch
            .expect("Could not get batch in test_valid_collection_batch_post_request");
        assert!(batch.id.is_none());
        assert!(!batch.commit);

        let result2 = post_collection("batch", &bso_body)
            .await
            .expect("Could not get result2 in test_valid_collection_batch_post_request");
        let batch2 = result2
            .batch
            .expect("Could not get batch2 in test_valid_collection_batch_post_request");
        assert!(batch2.id.is_none());
        assert!(!batch2.commit);

        let result3 = post_collection("batch=MTI%3D&commit=true", &bso_body)
            .await
            .expect("Could not get result3 in test_valid_collection_batch_post_request");
        let batch3 = result3
            .batch
            .expect("Could not get batch3 in test_valid_collection_batch_post_request");
        assert!(batch3.id.is_some());
        assert!(batch3.commit);
    }

    #[actix_rt::test]
    async fn test_invalid_collection_batch_post_request() {
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let req = TestRequest::with_uri("/")
            .method(Method::POST)
            .data(make_state())
            .to_http_request();
        let result = post_collection("commit=true", &bso_body).await;
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");
    }
}
