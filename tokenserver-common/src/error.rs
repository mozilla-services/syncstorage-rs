use std::{cmp::PartialEq, error::Error, fmt};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use backtrace::Backtrace;
use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};
use syncstorage_common::ReportableError;
use syncstorage_db_common::error::DbError;

#[derive(Clone, Debug)]
pub struct TokenserverError {
    pub status: &'static str,
    pub location: ErrorLocation,
    pub name: String,
    pub description: String,
    pub http_status: StatusCode,
    pub backtrace: Backtrace,
    /// The label used to report this error as a metric. A metric will be emitted for this error
    /// if this field is `Some` OR if the HTTP status is 4XX. If the HTTP status is 4XX, a label
    /// of `"other"` will be applied to the metric emission.
    pub metric_label: Option<&'static str>,
    /// For internal use only. Used to report any additional context behind an error to
    /// distinguish between similar errors in Sentry. The error will be reported to Sentry if and
    /// only if this field is `Some`.
    pub context: Option<String>,
}

impl Error for TokenserverError {}

// We implement `PartialEq` manually here because `Backtrace` doesn't implement `PartialEq`, so we
// can't derive it
impl PartialEq for TokenserverError {
    fn eq(&self, other: &Self) -> bool {
        self.status == other.status
            && self.location == other.location
            && self.name == other.name
            && self.description == other.description
            && self.http_status == other.http_status
            && self.context == other.context
            && self.metric_label == other.metric_label
    }
}

impl fmt::Display for TokenserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.context.clone().unwrap_or_else(|| "".to_owned())
        )
    }
}

impl Default for TokenserverError {
    fn default() -> Self {
        Self {
            status: "error",
            location: ErrorLocation::default(),
            name: "".to_owned(),
            description: "Unauthorized".to_owned(),
            http_status: StatusCode::UNAUTHORIZED,
            context: None,
            backtrace: Backtrace::new(),
            metric_label: None,
        }
    }
}

impl TokenserverError {
    pub fn invalid_generation() -> Self {
        Self {
            status: "invalid-generation",
            location: ErrorLocation::Body,
            ..Self::default()
        }
    }

    pub fn invalid_keys_changed_at() -> Self {
        Self {
            status: "invalid-keysChangedAt",
            location: ErrorLocation::Body,
            ..Self::default()
        }
    }

    pub fn invalid_key_id(description: String) -> Self {
        Self {
            status: "invalid-key-id",
            description,
            ..Self::default()
        }
    }

    pub fn invalid_credentials(description: String) -> Self {
        Self {
            status: "invalid-credentials",
            location: ErrorLocation::Body,
            description,
            ..Self::default()
        }
    }

    pub fn invalid_client_state(description: String) -> Self {
        Self {
            status: "invalid-client-state",
            description,
            name: "X-Client-State".to_owned(),
            ..Self::default()
        }
    }

    pub fn internal_error(context: String) -> Self {
        Self {
            status: "internal-error",
            location: ErrorLocation::Internal,
            description: "Server error".to_owned(),
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
            context: Some(context),
            ..Self::default()
        }
    }

    pub fn resource_unavailable(context: String) -> Self {
        Self {
            location: ErrorLocation::Body,
            description: "Resource is not available".to_owned(),
            http_status: StatusCode::SERVICE_UNAVAILABLE,
            context: Some(context),
            ..Default::default()
        }
    }

    pub fn unsupported(description: String, name: String) -> Self {
        Self {
            status: "error",
            location: ErrorLocation::Url,
            description,
            name,
            http_status: StatusCode::NOT_FOUND,
            ..Self::default()
        }
    }

    pub fn unauthorized(description: String) -> Self {
        Self {
            location: ErrorLocation::Body,
            description,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorLocation {
    Header,
    Url,
    Body,
    Internal,
}

impl Default for ErrorLocation {
    fn default() -> Self {
        Self::Header
    }
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
    description: String,
}

impl From<&TokenserverError> for ErrorResponse {
    fn from(error: &TokenserverError) -> Self {
        ErrorResponse {
            status: error.status,
            errors: [ErrorInstance {
                location: error.location,
                name: error.name.clone(),
                description: error.description.clone(),
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

impl From<DbError> for TokenserverError {
    fn from(db_error: DbError) -> Self {
        TokenserverError {
            description: db_error.to_string(),
            // We always want to report this error to Sentry, since any unhandled DbError is an
            // internal error we want to be made aware of
            context: Some(db_error.to_string()),
            backtrace: db_error.backtrace,
            http_status: if db_error.status.is_server_error() {
                // Use the status code from the DbError if it already suggests an internal error;
                // it might be more specific than `StatusCode::SERVICE_UNAVAILABLE`
                db_error.status
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            // An unhandled DbError in the Tokenserver code is an internal error
            ..TokenserverError::internal_error("Unhandled database error".to_owned())
        }
    }
}

impl From<TokenserverError> for HttpResponse {
    fn from(inner: TokenserverError) -> Self {
        ResponseError::error_response(&inner)
    }
}

impl ReportableError for TokenserverError {
    fn error_backtrace(&self) -> String {
        format!("{:#?}", self.backtrace)
    }

    fn is_sentry_event(&self) -> bool {
        self.context.is_some()
    }

    fn metric_label(&self) -> Option<String> {
        if self.http_status.is_client_error() {
            Some(format!(
                "error.{}",
                self.metric_label().unwrap_or_else(|| "other".to_owned())
            ))
        } else {
            None
        }
    }
}
