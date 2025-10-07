use actix_web::{
    dev::Payload,
    web::{Data, Query},
    Error, FromRequest, HttpRequest,
};
use futures::future::{LocalBoxFuture, TryFutureExt};
use serde::Deserialize;
use validator::{Validate, ValidationError};

use syncserver_common::X_WEAVE_RECORDS;

use super::{request_error, RequestErrorLocation, TRUE_REGEX};
use crate::{
    error::ApiError,
    server::ServerState,
    web::{error::ValidationErrorKind, transaction::DbTransactionPool},
};

/// Verifies the batch commit field is valid
pub fn validate_qs_commit(commit: &str) -> Result<(), ValidationError> {
    if !TRUE_REGEX.is_match(commit) {
        return Err(request_error(
            r#"commit parameter must be "true" to apply batches"#,
            RequestErrorLocation::QueryString,
        ));
    }
    Ok(())
}

#[derive(Debug, Default, Clone, Deserialize, Validate)]
#[serde(default)]
pub struct BatchParams {
    pub batch: Option<String>,
    #[validate(custom(function = "validate_qs_commit"))]
    pub commit: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct BatchRequest {
    pub id: Option<String>,
    pub commit: bool,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct BatchRequestOpt {
    pub opt: Option<BatchRequest>,
}

impl FromRequest for BatchRequestOpt {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<BatchRequestOpt, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        Box::pin(async move {
            let params = Query::<BatchParams>::from_request(&req, &mut payload)
                .map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::QueryString,
                        None,
                        None,
                    )
                })
                .await?
                .into_inner();
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        None,
                    )
                    .into());
                }
            };

            let limits = &state.limits;

            let checks = [
                (X_WEAVE_RECORDS, limits.max_post_records),
                ("X-Weave-Bytes", limits.max_post_bytes),
                ("X-Weave-Total-Records", limits.max_total_records),
                ("X-Weave-Total-Bytes", limits.max_total_bytes),
            ];
            for (header, limit) in &checks {
                let value = match req.headers().get(*header) {
                    Some(value) => value.to_str().map_err(|e| {
                        let err: ApiError = ValidationErrorKind::FromDetails(
                            e.to_string(),
                            RequestErrorLocation::Header,
                            Some((*header).to_owned()),
                            None,
                        )
                        .into();
                        err
                    })?,
                    None => continue,
                };
                let count = value.parse::<u32>().map_err(|_| {
                    let err: ApiError = ValidationErrorKind::FromDetails(
                        format!("Invalid integer value: {}", value),
                        RequestErrorLocation::Header,
                        Some((*header).to_owned()),
                        Some("request.validate.batch.invalid_x_weave"),
                    )
                    .into();
                    err
                })?;
                if count > *limit {
                    return Err(ValidationErrorKind::FromDetails(
                        "size-limit-exceeded".to_owned(),
                        RequestErrorLocation::Header,
                        None,
                        Some("request.validate.batch.size_exceeded"),
                    )
                    .into());
                }
            }

            if params.batch.is_none() && params.commit.is_none() {
                // No batch options requested
                return Ok(Self { opt: None });
            } else if params.batch.is_none() {
                // commit w/ no batch ID is an error
                return Err(ValidationErrorKind::FromDetails(
                    "Commit with no batch specified".to_string(),
                    RequestErrorLocation::Path,
                    None,
                    Some("request.validate.batch.missing_id"),
                )
                .into());
            }

            params.validate().map_err(|e| {
                let err: ApiError = ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::QueryString,
                    None,
                )
                .into();
                err
            })?;

            let id = match params.batch {
                None => None,
                Some(ref batch) if batch.is_empty() || TRUE_REGEX.is_match(batch) => None,
                Some(batch) => {
                    let transaction_pool = DbTransactionPool::extract(&req).await?;
                    let pool = transaction_pool.get_pool()?;

                    if pool.validate_batch_id(batch.clone()).is_err() {
                        return Err(ValidationErrorKind::FromDetails(
                            format!(r#"Invalid batch ID: "{}""#, batch),
                            RequestErrorLocation::QueryString,
                            Some("batch".to_owned()),
                            Some("request.validate.batch.invalid_id"),
                        )
                        .into());
                    }
                    Some(batch)
                }
            };

            Ok(Self {
                opt: Some(BatchRequest {
                    id,
                    commit: params.commit.is_some(),
                }),
            })
        })
    }
}
