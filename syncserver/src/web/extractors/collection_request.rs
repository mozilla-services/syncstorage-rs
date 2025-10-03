use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{FutureExt, LocalBoxFuture};

use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use super::{
    get_accepted, BsoQueryParams, CollectionParam, HawkIdentifier, RequestErrorLocation,
    ACCEPTED_CONTENT_TYPES,
};
use crate::{server::MetricsWrapper, web::error::ValidationErrorKind};

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_web::{
        dev::ServiceResponse, http::Method, test::TestRequest, FromRequest, HttpMessage,
        HttpResponse,
    };
    use futures::executor::block_on;

    use super::CollectionRequest;
    use crate::web::{
        auth::HawkPayload,
        extractors::test_utils::{
            create_valid_hawk_header, extract_body_as_str, make_db, make_state,
            INVALID_COLLECTION_NAME, SECRETS, TEST_HOST, TEST_PORT, USER_ID, USER_ID_STR,
        },
    };

    #[test]
    fn test_valid_collection_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/tabs", *USER_ID);
        let header =
            create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .insert_header(("accept", "application/json,text/plain:q=0.5"))
            .method(Method::GET)
            .param("uid", USER_ID_STR.as_str())
            .param("collection", "tabs")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(CollectionRequest::extract(&req))
            .expect("Could not get result in test_valid_collection_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
    }

    #[test]
    fn test_invalid_collection_request() {
        let hawk_payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/{}", *USER_ID, INVALID_COLLECTION_NAME);
        let header =
            create_valid_hawk_header(&hawk_payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .insert_header(("authorization", header))
            .method(Method::GET)
            .data(state)
            .data(secrets)
            .param("uid", USER_ID_STR.as_str())
            .param("collection", INVALID_COLLECTION_NAME)
            .to_http_request();
        req.extensions_mut().insert(make_db());

        let result = block_on(CollectionRequest::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "regex");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "collection");
        assert_eq!(err["errors"][0]["value"], INVALID_COLLECTION_NAME);
        */
    }
}
