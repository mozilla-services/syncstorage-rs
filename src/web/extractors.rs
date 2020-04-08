//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.
use std::{self, collections::HashMap, num::ParseIntError, str::FromStr};

use actix_web::{
    dev::{ConnectionInfo, Extensions, Payload, RequestHead},
    error::ErrorInternalServerError,
    http::{
        header::{qitem, Accept, ContentType, Header, HeaderMap},
        Uri,
    },
    web::{Data, Json, Query},
    Error, FromRequest, HttpMessage, HttpRequest,
};

use futures::future::{self, FutureExt, LocalBoxFuture, Ready, TryFutureExt};

use lazy_static::lazy_static;
use mime::STAR_STAR;
use regex::Regex;
use serde::{
    de::{Deserializer, Error as SerdeError, IgnoredAny},
    Deserialize, Serialize,
};
use serde_json::Value;
use validator::{Validate, ValidationError};

use crate::db::{util::SyncTimestamp, Db, Sorting};
use crate::error::ApiError;
use crate::server::{metrics, ServerState, BSO_ID_REGEX, COLLECTION_ID_REGEX};
use crate::settings::{Secrets, ServerLimits};
use crate::web::{
    auth::HawkPayload,
    error::{HawkErrorKind, ValidationErrorKind},
    tags::Tags,
    X_WEAVE_RECORDS,
};

const BATCH_MAX_IDS: usize = 100;

// BSO const restrictions
const BSO_MAX_TTL: u32 = 999_999_999;
const BSO_MAX_SORTINDEX_VALUE: i32 = 999_999_999;
const BSO_MIN_SORTINDEX_VALUE: i32 = -999_999_999;

const ACCEPTED_CONTENT_TYPES: [&str; 3] =
    ["application/json", "text/plain", "application/newlines"];

lazy_static! {
    static ref KNOWN_BAD_PAYLOAD_REGEX: Regex =
        Regex::new(r#"IV":\s*"AAAAAAAAAAAAAAAAAAAAAA=="#).unwrap();
    static ref VALID_ID_REGEX: Regex = Regex::new(&format!("^{}$", BSO_ID_REGEX)).unwrap();
    static ref VALID_COLLECTION_ID_REGEX: Regex =
        Regex::new(&format!("^{}$", COLLECTION_ID_REGEX)).unwrap();
    static ref TRUE_REGEX: Regex = Regex::new("^(?i)true$").unwrap();
}

#[derive(Deserialize)]
pub struct UidParam {
    #[allow(dead_code)] // Not really dead, but Rust can't see the deserialized use.
    uid: u64,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BatchBsoBody {
    #[validate(custom = "validate_body_bso_id")]
    pub id: String,
    #[validate(custom = "validate_body_bso_sortindex")]
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    #[validate(custom = "validate_body_bso_ttl")]
    pub ttl: Option<u32>,
}

impl BatchBsoBody {
    /// Function to convert valid raw JSON BSO body to a BatchBsoBody
    fn from_raw_bso(val: &Value) -> Result<BatchBsoBody, String> {
        let map = val.as_object().ok_or("invalid json")?;
        // Verify all the keys are valid. modified/collection are allowed but ignored
        let valid_keys = [
            "id",
            "sortindex",
            "payload",
            "ttl",
            "modified",
            "collection",
        ];
        for key_name in map.keys() {
            if !valid_keys.contains(&key_name.as_str()) {
                return Err(format!("unknown field {}", key_name));
            }
        }
        serde_json::from_value(val.clone())
            .map_err(|_| "invalid json".to_string())
            .and_then(|v: BatchBsoBody| match v.validate() {
                Ok(()) => Ok(v),
                Err(e) => Err(format!("invalid bso: {}", e)),
            })
    }
}

// This tries to do the right thing to get the Accepted header according to
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Accept, but some corners can absolutely be cut.
// This will pull the first accepted content type listed, or the highest rated non-accepted type.
fn get_accepted(req: &HttpRequest, accepted: &[&str], default: &'static str) -> String {
    let mut candidates = Accept::parse(req).unwrap_or_else(|_| {
        Accept(vec![qitem(
            mime::Mime::from_str(default).expect("Could not get accept in get_accepted"),
        )])
    });
    if candidates.is_empty() {
        return default.to_owned();
    }
    candidates.sort_by(|a, b| {
        b.quality
            .partial_cmp(&a.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for qitem in candidates.to_vec() {
        if qitem.item == STAR_STAR {
            return default.to_owned();
        }
        let lc = qitem.item.to_string().to_lowercase();
        if accepted.contains(&lc.as_str()) {
            return lc;
        }
    }
    "invalid".to_string()
}

#[derive(Default, Deserialize)]
pub struct BsoBodies {
    pub valid: Vec<BatchBsoBody>,
    pub invalid: HashMap<String, String>,
}

impl FromRequest for BsoBodies {
    type Config = ();
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
                        None,
                    )
                    .into(),
                ))
            }
        };
        let content_type = format!("{}/{}", ctype.type_(), ctype.subtype());
        debug!("content_type: {:?}", &content_type);

        if !ACCEPTED_CONTENT_TYPES.contains(&content_type.as_ref()) {
            return Box::pin(future::err(
                ValidationErrorKind::FromDetails(
                    format!("Invalid Content-Type {:?}", content_type),
                    RequestErrorLocation::Header,
                    Some("Content-Type".to_owned()),
                    None,
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
        fn make_error(tags: Option<Tags>) -> Error {
            ValidationErrorKind::FromDetails(
                "Invalid JSON in request body".to_owned(),
                RequestErrorLocation::Body,
                Some("bsos".to_owned()),
                tags,
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
                    if let Ok(raw_json) = serde_json::from_str::<Value>(&item) {
                        bsos.push(raw_json);
                    } else {
                        // Per Python version, BSO's must json deserialize
                        return future::err(make_error(None));
                    }
                }
                bsos
            } else if let Ok(json_vals) = serde_json::from_str::<Vec<Value>>(&body) {
                json_vals
            } else {
                // Per Python version, BSO's must json deserialize
                return future::err(make_error(None));
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
            let mut bso_ids: Vec<String> = Vec::with_capacity(bsos.len());

            for bso in bsos {
                // Error out if its not a JSON mapping type
                if !bso.is_object() {
                    return future::err(make_error(None));
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
                                None,
                            )
                            .into(),
                        );
                    } else {
                        bso_ids.push(id.clone());
                        id
                    }
                } else {
                    return future::err(
                        ValidationErrorKind::FromDetails(
                            "Input BSO has no ID".to_owned(),
                            RequestErrorLocation::Body,
                            Some("bsos".to_owned()),
                            None,
                        )
                        .into(),
                    );
                };
                match BatchBsoBody::from_raw_bso(&bso) {
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

#[derive(Default, Debug, Deserialize, Serialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct BsoBody {
    #[validate(custom = "validate_body_bso_id")]
    pub id: Option<String>,
    #[validate(custom = "validate_body_bso_sortindex")]
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    #[validate(custom = "validate_body_bso_ttl")]
    pub ttl: Option<u32>,
    /// Any client-supplied value for these fields are ignored
    #[serde(rename(deserialize = "modified"), skip_serializing)]
    pub _ignored_modified: Option<IgnoredAny>,
    #[serde(rename(deserialize = "collection"), skip_serializing)]
    pub _ignored_collection: Option<IgnoredAny>,
}

impl FromRequest for BsoBody {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<BsoBody, Self::Error>>;

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
                        None,
                    )
                    .into(),
                ))
            }
        };
        let content_type = format!("{}/{}", ctype.type_(), ctype.subtype());
        if !ACCEPTED_CONTENT_TYPES.contains(&content_type.as_ref()) {
            return Box::pin(future::err(
                ValidationErrorKind::FromDetails(
                    "Invalid Content-Type".to_owned(),
                    RequestErrorLocation::Header,
                    Some("Content-Type".to_owned()),
                    None,
                )
                .into(),
            ));
        }
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

        let fut = <Json<BsoBody>>::from_request(&req, payload)
            .map_err(|e| {
                warn!("⚠️ Could not parse BSO Body: {:?}", e);
                let err: ApiError = ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Body,
                    Some("bso".to_owned()),
                    None,
                )
                .into();
                err.into()
            })
            .and_then(move |bso: Json<BsoBody>| {
                // Check the max payload size manually with our desired limit
                if bso
                    .payload
                    .as_ref()
                    .map(std::string::String::len)
                    .unwrap_or_default()
                    > max_payload_size
                {
                    let err: ApiError = ValidationErrorKind::FromDetails(
                        "payload too large".to_owned(),
                        RequestErrorLocation::Body,
                        Some("bso".to_owned()),
                        None,
                    )
                    .into();
                    return future::err(err.into());
                }
                if let Err(e) = bso.validate() {
                    let err: ApiError = ValidationErrorKind::FromValidationErrors(
                        e,
                        RequestErrorLocation::Body,
                        None,
                    )
                    .into();
                    return future::err(err.into());
                }
                future::ok(bso.into_inner())
            });

        Box::pin(fut)
    }
}

