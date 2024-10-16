#[cfg(not(feature = "py"))]
mod crypto;

#[cfg(not(feature = "py"))]
pub use crypto::{JWTVerifier, JWTVerifierImpl};
#[allow(clippy::result_large_err)]
pub mod oauth;
#[allow(clippy::result_large_err)]
mod token;
use syncserver_common::Metrics;
pub use token::Tokenlib;

use std::fmt;

use async_trait::async_trait;
use dyn_clone::{self, DynClone};
use serde::{Deserialize, Serialize};
use tokenserver_common::TokenserverError;
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
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq)]
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

/// Implementers of this trait can be used to verify tokens for Tokenserver.
#[async_trait]
pub trait VerifyToken: DynClone + Sync + Send {
    type Output: Clone;

    /// Verifies the given token. This function is async because token verification often involves
    /// making a request to a remote server.
    async fn verify(
        &self,
        token: String,
        metrics: &Metrics,
    ) -> Result<Self::Output, TokenserverError>;
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

    async fn verify(&self, _token: String, _metrics: &Metrics) -> Result<T, TokenserverError> {
        self.valid
            .then(|| self.verify_output.clone())
            .ok_or_else(|| TokenserverError::invalid_credentials("Unauthorized".to_owned()))
    }
}
