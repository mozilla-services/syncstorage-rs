pub mod rejectua;
pub mod sentry;
pub mod weave;

// # Web Middleware
//
// Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use std::{future::Future, sync::Arc};

use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse},
    Error, HttpRequest,
};

use crate::db::util::SyncTimestamp;
use crate::error::{ApiError, ApiErrorKind, WeaveError};
use crate::server::{metrics::Metrics, ServerState};
use crate::settings::Secrets;
use crate::tokenserver::auth::TokenserverOrigin;
use crate::web::{extractors::HawkIdentifier, tags::Tags, DOCKER_FLOW_ENDPOINTS};
use actix_web::web::Data;

/// The resource in question's Timestamp
pub struct ResourceTimestamp(SyncTimestamp);

pub trait SyncServerRequest {
    fn get_hawk_id(&self) -> Result<HawkIdentifier, Error>;
}

impl SyncServerRequest for ServiceRequest {
    fn get_hawk_id(&self) -> Result<HawkIdentifier, Error> {
        if DOCKER_FLOW_ENDPOINTS.contains(&self.uri().path().to_lowercase().as_str()) {
            return Ok(HawkIdentifier::cmd_dummy());
        }
        let method = self.method().clone();
        // NOTE: `connection_info()` gets a mutable reference lock on `extensions()`, so
        // it must be cloned
        let ci = &self.connection_info().clone();
        let secrets = &self
            .app_data::<Data<Arc<Secrets>>>()
            .ok_or_else(|| -> ApiError {
                ApiErrorKind::Internal("No app_data Secrets".to_owned()).into()
            })?;
        HawkIdentifier::extrude(self, method.as_str(), self.uri(), ci, secrets)
    }
}

impl SyncServerRequest for HttpRequest {
    fn get_hawk_id(&self) -> Result<HawkIdentifier, Error> {
        if DOCKER_FLOW_ENDPOINTS.contains(&self.uri().path().to_lowercase().as_str()) {
            return Ok(HawkIdentifier::cmd_dummy());
        }
        let method = self.method().clone();
        // NOTE: `connection_info()` gets a mutable reference lock on `extensions()`, so
        // it must be cloned
        let ci = &self.connection_info().clone();
        let secrets = &self
            .app_data::<Data<Arc<Secrets>>>()
            .ok_or_else(|| -> ApiError {
                ApiErrorKind::Internal("No app_data Secrets".to_owned()).into()
            })?;
        HawkIdentifier::extrude(self, method.as_str(), self.uri(), ci, secrets)
    }
}

pub fn emit_http_status_with_tokenserver_origin(
    req: ServiceRequest,
    srv: &mut impl Service<
        Request = ServiceRequest,
        Response = ServiceResponse,
        Error = actix_web::Error,
    >,
) -> impl Future<Output = Result<ServiceResponse, actix_web::Error>> {
    let fut = srv.call(req);

    async move {
        let res = fut.await?;
        let req = res.request();
        let metrics = {
            let statsd_client = req
                .app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone())
                .ok_or_else(|| ApiError::from(ApiErrorKind::NoServerState))?;

            Metrics::from(&*statsd_client)
        };
        let tags = req
            .extensions()
            .get::<TokenserverOrigin>()
            .copied()
            .map(|origin| {
                let mut tags = Tags::default();

                tags.tags
                    .insert("tokenserver_origin".to_string(), origin.to_string());

                tags
            });

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

pub fn emit_metric_for_4xx_error(
    req: ServiceRequest,
    srv: &mut impl Service<
        Request = ServiceRequest,
        Response = ServiceResponse,
        Error = actix_web::Error,
    >,
) -> impl Future<Output = Result<ServiceResponse, actix_web::Error>> {
    let fut = srv.call(req);

    async move {
        let sresp = fut.await?;
        let metrics = {
            let statsd_client = sresp
                .request()
                .app_data::<Data<ServerState>>()
                .map(|state| state.metrics.clone())
                .ok_or_else(|| ApiError::from(ApiErrorKind::NoServerState))?;

            Metrics::from(&*statsd_client)
        };

        if sresp.status().is_client_error() {
            let mut tags = Tags::default();
            let weave_error_code = sresp
                .response()
                .error()
                .map(ToString::to_string)
                .unwrap_or_else(|| (WeaveError::UnknownError as i32).to_string());

            tags.tags
                .insert("weave_error_code".to_owned(), weave_error_code);
            metrics.incr_with_tags(&format!("http_{}", sresp.status()), Some(tags))
        }

        Ok(sresp)
    }
}
