pub mod browserid;
mod crypto;
use crypto::{Crypto, CryptoImpl};
pub mod oauth;

use std::fmt;

use async_trait::async_trait;
use base64::engine::Engine;
use dyn_clone::{self, DynClone};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tokenserver_common::TokenserverError;

const HKDF_SIGNING_INFO: &[u8] = b"services.mozilla.com/tokenlib/v1/signing";
const HKDF_INFO_DERIVE: &[u8] = b"services.mozilla.com/tokenlib/v1/derive/";

/// Represents the origin of the token used by Sync clients to access their data.
#[derive(Clone, Copy, Default, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenserverOrigin {
    /// The Python Tokenserver.
    #[default]
    Python,
    /// The Rust Tokenserver.
    Rust,
}

impl fmt::Display for TokenserverOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenserverOrigin::Python => write!(f, "python"),
            TokenserverOrigin::Rust => write!(f, "rust"),
        }
    }
}

/// The plaintext needed to build a token.
#[derive(Clone, Debug, Serialize)]
pub struct MakeTokenPlaintext {
    pub node: String,
    pub fxa_kid: String,
    pub fxa_uid: String,
    pub hashed_device_id: String,
    pub hashed_fxa_uid: String,
    pub expires: u64,
    pub uid: i64,
    pub tokenserver_origin: TokenserverOrigin,
}

/// An adapter to the tokenlib Python library.
pub struct Tokenlib;

impl Tokenlib {
    /// Builds the token and derived secret to be returned by Tokenserver.
    pub fn get_token_and_derived_secret(
        plaintext: MakeTokenPlaintext,
        shared_secret: &str,
    ) -> Result<(String, String), TokenserverError> {
        #[derive(Serialize)]
        struct Token<'a> {
            #[serde(flatten)]
            plaintext: MakeTokenPlaintext,
            salt: &'a str,
        }

        let mut salt_bytes = [0u8; 3];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(&mut salt_bytes);
        let salt = hex::encode(salt_bytes);
        let token_str = serde_json::to_string(&Token {
            plaintext,
            salt: &salt,
        })
        .map_err(|_| TokenserverError::internal_error())?;
        let crypto_lib = CryptoImpl {};
        let hmac_key = crypto_lib.hkdf(shared_secret, None, HKDF_SIGNING_INFO)?;
        let signature = crypto_lib.hmac_sign(&hmac_key, token_str.as_bytes())?;
        let mut token_bytes = Vec::with_capacity(token_str.len() + signature.len());
        token_bytes.extend_from_slice(token_str.as_bytes());
        token_bytes.extend_from_slice(&signature);
        let token = base64::engine::general_purpose::URL_SAFE.encode(token_bytes);
        // Now that we finialized the token, lets generate our per token secret
        let mut info = Vec::with_capacity(HKDF_INFO_DERIVE.len() + token.as_bytes().len());
        info.extend_from_slice(HKDF_INFO_DERIVE);
        info.extend_from_slice(token.as_bytes());

        let per_token_secret = crypto_lib.hkdf(shared_secret, Some(salt.as_bytes()), &info)?;
        let per_token_secret = base64::engine::general_purpose::URL_SAFE.encode(per_token_secret);
        Ok((token, per_token_secret))
    }
}

/// Implementers of this trait can be used to verify tokens for Tokenserver.
#[async_trait]
pub trait VerifyToken: DynClone + Sync + Send {
    type Output: Clone;

    /// Verifies the given token. This function is async because token verification often involves
    /// making a request to a remote server.
    async fn verify(&self, token: String) -> Result<Self::Output, TokenserverError>;
}

dyn_clone::clone_trait_object!(<T> VerifyToken<Output=T>);

/// A mock verifier to be used for testing purposes.
#[derive(Clone, Default)]
pub struct MockVerifier<T: Clone + Send + Sync> {
    pub valid: bool,
    pub verify_output: T,
}

#[async_trait]
impl<T: Clone + Send + Sync> VerifyToken for MockVerifier<T> {
    type Output = T;

    async fn verify(&self, _token: String) -> Result<T, TokenserverError> {
        self.valid
            .then(|| self.verify_output.clone())
            .ok_or_else(|| TokenserverError::invalid_credentials("Unauthorized".to_owned()))
    }
}