/// Bso id parameter extractor
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct BsoParam {
    #[validate(regex = "VALID_ID_REGEX")]
    pub bso: String,
}

impl BsoParam {
    pub fn bsoparam_from_path(uri: &Uri, tags: &Tags) -> Result<Self, Error> {
        // TODO: replace with proper path parser
        // path: "/1.5/{uid}/storage/{collection}/{bso}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        let elem = elements.get(3);
        if elem.is_none() || elem != Some(&"storage") || elements.len() != 6 {
            warn!("⚠️ Unexpected BSO URI: {:?}", uri.path(); tags);
            return Err(ValidationErrorKind::FromDetails(
                "Invalid BSO".to_owned(),
                RequestErrorLocation::Path,
                Some("bso".to_owned()),
                Some(tags.clone()),
            ))?;
        }
        if let Some(v) = elements.get(5) {
            let sv = String::from_str(v).map_err(|_| {
                warn!("⚠️ Invalid BsoParam Error: {:?}", v; tags);
                ValidationErrorKind::FromDetails(
                    "Invalid BSO".to_owned(),
                    RequestErrorLocation::Path,
                    Some("bso".to_owned()),
                    Some(tags.clone()),
                )
            })?;
            Ok(Self { bso: sv })
        } else {
            warn!("⚠️ Missing BSO: {:?}", uri.path(); tags);
            Err(ValidationErrorKind::FromDetails(
                "Missing BSO".to_owned(),
                RequestErrorLocation::Path,
                Some("bso".to_owned()),
                Some(tags.clone()),
            ))?
        }
    }

    pub fn extrude(head: &RequestHead, extensions: &mut Extensions) -> Result<Self, Error> {
        let uri = head.uri.clone();
        let tags = Tags::from_request_head(head);
        if let Some(bso) = extensions.get::<BsoParam>() {
            return Ok(bso.clone());
        }
        let bso = Self::bsoparam_from_path(&uri, &tags)?;
        bso.validate().map_err(|e| {
            ValidationErrorKind::FromValidationErrors(
                e,
                RequestErrorLocation::Path,
                Some(tags.clone()),
            )
        })?;
        extensions.insert(bso.clone());
        Ok(bso)
    }
}

impl FromRequest for BsoParam {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        future::ready(Self::extrude(req.head(), &mut req.extensions_mut()))
    }
}

/// Collection parameter Extractor
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct CollectionParam {
    #[validate(regex = "VALID_COLLECTION_ID_REGEX")]
    pub collection: String,
}

impl CollectionParam {
    fn col_from_path(uri: &Uri, tags: &Tags) -> Result<Option<CollectionParam>, Error> {
        // TODO: replace with proper path parser.
        // path: "/1.5/{uid}/storage/{collection}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        let elem = elements.get(3);
        if elem.is_none() || elem != Some(&"storage") || !(5..=6).contains(&elements.len()) {
            return Ok(None);
        }
        if let Some(v) = elements.get(4) {
            let sv = String::from_str(v).map_err(|_e| {
                ValidationErrorKind::FromDetails(
                    "Missing Collection".to_owned(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                    Some(tags.clone()),
                )
            })?;
            Ok(Some(Self { collection: sv }))
        } else {
            Err(ValidationErrorKind::FromDetails(
                "Missing Collection".to_owned(),
                RequestErrorLocation::Path,
                Some("collection".to_owned()),
                Some(tags.clone()),
            ))?
        }
    }

    pub fn extrude(
        uri: &Uri,
        extensions: &mut Extensions,
        tags: &Tags,
    ) -> Result<Option<Self>, Error> {
        if let Some(collection) = extensions.get::<Option<Self>>() {
            return Ok(collection.clone());
        }

        let collection = Self::col_from_path(&uri, tags)?;
        let result = if let Some(collection) = collection {
            collection.validate().map_err(|e| {
                ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::Path,
                    Some(tags.clone()),
                )
            })?;
            Some(collection)
        } else {
            None
        };
        extensions.insert(result.clone());
        Ok(result)
    }
}

impl FromRequest for CollectionParam {
    type Config = ();
    type Error = Error;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let fut = Tags::from_request(req, payload);
        let req = req.clone();
        Box::pin(async move {
            let tags = fut.await?;
            if let Some(collection) = Self::extrude(&req.uri(), &mut req.extensions_mut(), &tags)? {
                Ok(collection)
            } else {
                Err(ValidationErrorKind::FromDetails(
                    "Missing Collection".to_owned(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                    Some(tags),
                ))?
            }
        })
    }
}

/// Information Requests extractor
///
/// Only the database and user identifier is required for information
/// requests: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#general-info
pub struct MetaRequest {
    pub user_id: HawkIdentifier,
    pub db: Box<dyn Db>,
    pub metrics: metrics::Metrics,
    pub tags: Tags,
}

