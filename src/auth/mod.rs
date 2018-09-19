// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Types for parsing and authenticating HAWK headers.
//! Matches the [Python logic](https://github.com/mozilla-services/tokenlib).
//! We may want to extract this to its own repo/crate in due course.

#[cfg(test)]
mod test;

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use actix_web::{error::ResponseError, http::header::ToStrError, FromRequest, HttpRequest};
use base64::{self, DecodeError};
use chrono::offset::Utc;
use hawk::{Error as HawkError, Header as HawkHeader, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{
    crypto_mac::{InvalidKeyLength, MacError},
    Hmac, Mac,
};
use ring;
use serde_json::{self, Error as JsonError};
use sha2::Sha256;
use time::Duration;

use server::ServerState;
use settings::{Secrets, Settings};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HawkPayload {
    pub expires: f64,
    pub node: String,
    pub salt: String,
    pub uid: u64,
}

impl HawkPayload {
    fn new(
        header: &str,
        method: &str,
        path: &str,
        host: &str,
        port: u16,
        secrets: &Secrets,
        expiry: u64,
    ) -> AuthResult<HawkPayload> {
        if &header[0..5] != "Hawk " {
            return Err(AuthError);
        }

        let header: HawkHeader = header[5..].parse()?;
        let id = header.id.as_ref().ok_or(AuthError)?;

        let payload = HawkPayload::extract_and_validate(id, secrets, expiry)?;

        let token_secret = hkdf_expand_32(
            format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
            Some(payload.salt.as_bytes()),
            &secrets.master_secret,
        );
        let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);

        let request = RequestBuilder::new(method, host, port, path).request();
        if request.validate_header(
            &header,
            &Key::new(token_secret.as_bytes(), &ring::digest::SHA256),
            // Allow plenty of leeway for clock skew, because
            // client timestamps tend to be all over the shop
            Duration::weeks(52),
        ) {
            Ok(payload)
        } else {
            Err(AuthError)
        }
    }

    fn extract_and_validate(id: &str, secrets: &Secrets, expiry: u64) -> AuthResult<HawkPayload> {
        let decoded_id = base64::decode_config(id, base64::URL_SAFE)?;
        if decoded_id.len() <= 32 {
            return Err(AuthError);
        }

        let payload_length = decoded_id.len() - 32;
        let payload = &decoded_id[0..payload_length];
        let signature = &decoded_id[payload_length..];

        verify_hmac(payload, &secrets.signing_secret, signature)?;

        let payload: HawkPayload = serde_json::from_slice(payload)?;

        if (payload.expires.round() as u64) > expiry {
            Ok(payload)
        } else {
            Err(AuthError)
        }
    }
}

impl FromRequest<ServerState> for HawkPayload {
    type Config = Settings;
    type Result = AuthResult<HawkPayload>;

    /// Extract and validate HAWK payload from an actix request object.
    fn from_request(request: &HttpRequest<ServerState>, settings: &Self::Config) -> Self::Result {
        HawkPayload::new(
            request
                .headers()
                .get("authorization")
                .ok_or(AuthError)?
                .to_str()?,
            request.method().as_str(),
            request.uri().path_and_query().ok_or(AuthError)?.as_str(),
            request.uri().host().unwrap_or("127.0.0.1"),
            request.uri().port().unwrap_or(settings.port),
            &request.state().secrets,
            Utc::now().timestamp() as u64,
        )
    }
}

pub fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let hkdf: Hkdf<Sha256> = Hkdf::extract(salt, key);
    // This unwrap will never panic because 32 bytes is a valid size for Hkdf<Sha256>
    hkdf.expand(info, &mut result).unwrap();
    result
}

fn verify_hmac(info: &[u8], key: &[u8], expected: &[u8]) -> AuthResult<()> {
    let mut hmac: Hmac<Sha256> = Hmac::new_varkey(key)?;
    hmac.input(info);
    hmac.verify(expected).map_err(From::from)
}

pub type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug)]
pub struct AuthError;

impl Display for AuthError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "invalid hawk header")
    }
}

impl Error for AuthError {}
impl ResponseError for AuthError {}

macro_rules! from_error {
    ($error:ty) => {
        impl From<$error> for AuthError {
            fn from(_error: $error) -> AuthError {
                AuthError
            }
        }
    };
}

from_error!(DecodeError);
from_error!(HawkError);
from_error!(InvalidKeyLength);
from_error!(JsonError);
from_error!(MacError);
from_error!(ToStrError);
