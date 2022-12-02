use std::{cmp::PartialEq, error::Error, fmt};

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{InternalError, ReportableError};
use syncserver_db_common::error::MysqlError;

/// An error type that represents application-specific errors to Tokenserver. This error is not
/// used to represent database-related errors; database-related errors have their own type.
#[derive(Clone, Debug)]
pub struct TokenserverError {
    pub status: &'static str,
    pub location: ErrorLocation,
    pub name: String,
    pub description: String,
    pub http_status: StatusCode,
    /// For internal use only. Used to report any additional context behind an error to
    /// distinguish between similar errors in Sentry.
    pub context: String,
    pub backtrace: Box<Backtrace>,
    pub token_type: TokenType,
}

#[derive(Clone, Debug)]
pub enum TokenType {
    BrowserId,
    Oauth,
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
    }
}

impl fmt::Display for TokenserverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.context)
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
            context: "Unauthorized".to_owned(),
            backtrace: Box::new(Backtrace::new()),
            token_type: TokenType::Oauth,
        }
    }
}

impl TokenserverError {
    pub fn invalid_generation() -> Self {
        Self {
            status: "invalid-generation",
            location: ErrorLocation::Body,
            context: "Invalid generation".to_owned(),
            ..Self::default()
        }
    }

    pub fn invalid_keys_changed_at() -> Self {
        Self {
            status: "invalid-keysChangedAt",
            location: ErrorLocation::Body,
            context: "Invalid keys_changed_at".to_owned(),
            ..Self::default()
        }
    }

    pub fn invalid_key_id(description: String) -> Self {
        Self {
            status: "invalid-key-id",
            context: description.clone(),
            description,
            ..Self::default()
        }
    }

    pub fn invalid_credentials(description: String) -> Self {
        Self {
            status: "invalid-credentials",
            location: ErrorLocation::Body,
            context: description.clone(),
            description,
            ..Self::default()
        }
    }

    pub fn invalid_client_state(description: String) -> Self {
        Self {
            status: "invalid-client-state",
            context: description.clone(),
            description,
            name: "X-Client-State".to_owned(),
            ..Self::default()
        }
    }

    pub fn internal_error() -> Self {
        Self {
            status: "internal-error",
            location: ErrorLocation::Internal,
            description: "Server error".to_owned(),
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
            context: "Internal error".to_owned(),
            ..Self::default()
        }
    }

    pub fn resource_unavailable() -> Self {
        Self {
            location: ErrorLocation::Body,
            description: "Resource is not available".to_owned(),
            http_status: StatusCode::SERVICE_UNAVAILABLE,
            context: "Resource is not available".to_owned(),
            ..Default::default()
        }
    }

    pub fn unsupported(description: String, name: String) -> Self {
        Self {
            status: "error",
            location: ErrorLocation::Url,
            context: description.clone(),
            description,
            name,
            http_status: StatusCode::NOT_FOUND,
            ..Self::default()
        }
    }

    pub fn unauthorized(description: String) -> Self {
        Self {
            location: ErrorLocation::Body,
            context: description.clone(),
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

impl ReportableError for TokenserverError {
    fn error_backtrace(&self) -> String {
        format!("{:#?}", self.backtrace)
    }

    fn is_sentry_event(&self) -> bool {
        self.http_status.is_server_error() && self.metric_label().is_none()
    }

    fn metric_label(&self) -> Option<String> {
        if self.http_status.is_client_error() {
            match self.token_type {
                TokenType::BrowserId => Some("request.error.browser_id".to_owned()),
                TokenType::Oauth => Some("request.error.oauth".to_owned()),
            }
        } else {
            None
        }
    }
}

impl InternalError for TokenserverError {
    fn internal_error(message: String) -> Self {
        TokenserverError {
            context: message,
            ..TokenserverError::internal_error()
        }
    }
}

impl From<MysqlError> for TokenserverError {
    fn from(mysql_error: MysqlError) -> Self {
        TokenserverError {
            description: mysql_error.to_string(),
            context: mysql_error.to_string(),
            backtrace: mysql_error.backtrace,
            http_status: if mysql_error.status.is_server_error() {
                // Use the status code from the DbError if it already suggests an internal error;
                // it might be more specific than `StatusCode::SERVICE_UNAVAILABLE`
                mysql_error.status
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            },
            // An unhandled DbError in the Tokenserver code is an internal error
            ..TokenserverError::internal_error()
        }
    }
}
