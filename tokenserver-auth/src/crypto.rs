use std::time::{SystemTime, UNIX_EPOCH};

use hkdf::Hkdf;
use hmac::{Hmac, KeyInit, Mac};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, errors::ErrorKind, jwk::Jwk};
use ring::rand::{SecureRandom, SystemRandom};
use serde::de::DeserializeOwned;
use sha2::Sha256;
use tokenserver_common::TokenserverError;

pub const SHA256_OUTPUT_LEN: usize = 32;
/// A trait representing all the required cryptographic operations by the token server
pub trait Crypto {
    type Error;
    /// HKDF key derivation
    ///
    /// This expands `info` into a 32 byte value using `secret` and the optional `salt`.
    /// Salt is normally specified, except when this function is called in [syncserver-settings::Secrets::new] or when deriving
    /// a key to be used to sign the tokenserver tokens, so both syncserver and tokenserver can
    /// sign and validate the signatures
    fn hkdf(&self, secret: &str, salt: Option<&[u8]>, info: &[u8]) -> Result<Vec<u8>, Self::Error>;

    /// HMAC signiture
    ///
    /// Signs the `payload` using HMAC given the `key`
    fn hmac_sign(&self, key: &[u8], payload: &[u8]) -> Result<Vec<u8>, Self::Error>;

    #[allow(dead_code)]
    /// Verify an HMAC signature on a payload given a shared key
    #[cfg(test)]
    fn hmac_verify(&self, key: &[u8], payload: &[u8], signature: &[u8]) -> Result<(), Self::Error>;

    /// Generates random bytes using a cryptographic random number generator
    /// and fills `output` with those bytes
    fn rand_bytes(&self, output: &mut [u8]) -> Result<(), Self::Error>;
}

/// An implementation for the needed cryptographic using
///    the hmac crate for hmac and hkdf crate for hkdf
///    it uses ring for the random number generation
pub struct CryptoImpl {}

impl Crypto for CryptoImpl {
    type Error = TokenserverError;
    fn hkdf(&self, secret: &str, salt: Option<&[u8]>, info: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let hk = Hkdf::<Sha256>::new(salt, secret.as_bytes());
        let mut okm = [0u8; SHA256_OUTPUT_LEN];
        hk.expand(info, &mut okm)
            .map_err(|_| TokenserverError::internal_error())?;
        Ok(okm.to_vec())
    }

    fn hmac_sign(&self, key: &[u8], payload: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let mut mac: Hmac<Sha256> =
            Hmac::new_from_slice(key).map_err(|_| TokenserverError::internal_error())?;
        mac.update(payload);
        Ok(mac.finalize().into_bytes().to_vec())
    }

    #[cfg(test)]
    fn hmac_verify(&self, key: &[u8], payload: &[u8], signature: &[u8]) -> Result<(), Self::Error> {
        let mut mac: Hmac<Sha256> =
            Hmac::new_from_slice(key).map_err(|_| TokenserverError::internal_error())?;
        mac.update(payload);
        mac.verify_slice(signature)
            .map_err(|_| TokenserverError::internal_error())?;
        Ok(())
    }

    fn rand_bytes(&self, output: &mut [u8]) -> Result<(), Self::Error> {
        let rng = SystemRandom::new();
        rng.fill(output)
            .map_err(|_| TokenserverError::internal_error())?;
        Ok(())
    }
}

/// JWTVerifyError captures the errors possible while verifying a JWT
#[derive(Debug, thiserror::Error)]
pub enum JWTVerifyError {
    #[error("The signature has expired")]
    ExpiredSignature,
    #[error("Untrusted token")]
    TrustError,
    #[error("Invalid Key")]
    InvalidKey,
    #[error("Error decoding JWT")]
    DecodingError,
    #[error("The key was well formatted, but the signature was invalid")]
    InvalidSignature,
    #[error("The token is not of the expected type")]
    InvalidTokenType,
    #[error("The token's lifetime exceeds the permitted maximum")]
    ExcessiveLifetime,
    #[error("The token has no jti")]
    MissingJti,
}

