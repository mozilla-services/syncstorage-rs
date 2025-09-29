use std::collections::{HashMap, HashSet};

use actix_web::{
    dev::Payload,
    http::header::{ContentType, Header},
    web::Data,
    Error, FromRequest, HttpRequest,
};
use futures::{
    future::{self, LocalBoxFuture},
    TryFutureExt,
};
use serde::Deserialize;
use serde_json::Value;

use super::{BatchBsoBody, RequestErrorLocation, ACCEPTED_CONTENT_TYPES};
use crate::{server::ServerState, web::error::ValidationErrorKind};

#[derive(Default, Deserialize)]
pub struct BsoBodies {
    pub valid: Vec<BatchBsoBody>,
    pub invalid: HashMap<String, String>,
}

impl FromRequest for BsoBodies {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract the BSO Bodies from the request
    ///
    /// This extraction ensures the following conditions:
    ///   - Total payload size does not exceed `BATCH_MAX_BYTES`
    ///   - All BSO's deserialize from the request correctly
    ///   - Request content-type is a valid value
    ///   - Valid BSO's include a BSO id
    ///
    /// No collection id is used, so payload checks are not done here.
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // Only try and parse the body if its a valid content-type
        let ctype = match ContentType::parse(req) {
            Ok(v) => v,
            Err(e) => {
                return Box::pin(future::err(
                    ValidationErrorKind::FromDetails(
                        format!("Unreadable Content-Type: {:?}", e),
                        RequestErrorLocation::Header,
                        Some("Content-Type".to_owned()),
                        Some("request.error.invalid_content_type"),
                    )
                    .into(),
                ))
            }
        };
        let content_type = format!("{}/{}", ctype.type_(), ctype.subtype());
        trace!("BSO Body content_type: {:?}", &content_type);

        if !ACCEPTED_CONTENT_TYPES.contains(&content_type.as_ref()) {
            return Box::pin(future::err(
                ValidationErrorKind::FromDetails(
                    format!("Invalid Content-Type {:?}", content_type),
                    RequestErrorLocation::Header,
                    Some("Content-Type".to_owned()),
                    Some("request.error.invalid_content_type"),
                )
                .into(),
            ));
        }

        // Load the entire request into a String
        let fut = <String>::from_request(req, payload).map_err(|e| {
            warn!("⚠️ Payload read error: {:?}", e);
            ValidationErrorKind::FromDetails(
                "Mimetype/encoding/content-length error".to_owned(),
                RequestErrorLocation::Header,
                None,
                None,
            )
            .into()
        });

        // Avoid duplicating by defining our error func now, doesn't need the box wrapper
        fn make_error() -> Error {
            ValidationErrorKind::FromDetails(
                "Invalid JSON in request body".to_owned(),
                RequestErrorLocation::Body,
                Some("bsos".to_owned()),
                Some("request.validate.invalid_body_json"),
            )
            .into()
        }

        // Define a new bool to check from a static closure to release the reference on the
        // content_type header
        let newlines: bool = content_type == "application/newlines";

        // Grab the max sizes
        let state = match req.app_data::<Data<ServerState>>() {
            Some(s) => s,
            None => {
                error!("⚠️ Could not load the app state");
                return Box::pin(future::err(
                    ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                    .into(),
                ));
            }
        };

        let max_payload_size = state.limits.max_record_payload_bytes as usize;
        let max_post_bytes = state.limits.max_post_bytes as usize;

        let fut = fut.and_then(move |body| {
            // Get all the raw / values
            let bsos: Vec<Value> = if newlines {
                let mut bsos = Vec::new();
                for item in body.lines() {
                    // Check that its a valid JSON map like we expect
                    if let Ok(raw_json) = serde_json::from_str::<Value>(item) {
                        bsos.push(raw_json);
                    } else {
                        // Per Python version, BSO's must json deserialize
                        return future::err(make_error());
                    }
                }
                bsos
            } else if let Ok(json_vals) = serde_json::from_str::<Vec<Value>>(&body) {
                json_vals
            } else {
                // Per Python version, BSO's must json deserialize
                return future::err(make_error());
            };

            // Validate all the BSO's, move invalid to our other list. Assume they'll all make
            // it with our pre-allocation
            let mut valid: Vec<BatchBsoBody> = Vec::with_capacity(bsos.len());

            // Invalid BSO's are any BSO that can deserialize despite how wrong the contents are
            // per the way the Python version works.
            let mut invalid: HashMap<String, String> = HashMap::new();

            // Keep track of our total payload size
            let mut total_payload_size = 0;

            // Temporarily track the bso id's for dupe detection
            let mut bso_ids: HashSet<String> = HashSet::with_capacity(bsos.len());

            for bso in bsos {
                // Error out if its not a JSON mapping type
                if !bso.is_object() {
                    return future::err(make_error());
                }
                // Save all id's we get, check for missing id, or duplicate.
                let bso_id = if let Some(id) = bso.get("id").and_then(serde_json::Value::as_str) {
                    let id = id.to_string();
                    if bso_ids.contains(&id) {
                        return future::err(
                            ValidationErrorKind::FromDetails(
                                "Input BSO has duplicate ID".to_owned(),
                                RequestErrorLocation::Body,
                                Some("bsos".to_owned()),
                                Some("request.store.duplicate_bso_id"),
                            )
                            .into(),
                        );
                    } else {
                        bso_ids.insert(id.clone());
                        id
                    }
                } else {
                    return future::err(
                        ValidationErrorKind::FromDetails(
                            "Input BSO has no ID".to_owned(),
                            RequestErrorLocation::Body,
                            Some("bsos".to_owned()),
                            Some("request.store.missing_bso_id"),
                        )
                        .into(),
                    );
                };
                match BatchBsoBody::from_raw_bso(bso) {
                    Ok(b) => {
                        // Is this record too large? Deny if it is.
                        let payload_size = b
                            .payload
                            .as_ref()
                            .map(std::string::String::len)
                            .unwrap_or_default();
                        total_payload_size += payload_size;
                        if payload_size <= max_payload_size && total_payload_size <= max_post_bytes
                        {
                            valid.push(b);
                        } else {
                            invalid.insert(b.id, "retry bytes".to_string());
                        }
                    }
                    Err(e) => {
                        invalid.insert(bso_id, e);
                    }
                }
            }
            future::ok(BsoBodies { valid, invalid })
        });

        Box::pin(fut)
    }
}