impl FromRequest for MetaRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        async move {
            // Call the precondition stuff to init database handles and what-not
            let tags = {
                let exts = req.extensions();
                match exts.get::<Tags>() {
                    Some(t) => t.clone(),
                    None => Tags::from_request_head(req.head()),
                }
            };
            let user_id = HawkIdentifier::from_request(&req, &mut payload).await?;
            let db = extrude_db(&req.extensions())?;
            Ok(MetaRequest {
                user_id,
                db,
                metrics: metrics::Metrics::from(&req),
                tags,
            })
        }
        .boxed_local()
    }
}

/// Desired reply format for a Collection Get request
#[derive(Copy, Clone, Debug)]
pub enum ReplyFormat {
    Json,
    Newlines,
}

/// Collection Request Delete/Get extractor
///
/// Extracts/validates information needed for collection delete/get requests.
pub struct CollectionRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub reply: ReplyFormat,
    pub metrics: metrics::Metrics,
    pub tags: Option<Tags>,
}

impl FromRequest for CollectionRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        async move {
            let user_id = HawkIdentifier::from_request(&req, &mut payload).await?;
            let db = <Box<dyn Db>>::from_request(&req, &mut payload).await?;
            let query = BsoQueryParams::from_request(&req, &mut payload).await?;
            let collection = CollectionParam::from_request(&req, &mut payload)
                .await?
                .collection;
            let tags = {
                let exts = req.extensions();
                match exts.get::<Tags>() {
                    Some(t) => t.clone(),
                    None => Tags::from_request_head(req.head()),
                }
            };

            let accept = get_accepted(&req, &ACCEPTED_CONTENT_TYPES, "application/json");
            let reply = match accept.as_str() {
                "application/newlines" => ReplyFormat::Newlines,
                "application/json" | "" => ReplyFormat::Json,
                _ => {
                    return Err(ValidationErrorKind::FromDetails(
                        "Invalid accept".to_string(),
                        RequestErrorLocation::Header,
                        Some("accept".to_string()),
                        Some(tags),
                    )
                    .into());
                }
            };

            Ok(CollectionRequest {
                collection,
                db,
                user_id,
                query,
                reply,
                metrics: metrics::Metrics::from(&req),
                tags: Some(tags),
            })
        }
        .boxed_local()
    }
}

/// Collection Request Post extractor
///
/// Extracts/validates information needed for batch collection POST requests.
pub struct CollectionPostRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bsos: BsoBodies,
    pub batch: Option<BatchRequest>,
    pub metrics: metrics::Metrics,
}

impl FromRequest for CollectionPostRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<CollectionPostRequest, Self::Error>>;

    /// Extractor for Collection Posts (Batch BSO upload)
    ///
    /// Utilizes the `BsoBodies` for parsing, and add's two validation steps not
    /// done previously:
    ///   - If the collection is 'crypto', known bad payloads are checked for
    ///   - Any valid BSO's beyond `BATCH_MAX_RECORDS` are moved to invalid
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();
        Box::pin(async move {
            let tags = match req.extensions().get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            };
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        Some(tags),
                    )
                    .into());
                }
            };

            let max_post_records = i64::from(state.limits.max_post_records);
            let (user_id, db, collection, query, mut bsos) =
                <(
                    HawkIdentifier,
                    Box<dyn Db>,
                    CollectionParam,
                    BsoQueryParams,
                    BsoBodies,
                )>::from_request(&req, &mut payload)
                .await?;
            let collection = collection.collection;
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                for bso in &bsos.valid {
                    if let Some(ref data) = bso.payload {
                        if KNOWN_BAD_PAYLOAD_REGEX.is_match(data) {
                            return Err(ValidationErrorKind::FromDetails(
                                "Known-bad BSO payload".to_owned(),
                                RequestErrorLocation::Body,
                                Some("bsos".to_owned()),
                                Some(tags),
                            )
                            .into());
                        }
                    }
                }
            }

            // Trim the excess BSO's to be under the batch size
            let overage: i64 = (bsos.valid.len() as i64) - max_post_records;
            if overage > 0 {
                for _ in 1..=overage {
                    if let Some(last) = bsos.valid.pop() {
                        bsos.invalid.insert(last.id, "retry bso".to_string());
                    }
                }
            }

            // XXX: let's not use extract here (maybe convert to extrude?)
            let batch = BatchRequestOpt::extract(&req).await?;
            Ok(CollectionPostRequest {
                collection,
                db,
                user_id,
                query,
                bsos,
                batch: batch.opt,
                metrics: metrics::Metrics::from(&req),
            })
        })
    }
}

/// BSO Request Delete/Get extractor
///
/// Extracts/validates information needed for BSO delete/get requests.
#[derive(Debug)]
pub struct BsoRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bso: String,
    pub metrics: metrics::Metrics,
}

impl FromRequest for BsoRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();
        Box::pin(async move {
            let user_id = HawkIdentifier::from_request(&req, &mut payload).await?;
            let db = <Box<dyn Db>>::from_request(&req, &mut payload).await?;
            let query = BsoQueryParams::from_request(&req, &mut payload).await?;
            let collection = CollectionParam::from_request(&req, &mut payload)
                .await?
                .collection;
            let bso = BsoParam::from_request(&req, &mut payload).await?;

            Ok(BsoRequest {
                collection,
                db,
                user_id,
                query,
                bso: bso.bso,
                metrics: metrics::Metrics::from(&req),
            })
        })
    }
}

/// BSO Request Put extractor
///
/// Extracts/validates information needed for BSO put requests.
pub struct BsoPutRequest {
    pub collection: String,
    pub db: Box<dyn Db>,
    pub user_id: HawkIdentifier,
    pub query: BsoQueryParams,
    pub bso: String,
    pub body: BsoBody,
    pub metrics: metrics::Metrics,
}

impl FromRequest for BsoPutRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<BsoPutRequest, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let metrics = metrics::Metrics::from(req);
        let fut = <(
            HawkIdentifier,
            Box<dyn Db>,
            CollectionParam,
            BsoQueryParams,
            BsoParam,
            BsoBody,
            Tags,
        )>::from_request(req, payload)
        .and_then(|(user_id, db, collection, query, bso, body, tags)| {
            let collection = collection.collection;
            if collection == "crypto" {
                // Verify the client didn't mess up the crypto if we have a payload
                if let Some(ref data) = body.payload {
                    if KNOWN_BAD_PAYLOAD_REGEX.is_match(data) {
                        return future::err(
                            ValidationErrorKind::FromDetails(
                                "Known-bad BSO payload".to_owned(),
                                RequestErrorLocation::Body,
                                Some("bsos".to_owned()),
                                Some(tags),
                            )
                            .into(),
                        );
                    }
                }
            }
            future::ok(BsoPutRequest {
                collection,
                db,
                user_id,
                query,
                bso: bso.bso,
                body,
                metrics,
            })
        });
        Box::pin(fut)
    }
}

#[derive(Debug, Default, Serialize)]
pub struct ConfigRequest {
    pub limits: ServerLimits,
}