impl JWTVerifyError {
    pub fn metric_label(&self) -> &'static str {
        match self {
            Self::ExpiredSignature => "jwt.error.expired_signature",
            Self::TrustError => "jwt.error.trust_error",
            Self::InvalidKey => "jwt.error.invalid_key",
            Self::InvalidSignature => "jwt.error.invalid_signature",
            Self::DecodingError => "jwt.error.decoding_error",
            Self::InvalidTokenType => "jwt.error.invalid_typ",
            Self::ExcessiveLifetime => "jwt.error.excessive_lifetime",
            Self::MissingJti => "jwt.error.missing_jti",
        }
    }

    pub fn is_reportable_err(&self) -> bool {
        matches!(self, Self::InvalidKey | Self::DecodingError)
    }
}

impl From<jsonwebtoken::errors::Error> for JWTVerifyError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        match value.kind() {
            ErrorKind::InvalidKeyFormat => JWTVerifyError::InvalidKey,
            ErrorKind::InvalidSignature => JWTVerifyError::InvalidSignature,
            ErrorKind::ExpiredSignature => JWTVerifyError::ExpiredSignature,
            _ => JWTVerifyError::DecodingError,
        }
    }
}

/// Normalize a JWT's `typ` header for comparison.
///
/// Per RFC 7515 §4.1.9 `typ` is case-insensitive and carries an implicit
/// `application/` prefix when it contains no `/`. Shared by every verifier
/// below: divergent `typ` semantics between verifiers sharing one trust anchor
/// is how two token types stop being distinguishable.
fn normalized_typ(header: &jsonwebtoken::Header) -> Option<String> {
    header.typ.as_ref().map(|typ| {
        let typ = if typ.contains('/') {
            typ.to_owned()
        } else {
            format!("application/{}", typ)
        };
        typ.to_lowercase()
    })
}

/// A trait representing a JSON Web Token verifier <https://datatracker.ietf.org/doc/html/rfc7519>
pub trait JWTVerifier: TryFrom<Self::Key, Error = JWTVerifyError> + Sync + Send + Clone {
    type Key: DeserializeOwned;

    fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, JWTVerifyError>;
}

/// An implementation of the JWT verifier using the jsonwebtoken crate
#[derive(Clone)]
pub struct JWTVerifierImpl {
    key: DecodingKey,
    validation: Validation,
}

impl JWTVerifier for JWTVerifierImpl {
    type Key = Jwk;

    fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, JWTVerifyError> {
        let token_data = jsonwebtoken::decode::<T>(token, &self.key, &self.validation)?;
        match normalized_typ(&token_data.header).as_deref() {
            Some("application/at+jwt") => Ok(token_data.claims),
            _ => Err(JWTVerifyError::TrustError),
        }
    }
}

impl TryFrom<Jwk> for JWTVerifierImpl {
    type Error = JWTVerifyError;
    fn try_from(value: Jwk) -> Result<Self, Self::Error> {
        let decoding_key = DecodingKey::from_jwk(&value).map_err(|_| JWTVerifyError::InvalidKey)?;
        let mut validation = Validation::new(Algorithm::RS256);
        // The FxA OAuth ecosystem currently doesn't make good use of aud, and
        // instead relies on scope for restricting which services can accept
        // which tokens. So there's no value in checking it here, and in fact if
        // we check it here, it fails because the right audience isn't being
        // requested.
        validation.validate_aud = false;

        Ok(Self {
            key: decoding_key,
            validation,
        })
    }
}

/// Parsed claims from a FxA Security Event Token <https://datatracker.ietf.org/doc/html/rfc8417>.
#[derive(Debug, serde::Deserialize)]
pub struct FxaWebhookClaims {
    pub sub: String,
    pub iss: String,
    pub events: serde_json::Value,
}

/// An implementation of the JWT verifier for Security Event Tokens
/// <https://datatracker.ietf.org/doc/html/rfc8417>
#[derive(Clone)]
pub struct SETVerifierImpl {
    key: DecodingKey,
    validation: Validation,
}

impl SETVerifierImpl {
    pub fn new(jwk: &Jwk, client_id: &str, issuer_url: &str) -> Result<Self, JWTVerifyError> {
        let decoding_key = DecodingKey::from_jwk(jwk).map_err(|_| JWTVerifyError::InvalidKey)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[client_id]);
        validation.set_issuer(&[issuer_url]);
        // jsonwebtoken validates `exp` by default
        validation.required_spec_claims.remove("exp");
        Ok(Self {
            key: decoding_key,
            validation,
        })
    }

    pub fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, JWTVerifyError> {
        let token_data = jsonwebtoken::decode::<T>(token, &self.key, &self.validation)?;
        Ok(token_data.claims)
    }
}

