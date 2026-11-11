use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::jwk::Jwk;
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use serde_json::json;

/// RSA private key for TEST_JWK.
///
/// Shared with the Python integration tests, which have to sign assertions the
/// server will accept, so the material lives in one file rather than being
/// duplicated per language. `Dockerfile` already copies `tools/fxa_delete`
/// into the e2e image.
pub const TEST_PRIVATE_KEY_PEM: &[u8] =
    include_bytes!("../../tools/fxa_delete/testing/delete_test_key.pem");

/// Private key for testing mismatching signatures.
pub const OTHER_PRIVATE_KEY_PEM: &[u8] = b"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEAyd0B8fYfYJ8lKbkhii838MjGp3VTNnveK7C/sxWHysEcGioH
TUvL9huz6tx41stE8Gi+uIGeXNzfcX3ZX2AFBnsT3z1+ndaPMw9dIhRWghrBp3u3
KB8soG7M81b0fLtTyymBbUVCnA9NRgFjr9A2WJacuJp8CGHLzeNhKMXJLrskUJLa
RktWeW79eyXTYuqiz0LlcDQAVjBeg30WPr80Wo6Scw55gC+c1dZl+P8FTX4/URxp
dimwnyU9mfje21ZPlNiIYHjPFpg5Zim/QasjMHzUAVTOEBCvwWY2KRMWJRFPju0i
goRkSV/MNq3doYKhQXM7cxKrhxsgNEhk9CTu7wIDAQABAoIBAFjx/eJsjWTYmtpo
jYPCzIZXIVk2FCVkrP9ZUQ6KxRusvUI2FKOVa1iU2lD5NnGGfWjk7myECJBobjgm
uLoSqAQ0BQyPnxPTL6PS+DmE9p07RusSUyDlo5dJWxs5zF6NeB2Du1i3dOMoxua6
w/764odkTcf1ogNbfB7LOstpYv0ohf+OmLIndHDws1qqfh58KHZlsRf4nz1TK73s
Pe5XbxpfjeDNp79czKJBr1HsMPKkCXw3VYhUNpoP7vC/UeSwRhuPrI7skO2+bXt7
SXR6rA7SxCgPKR5f3+LOV6GiU4ixc2DRMU/n8BROb+CIGE52+qbcjhNJXN9XYWoY
XDLlg/ECgYEA7yxlZez5hutCtnt/JvRaxaGywEHzcOazhUJNFMazs5INni8TKqZG
MqNUH0aa/GdT8dOE/HFcilSgr2glcchiJb7cIg7O1rDobIP8iZkml40Xs6S20dEW
83/+juX3VknPxxBGYkFo5ywSFKxD1zX9O9OB+u9DGHL+gWRgr4yZCwkCgYEA2BCk
bE0wzyuwNx4qmSsEAdtxh4ZyEgv46pXS18DH0oEUd/jzbAbBgz2vYIrWsHqszBDD
mnJr8YOhpqXMRmmyuPBuxvrNPwX+I9WE55NmVkY5NV1uDv26UgSWcuVMIBH2k9At
72F7F03zfovytcAuW1VbfjNESIZkA41JBOe7EDcCgYEA2nhbRvdoFu3fSoEUbKjY
IZ7KgQO9M2wIn7koX8oBbA4FknC9uT+Y77hxpv//on9gFo139IA4X8Nd49vmGEFK
JeBphFKybTm7lSQbEjVrIxQmilnzBUVRCavpAu7dN1zFBri/EhFdmYyQF4Ijlfoj
DvrsyCK1zyd7gwYFq1VqlsECgYAnK3UzcRb9J9VtWJmuZN74GzlMsXHylZsNpBWy
KW/QWLhGO6qdlef1C/TEUscy/TpgUFW1pTKueQeQN5R922GcJ3JdvlABMevtwSKz
/MPbtiVe6E4wh40Em3JO6ATR94+1IlOBhzGSev4+nc5lZq7Avgu1KEQjxcFR54Yq
TnxaJwKBgQDPDdfixZlqxuKiWewJupuDI9T2HBKp0BnRBzKZjSn+IuvCHUQC8xtv
yyVcq33mCxMSP9YqFrvZwjnoVMWCbVTPU7f6xzP4IGxXpySkVf8QwGp7Fc9q/XG4
8DA43GmyC52UynXzmBzTZfURio3qSQRFPtyZApJDcicR1nUw22KolA==
-----END RSA PRIVATE KEY-----";

