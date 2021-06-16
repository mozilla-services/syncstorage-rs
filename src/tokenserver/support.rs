use actix_web::Error;
use pyo3::prelude::{IntoPy, PyAny, PyErr, PyModule, PyObject, Python};
use pyo3::types::{IntoPyDict, PyString};
use serde::Deserialize;

use crate::error::{ApiError, ApiErrorKind};
use crate::web::error::ValidationErrorKind;
use crate::web::extractors::RequestErrorLocation;

/// The plaintext needed to build a token.
#[derive(Clone)]
pub struct MakeTokenPlaintext {
    pub node: String,
    pub fxa_kid: String,
    pub fxa_uid: String,
    pub hashed_device_id: String,
    pub hashed_fxa_uid: String,
    pub expires: f64,
    pub uid: i64,
}

impl IntoPy<PyObject> for MakeTokenPlaintext {
    fn into_py(self, py: Python<'_>) -> PyObject {
        let dict = [
            ("node", self.node),
            ("fxa_kid", self.fxa_kid),
            ("fxa_uid", self.fxa_uid),
            ("hashed_device_id", self.hashed_device_id),
            ("hashed_fxa_uid", self.hashed_fxa_uid),
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
    ) -> Result<(String, String), Error> {
        Python::with_gil(|py| {
            let module = PyModule::import(py, "tokenlib").map_err(|e| {
                e.print_and_set_sys_last_vars(py);
                e
            })?;
            let kwargs = [("secret", shared_secret)].into_py_dict(py);
            let token = module
                .call("make_token", (plaintext,), Some(&kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .map(|x| x.extract().unwrap())?;
            let derived_secret = module
                .call("get_derived_secret", (&token,), Some(&kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .map(|x| x.extract().unwrap())?;

            Ok((token, derived_secret))
        })
        .map_err(pyerr_to_actix_error)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct TokenData {
    pub user: String,
    pub client_id: String,
    pub scope: Vec<String>,
    pub generation: i64,
    pub profile_changed_at: i64,
}

/// Implementers of this trait can be used to verify OAuth tokens for Tokenserver.
pub trait VerifyToken {
    fn verify_token(&self, token: &str) -> Result<TokenData, Error>;
}

/// An adapter to the PyFxA Python library.
pub struct OAuthVerifier {
    pub fxa_oauth_server_url: Option<String>,
}

impl OAuthVerifier {
    const FILENAME: &'static str = "verify.py";
}

impl VerifyToken for OAuthVerifier {
    /// Verifies an OAuth token. Returns `TokenData` for valid tokens and an `Error` for invalid
    /// tokens.
    fn verify_token(&self, token: &str) -> Result<TokenData, Error> {
        let maybe_token_data_string = Python::with_gil(|py| {
            let code = include_str!("verify.py");
            let module = PyModule::from_code(py, code, Self::FILENAME, Self::FILENAME)?;
            let kwargs = self
                .fxa_oauth_server_url
                .clone()
                .map(|url| [("server_url", url)].into_py_dict(py));
            let result: &PyAny = module.call("verify_token", (token,), kwargs).map_err(|e| {
                e.print_and_set_sys_last_vars(py);
                e
            })?;

            if result.is_none() {
                Ok(None)
            } else {
                let token_data_python_string = result.downcast::<PyString>()?;
                token_data_python_string.extract::<String>().map(Some)
            }
        })
        .map_err(pyerr_to_actix_error)?;

        match maybe_token_data_string {
            Some(token_data_string) => serde_json::from_str(&token_data_string).map_err(Into::into),
            None => Err(ValidationErrorKind::FromDetails(
                "Invalid bearer auth token".to_owned(),
                RequestErrorLocation::Header,
                Some("Bearer".to_owned()),
                label!("request.error.invalid_bearer_auth"),
            )
            .into()),
        }
    }
}

/// A mock OAuth verifier to be used for testing purposes.
#[derive(Default)]
pub struct MockOAuthVerifier {
    pub valid: bool,
    pub token_data: TokenData,
}

impl VerifyToken for MockOAuthVerifier {
    fn verify_token(&self, _token: &str) -> Result<TokenData, Error> {
        self.valid.then(|| self.token_data.clone()).ok_or_else(|| {
            ValidationErrorKind::FromDetails(
                "Invalid bearer auth token".to_owned(),
                RequestErrorLocation::Header,
                Some("Bearer".to_owned()),
                label!("request.error.invalid_bearer_auth"),
            )
            .into()
        })
    }
}

fn pyerr_to_actix_error(e: PyErr) -> Error {
    let api_error: ApiError = ApiErrorKind::Internal(e.to_string()).into();
    api_error.into()
}
