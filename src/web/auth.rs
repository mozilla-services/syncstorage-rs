//! Types for parsing and authenticating HAWK headers.
//! Matches the [Python logic](https://github.com/mozilla-services/tokenlib).
//! We may want to extract this to its own repo/crate in due course.

use actix_web::{FromRequest, HttpRequest};
use base64;
use chrono::offset::Utc;
use hawk::{Header as HawkHeader, Key, RequestBuilder};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use ring;
use serde_json;
use sha2::Sha256;
use time::Duration;

use super::{
    error::{HawkErrorKind, ValidationErrorKind},
    extractors::RequestErrorLocation,
};
use error::ApiResult;
use server::ServerState;
use settings::Secrets;

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
            Err(HawkErrorKind::InvalidHeader)?
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

        verify_hmac(payload, &secrets.signing_secret, signature)?;

        let payload: HawkPayload = serde_json::from_slice(payload)?;

        if (payload.expires.round() as u64) > expiry {
            Ok(payload)
        } else {
            Err(HawkErrorKind::Expired)?
        }
    }

    #[cfg(test)]
    pub fn test_default() -> Self {
        HawkPayload {
            expires: Utc::now().timestamp() as f64 + 200000.0,
            node: "friendly-node".to_string(),
            salt: "saltysalt".to_string(),
            user_id: 1,
        }
    }
}

impl FromRequest<ServerState> for HawkPayload {
    /// Default [`Settings`](../../settings/struct.Settings.html) instance.
    ///
    /// Not hugely useful, all of the configurable settings
    /// can be found on the [request state](../../server/struct.ServerState.html) instead.
    type Config = ();

    /// Result-wrapped `HawkPayload` instance.
    type Result = ApiResult<HawkPayload>;

    /// Parse and authenticate a Hawk payload
    /// from the `Authorization` header
    /// of an actix request object.
    fn from_request(request: &HttpRequest<ServerState>, _: &Self::Config) -> Self::Result {
        let ci = request.connection_info();
        let host_port: Vec<_> = ci.host().splitn(2, ':').collect();
        let host = host_port[0];
        let port = if host_port.len() == 2 {
            host_port[1].parse().map_err(|_| {
                ValidationErrorKind::FromDetails(
                    "Invalid port (hostname:port) specified".to_owned(),
                    RequestErrorLocation::Header,
                    None,
                )
            })?
        } else if ci.scheme() == "https" {
            443
        } else {
            80
        };

        HawkPayload::new(
            request
                .headers()
                .get("authorization")
                .ok_or(HawkErrorKind::MissingHeader)?
                .to_str()?,
            request.method().as_str(),
            request
                .uri()
                .path_and_query()
                .ok_or(HawkErrorKind::MissingPath)?
                .as_str(),
            host,
            port,
            &request.state().secrets,
            Utc::now().timestamp() as u64,
        )
    }
}

/// Helper function for [HKDF](https://tools.ietf.org/html/rfc5869) expansion to 32 bytes.
pub fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> [u8; 32] {
    let mut result = [0u8; 32];
    let hkdf: Hkdf<Sha256> = Hkdf::extract(salt, key);
    // This unwrap will never panic because 32 bytes is a valid size for Hkdf<Sha256>
    hkdf.expand(info, &mut result).unwrap();
    result
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
    use settings::Settings;

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
                settings: Settings {
                    debug: false,
                    port: 0,
                    database_url: "".to_string(),
                    database_pool_max_size: None,
                    database_use_test_transactions: false,
                    limits: Default::default(),
                    master_secret: Secrets::new("Ted Koppel is a robot"),
                },
                expected: HawkPayload {
                    expires: 1536199274.0,
                    node: "http://localhost:5000".to_string(),
                    salt: "b0260e".to_string(),
                    user_id: 1,
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
