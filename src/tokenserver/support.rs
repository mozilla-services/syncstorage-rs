use actix_web::Error;
use pyo3::prelude::{IntoPy, PyAny, PyErr, PyModule, PyObject, Python};
use pyo3::types::{IntoPyDict, PyString};
use serde::{Deserialize, Serialize};

use super::error::TokenserverError;
use crate::error::{ApiError, ApiErrorKind};

/// The plaintext needed to build a token.
#[derive(Clone)]
pub struct MakeTokenPlaintext {
    pub node: String,
    pub fxa_kid: String,
    pub fxa_uid: String,
    pub hashed_device_id: String,
    pub hashed_fxa_uid: String,
    pub expires: u64,
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
                .getattr("make_token")?
                .call((plaintext,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;
            let derived_secret = module
                .getattr("get_derived_secret")?
                .call((&token,), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })
                .and_then(|x| x.extract())?;

            Ok((token, derived_secret))
        })
        .map_err(pyerr_to_actix_error)
    }
}

pub fn derive_node_secrets(secrets: Vec<&str>, node: &str) -> Result<Vec<String>, Error> {
    const FILENAME: &str = "secrets.py";

    Python::with_gil(|py| {
        let code = include_str!("secrets.py");
        let module = PyModule::from_code(py, code, FILENAME, FILENAME)?;

        module
            .getattr("derive_secrets")?
            .call((secrets, node), None)
            .map_err(|e| {
                e.print_and_set_sys_last_vars(py);
                e
            })
            .and_then(|x| x.extract())
    })
    .map_err(pyerr_to_actix_error)
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct TokenData {
    pub user: String,
    pub client_id: String,
    pub scope: Vec<String>,
    pub generation: Option<i64>,
    pub profile_changed_at: Option<i64>,
}

/// Implementers of this trait can be used to verify OAuth tokens for Tokenserver.
pub trait VerifyToken: Sync + Send {
    fn verify_token(&self, token: &str) -> Result<TokenData, TokenserverError>;
    fn box_clone(&self) -> Box<dyn VerifyToken>;
}

impl Clone for Box<dyn VerifyToken> {
    fn clone(&self) -> Box<dyn VerifyToken> {
        self.box_clone()
    }
}

#[derive(Clone)]
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
    fn verify_token(&self, token: &str) -> Result<TokenData, TokenserverError> {
        let maybe_token_data_string = Python::with_gil(|py| {
            let code = include_str!("verify.py");
            let module = PyModule::from_code(py, code, Self::FILENAME, Self::FILENAME)?;
            let kwargs = self
                .fxa_oauth_server_url
                .clone()
                .map(|url| [("server_url", url)].into_py_dict(py));
            let result: &PyAny = module
                .getattr("verify_token")?
                .call((token,), kwargs)
                .map_err(|e| {
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
        .map_err(|_| TokenserverError::invalid_credentials("Unauthorized"))?;

        match maybe_token_data_string {
            Some(token_data_string) => serde_json::from_str(&token_data_string)
                .map_err(|_| TokenserverError::invalid_credentials("Unauthorized")),
            None => Err(TokenserverError::invalid_credentials("Unauthorized")),
        }
    }

    fn box_clone(&self) -> Box<dyn VerifyToken> {
        Box::new(self.clone())
    }
}

#[derive(Deserialize, Serialize)]
struct JwtPayload {
    client_id: String,
    scope: String,
    sub: String,
    #[serde(rename(serialize = "fxa-generation", deserialize = "fxa-generation"))]
    fxa_generation: Option<i64>,
    #[serde(rename(
        serialize = "fxa-profileChangedAt",
        deserialize = "fxa-profileChangedAt"
    ))]
    fxa_profile_changed_at: Option<i64>,
}

#[derive(Clone)]
pub struct TestModeOAuthVerifier;

impl VerifyToken for TestModeOAuthVerifier {
    fn verify_token(&self, token: &str) -> Result<TokenData, TokenserverError> {
        let token_components: Vec<&str> = token.split('.').collect();

        if token_components.len() != 3 {
            return Err(TokenserverError::invalid_credentials("Invalid JWT"));
        }

        let payload_bytes = base64::decode_config(token_components[1], base64::URL_SAFE_NO_PAD)
            .map_err(|_| TokenserverError::invalid_credentials("Invalid JWT base64"))?;
        let payload_string = String::from_utf8(payload_bytes)
            .map_err(|_| TokenserverError::invalid_credentials("JWT payload not a valid string"))?;
        let payload: JwtPayload = serde_json::from_str(&payload_string)
            .map_err(|_| TokenserverError::invalid_credentials("Invalid JWT payload"))?;

        Ok(TokenData {
            user: payload.sub,
            client_id: payload.client_id,
            scope: payload.scope.split(' ').map(String::from).collect(),
            generation: payload.fxa_generation,
            profile_changed_at: payload.fxa_profile_changed_at,
        })
    }

    fn box_clone(&self) -> Box<dyn VerifyToken> {
        Box::new(self.clone())
    }
}

/// A mock OAuth verifier to be used for testing purposes.
#[derive(Clone, Default)]
pub struct MockOAuthVerifier {
    pub valid: bool,
    pub token_data: TokenData,
}

impl VerifyToken for MockOAuthVerifier {
    fn verify_token(&self, _token: &str) -> Result<TokenData, TokenserverError> {
        self.valid
            .then(|| self.token_data.clone())
            .ok_or_else(|| TokenserverError::invalid_credentials("Unauthorized"))
    }

    fn box_clone(&self) -> Box<dyn VerifyToken> {
        Box::new(self.clone())
    }
}

fn pyerr_to_actix_error(e: PyErr) -> Error {
    let api_error: ApiError = ApiErrorKind::Internal(e.to_string()).into();
    api_error.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    use jsonwebtoken::{EncodingKey, Header};

    #[test]
    fn test_test_mode_oauth_verifier() {
        let test_mode_oauth_verifier = TestModeOAuthVerifier;
        let claims = JwtPayload {
            sub: "test user".to_owned(),
            client_id: "test client ID".to_owned(),
            scope: "test1 test2".to_owned(),
            fxa_generation: Some(1234),
            fxa_profile_changed_at: Some(5678),
        };

        let token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_ref()),
        )
        .unwrap();
        let decoded_claims = test_mode_oauth_verifier.verify_token(&token).unwrap();
        let expected_claims = TokenData {
            user: "test user".to_owned(),
            client_id: "test client ID".to_owned(),
            scope: vec!["test1".to_owned(), "test2".to_owned()],
            generation: Some(1234),
            profile_changed_at: Some(5678),
        };

        assert_eq!(expected_claims, decoded_claims);
    }

    #[test]
    fn test_derive_secret_success() {
        let secrets = vec!["deadbeefdeadbeefdeadbeefdeadbeef"];
        let node = "https://node";
        let derived_secrets = derive_node_secrets(secrets, node).unwrap();

        assert_eq!(
            derived_secrets,
            vec!["a227eb0deb5fb4fd8002166f555c9071".to_owned()]
        );
    }
}
