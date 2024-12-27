use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use jsonwebtoken::{errors::ErrorKind, jwk::Jwk, Algorithm, DecodingKey, Validation};
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

/// OAuthVerifyError captures the errors possible while verifing an OAuth JWT access token
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

impl OAuthVerifyError {
    pub fn metric_label(&self) -> &'static str {
        match self {
            Self::ExpiredSignature => "oauth.error.expired_signature",
            Self::TrustError => "oauth.error.trust_error",
            Self::InvalidKey => "oauth.error.invalid_key",
            Self::InvalidSignature => "oauth.error.invalid_signature",
            Self::DecodingError => "oauth.error.decoding_error",
        }
    }

    pub fn is_reportable_err(&self) -> bool {
        matches!(self, Self::InvalidKey | Self::DecodingError)
    }
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
pub trait JWTVerifier: TryFrom<Self::Key, Error = OAuthVerifyError> + Sync + Send + Clone {
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
