// pub mod db;
pub mod rejectua;
pub mod sentry;
pub mod weave;

// # Web Middleware
//
// Matches the [Sync Storage middleware](https://github.com/mozilla-services/server-syncstorage/blob/master/syncstorage/tweens.py) (tweens).

use actix_web::{dev::ServiceRequest, Error, HttpRequest};

use crate::db::util::SyncTimestamp;
use crate::error::{ApiError, ApiErrorKind};
use crate::server::ServerState;
use crate::web::{extractors::HawkIdentifier, DOCKER_FLOW_ENDPOINTS};
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
        let state = &self.app_data::<ServerState>().ok_or_else(|| -> ApiError {
            ApiErrorKind::Internal("No app_data ServerState".to_owned()).into()
        })?;
        HawkIdentifier::extrude(self, &method.as_str(), &self.uri(), &ci, &state)
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
        let state = &self
            .app_data::<Data<ServerState>>()
            .ok_or_else(|| -> ApiError {
                ApiErrorKind::Internal("No app_data ServerState".to_owned()).into()
            })?;
        HawkIdentifier::extrude(self, &method.as_str(), &self.uri(), &ci, &state)
    }
}