// Separation between the three FxA-signed credentials tokenserver accepts.
//
// All three ride the same FxA JWKS and share an `iss`, so `iss` separates
// nothing. `jsonwebtoken::decode` does not match `kid` either — every
// configured JWK is tried for every verifier — so a distinct signing key
// would buy nothing. Only `typ` and `aud` do real work:
//
// | credential                        | `iss`        | `aud`                | `typ`                  | `exp`                   |
// |-----------------------------------|--------------|----------------------|------------------------|-------------------------|
// | oldsync token (`JWTVerifierImpl`) | not checked  | **off**, see below   | `application/at+jwt`   | required                |
// | webhook SET (`SETVerifierImpl`)   | if present   | **if present**       | none — FxA sends none  | checked if present      |
// | delete assertion (this)           | required     | required, single     | required, exact        | required + capped       |
//
// The delete assertion is the strict one deliberately: it authorizes an
// irreversible operation, and it is the only one of the three with no legacy
// traffic constraining what it may demand.

/// `typ` the FxA Auth Server signs delete assertions with, in normalized form.
///
/// FxA emits the bare `fxa-sync-delete+jwt`; [`normalized_typ`] applies the
/// implicit `application/` prefix, exactly as it does for `at+JWT`. Writing
/// this constant unprefixed would therefore never match.
///
/// A constant rather than a setting on purpose: `typ` is a protocol value, and
/// making it configurable creates a way to widen acceptance to another token
/// type without review.
const DELETE_ASSERTION_TYP: &str = "application/fxa-sync-delete+jwt";

/// Furthest ahead a delete assertion's `exp` may sit.
///
/// `jsonwebtoken` checks that `exp` is in the future but not how far; without
/// this an issuer could mint a years-long standing capability to destroy a
/// user's data.
const DELETE_ASSERTION_MAX_LIFETIME_SECS: i64 = 120;

/// Clock skew allowance.
///
/// Deliberately not `Validation`'s default of 60. These assertions are minted
/// seconds before use, so a 60s leeway would *be* the window rather than
/// bounding it. Worst-case acceptance is
/// `MAX_LIFETIME + 2 × leeway` — 150s here, against 240s at the default.
const DELETE_ASSERTION_LEEWAY_SECS: u64 = 15;

/// Claims of an FxA delete assertion.
#[derive(Debug, serde::Deserialize)]
pub struct FxaDeleteAssertionClaims {
    /// The FxA uid whose data may be deleted. Authoritative: the caller does
    /// not get to name a different target in the path.
    pub sub: String,
    pub iss: String,
    /// Deliberately `String` and not `Vec<String>`. `jsonwebtoken` matches
    /// audiences by set *intersection*, so an array carrying both this
    /// audience and, say, the oldsync tokenserver URL would satisfy two
    /// verifiers at once. A multi-valued `aud` fails to deserialize here.
    pub aud: String,
    pub exp: i64,
    /// Required by protocol and logged for correlation, deliberately not
    /// stored — see the ADR-0052 reply, §3. This is *not* replay protection;
    /// `exp` plus the lifetime cap are what bound replay.
    pub jti: Option<String>,
    pub iat: Option<i64>,
}

/// Verifies the FxA-signed assertion authorizing deletion of one user's
/// syncstorage data.
#[derive(Clone)]
pub struct FxaDeleteAssertionVerifier {
    key: DecodingKey,
    validation: Validation,
}