/// Public JWK of TEST_PRIVATE_KEY_PEM. See the note there on the shared file.
pub const TEST_JWK: &str = include_str!("../../tools/fxa_delete/testing/delete_test_jwk.json");

/// Public JWK of OTHER_PRIVATE_KEY_PEM.
///
/// A second, genuinely distinct key, so key-rotation tests exercise two real
/// keys rather than the same one twice.
pub const OTHER_TEST_JWK: &str = r#"{
    "kty": "RSA",
    "alg": "RS256",
    "n": "yd0B8fYfYJ8lKbkhii838MjGp3VTNnveK7C_sxWHysEcGioHTUvL9huz6tx41stE8Gi-uIGeXNzfcX3ZX2AFBnsT3z1-ndaPMw9dIhRWghrBp3u3KB8soG7M81b0fLtTyymBbUVCnA9NRgFjr9A2WJacuJp8CGHLzeNhKMXJLrskUJLaRktWeW79eyXTYuqiz0LlcDQAVjBeg30WPr80Wo6Scw55gC-c1dZl-P8FTX4_URxpdimwnyU9mfje21ZPlNiIYHjPFpg5Zim_QasjMHzUAVTOEBCvwWY2KRMWJRFPju0igoRkSV_MNq3doYKhQXM7cxKrhxsgNEhk9CTu7w",
    "e": "AQAB"
}"#;

pub fn other_test_jwk() -> Jwk {
    serde_json::from_str(OTHER_TEST_JWK).unwrap()
}

pub fn test_jwk() -> Jwk {
    serde_json::from_str(TEST_JWK).unwrap()
}

/// Builds delete assertions for tests.
///
/// A builder rather than a wide positional function because the
/// non-interchangeability tests vary exactly one field at a time, and a
/// `make_delete_assertion(sub, aud, iss, typ, exp, jti, key)` call site is
/// unreadable at the point where it matters most.
///
/// Defaults produce a token `FxaDeleteAssertionVerifier` accepts.
pub struct DeleteAssertionBuilder {
    sub: String,
    aud: Option<String>,
    iss: Option<String>,
    /// Raw header value. FxA signs the unprefixed form; the verifier applies
    /// RFC 7515 §4.1.9's implicit `application/` prefix.
    typ: Option<String>,
    exp_in: i64,
    jti: Option<String>,
    iat: Option<i64>,
    key: Vec<u8>,
}

impl DeleteAssertionBuilder {
    pub fn new(sub: &str, aud: &str, iss: &str) -> Self {
        Self {
            sub: sub.to_owned(),
            aud: Some(aud.to_owned()),
            iss: Some(iss.to_owned()),
            typ: Some("fxa-sync-delete+jwt".to_owned()),
            exp_in: 30,
            jti: Some("test-jti".to_owned()),
            iat: None,
            key: TEST_PRIVATE_KEY_PEM.to_vec(),
        }
    }

    /// Drop a claim entirely, rather than setting it wrong. jsonwebtoken only
    /// checks `aud`/`iss` when the claim is *present*, so omission is a
    /// distinct and more dangerous case than a mismatch.
    pub fn without_aud(mut self) -> Self {
        self.aud = None;
        self
    }
    pub fn without_iss(mut self) -> Self {
        self.iss = None;
        self
    }
    pub fn without_jti(mut self) -> Self {
        self.jti = None;
        self
    }
    pub fn without_typ(mut self) -> Self {
        self.typ = None;
        self
    }

