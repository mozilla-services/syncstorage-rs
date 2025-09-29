use actix_web::{
    dev::{ConnectionInfo, Extensions, Payload},
    http::Uri,
    web::Data,
    Error, FromRequest, HttpMessage, HttpRequest,
};
use futures::future::{self, Ready};
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use syncserver_settings::Secrets;
use syncstorage_db::UserIdentifier;
use tokenserver_auth::TokenserverOrigin;

use crate::{
    error::{ApiError, ApiErrorKind},
    web::{
        auth::HawkPayload,
        error::{HawkErrorKind, ValidationErrorKind},
        extractors::{urldecode, RequestErrorLocation},
        DOCKER_FLOW_ENDPOINTS,
    },
};
use syncserver_common::Taggable;

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
    pub hashed_fxa_uid: String,
    pub hashed_device_id: String,
    pub tokenserver_origin: TokenserverOrigin,
}

impl HawkIdentifier {
    pub fn cmd_dummy() -> Self {
        // Create a "dummy" HawkID for use by DockerFlow commands
        Self {
            legacy_id: 0,
            fxa_uid: "cmd".to_owned(),
            fxa_kid: "cmd".to_owned(),
            hashed_fxa_uid: "cmd".to_owned(),
            hashed_device_id: "cmd".to_owned(),
            tokenserver_origin: TokenserverOrigin::default(),
        }
    }

    fn uid_from_path(uri: &Uri) -> Result<u64, Error> {
        // TODO: replace with proper path parser.
        // path: "/1.5/{uid}"
        let elements: Vec<&str> = uri.path().split('/').collect();
        if let Some(v) = elements.get(2) {
            let clean = match urldecode(v) {
                Err(e) => {
                    warn!("⚠️ HawkIdentifier Error invalid UID {:?} {:?}", v, e);
                    return Err(ValidationErrorKind::FromDetails(
                        "Invalid UID".to_owned(),
                        RequestErrorLocation::Path,
                        Some("uid".to_owned()),
                        Some("request.validate.hawk.invalid_uid"),
                    )
                    .into());
                }
                Ok(v) => v,
            };
            u64::from_str(&clean).map_err(|e| {
                warn!("⚠️ HawkIdentifier Error invalid UID {:?} {:?}", v, e);
                ValidationErrorKind::FromDetails(
                    "Invalid UID".to_owned(),
                    RequestErrorLocation::Path,
                    Some("uid".to_owned()),
                    Some("request.validate.hawk.invalid_uid"),
                )
                .into()
            })
        } else {
            warn!("⚠️ HawkIdentifier Error missing UID {:?}", uri);
            Err(ValidationErrorKind::FromDetails(
                "Missing UID".to_owned(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
                Some("request.validate.hawk.missing_uid"),
            ))?
        }
    }

    pub fn extrude<T>(
        msg: &T,
        method: &str,
        uri: &Uri,
        ci: &ConnectionInfo,
        secrets: &Secrets,
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
        let identifier = Self::generate(
            secrets,
            method,
            auth_header,
            ci,
            uri,
            &mut msg.extensions_mut(),
        )?;
        msg.extensions_mut().insert(identifier.clone());
        Ok(identifier)
    }

    pub fn generate(
        secrets: &Secrets,
        method: &str,
        header: &str,
        connection_info: &ConnectionInfo,
        uri: &Uri,
        exts: &mut Extensions,
    ) -> Result<Self, Error> {
        let payload = HawkPayload::extrude(header, method, secrets, connection_info, uri)?;
        let puid = Self::uid_from_path(uri)?;
        if payload.user_id != puid {
            warn!("⚠️ Hawk UID not in URI: {:?} {:?}", payload.user_id, uri);
            Err(ValidationErrorKind::FromDetails(
                "conflicts with payload".to_owned(),
                RequestErrorLocation::Path,
                Some("uid".to_owned()),
                Some("request.validate.hawk.uri_missing_uid"),
            ))?;
        }

        // Store the origin of the token so we can later use it as a tag when emitting metrics
        exts.insert(payload.tokenserver_origin);

        let user_id = HawkIdentifier {
            legacy_id: payload.user_id,
            fxa_uid: payload.fxa_uid,
            fxa_kid: payload.fxa_kid,
            hashed_fxa_uid: payload.hashed_fxa_uid,
            hashed_device_id: payload.hashed_device_id,
            tokenserver_origin: payload.tokenserver_origin,
        };
        Ok(user_id)
    }
}

impl From<HawkIdentifier> for UserIdentifier {
    fn from(hawk_id: HawkIdentifier) -> Self {
        Self {
            legacy_id: hawk_id.legacy_id,
            fxa_uid: hawk_id.fxa_uid,
            fxa_kid: hawk_id.fxa_kid,
            hashed_fxa_uid: hawk_id.hashed_fxa_uid,
            hashed_device_id: hawk_id.hashed_device_id,
        }
    }
}

impl FromRequest for HawkIdentifier {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    /// Use HawkPayload extraction and format as HawkIdentifier.
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Dummy token if a Docker Flow request is detected.
        if DOCKER_FLOW_ENDPOINTS.contains(&req.uri().path().to_lowercase().as_str()) {
            return future::ready(Ok(HawkIdentifier::cmd_dummy()));
        }
        let req = req.clone();
        let uri = req.uri();
        // NOTE: `connection_info()` will get a mutable reference lock on `extensions()`
        let connection_info = req.connection_info().clone();
        let method = req.method().clone();
        // Tried collapsing this to a `.or_else` and hit problems with the return resolving
        // to an appropriate error state. Can't use `?` since the function does not return a result.
        let secrets = match req.app_data::<Data<Arc<Secrets>>>() {
            Some(v) => v,
            None => {
                let err: ApiError = ApiErrorKind::Internal("No app_data Secrets".to_owned()).into();
                return future::ready(Err(err.into()));
            }
        };

        let result = Self::extrude(&req, method.as_str(), uri, &connection_info, secrets);

        if let Ok(ref hawk_id) = result {
            // Store the origin of the token as an extra to be included when emitting a Sentry error
            req.add_extra(
                "tokenserver_origin".to_owned(),
                hawk_id.tokenserver_origin.to_string(),
            );
        }

        future::ready(result)
    }
}