impl FxaDeleteAssertionVerifier {
    pub fn new(jwk: &Jwk, audience: &str, issuer: &str) -> Result<Self, JWTVerifyError> {
        let key = DecodingKey::from_jwk(jwk).map_err(|_| JWTVerifyError::InvalidKey)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[audience]);
        validation.set_issuer(&[issuer]);
        // `set_audience`/`set_issuer` only record what the value must be *if
        // present*; a token omitting the claim entirely passes otherwise.
        // Note this replaces the required set, so `exp` has to be relisted.
        validation.set_required_spec_claims(&["exp", "aud", "iss", "sub"]);
        validation.leeway = DELETE_ASSERTION_LEEWAY_SECS;
        // `validate_aud` stays at its `true` default.
        Ok(Self { key, validation })
    }

    pub fn verify(&self, token: &str) -> Result<FxaDeleteAssertionClaims, JWTVerifyError> {
        let token_data =
            jsonwebtoken::decode::<FxaDeleteAssertionClaims>(token, &self.key, &self.validation)?;

        if normalized_typ(&token_data.header).as_deref() != Some(DELETE_ASSERTION_TYP) {
            return Err(JWTVerifyError::InvalidTokenType);
        }

        let claims = token_data.claims;

        // Bound how long a leaked assertion stays useful. Allow the leeway on
        // top so that FxA's clock running ahead is a non-event rather than a
        // false rejection.
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| JWTVerifyError::DecodingError)?
            .as_secs() as i64;
        if claims.exp - now
            > DELETE_ASSERTION_MAX_LIFETIME_SECS + DELETE_ASSERTION_LEEWAY_SECS as i64
        {
            return Err(JWTVerifyError::ExcessiveLifetime);
        }

        if claims.jti.is_none() {
            return Err(JWTVerifyError::MissingJti);
        }

        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        DeleteAssertionBuilder, OTHER_PRIVATE_KEY_PEM, TEST_PRIVATE_KEY_PEM, make_set,
        make_set_with_exp, test_jwk,
    };
    use serde_json::json;

    const DEL_AUD: &str = "urn:mozilla:fxa:tokenserver:sync-data-delete";
    const DEL_ISS: &str = "https://accounts.firefox.com";
    const DEL_SUB: &str = "deadbeef00004be4ae957006c0ceb620";

    fn delete_verifier() -> FxaDeleteAssertionVerifier {
        FxaDeleteAssertionVerifier::new(&test_jwk(), DEL_AUD, DEL_ISS).unwrap()
    }

    fn assertion() -> DeleteAssertionBuilder {
        DeleteAssertionBuilder::new(DEL_SUB, DEL_AUD, DEL_ISS)
    }

    #[test]
    fn test_verify_valid_set() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let claims: FxaWebhookClaims = verifier.verify(&token).unwrap();
        assert_eq!(claims.sub, "quux");
        assert_eq!(claims.iss, "https://accounts.firefox.com/");
    }

    #[test]
    fn test_verify_wrong_audience_set() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let token = make_set("quux", "wrong-client-id", json!({}), TEST_PRIVATE_KEY_PEM);
        assert!(verifier.verify::<FxaWebhookClaims>(&token).is_err());
    }

    #[test]
    fn test_verify_wrong_issuer_set() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.stage.mozaws.net")
                .unwrap();
        let token = make_set("quux", "testo", json!({}), TEST_PRIVATE_KEY_PEM);
        assert!(verifier.verify::<FxaWebhookClaims>(&token).is_err());
    }

    #[test]
    fn test_verify_wrong_key_set() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let token = make_set("quux", "testo", json!({}), OTHER_PRIVATE_KEY_PEM);
        let err = verifier.verify::<FxaWebhookClaims>(&token).unwrap_err();
        assert!(matches!(err, JWTVerifyError::InvalidSignature));
    }

    // ---- delete assertion verifier ----

    #[test]
    fn test_delete_assertion_happy_path() {
        let claims = delete_verifier().verify(&assertion().build()).unwrap();
        assert_eq!(claims.sub, DEL_SUB);
        assert_eq!(claims.aud, DEL_AUD);
        assert_eq!(claims.jti.as_deref(), Some("test-jti"));
    }

    /// `typ` is case-insensitive and carries an implicit `application/` prefix
    /// (RFC 7515 §4.1.9). FxA signs the bare lowercase form; these must all be
    /// the same token type.
    #[test]
    fn test_delete_assertion_typ_is_normalized() {
        for typ in [
            "fxa-sync-delete+jwt",
            "FXA-Sync-Delete+JWT",
            "application/fxa-sync-delete+jwt",
        ] {
            assert!(
                delete_verifier()
                    .verify(&assertion().typ(typ).build())
                    .is_ok(),
                "typ {typ:?} should be accepted"
            );
        }
    }

    /// The three highest-value cases: `jsonwebtoken` only checks `aud`/`iss`
    /// when the claim is *present*, so a token omitting one entirely passes
    /// unless the claim is in `required_spec_claims`. These fail against the
    /// obvious `set_audience`-only implementation.
    #[test]
    fn test_delete_assertion_requires_claims_to_be_present() {
        let v = delete_verifier();
        assert!(
            v.verify(&assertion().without_aud().build()).is_err(),
            "no aud"
        );
        assert!(
            v.verify(&assertion().without_iss().build()).is_err(),
            "no iss"
        );
        // `exp` has to survive set_required_spec_claims replacing the set.
        // Well past the leeway: at -1s this would still be accepted, which is
        // the leeway doing exactly what it is for.
        assert!(
            v.verify(&assertion().exp_in(-60).build()).is_err(),
            "expired"
        );
    }

    /// A multi-valued `aud` would otherwise satisfy several verifiers at once,
    /// because jsonwebtoken matches audiences by set intersection.
    #[test]
    fn test_delete_assertion_rejects_multi_valued_aud() {
        let token = assertion().build_with_aud_array(&[DEL_AUD, "https://tokenserver.example"]);
        assert!(delete_verifier().verify(&token).is_err());
    }

    #[test]
    fn test_delete_assertion_rejects_bad_tokens() {
        let v = delete_verifier();
        let cases = [
            (
                "wrong aud",
                assertion().aud("urn:mozilla:fxa:something-else").build(),
            ),
            ("wrong iss", assertion().iss("https://evil.example").build()),
            ("wrong key", assertion().key(OTHER_PRIVATE_KEY_PEM).build()),
            ("no typ", assertion().without_typ().build()),
            ("generic JWT typ", assertion().typ("JWT").build()),
            ("access token typ", assertion().typ("at+jwt").build()),
            ("no jti", assertion().without_jti().build()),
            // Bounds how long a leaked assertion stays useful; jsonwebtoken
            // itself places no ceiling on exp.
            ("exp a day out", assertion().exp_in(86400).build()),
            ("garbage", "not.a.jwt".to_owned()),
        ];
        for (name, token) in cases {
            assert!(v.verify(&token).is_err(), "{name} should be rejected");
        }
    }

    #[test]
    fn test_delete_assertion_lifetime_cap_allows_the_intended_window() {
        // 30s is the proposed FxA window; it must sit inside the cap.
        assert!(
            delete_verifier()
                .verify(&assertion().exp_in(30).build())
                .is_ok()
        );
    }

    // ---- non-interchangeability with the other two FxA credentials ----

    /// A webhook SET must not authorize a deletion. Blocked three ways; the
    /// follow-ups isolate each, so that removing any single check fails a test
    /// rather than silently leaving two.
    #[test]
    fn test_delete_verifier_rejects_a_webhook_set() {
        let set = make_set(
            DEL_SUB,
            "fxa-webhook-client-id",
            json!({}),
            TEST_PRIVATE_KEY_PEM,
        );
        assert!(delete_verifier().verify(&set).is_err());
    }

    #[test]
    fn test_delete_verifier_rejects_a_set_even_with_exp_aud_and_iss() {
        // Neutralize every barrier except `typ`, so this fails if `typ` alone
        // ever stops being checked. Note make_set issues from
        // "https://accounts.firefox.com/" *with* a trailing slash: `iss` is an
        // exact string match, and that slash is a classic way to think a check
        // is doing work when it is really the mismatch doing it.
        let verifier =
            FxaDeleteAssertionVerifier::new(&test_jwk(), DEL_AUD, "https://accounts.firefox.com/")
                .unwrap();
        let set = make_set_with_exp(DEL_SUB, DEL_AUD, json!({}), TEST_PRIVATE_KEY_PEM, Some(30));
        assert_eq!(
            verifier.verify(&set).unwrap_err().metric_label(),
            "jwt.error.invalid_typ"
        );
    }

    /// An oldsync access token must not authorize a deletion either. With the
    /// audience neutralized, `typ` is what remains.
    #[test]
    fn test_delete_verifier_rejects_an_access_token_shape() {
        let token = assertion().typ("at+JWT").build();
        assert_eq!(
            delete_verifier().verify(&token).unwrap_err().metric_label(),
            "jwt.error.invalid_typ"
        );
    }

    /// The reverse direction. Today this is blocked by `aud` and by
    /// `FxaWebhookClaims` requiring `events` — the latter an accident of a
    /// serde field list rather than a check. If this test ever starts failing,
    /// the webhook has widened; see the ADR-0052 reply follow-ups.
    #[test]
    fn test_set_verifier_rejects_a_delete_assertion() {
        let verifier = SETVerifierImpl::new(&test_jwk(), "fxa-webhook-client-id", DEL_ISS).unwrap();
        let token = assertion().build();
        assert!(verifier.verify::<FxaWebhookClaims>(&token).is_err());
    }
}
