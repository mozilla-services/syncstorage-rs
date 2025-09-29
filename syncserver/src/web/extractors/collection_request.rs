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
            get_accepted, BsoQueryParams, CollectionParam, HawkIdentifier, RequestErrorLocation,
            ACCEPTED_CONTENT_TYPES,
        },
    },
};

/// Desired reply format for a Collection Get request
#[derive(Copy, Clone, Debug)]
pub enum ReplyFormat {
    Json,
    Newlines,
}

/// Collection Request Delete/Get extractor
///
/// Extracts/validates information needed for collection delete/get requests.
pub struct CollectionRequest {
    pub collection: String,
    pub user_id: UserIdentifier,
    pub tokenserver_origin: TokenserverOrigin,
    pub query: BsoQueryParams,
    pub reply: ReplyFormat,
    pub metrics: Metrics,
}

impl FromRequest for CollectionRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        async move {
            let (user_id, query, collection) =
                <(HawkIdentifier, BsoQueryParams, CollectionParam)>::from_request(
                    &req,
                    &mut payload,
                )
                .await?;
            let collection = collection.collection;

            let accept = get_accepted(&req, &ACCEPTED_CONTENT_TYPES, "application/json");
            let reply = match accept.as_str() {
                "application/newlines" => ReplyFormat::Newlines,
                "application/json" | "" => ReplyFormat::Json,
                _ => {
                    return Err(ValidationErrorKind::FromDetails(
                        format!("Invalid Accept header specified: {:?}", accept),
                        RequestErrorLocation::Header,
                        Some("accept".to_string()),
                        Some("request.validate.invalid_accept_header"),
                    )
                    .into());
                }
            };

            Ok(CollectionRequest {
                collection,
                tokenserver_origin: user_id.tokenserver_origin,
                user_id: user_id.into(),
                query,
                reply,
                metrics: MetricsWrapper::extract(&req).await?.0,
            })
        }
        .boxed_local()
    }
}