impl FromRequest for ConfigRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            }
        };

        let state = match req.app_data::<Data<ServerState>>() {
            Some(s) => s,
            None => {
                error!("⚠️ Could not load the app state");
                return Box::pin(future::err(
                    ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        Some(tags),
                    )
                    .into(),
                ));
            }
        };

        let data = &state.limits;
        Box::pin(future::ok(Self {
            limits: ServerLimits {
                max_post_bytes: data.max_post_bytes,
                max_post_records: data.max_post_records,
                max_record_payload_bytes: data.max_record_payload_bytes,
                max_request_bytes: data.max_request_bytes,
                max_total_bytes: data.max_total_bytes,
                max_total_records: data.max_total_records,
            },
        }))
    }
}

#[derive(Clone, Debug)]
pub struct HeartbeatRequest {
    pub headers: HeaderMap,
    pub db: Box<dyn Db>,
}

impl FromRequest for HeartbeatRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers().clone();
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            }
        };

        let state = match req.app_data::<Data<ServerState>>() {
            Some(s) => s,
            None => {
                error!("⚠️ Could not load the app state");
                return Box::pin(future::err(
                    ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        Some(tags),
                    )
                    .into(),
                ));
            }
        };
        let fut = state
            .db_pool
            .get()
            .map_err(Into::into)
            .and_then(|db| future::ok(HeartbeatRequest { headers, db }));
        Box::pin(fut)
    }
}

#[derive(Debug)]
pub struct TestErrorRequest {
    pub headers: HeaderMap,
    pub tags: Option<Tags>,
}

impl FromRequest for TestErrorRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let headers = req.headers().clone();
        let tags = {
            let exts = req.extensions();
            match exts.get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            }
        };

        Box::pin(future::ok(TestErrorRequest {
            headers,
            tags: Some(tags),
        }))
    }
}

/// Extract a user-identifier from the authentication token and validate against the URL
///
/// This token should be adapted as needed for the storage system to store data
/// for the user.
#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct HawkIdentifier {
    /// For MySQL database backends as the primary key
    pub legacy_id: u64,
    /// For NoSQL database backends that require randomly distributed primary keys
    pub fxa_uid: String,
    pub fxa_kid: String,
}

impl HawkIdentifier {
    /// Create a new legacy id user identifier
    pub fn new_legacy(user_id: u64) -> HawkIdentifier {
        HawkIdentifier {
            legacy_id: user_id,
            ..Default::default()
        }
    }

    pub fn cmd_dummy() -> Self {
        // Create a "dummy" HawkID for use by DockerFlow commands
        Self {
            legacy_id: 0,
            fxa_uid: "cmd".to_owned(),
            fxa_kid: "cmd".to_owned(),
        }
    }

    fn uid_from_path(uri: &Uri, tags: Option<Tags>) -> Result<u64, Error> {
        // TODO: replace with proper path parser.
        // path: "/1.5/{uid}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        if let Some(v) = elements.get(2) {
            u64::from_str(v).map_err(|e| {
                warn!("⚠️ HawkIdentifier Error invalid UID {:?} {:?}", v, e);
                ValidationErrorKind::FromDetails(
                    "Invalid UID".to_owned(),
                    RequestErrorLocation::Path,
                    Some("uid".to_owned()),
                    tags.clone(),
                )
                .into()
            })
        } else {
            warn!("⚠️ HawkIdentifier Error missing UID {:?}", uri);
            Err(ValidationErrorKind::FromDetails(
                "Missing UID".to_owned(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
                tags,
            ))?
        }
    }

    pub fn extrude<T>(
        msg: &T,
        method: &str,
        uri: &Uri,
        ci: &ConnectionInfo,
        state: &ServerState,
        tags: Option<Tags>,
    ) -> Result<Self, Error>
    where
        T: HttpMessage,
    {
        if let Some(user_id) = msg.extensions().get::<HawkIdentifier>() {
            return Ok(user_id.clone());
        }

        let auth_header = msg
            .headers()
            .get("authorization")
            .ok_or_else(|| -> ApiError { HawkErrorKind::MissingHeader.into() })?
            .to_str()
            .map_err(|e| -> ApiError { HawkErrorKind::Header(e).into() })?;
        let identifier = Self::generate(&state.secrets, method, auth_header, ci, uri, tags)?;
        msg.extensions_mut().insert(identifier.clone());
        Ok(identifier)
    }

    pub fn generate(
        secrets: &Secrets,
        method: &str,
        header: &str,
        connection_info: &ConnectionInfo,
        uri: &Uri,
        tags: Option<Tags>,
    ) -> Result<Self, Error> {
        let payload =
            HawkPayload::extrude(header, method, secrets, connection_info, uri, tags.clone())?;
        let puid = Self::uid_from_path(&uri, tags.clone())?;
        if payload.user_id != puid {
            warn!("⚠️ Hawk UID not in URI: {:?} {:?}", payload.user_id, uri);
            Err(ValidationErrorKind::FromDetails(
                "conflicts with payload".to_owned(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
                tags,
            ))?;
        }

        let user_id = HawkIdentifier {
            legacy_id: payload.user_id,
            fxa_uid: payload.fxa_uid,
            fxa_kid: payload.fxa_kid,
        };
        Ok(user_id)
    }
}

impl FromRequest for HawkIdentifier {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Use HawkPayload extraction and format as HawkIdentifier.
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;

        Box::pin(async move {
            let tags = Tags::from_request(&req, &mut payload).await?;
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("state".to_owned()),
                        Some(tags),
                    )
                    .into());
                }
            };
            // NOTE: `connection_info()` will get a mutable reference lock on `extensions()`
            let connection_info = req.connection_info().clone();
            let method = req.method().as_str();
            let uri = req.uri();
            Self::extrude(&req, method, uri, &connection_info, &state, Some(tags))
        })
    }
}

impl From<u32> for HawkIdentifier {
    fn from(val: u32) -> Self {
        HawkIdentifier {
            legacy_id: val.into(),
            ..Default::default()
        }
    }
}

pub fn extrude_db(exts: &Extensions) -> Result<Box<dyn Db>, Error> {
    exts.get::<Box<dyn Db>>().cloned().ok_or_else(|| {
        error!("DB Error: No db");
        ErrorInternalServerError("Unexpected Db error: No DB".to_owned())
    })
}

impl FromRequest for Box<dyn Db> {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        Box::pin(future::ready(extrude_db(&req.extensions())))
    }
}

#[derive(Debug, Default, Clone, Deserialize, Validate)]
#[serde(default)]
pub struct Offset {
    pub timestamp: Option<SyncTimestamp>,
    pub offset: i64,
}

impl ToString for Offset {
    fn to_string(&self) -> String {
        match self.timestamp {
            None => format!("{}", self.offset),
            Some(ts) => format!("{}:{}", ts.as_i64(), self.offset),
        }
    }
}

