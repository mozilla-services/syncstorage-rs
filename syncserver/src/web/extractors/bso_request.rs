use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::LocalBoxFuture;
use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use crate::{
    server::MetricsWrapper,
    web::extractors::{BsoParam, BsoQueryParams, CollectionParam, HawkIdentifier},
};

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
