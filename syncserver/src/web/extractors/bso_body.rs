use actix_web::{
    dev::Payload,
    http::header::{ContentType, Header},
    web::Data,
    Error, FromRequest, HttpRequest,
};
use futures::future::LocalBoxFuture;
use serde::{de::IgnoredAny, Deserialize, Serialize};
use validator::Validate;

use super::{
    validate_body_bso_id, validate_body_bso_sortindex, validate_body_bso_ttl, RequestErrorLocation,
    ACCEPTED_CONTENT_TYPES,
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
        let mut payload = payload.take();

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
                    .into())
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

            let max_payload_size = state.limits.max_record_payload_bytes as usize;

            let bso = <actix_web::web::Json<BsoBody>>::from_request(&req, &mut payload)
                .await
                .map_err(|e| {
                    warn!("⚠️ Could not parse BSO Body: {:?}", e);

                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::Body,
                        Some("bso".to_owned()),
                        Some("request.validate.bad_bso_body"),
                    )
                })?;

            // Check the max payload size manually with our desired limit
            if bso
                .payload
                .as_ref()
                .map(std::string::String::len)
                .unwrap_or_default()
                > max_payload_size
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
            Ok(bso.into_inner())
        })
    }
}
