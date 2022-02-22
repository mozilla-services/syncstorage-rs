use std::fmt;

use actix_web::{
    error::{BlockingError, ResponseError},
    http::StatusCode,
    HttpResponse,
};
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};

#[derive(Clone, Debug, PartialEq)]
pub struct TokenserverError {
    status: &'static str,
    location: ErrorLocation,
    name: String,
    description: &'static str,
    http_status: StatusCode,
}

impl Default for TokenserverError {
    fn default() -> Self {
        TokenserverError {
            status: "error",
            location: ErrorLocation::Header,
            name: "".to_owned(),
            description: "",
            http_status: StatusCode::UNAUTHORIZED,
        }
    }
}

#[derive(Default)]
pub struct TokenserverErrorBuilder(TokenserverError);

impl TokenserverErrorBuilder {
    pub fn build(self) -> TokenserverError {
        self.0
    }

    pub fn invalid_generation() -> Self {
        Self::default()
            .description("Unauthorized")
            .status("invalid-generation")
            .in_body()
    }

    pub fn invalid_keys_changed_at() -> Self {
        Self::default()
            .description("Unauthorized")
            .status("invalid-keysChangedAt")
            .in_body()
    }

    pub fn invalid_key_id(description: &'static str) -> Self {
        Self::default()
            .description("Unauthorized")
            .status("invalid-key-id")
            .description(description)
    }

    pub fn invalid_client_state() -> Self {
        Self::default()
            .description("Unauthorized")
            .status("invalid-client-state")
            .name("X-Client-State".to_owned())
    }

    pub fn invalid_credentials(description: &'static str) -> Self {
        Self::default()
            .status("invalid-credentials")
            .description(description)
            .in_body()
    }

    pub fn unsupported(description: &'static str, name: String) -> Self {
        Self::default()
            .in_url()
            .description(description)
            .name(name)
            .status_code(StatusCode::NOT_FOUND)
    }

    pub fn resource_unavailable() -> Self {
        Self::default()
            .in_body()
            .description("Resource is not available")
            .status_code(StatusCode::SERVICE_UNAVAILABLE)
    }

    pub fn status(mut self, status: &'static str) -> Self {
        self.0.status = status;
        self
    }

    pub fn in_body(mut self) -> Self {
        self.0.location = ErrorLocation::Body;
        self
    }

    pub fn in_header(mut self) -> Self {
        self.0.location = ErrorLocation::Header;
        self
    }

    pub fn in_url(mut self) -> Self {
        self.0.location = ErrorLocation::Url;
        self
    }

    pub fn internal() -> Self {
        let mut error = Self::default();
        error.0.location = ErrorLocation::Internal;

        error
            .status("internal-error")
            .description("Server error")
            .status_code(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn description(mut self, description: &'static str) -> Self {
        self.0.description = description;
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.0.name = name;
        self
    }

    pub fn status_code(mut self, status: StatusCode) -> Self {
        self.0.http_status = status;
        self
    }
}

impl From<BlockingError<TokenserverError>> for TokenserverError {
    fn from(inner: BlockingError<TokenserverError>) -> Self {
        match inner {
            BlockingError::Error(e) => e,
            BlockingError::Canceled => {
                error!("Tokenserver threadpool operation canceled");
                TokenserverErrorBuilder::internal().build()
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ErrorLocation {
    Header,
    Url,
    Body,
    Internal,
}

impl fmt::Display for ErrorLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header => write!(f, "header"),
            Self::Url => write!(f, "url"),
            Self::Body => write!(f, "body"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

impl fmt::Display for TokenserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).map_err(|_| fmt::Error)?
        )
    }
}

impl ResponseError for TokenserverError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.http_status).json(ErrorResponse::from(self))
    }

    fn status_code(&self) -> StatusCode {
        self.http_status
    }
}

struct ErrorResponse {
    status: &'static str,
    errors: [ErrorInstance; 1],
}

struct ErrorInstance {
    location: ErrorLocation,
    name: String,
    description: &'static str,
}

impl From<&TokenserverError> for ErrorResponse {
    fn from(error: &TokenserverError) -> Self {
        ErrorResponse {
            status: error.status,
            errors: [ErrorInstance {
                location: error.location,
                name: error.name.clone(),
                description: error.description,
            }],
        }
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

impl Serialize for TokenserverError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ErrorResponse::from(self).serialize(serializer)
    }
}