impl FromStr for Offset {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // issue559: Disable ':' support for now: simply parse as i64 as
        // previously (it was u64 previously but i64's close enough)
        let result = Offset {
            timestamp: None,
            offset: s.parse::<i64>()?,
        };
        /*
        let result = match s.chars().position(|c| c == ':') {
            None => Offset {
                timestamp: None,
                offset: s.parse::<i64>()?,
            },
            Some(_colon_position) => {
                let mut parts = s.split(':');
                let timestamp_string = parts.next().unwrap_or("0");
                let timestamp = SyncTimestamp::from_milliseconds(timestamp_string.parse::<u64>()?);
                let offset = parts.next().unwrap_or("0").parse::<i64>()?;
                Offset {
                    timestamp: Some(timestamp),
                    offset,
                }
            }
        };
        */
        Ok(result)
    }
}

/// Validator to extract BSO search parameters from the query string.
///
/// This validator will extract and validate the following search params used in
/// multiple handler functions. Not all query params are used in each handler.
#[derive(Debug, Default, Clone, Deserialize, Validate)]
#[serde(default)]
pub struct BsoQueryParams {
    /// lower-bound on last-modified time
    #[serde(deserialize_with = "deserialize_sync_timestamp")]
    pub newer: Option<SyncTimestamp>,

    /// upper-bound on last-modified time
    #[serde(deserialize_with = "deserialize_sync_timestamp")]
    pub older: Option<SyncTimestamp>,

    /// order in which to return results (string)
    #[serde(default)]
    pub sort: Sorting,

    /// maximum number of items to return (integer)
    pub limit: Option<u32>,

    /// position at which to restart search (string)
    #[serde(deserialize_with = "deserialize_offset")]
    pub offset: Option<Offset>,

    /// a comma-separated list of BSO ids (list of strings)
    #[serde(deserialize_with = "deserialize_comma_sep_string", default)]
    #[validate(custom = "validate_qs_ids")]
    pub ids: Vec<String>,

    // flag, whether to include full bodies (bool)
    #[serde(deserialize_with = "deserialize_present_value")]
    pub full: bool,
}

impl FromRequest for BsoQueryParams {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract and validate the query parameters
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        Box::pin(async move {
            let tags = Tags::from_request(&req, &mut payload).await?;

            let params = Query::<BsoQueryParams>::from_request(&req, &mut payload)
                .map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::QueryString,
                        None,
                        Some(tags.clone()),
                    )
                })
                .await?
                .into_inner();
            params.validate().map_err(|e| {
                ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::QueryString,
                    Some(tags.clone()),
                )
            })?;
            // issue559: Dead code (timestamp always None)
            /*
               if params.sort != Sorting::Index {
                   if let Some(timestamp) = params.offset.as_ref().and_then(|offset| offset.timestamp)
                   {
                       let bound = timestamp.as_i64();
                       if let Some(newer) = params.newer {
                           if bound < newer.as_i64() {
                               return Err(ValidationErrorKind::FromDetails(
                                   format!("Invalid Offset {} {}", bound, newer.as_i64()),
                                   RequestErrorLocation::QueryString,
                                   Some("newer".to_owned()),
                                   None,
                               )
                               .into());
                           }
                       } else if let Some(older) = params.older {
                           if bound > older.as_i64() {
                               return Err(ValidationErrorKind::FromDetails(
                                   "Invalid Offset".to_owned(),
                                   RequestErrorLocation::QueryString,
                                   Some("older".to_owned()),
                                   None,
                               )
                               .into());
                           }
                       }
                   }
               }
            */
            Ok(params)
        })
    }
}

