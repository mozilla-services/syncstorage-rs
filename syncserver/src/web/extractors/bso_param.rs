use std::str::FromStr;

use actix_web::{
    dev::{Extensions, Payload, RequestHead},
    http::Uri,
    Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{self, Ready};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use validator::Validate;

use super::{urldecode, RequestErrorLocation};
use crate::{server::BSO_ID_REGEX, web::error::ValidationErrorKind};

lazy_static! {
    static ref VALID_ID_REGEX: Regex = Regex::new(&format!("^{}$", BSO_ID_REGEX)).unwrap();
}

/// Bso id parameter extractor
#[derive(Clone, Debug, Deserialize, Validate)]
pub struct BsoParam {
    #[validate(regex(path = *VALID_ID_REGEX))]
    pub bso: String,
}

impl BsoParam {
    fn bsoparam_from_path(uri: &Uri) -> Result<Self, Error> {
        // TODO: replace with proper path parser
        // path: "/1.5/{uid}/storage/{collection}/{bso}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        let elem = elements.get(3);
        if elem.is_none() || elem != Some(&"storage") || elements.len() != 6 {
            return Err(ValidationErrorKind::FromDetails(
                "Invalid BSO".to_owned(),
                RequestErrorLocation::Path,
                Some("bso".to_owned()),
                Some("request.process.invalid_bso"),
            ))?;
        }
        if let Some(v) = elements.get(5) {
            let sv = urldecode(&String::from_str(v).map_err(|e| {
                warn!("⚠️ Invalid BsoParam Error: {:?} {:?}", v, e);
                ValidationErrorKind::FromDetails(
                    "Invalid BSO".to_owned(),
                    RequestErrorLocation::Path,
                    Some("bso".to_owned()),
                    Some("request.process.invalid_bso"),
                )
            })?)
            .map_err(|e| {
                warn!("⚠️ Invalid BsoParam Error: {:?} {:?}", v, e);
                ValidationErrorKind::FromDetails(
                    "Invalid BSO".to_owned(),
                    RequestErrorLocation::Path,
                    Some("bso".to_owned()),
                    Some("request.process.invalid_bso"),
                )
            })?;
            Ok(Self { bso: sv })
        } else {
            warn!("⚠️ Missing BSO: {:?}", uri.path());
            Err(ValidationErrorKind::FromDetails(
                "Missing BSO".to_owned(),
                RequestErrorLocation::Path,
                Some("bso".to_owned()),
                Some("request.process.missing_bso"),
            ))?
        }
    }

    pub fn extrude(head: &RequestHead, extensions: &mut Extensions) -> Result<Self, Error> {
        let uri = head.uri.clone();
        if let Some(bso) = extensions.get::<BsoParam>() {
            return Ok(bso.clone());
        }
        let bso = Self::bsoparam_from_path(&uri)?;
        bso.validate().map_err(|e| {
            ValidationErrorKind::FromValidationErrors(e, RequestErrorLocation::Path, None)
        })?;
        extensions.insert(bso.clone());
        Ok(bso)
    }
}

impl FromRequest for BsoParam {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        future::ready(Self::extrude(req.head(), &mut req.extensions_mut()))
    }
}
