pub mod rejectua;
pub mod weave;

// # Web Middleware
//
// Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use std::collections::HashMap;
use std::future::Future;

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    web::Data,
    HttpMessage,
};
use syncserver_common::Metrics;
use tokenserver_auth::TokenserverOrigin;

use crate::error::{ApiError, ApiErrorKind};
use crate::server::ServerState;

pub fn emit_http_status_with_tokenserver_origin<B>(
    req: ServiceRequest,
    srv: &impl Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error>,
) -> impl Future<Output = Result<ServiceResponse<B>, actix_web::Error>> {
    let fut = srv.call(req);

    async move {
        let res = fut.await?;
        let req = res.request();
        let metrics = {
            let statsd_client = req
                .app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone())
                .ok_or_else(|| ApiError::from(ApiErrorKind::NoServerState))?;

            Metrics::from(&statsd_client)
        };

        let mut tags = HashMap::default();
        if let Some(origin) = req.extensions().get::<TokenserverOrigin>().copied() {
            tags.insert("tokenserver_origin".to_string(), origin.to_string());
        };

        if res.status().is_informational() {
            metrics.incr_with_tags("http_1XX", tags);
        } else if res.status().is_success() {
            metrics.incr_with_tags("http_2XX", tags);
        } else if res.status().is_redirection() {
            metrics.incr_with_tags("http_3XX", tags);
        } else if res.status().is_client_error() {
            metrics.incr_with_tags("http_4XX", tags);
        } else if res.status().is_server_error() {
            metrics.incr_with_tags("http_5XX", tags);
        }

        Ok(res)
    }
}
