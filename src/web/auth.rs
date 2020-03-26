//! Types for parsing and authenticating HAWK headers.
//! Matches the [Python logic](https://github.com/mozilla-services/tokenlib).
//! We may want to extract this to its own repo/crate in due course.
#![cfg_attr(
    feature = "no_auth",
    allow(dead_code, unused_imports, unused_variables)
)]

use std::convert::TryInto;

use base64;
use chrono::offset::Utc;
use hawk::{self, Header as HawkHeader, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::Sha256;
use time::Duration;

use actix_web::dev::ConnectionInfo;
use actix_web::http::Uri;

use super::tags::Tags;
use super::{
    error::{HawkErrorKind, ValidationErrorKind},
    extractors::RequestErrorLocation,
};
use crate::error::{ApiErrorKind, ApiResult};
use crate::settings::Secrets;

/// A parsed and authenticated JSON payload
/// extracted from the signed `id` property
/// of a Hawk `Authorization` header.
///
/// Not included here are the `fxa_uid` and `device_id` properties,
/// which may also be present in the JSON payload.
#[derive(Debug, Deserialize, PartialEq, Serialize)]
pub struct HawkPayload {
    /// Expiry time for the payload, in seconds.
    pub expires: f64,

    /// Base URI for the storage node.
    pub node: String,

    /// Salt used during HKDF-expansion of the token secret.
    pub salt: String,

    /// User identifier.
    #[serde(rename = "uid")]
    pub user_id: u64,

    #[serde(default)]
    pub fxa_uid: String,

    #[serde(default)]
    pub fxa_kid: String,

    #[serde(default, rename = "hashed_device_id")]
    pub device_id: String,
}

impl HawkPayload {
    /// Parse and authenticate a payload
    /// using the supplied arguments.
    ///
    /// Assumes that the header string
    /// includes the `Hawk ` prefix.
    fn new(
        header: &str,
        method: &str,
        path: &str,
        host: &str,
        port: u16,
        secrets: &Secrets,
        expiry: u64,
    ) -> ApiResult<HawkPayload> {
        if &header[0..5] != "Hawk " {
            Err(HawkErrorKind::MissingPrefix)?;
        }

        let header: HawkHeader = header[5..].parse()?;
        let id = header.id.as_ref().ok_or(HawkErrorKind::MissingId)?;

        let payload = HawkPayload::extract_and_validate(id, secrets, expiry)?;

        let token_secret = hkdf_expand_32(
            format!("services.mozilla.com/tokenlib/v1/derive/{}", id).as_bytes(),
            Some(payload.salt.as_bytes()),
            &secrets.master_secret,
        )?;
        let token_secret = base64::encode_config(&token_secret, base64::URL_SAFE);

        let request = RequestBuilder::new(method, host, port, path).request();

        #[cfg(feature = "no_auth")]
        {
            Ok(payload)
        }

        #[cfg(not(feature = "no_auth"))]
        {
            let mut duration: std::time::Duration = Duration::weeks(52)
                .try_into()
                .map_err(|_| ApiErrorKind::Internal("Duration::weeks".to_owned()))?;
            if cfg!(test) {
                // test cases are valid until 3018. Add millenia as required.
                duration *= 1000;
            }
            if request.validate_header(
                &header,
                &Key::new(token_secret.as_bytes(), hawk::DigestAlgorithm::Sha256)?,
                // Allow plenty of leeway for clock skew, because
                // client timestamps tend to be all over the shop
                duration,
            ) {
                Ok(payload)
            } else {
                Err(HawkErrorKind::InvalidHeader)?
            }
        }
    }

    /// Decode the `id` property of a Hawk header
    /// and verify the payload part against the signature part.
    fn extract_and_validate(id: &str, secrets: &Secrets, expiry: u64) -> ApiResult<HawkPayload> {
        let decoded_id = base64::decode_config(id, base64::URL_SAFE)?;
        if decoded_id.len() <= 32 {
            Err(HawkErrorKind::TruncatedId)?;
        }

        let payload_length = decoded_id.len() - 32;
        let payload = &decoded_id[0..payload_length];
        let signature = &decoded_id[payload_length..];

        #[cfg(not(feature = "no_auth"))]
        verify_hmac(payload, &secrets.signing_secret, signature)?;

        let payload: HawkPayload = serde_json::from_slice(payload)?;

        if expiry == 0 || (payload.expires.round() as u64) > expiry {
            Ok(payload)
        } else {
            Err(HawkErrorKind::Expired)?
        }
    }

    #[cfg(test)]
    pub fn test_default(user_id: u64) -> Self {
        HawkPayload {
            expires: Utc::now().timestamp() as f64 + 200_000.0,
            node: "friendly-node".to_string(),
            salt: "saltysalt".to_string(),
            user_id,
            fxa_uid: "xxx_test".to_owned(),
            fxa_kid: "xxx_test".to_owned(),
            device_id: "xxx_test".to_owned(),
        }
    }
}

impl HawkPayload {
    pub fn extrude(
        header: &str,
        method: &str,
        secrets: &Secrets,
        ci: &ConnectionInfo,
        uri: &Uri,
        tags: Option<Tags>,
    ) -> ApiResult<Self> {
        let host_port: Vec<_> = ci.host().splitn(2, ':').collect();
        let host = host_port[0];
        let port = if host_port.len() == 2 {
            host_port[1].parse().map_err(|_| {
                ValidationErrorKind::FromDetails(
                    "Invalid port (hostname:port) specified".to_owned(),
                    RequestErrorLocation::Header,
                    None,
                    tags,
                )
            })?
        } else if ci.scheme() == "https" {
            443
        } else {
            80
        };
        let path = uri.path_and_query().ok_or(HawkErrorKind::MissingPath)?;
        let expiry = if path.path().ends_with("/info/collections") {
            0
        } else {
            Utc::now().timestamp() as u64
        };

        HawkPayload::new(header, method, path.as_str(), host, port, &secrets, expiry)
    }
}

/// Helper function for [HKDF](https://tools.ietf.org/html/rfc5869) expansion to 32 bytes.
pub fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> ApiResult<[u8; 32]> {
    let mut result = [0u8; 32];
    let hkdf = Hkdf::<Sha256>::new(salt, key);
    // This unwrap will never panic because 32 bytes is a valid size for Hkdf<Sha256>
    hkdf.expand(info, &mut result)
        .map_err(|e| ApiErrorKind::Internal(format!("HKDF Error: {:?}", e)))?;
    Ok(result)
}

/// Helper function for [HMAC](https://tools.ietf.org/html/rfc2104) verification.
fn verify_hmac(info: &[u8], key: &[u8], expected: &[u8]) -> ApiResult<()> {
    let mut hmac: Hmac<Sha256> = Hmac::new_varkey(key)?;
    hmac.input(info);
    hmac.verify(expected).map_err(From::from)
}

#[cfg(test)]
mod tests {
    use super::{HawkPayload, Secrets};
    use crate::settings::Settings;

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
        fixture.header.mac = "E7j1UjN7//mh7pYXsgGi3n0KGR+sUPpuyogDVzJWaHg=".to_string();
        fixture.header.nonce = "4Rj7c+0=".to_string();
        fixture.header.ts = 1_569_608_439;
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
            &Secrets::new("wibble").unwrap(),
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
        fixture.header.ts = 1_536_198_978;

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
            TestFixture {
                header: HawkHeader::new(
                    "eyJ1aWQiOiAxLCAibm9kZSI6ICJodHRwOi8vbG9jYWxob3N0OjUwMDAiLCAiZXhwaXJlcyI6IDE4ODQ5Njg0MzkuMCwgImZ4YV91aWQiOiAiMzE5Yjk4Zjk5NjFmZjFkYmRkMDczMTNjZDZiYTkyNWEiLCAiZnhhX2tpZCI6ICJkZTY5N2FkNjZkODQ1YjI4NzNjOWQ3ZTEzYjg5NzFhZiIsICJoYXNoZWRfZnhhX3VpZCI6ICIwZThkZjVkNDEzOThhMzg5OTEzYmQ4NDAyNDM1NjQ5NTE4YWY0NjQ5M2RhMWQ0YTQzN2E0NmRjMTc4NGM1MDFhIiwgImhhc2hlZF9kZXZpY2VfaWQiOiAiMmJjYjkyZjRkNDY5OGMzZDdiMDgzYTNjNjk4YTE2Y2NkNzhiYzJhOGQyMGE5NmU0YmIxMjhkZGNlYWY0ZTBiNiIsICJzYWx0IjogIjJiMzA3YiJ9lXaC5pIOenf7qL1AWlgKFvYH63nakyniTXP-7acS5cw=",
                    "UwDpC+DSrHCSTQSfMOWlueB6kM6gHb0Hsv8eU9ZcTVs=",
                    "h1Ch4vo=",
                    1_569_608_439,
                ),
                request: Request::new(
                    "GET",
                    "/storage/1.5/1/storage/col2",
                    "localhost",
                    5000,
                ),
                settings: Settings {
                    debug: false,
                    port: 0,
                    host: "127.0.0.1".to_string(),
                    database_url: "".to_string(),
                    database_pool_max_size: None,
                    database_use_test_transactions: false,
                    limits: Default::default(),
                    master_secret: Secrets::new("Ted Koppel is a robot").unwrap(),
                    ..Default::default()
                },
                expected: HawkPayload {
                    expires: 1_884_968_439.0,
                    node: "http://localhost:5000".to_string(),
                    salt: "2b307b".to_string(),
                    user_id: 1,
                    fxa_uid: "319b98f9961ff1dbdd07313cd6ba925a".to_owned(),
                    fxa_kid: "de697ad66d845b2873c9d7e13b8971af".to_owned(),
                    device_id: "2bcb92f4d4698c3d7b083a3c698a16ccd78bc2a8d20a96e4bb128ddceaf4e0b6".to_owned(),
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
