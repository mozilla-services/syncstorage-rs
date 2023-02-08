use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::{impl_fmt_display, ReportableError};
use thiserror::Error;

/// Errors common to all supported syncstorage database backends. These errors can be thought of
/// as being related more to the syncstorage application logic as opposed to a particular
/// database backend.
#[derive(Debug)]
pub struct SyncstorageDbError {
    kind: SyncstorageDbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
enum SyncstorageDbErrorKind {
    #[error("Specified collection does not exist")]
    CollectionNotFound,

    #[error("Specified bso does not exist")]
    BsoNotFound,

    #[error("Specified batch does not exist")]
    BatchNotFound,

    #[error("An attempt at a conflicting write")]
    Conflict,

    #[error("Unexpected error: {}", _0)]
    Internal(String),

    #[error("User over quota")]
    Quota,
}

impl SyncstorageDbError {
    pub fn batch_not_found() -> Self {
        SyncstorageDbErrorKind::BatchNotFound.into()
    }

    pub fn bso_not_found() -> Self {
        SyncstorageDbErrorKind::BsoNotFound.into()
    }

    pub fn collection_not_found() -> Self {
        SyncstorageDbErrorKind::CollectionNotFound.into()
    }

    pub fn conflict() -> Self {
        SyncstorageDbErrorKind::Conflict.into()
    }

    pub fn internal(msg: String) -> Self {
        SyncstorageDbErrorKind::Internal(msg).into()
    }

    pub fn quota() -> Self {
        SyncstorageDbErrorKind::Quota.into()
    }
}

pub trait DbErrorIntrospect {
    fn is_collection_not_found(&self) -> bool;
    fn is_conflict(&self) -> bool;
    fn is_quota(&self) -> bool;
    fn is_bso_not_found(&self) -> bool;
    fn is_batch_not_found(&self) -> bool;
}

impl DbErrorIntrospect for SyncstorageDbError {
    fn is_collection_not_found(&self) -> bool {
        matches!(self.kind, SyncstorageDbErrorKind::CollectionNotFound)
    }

    fn is_conflict(&self) -> bool {
        matches!(self.kind, SyncstorageDbErrorKind::Conflict)
    }

    fn is_quota(&self) -> bool {
        matches!(self.kind, SyncstorageDbErrorKind::Quota)
    }

    fn is_bso_not_found(&self) -> bool {
        matches!(self.kind, SyncstorageDbErrorKind::BsoNotFound)
    }

    fn is_batch_not_found(&self) -> bool {
        matches!(self.kind, SyncstorageDbErrorKind::BatchNotFound)
    }
}

impl ReportableError for SyncstorageDbError {
    fn is_sentry_event(&self) -> bool {
        !matches!(&self.kind, SyncstorageDbErrorKind::Conflict)
    }

    fn metric_label(&self) -> Option<String> {
        match &self.kind {
            SyncstorageDbErrorKind::Conflict => Some("storage.conflict".to_owned()),
            _ => None,
        }
    }

    fn error_backtrace(&self) -> String {
        format!("{:#?}", self.backtrace)
    }
}

impl From<SyncstorageDbErrorKind> for SyncstorageDbError {
    fn from(kind: SyncstorageDbErrorKind) -> Self {
        let status = match kind {
            SyncstorageDbErrorKind::CollectionNotFound | SyncstorageDbErrorKind::BsoNotFound => {
                StatusCode::NOT_FOUND
            }
            // Matching the Python code here (a 400 vs 404)
            SyncstorageDbErrorKind::BatchNotFound => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            SyncstorageDbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            SyncstorageDbErrorKind::Quota => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            kind,
            status,
            backtrace: Backtrace::new(),
        }
    }
}

impl_fmt_display!(SyncstorageDbError, SyncstorageDbErrorKind);
