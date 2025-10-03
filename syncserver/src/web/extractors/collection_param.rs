use std::str::FromStr;

use actix_web::{
    dev::{Extensions, Payload},
    http::Uri,
    Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::LocalBoxFuture;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use validator::Validate;

use super::{urldecode, RequestErrorLocation};
use crate::{server::COLLECTION_ID_REGEX, web::error::ValidationErrorKind};

lazy_static! {
    static ref VALID_COLLECTION_ID_REGEX: Regex =
        Regex::new(&format!("^{}$", COLLECTION_ID_REGEX)).unwrap();
}

/// Collection parameter Extractor
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct CollectionParam {
    #[validate(regex(path = *VALID_COLLECTION_ID_REGEX))]
    pub collection: String,
}

impl CollectionParam {
    fn col_from_path(uri: &Uri) -> Result<Option<CollectionParam>, Error> {
        // TODO: replace with proper path parser.
        // path: "/1.5/{uid}/storage/{collection}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        let elem = elements.get(3);
        if elem.is_none() || elem != Some(&"storage") || !(5..=6).contains(&elements.len()) {
            return Ok(None);
        }
        if let Some(v) = elements.get(4) {
            let mut sv = String::from_str(v).map_err(|_e| {
                ValidationErrorKind::FromDetails(
                    "Missing Collection".to_owned(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                    Some("request.process.missing_collection"),
                )
            })?;
            sv = urldecode(&sv).map_err(|_e| {
                ValidationErrorKind::FromDetails(
                    "Invalid Collection".to_owned(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                    Some("request.process.invalid_collection"),
                )
            })?;
            Ok(Some(Self { collection: sv }))
        } else {
            Err(ValidationErrorKind::FromDetails(
                "Missing Collection".to_owned(),
                RequestErrorLocation::Path,
                Some("collection".to_owned()),
                Some("request.process.missing_collection"),
            ))?
        }
    }

    pub fn extrude(uri: &Uri, extensions: &mut Extensions) -> Result<Option<Self>, Error> {
        if let Some(collection) = extensions.get::<Option<Self>>() {
            return Ok(collection.clone());
        }

        let collection = Self::col_from_path(uri)?;
        let result = if let Some(collection) = collection {
            collection.validate().map_err(|e| {
                ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::Path, None)
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
    type Error = Error;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            if let Some(collection) = Self::extrude(req.uri(), &mut req.extensions_mut())? {
                Ok(collection)
            } else {
                Err(ValidationErrorKind::FromDetails(
                    "Missing Collection".to_owned(),
                    RequestErrorLocation::Path,
                    Some("collection".to_owned()),
                    Some("request.process.missing_collection"),
                ))?
            }
        })
    }
}
