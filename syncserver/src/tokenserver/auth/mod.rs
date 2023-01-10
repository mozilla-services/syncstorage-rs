pub mod browserid;
pub mod oauth;

use std::fmt;

use async_trait::async_trait;
use dyn_clone::{self, DynClone};
use pyo3::{
    prelude::{IntoPy, PyErr, PyModule, PyObject, Python},
    types::IntoPyDict,
};
use serde::{Deserialize, Serialize};
use tokenserver_common::TokenserverError;

/// Represents the origin of the token used by Sync clients to access their data.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TokenserverOrigin {
    /// The Python Tokenserver.
    Python,
    /// The Rust Tokenserver.
    Rust,
}

impl Default for TokenserverOrigin {
    fn default() -> Self {
        TokenserverOrigin::Python
    }
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
#[derive(Clone, Debug)]
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

impl IntoPy<PyObject> for MakeTokenPlaintext {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let dict = [
            ("node", self.node),
            ("fxa_kid", self.fxa_kid),
            ("fxa_uid", self.fxa_uid),
            ("hashed_device_id", self.hashed_device_id),
            ("hashed_fxa_uid", self.hashed_fxa_uid),
            ("tokenserver_origin", self.tokenserver_origin.to_string()),
        ]
        .into_py_dict(py);

        // These need to be set separately since they aren't strings, and
        // Rust doesn't support heterogeneous arrays
        dict.set_item("expires", self.expires).unwrap();
        dict.set_item("uid", self.uid).unwrap();

        dict.into()
    }
}

/// An adapter to the tokenlib Python library.
pub struct Tokenlib;

impl Tokenlib {
    /// Builds the token and derived secret to be returned by Tokenserver.
    pub fn get_token_and_derived_secret(
        plaintext: MakeTokenPlaintext,
        shared_secret: &str,
    ) -> Result<(String, String), TokenserverError> {
        Python::with_gil(|py| {
            // `import tokenlib`
            let module = PyModule::import(py, "tokenlib").map_err(|e| {
                e.print_and_set_sys_last_vars(py);
                e
            })?;
            // `kwargs = { 'secret': shared_secret }`
            let kwargs = [("secret", shared_secret)].into_py_dict(py);
            // `token = tokenlib.make_token(plaintext, **kwargs)`
            let token = module
                .getattr("make_token")?
                .call((plaintext,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;
            // `derived_secret = tokenlib.get_derived_secret(token, **kwargs)`
            let derived_secret = module
                .getattr("get_derived_secret")?
                .call((&token,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;
            // `return (token, derived_secret)`
            Ok((token, derived_secret))
        })
        .map_err(pyerr_to_tokenserver_error)
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

fn pyerr_to_tokenserver_error(e: PyErr) -> TokenserverError {
    TokenserverError {
        context: e.to_string(),
        ..TokenserverError::internal_error()
    }
}
