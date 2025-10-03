use actix_web::{
    dev::Payload, http::header::HeaderMap, web::Data, Error, FromRequest, HttpRequest,
};
use futures::future::{FutureExt, LocalBoxFuture};
use serde::Serialize;

use syncstorage_db::{DbError, DbPool};

use super::RequestErrorLocation;
use crate::{server::ServerState, web::error::ValidationErrorKind};

/// Quota information for heartbeat responses
#[derive(Clone, Copy, Debug, Serialize)]
pub struct QuotaInfo {
    pub enabled: bool,
    pub size: u32,
}

#[derive(Clone, Debug)]
pub struct HeartbeatRequest {
    pub headers: HeaderMap,
    pub db_pool: Box<dyn DbPool<Error = DbError>>,
    pub quota: QuotaInfo,
}

impl FromRequest for HeartbeatRequest {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();

        async move {
            let headers = req.headers().clone();
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        None,
                    )
                    .into());
                }
            };
            let db_pool = state.db_pool.clone();
            let quota = QuotaInfo {
                enabled: state.quota_enabled,
                size: state.limits.max_quota_limit,
            };

            Ok(HeartbeatRequest {
                headers,
                db_pool,
                quota,
            })
        }
        .boxed_local()
    }
}
