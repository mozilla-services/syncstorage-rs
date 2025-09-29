use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{FutureExt, LocalBoxFuture};

use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use super::{
    BsoBody, BsoParam, BsoQueryParams, CollectionParam, HawkIdentifier, RequestErrorLocation,
    KNOWN_BAD_PAYLOAD_REGEX,
};
use crate::{server::MetricsWrapper, web::error::ValidationErrorKind};

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use actix_http::h1;
    use actix_web::{
        dev::ServiceResponse, http::Method, test::TestRequest, web::Bytes, FromRequest,
        HttpMessage, HttpResponse,
    };
    use futures::executor::block_on;
    use serde_json::json;

    use crate::web::{
        auth::HawkPayload,
        extractors::test_utils::{
            create_valid_hawk_header, extract_body_as_str, make_db, make_state, SECRETS, TEST_HOST,
            TEST_PORT, USER_ID, USER_ID_STR,
        },
    };

    use super::BsoPutRequest;

    #[test]
    fn test_valid_bso_post_body() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header =
            create_valid_hawk_header(&payload, &secrets, "POST", &uri, TEST_HOST, TEST_PORT);
        let bso_body = json!({
            "id": "128", "payload": "x"
        });
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .insert_header(("content-type", "application/json"))
            .method(Method::POST)
            .set_payload(bso_body.to_string())
            .param("uid", USER_ID_STR.as_str())
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let (_sender, mut payload) = h1::Payload::create(true);
        payload.unread_data(Bytes::from(bso_body.to_string()));
        let result = block_on(BsoPutRequest::from_request(&req, &mut payload.into()))
            .expect("Could not get result in test_valid_bso_post_body");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
        assert_eq!(result.body.payload, Some("x".to_string()));
    }

    #[test]
    fn test_invalid_bso_post_body() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header =
            create_valid_hawk_header(&payload, &secrets, "POST", &uri, TEST_HOST, TEST_PORT);
        let bso_body = json!({
            "payload": "xxx", "sortindex": -9_999_999_999_i64,
        });
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .insert_header(("content-type", "application/json"))
            .method(Method::POST)
            .set_payload(bso_body.to_string())
            .param("uid", USER_ID_STR.as_str())
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoPutRequest::extract(&req));
        let response: HttpResponse = result
            .err()
            .expect("Could not get response in test_invalid_bso_post_body")
            .into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "8")

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["location"], "body");
        assert_eq!(&err["errors"][0]["name"], "bso");
        */
    }
}
