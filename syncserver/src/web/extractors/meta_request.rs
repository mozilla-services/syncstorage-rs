use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{FutureExt, LocalBoxFuture};

use syncserver_common::Metrics;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use super::HawkIdentifier;
use crate::server::MetricsWrapper;

/// Information Requests extractor
///
/// Only the database and user identifier is required for information
/// requests: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#general-info
pub struct MetaRequest {
    pub user_id: UserIdentifier,
    pub tokenserver_origin: TokenserverOrigin,
    pub metrics: Metrics,
}

impl FromRequest for MetaRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        async move {
            // Call the precondition stuff to init database handles and what-not
            let user_id = HawkIdentifier::from_request(&req, &mut payload).await?;

            Ok(MetaRequest {
                tokenserver_origin: user_id.tokenserver_origin,
                user_id: user_id.into(),
                metrics: MetricsWrapper::extract(&req).await?.0,
            })
        }
        .boxed_local()
    }
}
