use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{FutureExt, LocalBoxFuture};
use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use crate::{
    server::MetricsWrapper,
    web::{
        error::ValidationErrorKind,
        extractors::{
            BsoBody, BsoParam, BsoQueryParams, CollectionParam, HawkIdentifier,
            RequestErrorLocation, KNOWN_BAD_PAYLOAD_REGEX,
        },
    },
};

/// BSO Request Put extractor
///
/// Extracts/validates information needed for BSO put requests.
pub struct BsoPutRequest {
    pub collection: String,
    pub user_id: UserIdentifier,
    pub tokenserver_origin: TokenserverOrigin,
    pub query: BsoQueryParams,
    pub bso: String,
    pub body: BsoBody,
    pub metrics: Metrics,
}

impl FromRequest for BsoPutRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        async move {
            let metrics = MetricsWrapper::extract(&req).await?.0;
            let (user_id, collection, query, bso, body) =
                <(
                    HawkIdentifier,
                    CollectionParam,
                    BsoQueryParams,
                    BsoParam,
                    BsoBody,
                )>::from_request(&req, &mut payload)
                .await?;

            let collection = collection.collection;
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                if let Some(ref data) = body.payload {
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
            Ok(BsoPutRequest {
                collection,
                tokenserver_origin: user_id.tokenserver_origin,
                user_id: user_id.into(),
                query,
                bso: bso.bso,
                body,
                metrics,
            })
        }
        .boxed_local()
    }
}
