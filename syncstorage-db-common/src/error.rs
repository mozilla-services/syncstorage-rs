use std::fmt;

use backtrace::Backtrace;
use http::StatusCode;
use syncserver_common::impl_fmt_display;
use thiserror::Error;

/// Errors common to all supported syncstorage database backends. These errors can be thought of
/// as being related more to the syncstorage application logic as opposed to a particular
/// database backend.
#[derive(Debug)]
pub struct CommonDbError {
    kind: CommonDbErrorKind,
    pub status: StatusCode,
    pub backtrace: Backtrace,
}

#[derive(Debug, Error)]
enum CommonDbErrorKind {
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

impl CommonDbError {
    pub fn batch_not_found() -> Self {
        CommonDbErrorKind::BatchNotFound.into()
    }

    pub fn bso_not_found() -> Self {
        CommonDbErrorKind::BsoNotFound.into()
    }

    pub fn collection_not_found() -> Self {
        CommonDbErrorKind::CollectionNotFound.into()
    }

    pub fn conflict() -> Self {
        CommonDbErrorKind::Conflict.into()
    }

    pub fn internal(msg: String) -> Self {
        CommonDbErrorKind::Internal(msg).into()
    }

    pub fn quota() -> Self {
        CommonDbErrorKind::Quota.into()
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

impl DbErrorIntrospect for CommonDbError {
    fn is_sentry_event(&self) -> bool {
        !matches!(&self.kind, CommonDbErrorKind::Conflict)
    }

    fn metric_label(&self) -> Option<String> {
        match &self.kind {
            CommonDbErrorKind::Conflict => Some("storage.conflict".to_owned()),
            _ => None,
        }
    }

    fn is_collection_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::CollectionNotFound)
    }

    fn is_conflict(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::Conflict)
    }

    fn is_quota(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::Quota)
    }

    fn is_bso_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::BsoNotFound)
    }

    fn is_batch_not_found(&self) -> bool {
        matches!(self.kind, CommonDbErrorKind::BatchNotFound)
    }
}

impl From<CommonDbErrorKind> for CommonDbError {
    fn from(kind: CommonDbErrorKind) -> Self {
        let status = match kind {
            CommonDbErrorKind::CollectionNotFound | CommonDbErrorKind::BsoNotFound => {
                StatusCode::NOT_FOUND
            }
            // Matching the Python code here (a 400 vs 404)
            CommonDbErrorKind::BatchNotFound => StatusCode::BAD_REQUEST,
            // NOTE: the protocol specification states that we should return a
            // "409 Conflict" response here, but clients currently do not
            // handle these respones very well:
            //  * desktop bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959034
            //  * android bug: https://bugzilla.mozilla.org/show_bug.cgi?id=959032
            CommonDbErrorKind::Conflict => StatusCode::SERVICE_UNAVAILABLE,
            CommonDbErrorKind::Quota => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        Self {
            kind,
            status,
            backtrace: Backtrace::new(),
        }
    }
}

impl_fmt_display!(CommonDbError, CommonDbErrorKind);
