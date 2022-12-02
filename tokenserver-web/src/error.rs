use std::fmt;

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use syncserver_common::ReportableError;
use thiserror::Error;
use tokenserver_common::{ErrorLocation, TokenserverError};
use tokenserver_db::DbError;

#[derive(Debug, Error)]
pub struct ApiError(TokenserverError);

impl From<TokenserverError> for ApiError {
    fn from(e: TokenserverError) -> Self {
        Self(e)
    }
}

impl From<DbError> for ApiError {
    fn from(e: DbError) -> Self {
        Self(e.into())
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.0.http_status).json(ErrorResponse::from(self))
    }

    fn status_code(&self) -> StatusCode {
        self.0.http_status
    }
}

impl ReportableError for ApiError {
    fn error_backtrace(&self) -> String {
        self.0.error_backtrace()
    }

    fn is_sentry_event(&self) -> bool {
        self.0.is_sentry_event()
    }

    fn metric_label(&self) -> Option<String> {
        self.0.metric_label()
    }
}

#[derive(Debug)]
pub struct ErrorResponse {
    status: &'static str,
    errors: [ErrorInstance; 1],
}

#[derive(Debug)]
struct ErrorInstance {
    location: ErrorLocation,
    name: String,
    description: String,
}

impl From<&ApiError> for ErrorResponse {
    fn from(ApiError(error): &ApiError) -> Self {
        Self {
            status: error.status,
            errors: [ErrorInstance {
                location: error.location,
                name: error.name.clone(),
                description: error.description.clone(),
            }],
        }
    }
}

impl Serialize for ApiError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(&ErrorResponse::from(self), serializer)
    }
}

impl Serialize for ErrorInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(3))?;
        map.serialize_entry("location", &self.location.to_string())?;
        map.serialize_entry("name", &self.name)?;
        map.serialize_entry("description", &self.description)?;
        map.end()
    }
}

impl Serialize for ErrorResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("status", &self.status)?;
        map.serialize_entry("errors", &self.errors)?;
        map.end()
    }
}