#[derive(Debug, Default, Clone, Deserialize, Validate)]
#[serde(default)]
pub struct BatchParams {
    pub batch: Option<String>,
    #[validate(custom = "validate_qs_commit")]
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
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<BatchRequestOpt, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        Box::pin(async move {
            let tags = Tags::from_request(&req, &mut payload).await?;
            // let tags = Tags::from_request_head(req.head());
            let ftags = tags.clone();
            let params = Query::<BatchParams>::from_request(&req, &mut payload)
                .map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::QueryString,
                        None,
                        Some(tags.clone()),
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
                        Some(tags),
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
                            Some(tags.clone()),
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
                        Some(tags.clone()),
                    )
                    .into();
                    err
                })?;
                if count > *limit {
                    return Err(ValidationErrorKind::FromDetails(
                        "size-limit-exceeded".to_owned(),
                        RequestErrorLocation::Header,
                        None,
                        Some(tags.clone()),
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
                    Some(tags),
                )
                .into());
            }

            params.validate().map_err(|e| {
                let err: ApiError = ValidationErrorKind::FromValidationErrors(
                    e,
                    RequestErrorLocation::QueryString,
                    Some(tags.clone()),
                )
                .into();
                err
            })?;

            let id = match params.batch {
                None => None,
                Some(ref batch) if batch == "" || TRUE_REGEX.is_match(&batch) => None,
                Some(batch) => {
                    let db = extrude_db(&req.extensions())?;
                    if db.validate_batch_id(batch.clone()).is_err() {
                        return Err(ValidationErrorKind::FromDetails(
                            format!(r#"Invalid batch ID: "{}""#, batch),
                            RequestErrorLocation::QueryString,
                            Some("batch".to_owned()),
                            Some(ftags),
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

/// PreCondition Header
///
/// It's valid to include a X-If-Modified-Since or X-If-Unmodified-Since header but not
/// both.
///
/// Used with Option<PreConditionHeader> to extract a possible PreConditionHeader.
#[derive(Debug, Clone, PartialEq)]
pub enum PreConditionHeader {
    IfModifiedSince(SyncTimestamp),
    IfUnmodifiedSince(SyncTimestamp),
    NoHeader,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PreConditionHeaderOpt {
    pub opt: Option<PreConditionHeader>,
}

impl PreConditionHeaderOpt {
    pub fn extrude(headers: &HeaderMap, tags: Option<Tags>) -> Result<Self, Error> {
        let modified = headers.get("X-If-Modified-Since");
        let unmodified = headers.get("X-If-Unmodified-Since");
        if modified.is_some() && unmodified.is_some() {
            // TODO: See following error,
            return Err(ValidationErrorKind::FromDetails(
                "conflicts with X-If-Modified-Since".to_owned(),
                RequestErrorLocation::Header,
                Some("X-If-Unmodified-Since".to_owned()),
                tags,
            )
            .into());
        };
        let (value, field_name) = if let Some(modified_value) = modified {
            (modified_value, "X-If-Modified-Since")
        } else if let Some(unmodified_value) = unmodified {
            (unmodified_value, "X-If-Unmodified-Since")
        } else {
            return Ok(Self { opt: None });
        };
        if value
            .to_str()
            .unwrap_or("0.0")
            .parse::<f64>()
            .unwrap_or(0.0)
            < 0.0
        {
            // TODO: This is the right error, but it's not being returned correctly.
            return Err(ValidationErrorKind::FromDetails(
                "value is negative".to_owned(),
                RequestErrorLocation::Header,
                Some("X-If-Modified-Since".to_owned()),
                tags,
            )
            .into());
        }
        value
            .to_str()
            .map_err(|e| {
                ValidationErrorKind::FromDetails(
                    e.to_string(),
                    RequestErrorLocation::Header,
                    Some(field_name.to_owned()),
                    tags.clone(),
                )
                .into()
            })
            .and_then(|v| {
                SyncTimestamp::from_header(v).map_err(|e| {
                    ValidationErrorKind::FromDetails(
                        e.to_string(),
                        RequestErrorLocation::Header,
                        Some(field_name.to_owned()),
                        tags.clone(),
                    )
                    .into()
                })
            })
            .map(|v| {
                let header = if field_name == "X-If-Modified-Since" {
                    PreConditionHeader::IfModifiedSince(v)
                } else {
                    PreConditionHeader::IfUnmodifiedSince(v)
                };
                Self { opt: Some(header) }
            })
    }
}

impl FromRequest for PreConditionHeaderOpt {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract and validate the precondition headers
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = Payload::None;
        Box::pin(async move {
            let tags = Tags::from_request(&req, &mut payload).await?;
            Self::extrude(req.headers(), Some(tags)).map_err(Into::into)
        })
    }
}

/// Validation Error Location in the request
#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RequestErrorLocation {
    Body,
    QueryString,
    Url,
    Header,
    Path,
    Cookies,
    Method,
    Unknown,
}

/// Convenience function to create a `ValidationError` with additional context
fn request_error(message: &'static str, location: RequestErrorLocation) -> ValidationError {
    let mut err = ValidationError::new(message);
    err.add_param("location".into(), &location);
    err
}

/// Verifies that the list of id's is not too long and that the ids are valid
fn validate_qs_ids(ids: &[String]) -> Result<(), ValidationError> {
    if ids.len() > BATCH_MAX_IDS {
        return Err(request_error(
            "Too many ids provided",
            RequestErrorLocation::QueryString,
        ));
    }
    for id in ids {
        if !VALID_ID_REGEX.is_match(&id) {
            return Err(request_error(
                "Invalid id in ids",
                RequestErrorLocation::QueryString,
            ));
        }
    }
    Ok(())
}

/// Verifies the batch commit field is valid
fn validate_qs_commit(commit: &str) -> Result<(), ValidationError> {
    if !TRUE_REGEX.is_match(commit) {
        return Err(request_error(
            r#"commit parameter must be "true" to apply batches"#,
            RequestErrorLocation::QueryString,
        ));
    }
    Ok(())
}

/// Verifies the BSO sortindex is in the valid range
fn validate_body_bso_sortindex(sort: i32) -> Result<(), ValidationError> {
    if BSO_MIN_SORTINDEX_VALUE <= sort && sort <= BSO_MAX_SORTINDEX_VALUE {
        Ok(())
    } else {
        Err(request_error("invalid value", RequestErrorLocation::Body))
    }
}

/// Verifies the BSO id string is valid
fn validate_body_bso_id(id: &str) -> Result<(), ValidationError> {
    if !VALID_ID_REGEX.is_match(id) {
        return Err(request_error("Invalid id", RequestErrorLocation::Body));
    }
    Ok(())
}

/// Verifies the BSO ttl is valid
fn validate_body_bso_ttl(ttl: u32) -> Result<(), ValidationError> {
    if ttl > BSO_MAX_TTL {
        return Err(request_error("Invalid TTL", RequestErrorLocation::Body));
    }
    Ok(())
}

/// Deserialize a comma separated string
fn deserialize_comma_sep_string<'de, D, E>(deserializer: D) -> Result<Vec<E>, D::Error>
where
    D: Deserializer<'de>,
    E: FromStr,
{
    let str: String = Deserialize::deserialize(deserializer)?;
    let lst: Vec<String> = str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut parsed_lst: Vec<E> = Vec::new();
    for item in lst {
        parsed_lst.push(
            item.parse::<E>()
                .map_err(|_| SerdeError::custom("Invalid value in list"))?,
        );
    }
    Ok(parsed_lst)
}

/// Deserialize a value as True if it exists, False otherwise
fn deserialize_present_value<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Option::deserialize(deserializer).unwrap_or(None);
    Ok(maybe_str.is_some())
}

/// Deserialize a header string value (epoch seconds with 2 decimal places) as SyncTimestamp
fn deserialize_sync_timestamp<'de, D>(deserializer: D) -> Result<Option<SyncTimestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(val) = maybe_str {
        let result = SyncTimestamp::from_header(&val).map_err(SerdeError::custom);
        Ok(Some(result?))
    } else {
        Ok(None)
    }
}

fn deserialize_offset<'de, D>(deserializer: D) -> Result<Option<Offset>, D::Error>
where
    D: Deserializer<'de>,
{
    let maybe_str: Option<String> = Deserialize::deserialize(deserializer)?;
    if let Some(val) = maybe_str {
        return Ok(Some(Offset::from_str(&val).map_err(SerdeError::custom)?));
    }
    Ok(None)
}

// Tokenserver extractor
#[derive(Debug, Default, Clone, Deserialize)]
pub struct TokenServerRequest {
    // TODO extract required headers from the request into this struct.
}

impl FromRequest for TokenServerRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    /// Extract and validate the precondition headers
    fn from_request(_req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        Box::pin(async move { Ok(Self {}) })
    }
}

#[cfg(test)]
mod tests {
    use actix_http::h1;
    use futures::executor::block_on;

    use super::*;

    use std::sync::Arc;

    use actix_web::{
        dev::ServiceResponse,
        http::{
            header::{HeaderValue, ACCEPT},
            Method,
        },
        test::{self, TestRequest},
        Error, HttpResponse,
    };
    use base64;
    use hawk::{Credentials, Key, RequestBuilder};
    use hmac::{Hmac, Mac};
    use rand::{thread_rng, Rng};
    use serde_json::{self, json};
    use sha2::Sha256;

    use crate::db::mock::{MockDb, MockDbPool};
    use crate::server::{metrics, ServerState};
    use crate::settings::{Secrets, ServerLimits, Settings};

    use crate::web::auth::{hkdf_expand_32, HawkPayload};

    lazy_static! {
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
        static ref USER_ID: u64 = thread_rng().gen_range(0, 10000);
        static ref USER_ID_STR: String = USER_ID.to_string();
    }

    const TEST_HOST: &str = "localhost";
    const TEST_PORT: u16 = 8080;
    // String is too long for valid name
    const INVALID_COLLECTION_NAME: &str = "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";
    const INVALID_BSO_NAME: &str =
        "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz";

    fn make_db() -> Box<dyn Db> {
        Box::new(MockDb::new())
    }

    fn make_state() -> ServerState {
        let settings = Settings::default();
        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            secrets: Arc::clone(&SECRETS),
            port: 8000,
            metrics: Box::new(metrics::metrics_from_opts(&settings).unwrap()),
        }
    }

    fn extract_body_as_str(sresponse: ServiceResponse) -> String {
        String::from_utf8(block_on(test::read_body(sresponse)).to_vec()).unwrap()
    }

    fn create_valid_hawk_header(
        payload: &HawkPayload,
        state: &ServerState,
        method: &str,
        path: &str,
        host: &str,
        port: u16,
    ) -> String {
        let salt = payload.salt.clone();
        let payload = serde_json::to_string(payload).unwrap();
        let mut hmac: Hmac<Sha256> = Hmac::new_varkey(&state.secrets.signing_secret).unwrap();
        hmac.input(payload.as_bytes());
        let payload_hash = hmac.result().code();
        let mut id = payload.as_bytes().to_vec();
        id.extend(payload_hash.to_vec());
        let id = base64::encode_config(&id, base64::URL_SAFE);
        let token_secret = hkdf_expand_32(
            format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
            Some(salt.as_bytes()),
            &SECRETS.master_secret,
        )
        .unwrap();
        let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);
        let credentials = Credentials {
            id,
            key: Key::new(token_secret.as_bytes(), hawk::DigestAlgorithm::Sha256).unwrap(),
        };
        let request = RequestBuilder::new(method, host, port, path)
            .hash(&payload_hash[..])
            .request();
        format!("Hawk {}", request.make_header(&credentials).unwrap())
    }

    async fn post_collection(
        qs: &str,
        body: &serde_json::Value,
    ) -> Result<CollectionPostRequest, Error> {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let path = format!(
            "/1.5/{}/storage/tabs{}{}",
            *USER_ID,
            if !qs.is_empty() { "?" } else { "" },
            qs
        );
        let bod_str = body.to_string();
        let header =
            create_valid_hawk_header(&payload, &state, "POST", &path, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&format!("http://{}:{}{}", TEST_HOST, TEST_PORT, path))
            .data(state)
            .method(Method::POST)
            .header("authorization", header)
            .header("content-type", "application/json; charset=UTF-8")
            .header("accept", "application/json;q=0.9,/;q=0.2")
            .set_payload(bod_str.to_owned())
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .to_http_request();
        req.extensions_mut().insert(make_db());

        // Not sure why but sending req through *::extract loses the body.
        // Compose a payload here and call the *::from_request
        let (_sender, mut payload) = h1::Payload::create(true);
        payload.unread_data(bytes::Bytes::from(bod_str.to_owned()));
        CollectionPostRequest::from_request(&req, &mut payload.into()).await
    }

    #[test]
    fn test_invalid_query_args() {
        let state = make_state();
        let req = TestRequest::with_uri("/?lower=-1.23&sort=whatever")
            .data(state)
            .to_http_request();
        let result = block_on(BsoQueryParams::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);
        assert_eq!(err["reason"], "Bad Request");

        let (_lower_error, sort_error) = if err["errors"][0]["name"] == "lower" {
            (&err["errors"][0], &err["errors"][1])
        } else {
            (&err["errors"][1], &err["errors"][0])
        };

        assert_eq!(sort_error["location"], "querystring");
        */
    }

    #[test]
    fn test_weighted_header() {
        // test non-priority, full weight selection
        let req = TestRequest::with_header(
            ACCEPT,
            HeaderValue::from_static("application/json;q=0.9,text/plain"),
        );
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "text/plain".to_owned());

        // test default for */*
        let req = TestRequest::with_header(ACCEPT, HeaderValue::from_static("*/*;q=0.2,foo/bar"));
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "application/json".to_owned());

        // test default for selected weighted.
        let req = TestRequest::with_header(
            ACCEPT,
            HeaderValue::from_static("foo/bar;q=0.1,application/json;q=0.5,text/plain;q=0.9"),
        );
        let selected = get_accepted(
            &req.to_http_request(),
            &ACCEPTED_CONTENT_TYPES,
            "application/json",
        );
        assert_eq!(selected, "text/plain".to_owned());
    }

    #[test]
    fn test_valid_query_args() {
        let req = TestRequest::with_uri("/?ids=1,2&full=&sort=index&older=2.43")
            .data(make_state())
            .to_http_request();
        let result = block_on(BsoQueryParams::extract(&req)).unwrap();
        assert_eq!(result.ids, vec!["1", "2"]);
        assert_eq!(result.sort, Sorting::Index);
        assert_eq!(result.older.unwrap(), SyncTimestamp::from_seconds(2.43));
        assert_eq!(result.full, true);
    }

    #[test]
    fn test_valid_bso_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header = create_valid_hawk_header(&payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .method(Method::GET)
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoRequest::extract(&req))
            .expect("Could not get result in test_valid_bso_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
    }

    #[test]
    fn test_invalid_bso_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/tabs/{}", *USER_ID, INVALID_BSO_NAME);
        let header = create_valid_hawk_header(&payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .method(Method::GET)
            // `param` sets the value that would be extracted from the tokenized URI, as if the router did it.
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .param("bso", INVALID_BSO_NAME)
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoRequest::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "regex");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "bso");
        assert_eq!(err["errors"][0]["value"], INVALID_BSO_NAME);
        */
    }

    #[test]
    fn test_valid_bso_post_body() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header = create_valid_hawk_header(&payload, &state, "POST", &uri, TEST_HOST, TEST_PORT);
        let bso_body = json!({
            "id": "128", "payload": "x"
        });
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .header("content-type", "application/json")
            .method(Method::POST)
            .set_payload(bso_body.to_string())
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let (_sender, mut payload) = h1::Payload::create(true);
        payload.unread_data(bytes::Bytes::from(bso_body.to_string()));
        let result = block_on(BsoPutRequest::from_request(&req, &mut payload.into()))
            .expect("Could not get result in test_valid_bso_post_body");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(&result.bso, "asdf");
        assert_eq!(result.body.payload, Some("x".to_string()));
    }

    #[test]
    fn test_invalid_bso_post_body() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/tabs/asdf", *USER_ID);
        let header = create_valid_hawk_header(&payload, &state, "POST", &uri, TEST_HOST, TEST_PORT);
        let bso_body = json!({
            "payload": "xxx", "sortindex": -9_999_999_999 as i64,
        });
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .header("content-type", "application/json")
            .method(Method::POST)
            .set_payload(bso_body.to_string())
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .param("bso", "asdf")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(BsoPutRequest::extract(&req));
        let response: HttpResponse = result
            .err()
            .expect("Could not get response in test_invalid_bso_post_body")
            .into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "8")

        /* New tests for when we can use descriptive errors
        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["location"], "body");
        assert_eq!(&err["errors"][0]["name"], "bso");
        */
    }

    #[test]
    fn test_valid_collection_request() {
        let payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/tabs", *USER_ID);
        let header = create_valid_hawk_header(&payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .param("uid", &USER_ID_STR)
            .param("collection", "tabs")
            .to_http_request();
        req.extensions_mut().insert(make_db());
        let result = block_on(CollectionRequest::extract(&req))
            .expect("Could not get result in test_valid_collection_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
    }

    #[test]
    fn test_invalid_collection_request() {
        let hawk_payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/{}", *USER_ID, INVALID_COLLECTION_NAME);
        let header =
            create_valid_hawk_header(&hawk_payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .header("authorization", header)
            .method(Method::GET)
            .data(state)
            .param("uid", &USER_ID_STR)
            .param("collection", INVALID_COLLECTION_NAME)
            .to_http_request();
        req.extensions_mut().insert(make_db());

        let result = block_on(CollectionRequest::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "regex");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "collection");
        assert_eq!(err["errors"][0]["value"], INVALID_COLLECTION_NAME);
        */
    }

    #[actix_rt::test]
    async fn test_valid_collection_post_request() {
        // Batch requests require id's on each BSO
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        assert!(result.batch.is_none());
    }

    #[actix_rt::test]
    async fn test_invalid_collection_post_request() {
        // Add extra fields, these will be invalid
        let bso_body = json!([
            {"id": "1", "sortindex": 23, "jump": 1},
            {"id": "2", "sortindex": -99, "hop": "low"}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_invalid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.invalid.len(), 2);
    }

    #[actix_rt::test]
    async fn test_valid_collection_batch_post_request() {
        // If the "batch" parameter is has no value or has a value of "true"
        // then a new batch will be created.
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let result = post_collection("batch=True", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_batch_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        let batch = result
            .batch
            .expect("Could not get batch in test_valid_collection_batch_post_request");
        assert!(batch.id.is_none());
        assert_eq!(batch.commit, false);

        let result2 = post_collection("batch", &bso_body)
            .await
            .expect("Could not get result2 in test_valid_collection_batch_post_request");
        let batch2 = result2
            .batch
            .expect("Could not get batch2 in test_valid_collection_batch_post_request");
        assert!(batch2.id.is_none());
        assert_eq!(batch2.commit, false);

        let result3 = post_collection("batch=MTI%3D&commit=true", &bso_body)
            .await
            .expect("Could not get result3 in test_valid_collection_batch_post_request");
        let batch3 = result3
            .batch
            .expect("Could not get batch3 in test_valid_collection_batch_post_request");
        assert!(batch3.id.is_some());
        assert_eq!(batch3.commit, true);
    }

    #[actix_rt::test]
    async fn test_invalid_collection_batch_post_request() {
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23}
        ]);
        let req = TestRequest::with_uri("/")
            .method(Method::POST)
            .data(make_state())
            .to_http_request();
        let result = post_collection("commit=true", &bso_body).await;
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");
    }

    #[test]
    fn test_invalid_precondition_headers() {
        fn assert_invalid_header(req: HttpRequest, _error_header: &str, _error_message: &str) {
            let tags = match req.extensions().get::<Tags>() {
                Some(t) => t.clone(),
                None => Tags::from_request_head(req.head()),
            };
            let result = PreConditionHeaderOpt::extrude(&req.headers(), Some(tags));
            assert!(result.is_err());
            let response: HttpResponse = result.err().unwrap().into();
            assert_eq!(response.status(), 400);
            let body = extract_body_as_str(ServiceResponse::new(req, response));

            assert_eq!(body, "0");

            /* New tests for when we can use descriptive errors
            let err: serde_json::Value = serde_json::from_str(&body).unwrap();
            assert_eq!(err["status"], 400);

            assert_eq!(err["errors"][0]["description"], error_message);
            assert_eq!(err["errors"][0]["location"], "header");
            assert_eq!(err["errors"][0]["name"], error_header);
            */
        }
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .header("X-If-Modified-Since", "32124.32")
            .header("X-If-Unmodified-Since", "4212.12")
            .to_http_request();
        assert_invalid_header(
            req,
            "X-If-Unmodified-Since",
            "conflicts with X-If-Modified-Since",
        );
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .header("X-If-Modified-Since", "-32.1")
            .to_http_request();
        assert_invalid_header(req, "X-If-Modified-Since", "Invalid value");
    }

    #[test]
    fn test_valid_precondition_headers() {
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .header("X-If-Modified-Since", "32.1")
            .to_http_request();
        let result = PreConditionHeaderOpt::extrude(&req.headers(), None)
            .unwrap()
            .opt
            .unwrap();
        assert_eq!(
            result,
            PreConditionHeader::IfModifiedSince(SyncTimestamp::from_seconds(32.1))
        );
        let req = TestRequest::with_uri("/")
            .data(make_state())
            .header("X-If-Unmodified-Since", "32.14")
            .to_http_request();
        let result = PreConditionHeaderOpt::extrude(&req.headers(), None)
            .unwrap()
            .opt
            .unwrap();
        assert_eq!(
            result,
            PreConditionHeader::IfUnmodifiedSince(SyncTimestamp::from_seconds(32.14))
        );
    }

    #[test]
    fn valid_header_with_valid_path() {
        let hawk_payload = HawkPayload::test_default(*USER_ID);
        let state = make_state();
        let uri = format!("/1.5/{}/storage/col2", *USER_ID);
        let header =
            create_valid_hawk_header(&hawk_payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .header("authorization", header)
            .method(Method::GET)
            .data(state)
            .param("uid", &USER_ID_STR)
            .to_http_request();
        let mut payload = Payload::None;
        let result = block_on(HawkIdentifier::from_request(&req, &mut payload))
            .expect("Could not get result in valid_header_with_valid_path");
        assert_eq!(result.legacy_id, *USER_ID);
    }

    #[test]
    fn valid_header_with_invalid_uid_in_path() {
        // the uid in the hawk payload should match the UID in the path.
        let hawk_payload = HawkPayload::test_default(*USER_ID);
        let mismatch_uid = "5";
        let state = make_state();
        let uri = format!("/1.5/{}/storage/col2", mismatch_uid);
        let header =
            create_valid_hawk_header(&hawk_payload, &state, "GET", &uri, TEST_HOST, TEST_PORT);
        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", header)
            .method(Method::GET)
            .param("uid", mismatch_uid)
            .to_http_request();
        let result = block_on(HawkIdentifier::extract(&req));
        assert!(result.is_err());
        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
        let body = extract_body_as_str(ServiceResponse::new(req, response));
        assert_eq!(body, "0");

        /* New tests for when we can use descriptive errors

        let err: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(err["status"], 400);

        assert_eq!(err["errors"][0]["description"], "conflicts with payload");
        assert_eq!(err["errors"][0]["location"], "path");
        assert_eq!(err["errors"][0]["name"], "uid");
        */
    }

    #[actix_rt::test]
    async fn test_max_ttl() {
        let bso_body = json!([
            {"id": "123", "payload": "xxx", "sortindex": 23, "ttl": 94_608_000},
            {"id": "456", "payload": "xxxasdf", "sortindex": 23, "ttl": 999_999_999},
            {"id": "789", "payload": "xxxfoo", "sortindex": 23, "ttl": 1_000_000_000}
        ]);
        let result = post_collection("", &bso_body)
            .await
            .expect("Could not get result in test_valid_collection_post_request");
        assert_eq!(result.user_id.legacy_id, *USER_ID);
        assert_eq!(&result.collection, "tabs");
        assert_eq!(result.bsos.valid.len(), 2);
        assert_eq!(result.bsos.invalid.len(), 1);
        assert!(result.bsos.invalid.contains_key("789"));
    }
}
