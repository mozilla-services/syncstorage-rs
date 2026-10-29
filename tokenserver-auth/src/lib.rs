#[allow(dead_code)]
mod crypto;
#[cfg(any(test, feature = "test-support"))]
pub mod test_utils;

pub use crypto::{FxaWebhookClaims, JWTVerifyError, SETVerifierImpl};
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
    /// Tokens without an origin, now only minted by the Tokenserver purge script.
    #[default]
    #[serde(rename = "purge_script", alias = "python")]
    PurgeScript,
    /// The Rust Tokenserver.
    Rust,
}

impl fmt::Display for TokenserverOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenserverOrigin::PurgeScript => write!(f, "purge_script"),
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

#[cfg(test)]
mod tests {
    use super::TokenserverOrigin;

    #[test]
    fn test_tokenserver_origin_purge_script() {
        let parse = |json| serde_json::from_str::<TokenserverOrigin>(json).unwrap();
        // Tokens from the purge script have no origin field, so they get the default
        assert_eq!(TokenserverOrigin::default(), TokenserverOrigin::PurgeScript);
        // Tokens minted before the rename still say "python"
        assert_eq!(parse(r#""python""#), TokenserverOrigin::PurgeScript);
        assert_eq!(parse(r#""purge_script""#), TokenserverOrigin::PurgeScript);
        assert_eq!(parse(r#""rust""#), TokenserverOrigin::Rust);
        assert_eq!(
            serde_json::to_string(&TokenserverOrigin::PurgeScript).unwrap(),
            r#""purge_script""#
        );
        assert_eq!(TokenserverOrigin::PurgeScript.to_string(), "purge_script");
        assert_eq!(TokenserverOrigin::Rust.to_string(), "rust");
    }
}
