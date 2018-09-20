//! Types for parsing and authenticating HAWK headers.
//! Matches the [Python logic](https://github.com/mozilla-services/tokenlib).
//! We may want to extract this to its own repo/crate in due course.

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

#[cfg(test)]
mod tests {
    use super::{HawkPayload, Secrets, Settings};

    #[test]
    fn valid_header() {
        let fixture = TestFixture::new();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_ok());
        result
            .map(|payload| assert_eq!(payload, fixture.expected))
            .unwrap();
    }

    #[test]
    fn valid_header_with_querystring() {
        let mut fixture = TestFixture::new();
        fixture.header.mac = "xRVjP7607eZUWCBxJKwTo1CsLcNf4TZwUUNrLPUqkdQ=".to_string();
        fixture.header.nonce = "1d4mRs0=".to_string();
        fixture.header.ts = 1536198978;
        fixture.request.method = "POST".to_string();
        fixture
            .request
            .path
            .push_str("?batch=MTUzNjE5ODk3NjkyMQ==&commit=true");

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_ok());
        result
            .map(|payload| assert_eq!(payload, fixture.expected))
            .unwrap();
    }

    #[test]
    fn missing_hawk_prefix() {
        let fixture = TestFixture::new();

        let result = HawkPayload::new(
            &fixture.header.to_string()[1..],
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_master_secret() {
        let fixture = TestFixture::new();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &Secrets::new("wibble"),
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_signature() {
        let mut fixture = TestFixture::new();
        let signature_index = fixture.header.id.len() - 32;
        fixture
            .header
            .id
            .replace_range(signature_index.., "01234567890123456789012345678901");

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn expired_payload() {
        let fixture = TestFixture::new();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_mac() {
        let mut fixture = TestFixture::new();
        fixture.header.mac = "xRVjP7607eZUWCBxJKwTo1CsLcNf4TZwUUNrLPUqkdQ=".to_string();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_nonce() {
        let mut fixture = TestFixture::new();
        fixture.header.nonce = "1d4mRs0=".to_string();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_ts() {
        let mut fixture = TestFixture::new();
        fixture.header.ts = 1536198978;

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_method() {
        let mut fixture = TestFixture::new();
        fixture.request.method = "POST".to_string();

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_path() {
        let mut fixture = TestFixture::new();
        fixture
            .request
            .path
            .push_str("?batch=MTUzNjE5ODk3NjkyMQ==&commit=true");

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_host() {
        let mut fixture = TestFixture::new();
        fixture.request.host.push_str(".com");

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[test]
    fn bad_port() {
        let mut fixture = TestFixture::new();
        fixture.request.port += 1;

        let result = HawkPayload::new(
            &fixture.header.to_string(),
            &fixture.request.method,
            &fixture.request.path,
            &fixture.request.host,
            fixture.request.port,
            &fixture.settings.master_secret,
            fixture.expected.expires.round() as u64 - 1,
        );

        assert!(result.is_err());
    }

    #[derive(Debug)]
    struct TestFixture {
        pub header: HawkHeader,
        pub request: Request,
        pub settings: Settings,
        pub expected: HawkPayload,
    }

    impl TestFixture {
        fn new() -> TestFixture {
            let mut settings = Settings::with_env_and_config_file(None).unwrap();
            settings.master_secret = Secrets::new("Ted Koppel is a robot");
            TestFixture {
                header: HawkHeader::new(
                    "eyJub2RlIjogImh0dHA6Ly9sb2NhbGhvc3Q6NTAwMCIsICJ1aWQiOiAxLCAiZXhwaXJlcyI6IDE1MzYxOTkyNzQsICJmeGFfdWlkIjogIjMxOWI5OGY5OTYxZmYxZGJkZDA3MzEzY2Q2YmE5MjVhIiwgInNhbHQiOiAiYjAyNjBlIiwgImRldmljZV9pZCI6ICJjMDlkMjZmYWYyYjQ5YWI2NGEyODgyOTA3MjA2ZDBiNSJ96drmQ_KNFOe7U3g1D8ZX5-he2Bv2aRvKZzBPrCjHKO4=",
                    "+1oGdzqpxYndK5ejQLdnZpXgGSt/IlxNh5MvcR6j7t4=",
                    "omxLZWE=",
                    1536198980,
                ),
                request: Request::new(
                    "GET",
                    "/storage/1.5/1/storage/col2",
                    "localhost",
                    5000,
                ),
                settings,
                expected: HawkPayload {
                    expires: 1536199274.0,
                    node: "http://localhost:5000".to_string(),
                    salt: "b0260e".to_string(),
                    uid: 1,
                },
            }
        }
    }

    #[derive(Debug)]
    struct HawkHeader {
        pub id: String,
        pub mac: String,
        pub nonce: String,
        pub ts: u64,
    }

    impl HawkHeader {
        fn new(id: &str, mac: &str, nonce: &str, ts: u64) -> HawkHeader {
            HawkHeader {
                id: id.to_string(),
                mac: mac.to_string(),
                nonce: nonce.to_string(),
                ts,
            }
        }
    }

    impl ToString for HawkHeader {
        fn to_string(&self) -> String {
            format!(
                "Hawk id=\"{}\", mac=\"{}\", nonce=\"{}\", ts=\"{}\"",
                self.id, self.mac, self.nonce, self.ts
            )
        }
    }

    #[derive(Debug)]
    struct Request {
        pub method: String,
        pub path: String,
        pub host: String,
        pub port: u16,
    }

    impl Request {
        fn new(method: &str, path: &str, host: &str, port: u16) -> Request {
            Request {
                method: method.to_string(),
                path: path.to_string(),
                host: host.to_string(),
                port,
            }
        }
    }
}
