use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::impl_fmt_display;
use thiserror::Error;

#[derive(Debug)]
pub struct DbError {
    kind: DbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
pub enum DbErrorKind {
    #[error("Specified collection does not exist")]
    CollectionNotFound,

    #[error("Specified bso does not exist")]
    BsoNotFound,

    #[error("Specified batch does not exist")]
    BatchNotFound,

    #[error("An attempt at a conflicting write")]
    Conflict,

    #[error("Invalid database URL: {}", _0)]
    InvalidUrl(String),

    #[error("Unexpected error: {}", _0)]
    Internal(String),

    #[error("User over quota")]
    Quota,
}

impl DbError {
    pub fn internal(msg: &str) -> Self {
        DbErrorKind::Internal(msg.to_owned()).into()
    }
}

pub trait DbErrorIntrospect {
    fn is_sentry_event(&self) -> bool;
    fn metric_label(&self) -> Option<String>;
    fn is_collection_not_found(&self) -> bool;
    fn is_conflict(&self) -> bool;
    fn is_quota(&self) -> bool;
    fn is_bso_not_found(&self) -> bool;
    fn is_batch_not_found(&self) -> bool;
}

impl DbErrorIntrospect for DbError {
    fn is_sentry_event(&self) -> bool {
        !matches!(&self.kind, DbErrorKind::Conflict)
    }

    fn metric_label(&self) -> Option<String> {
        match &self.kind {
            DbErrorKind::Conflict => Some("storage.conflict".to_owned()),
            _ => None,
        }
    }

    fn is_collection_not_found(&self) -> bool {
        matches!(self.kind, DbErrorKind::CollectionNotFound)
    }

    fn is_conflict(&self) -> bool {
        matches!(self.kind, DbErrorKind::Conflict)
    }

    fn is_quota(&self) -> bool {
        matches!(self.kind, DbErrorKind::Quota)
    }

    fn is_bso_not_found(&self) -> bool {
        matches!(self.kind, DbErrorKind::BsoNotFound)
    }

    fn is_batch_not_found(&self) -> bool {
        matches!(self.kind, DbErrorKind::BatchNotFound)
    }
}

impl From<DbErrorKind> for DbError {
    fn from(kind: DbErrorKind) -> Self {
        let status = match kind {
            DbErrorKind::CollectionNotFound | DbErrorKind::BsoNotFound => StatusCode::NOT_FOUND,
            // Matching the Python code here (a 400 vs 404)
            DbErrorKind::BatchNotFound => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            DbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            DbErrorKind::Quota => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            kind,
            status,
            backtrace: Backtrace::new(),
        }
    }
}

impl_fmt_display!(DbError, DbErrorKind);
