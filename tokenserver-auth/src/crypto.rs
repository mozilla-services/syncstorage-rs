use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use jsonwebtoken::{errors::ErrorKind, jwk::Jwk, Algorithm, DecodingKey, Validation};
use serde::de::DeserializeOwned;
use sha2::Sha256;
use tokenserver_common::TokenserverError;
const SHA256_OUTPUT_LEN: usize = 32;
/// A triat representing all the required cryptographic operations by the token server
pub trait Crypto {
    type Error;
    /// HKDF key derivation
    fn hkdf(&self, secret: &str, salt: Option<&[u8]>, info: &[u8]) -> Result<Vec<u8>, Self::Error>;

    /// HMAC signiture
    fn hmac_sign(&self, key: &[u8], payload: &[u8]) -> Result<Vec<u8>, Self::Error>;
}

/// An implementation for the needed cryptographic using
///    the hmac crate for hmac and hkdf crate for hkdf
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
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthVerifyError {
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
}

impl From<jsonwebtoken::errors::Error> for OAuthVerifyError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        match value.kind() {
            ErrorKind::InvalidKeyFormat => OAuthVerifyError::InvalidKey,
            ErrorKind::InvalidSignature => OAuthVerifyError::InvalidSignature,
            ErrorKind::ExpiredSignature => OAuthVerifyError::ExpiredSignature,
            _ => OAuthVerifyError::DecodingError,
        }
    }
}

/// A trait representing a JSON Web Token verifier <https://datatracker.ietf.org/doc/html/rfc7519>
pub trait JWTVerifier: TryFrom<Self::Key> + Sync + Send + Clone {
    type Key: DeserializeOwned;

    fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, OAuthVerifyError>;
}

/// An implementation of the JWT verifier using the jsonwebtoken crate
#[derive(Clone)]
pub struct JWTVerifierImpl {
    key: DecodingKey,
    validation: Validation,
}

impl JWTVerifier for JWTVerifierImpl {
    type Key = Jwk;

    fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T, OAuthVerifyError> {
        let token_data = jsonwebtoken::decode::<T>(token, &self.key, &self.validation)?;
        token_data
            .header
            .typ
            .ok_or(OAuthVerifyError::TrustError)
            .and_then(|typ| {
                // Ref https://tools.ietf.org/html/rfc7515#section-4.1.9 the `typ` header
                // is lowercase and has an implicit default `application/` prefix.
                let typ = if !typ.contains('/') {
                    format!("application/{}", typ)
                } else {
                    typ
                };
                if typ.to_lowercase() != "application/at+jwt" {
                    return Err(OAuthVerifyError::TrustError);
                }
                Ok(typ)
            })?;
        Ok(token_data.claims)
    }
}

impl TryFrom<Jwk> for JWTVerifierImpl {
    type Error = OAuthVerifyError;
    fn try_from(value: Jwk) -> Result<Self, Self::Error> {
        let decoding_key =
            DecodingKey::from_jwk(&value).map_err(|_| OAuthVerifyError::InvalidKey)?;
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
