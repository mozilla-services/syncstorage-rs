use actix_web::{dev::Payload, web::Data, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;
use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use crate::{
    server::{MetricsWrapper, ServerState},
    web::{
        error::ValidationErrorKind,
        extractors::{
            BatchRequest, BatchRequestOpt, BsoBodies, BsoQueryParams, CollectionParam,
            HawkIdentifier, RequestErrorLocation, KNOWN_BAD_PAYLOAD_REGEX,
        },
    },
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