    pub fn aud(mut self, aud: &str) -> Self {
        self.aud = Some(aud.to_owned());
        self
    }
    pub fn iss(mut self, iss: &str) -> Self {
        self.iss = Some(iss.to_owned());
        self
    }
    pub fn typ(mut self, typ: &str) -> Self {
        self.typ = Some(typ.to_owned());
        self
    }
    /// Seconds from now; negative for an already-expired assertion.
    pub fn exp_in(mut self, secs: i64) -> Self {
        self.exp_in = secs;
        self
    }
    pub fn key(mut self, pem: &[u8]) -> Self {
        self.key = pem.to_vec();
        self
    }

    /// A multi-valued `aud`. jsonwebtoken matches audiences by set
    /// intersection, so without a single-valued claims type one token could
    /// satisfy two differently configured verifiers.
    pub fn build_with_aud_array(self, auds: &[&str]) -> String {
        self.encode(Some(json!(auds)))
    }

    pub fn build(self) -> String {
        let aud = self.aud.clone().map(|a| json!(a));
        self.encode(aud)
    }

    fn encode(self, aud: Option<serde_json::Value>) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let mut claims = serde_json::Map::new();
        claims.insert("sub".to_owned(), json!(self.sub));
        claims.insert("exp".to_owned(), json!(now + self.exp_in));
        claims.insert("iat".to_owned(), json!(self.iat.unwrap_or(now)));
        if let Some(aud) = aud {
            claims.insert("aud".to_owned(), aud);
        }
        if let Some(iss) = self.iss {
            claims.insert("iss".to_owned(), json!(iss));
        }
        if let Some(jti) = self.jti {
            claims.insert("jti".to_owned(), json!(jti));
        }

        let mut header = Header::new(Algorithm::RS256);
        // Header::new sets `typ: Some("JWT")`, so clearing it needs an
        // explicit None rather than just not setting it.
        header.typ = self.typ;

        let key = EncodingKey::from_rsa_pem(&self.key).expect("invalid PEM key");
        encode(&header, &serde_json::Value::Object(claims), &key).unwrap()
    }
}

/// Happy-path delete assertion, signed with TEST_PRIVATE_KEY_PEM.
pub fn make_delete_assertion(sub: &str, aud: &str, iss: &str) -> String {
    DeleteAssertionBuilder::new(sub, aud, iss).build()
}

/// Sign a SET with the given PEM. Mirrors the claims the FxA Event Broker emits: {iss, aud, sub,
/// iat, jti, events}
pub fn make_set(sub: &str, aud: &str, events: serde_json::Value, private_key_pem: &[u8]) -> String {
    make_set_with_exp(sub, aud, events, private_key_pem, None)
}

/// As [`make_set`], but able to add an `exp`.
///
/// Real FxA SETs carry no `exp` — that absence is one of the barriers stopping
/// a SET being accepted as a delete assertion. Tests that want to prove the
/// *other* barriers are load-bearing need to remove it from the equation.
pub fn make_set_with_exp(
    sub: &str,
    aud: &str,
    events: serde_json::Value,
    private_key_pem: &[u8],
    exp_in: Option<i64>,
) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let mut claims = json!({
        "iss": "https://accounts.firefox.com/",
        "sub": sub,
        "aud": aud,
        "iat": now,
        "jti": "wibble",
        "events": events,
    });
    if let Some(exp_in) = exp_in {
        claims["exp"] = json!(now + exp_in);
    }
    let key = EncodingKey::from_rsa_pem(private_key_pem).expect("invalid PEM key");
    // Header::new sets `typ: Some("JWT")`. Real FxA SETs carry no typ at all
    // (jwtool signSync emits only alg/jku/kid), but nothing here depends on
    // that, and leaving the default keeps this closer to a generic JWT.
    encode(&Header::new(Algorithm::RS256), &claims, &key).unwrap()
}
