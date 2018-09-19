// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

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
                master_secret: Secrets::new("Ted Koppel is a robot"),
            },
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
