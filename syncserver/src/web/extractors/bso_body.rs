use actix_web::{
    Error, FromRequest, HttpMessage, HttpRequest,
    body::{BodyStream, to_bytes_limited},
    dev::Payload,
    http::header::{ContentType, Header},
    web::Data,
};
use futures::future::LocalBoxFuture;
use serde::{Deserialize, Serialize, de::IgnoredAny};
use validator::Validate;

use super::{
    ACCEPTED_CONTENT_TYPES, CollectionParam, RequestErrorLocation,
    utils::{check_content_length, size_limit_exceeded},
    validate_body_bso_id, validate_body_bso_sortindex, validate_body_bso_ttl,
};
use crate::{server::ServerState, web::error::ValidationErrorKind};

#[derive(Default, Debug, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BsoBody {
    #[validate(custom(function = "validate_body_bso_id"))]
    pub id: Option<String>,
    #[validate(custom(function = "validate_body_bso_sortindex"))]
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    #[validate(custom(function = "validate_body_bso_ttl"))]
    pub ttl: Option<u32>,
    /// Any client-supplied value for these fields are ignored
    #[serde(rename(deserialize = "modified"), skip_serializing)]
    pub _ignored_modified: Option<IgnoredAny>,
    #[serde(rename(deserialize = "collection"), skip_serializing)]
    pub _ignored_collection: Option<IgnoredAny>,
}

impl FromRequest for BsoBody {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<BsoBody, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // req.clone() allows move into async block since it is borrowed
        // payload.take() grabs request body payload, replacing the one passed in
        // with an empty payload so we strictly read the request body payload once
        // and dispense with it
        let req = req.clone();
        let payload = payload.take();

        Box::pin(async move {
            // Only try and parse the body if its a valid content-type
            let ctype = match ContentType::parse(&req) {
                Ok(v) => v,
                Err(e) => {
                    return Err(ValidationErrorKind::FromDetails(
                        format!("Unreadable Content-Type: {:?}", e),
                        RequestErrorLocation::Header,
                        Some("Content-Type".to_owned()),
                        Some("request.error.invalid_content_type"),
                    )
                    .into());
                }
            };

            let content_type = format!("{}/{}", ctype.type_(), ctype.subtype());
            if !ACCEPTED_CONTENT_TYPES.contains(&content_type.as_ref()) {
                return Err(ValidationErrorKind::FromDetails(
                    "Invalid Content-Type".to_owned(),
                    RequestErrorLocation::Header,
                    Some("Content-Type".to_owned()),
                    Some("request.error.invalid_content_type"),
                )
                .into());
            }
            // A single BSO is never a series of newline delimited records.
            // `Json` used to turn this away for us before we parsed the body
            // ourselves, so keep returning what it did.
            if content_type == "application/newlines" {
                return Err(bad_bso_body("Content type error."));
            }
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                    .into());
                }
            };

            // `max_record_payload_bytes` and `max_request_bytes` can each be
            // overridden per collection.
            let collection = CollectionParam::extrude(req.uri(), &mut req.extensions_mut())
                .ok()
                .flatten()
                .map(|c| c.collection);
            let coll_limits = state.limits.limits_for(collection.as_deref());

            check_content_length(&req, coll_limits.max_request_bytes as usize)?;
            // Load the body, holding it to the same limit as it's read: a
            // chunked request has no Content-Length to check
            let body = match to_bytes_limited(
                BodyStream::new(payload),
                coll_limits.max_request_bytes as usize,
            )
            .await
            {
                Ok(Ok(body)) => body,
                Ok(Err(e)) => {
                    warn!("⚠️ Could not read BSO Body: {:?}", e);
                    return Err(bad_bso_body(&e.to_string()));
                }
                Err(_) => return Err(size_limit_exceeded().into()),
            };
            let bso: BsoBody = serde_json::from_slice(&body).map_err(|e| {
                warn!("⚠️ Could not parse BSO Body: {:?}", e);
                bad_bso_body(&e.to_string())
            })?;

            // Check the max payload size manually with our desired limit
            if bso
                .payload
                .as_ref()
                .map(std::string::String::len)
                .unwrap_or_default()
                > coll_limits.max_record_payload_bytes as usize
            {
                return Err(ValidationErrorKind::FromDetails(
                    "payload too large".to_owned(),
                    RequestErrorLocation::Body,
                    Some("bso".to_owned()),
                    Some("request.validate.payload_too_large"),
                )
                .into());
            }
            if let Err(e) = bso.validate() {
                return Err(ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::Body,
                    None,
                )
                .into());
            }
            Ok(bso)
        })
    }
}

/// Return an error for a BSO body we couldn't read or parse
fn bad_bso_body(description: &str) -> Error {
    ValidationErrorKind::FromDetails(
        description.to_owned(),
        RequestErrorLocation::Body,
        Some("bso".to_owned()),
        Some("request.validate.bad_bso_body"),
    )
    .into()
}
