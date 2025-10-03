use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;

use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use super::{BsoParam, BsoQueryParams, CollectionParam, HawkIdentifier};
use crate::server::MetricsWrapper;

/// BSO Request Delete/Get extractor
///
/// Extracts/validates information needed for BSO delete/get requests.
#[derive(Debug)]
pub struct BsoRequest {
    pub collection: String,
    pub user_id: UserIdentifier,
    pub tokenserver_origin: TokenserverOrigin,
    pub query: BsoQueryParams,
    pub bso: String,
    pub metrics: Metrics,
}

impl FromRequest for BsoRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();
        Box::pin(async move {
            let (user_id, query, collection, bso) =
                <(HawkIdentifier, BsoQueryParams, CollectionParam, BsoParam)>::from_request(
                    &req,
                    &mut payload,
                )
                .await?;
            let collection = collection.collection;

            Ok(BsoRequest {
                collection,
                tokenserver_origin: user_id.tokenserver_origin,
                user_id: user_id.into(),
                query,
                bso: bso.bso,
                metrics: MetricsWrapper::extract(&req).await?.0,
            })
        })
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

    use super::BsoRequest;
    use crate::web::{
        auth::HawkPayload,
        extractors::test_utils::{
            create_valid_hawk_header, extract_body_as_str, make_db, make_state, INVALID_BSO_NAME,
            SECRETS, TEST_HOST, TEST_PORT, USER_ID, USER_ID_STR,
        },
    };

    #[test]
    fn test_valid_bso_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header =
            create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .method(Method::GET)
            .param("uid", USER_ID_STR.as_str())
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoRequest::extract(&req))
            .expect("Could not get result in test_valid_bso_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
    }

    #[test]
    fn test_invalid_bso_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!("/1.5/{}/storage/tabs/{}", *USER_ID, INVALID_BSO_NAME);
        let header =
            create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .method(Method::GET)
            // `param` sets the value that would be extracted from the tokenized URI, as if the router did it.
            .param("uid", USER_ID_STR.as_str())
            .param("collection", "tabs")
            .param("bso", INVALID_BSO_NAME)
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoRequest::extract(&req));
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
        assert_eq!(err["errors"][0]["name"], "bso");
        assert_eq!(err["errors"][0]["value"], INVALID_BSO_NAME);
        */
    }

    #[test]
    fn test_quoted_bso() {
        let payload = HawkPayload::test_default(*USER_ID);
        let altered_bso = format!("\"{{{}}}\"", *USER_ID);
        let state = make_state();
        let secrets = Arc::clone(&SECRETS);
        let uri = format!(
            "/1.5/{}/storage/tabs/{}",
            *USER_ID,
            urlencoding::encode(&altered_bso)
        );
        let header =
            create_valid_hawk_header(&payload, &secrets, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .data(secrets)
            .insert_header(("authorization", header))
            .insert_header(("accept", "application/json,text/plain:q=0.5"))
            .method(Method::GET)
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoRequest::extract(&req))
            .expect("Could not get result in test_valid_collection_request");
        // make sure the altered bsoid matches the unaltered one, without the quotes and cury braces.
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(altered_bso.as_str(), result.bso);
    }
}
