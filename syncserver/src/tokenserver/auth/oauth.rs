use actix_web::{error::BlockingError, web};
use async_trait::async_trait;
use pyo3::{
    prelude::{Py, PyAny, PyErr, PyModule, Python},
    types::{IntoPyDict, PyString},
};
use serde::{Deserialize, Serialize};
use serde_json;
use tokenserver_common::TokenserverError;
use tokenserver_settings::{Jwk, Settings};
use tokio::time;

use super::VerifyToken;

use core::time::Duration;
use std::convert::TryFrom;

/// The information extracted from a valid OAuth token.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifyOutput {
    #[serde(rename = "user")]
    pub fxa_uid: String,
    pub generation: Option<i64>,
}

/// The verifier used to verify OAuth tokens.
#[derive(Clone)]
pub struct Verifier {
    // Note that we do not need to use an Arc here, since Py is already a reference-counted
    // pointer
    inner: Py<PyAny>,
    timeout: u64,
}

impl Verifier {
    const FILENAME: &'static str = "verify.py";
}

impl TryFrom<&Settings> for Verifier {
    type Error = TokenserverError;

    fn try_from(settings: &Settings) -> Result<Self, TokenserverError> {
        let inner: Py<PyAny> = Python::with_gil::<_, Result<Py<PyAny>, PyErr>>(|py| {
            let code = include_str!("verify.py");
            let module = PyModule::from_code(py, code, Self::FILENAME, Self::FILENAME)?;
            let kwargs = {
                let dict = [("server_url", &settings.fxa_oauth_server_url)].into_py_dict(py);
                let parse_jwk = |jwk: &Jwk| {
                    let dict = [
                        ("kty", &jwk.kty),
                        ("alg", &jwk.alg),
                        ("kid", &jwk.kid),
                        ("use", &jwk.use_of_key),
                        ("n", &jwk.n),
                        ("e", &jwk.e),
                    ]
                    .into_py_dict(py);
                    dict.set_item("fxa-createdAt", jwk.fxa_created_at).unwrap();

                    dict
                };

                let jwks = match (
                    &settings.fxa_oauth_primary_jwk,
                    &settings.fxa_oauth_secondary_jwk,
                ) {
                    (Some(primary_jwk), Some(secondary_jwk)) => {
                        Some(vec![parse_jwk(primary_jwk), parse_jwk(secondary_jwk)])
                    }
                    (Some(jwk), None) | (None, Some(jwk)) => Some(vec![parse_jwk(jwk)]),
                    (None, None) => None,
                };
                dict.set_item("jwks", jwks).unwrap();
                dict
            };
            let object: Py<PyAny> = module
                .getattr("FxaOAuthClient")?
                .call((), Some(kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    e
                })?
                .into();

            Ok(object)
        })
        .map_err(super::pyerr_to_tokenserver_error)?;

        Ok(Self {
            inner,
            timeout: settings.fxa_oauth_request_timeout,
        })
    }
}

#[async_trait]
impl VerifyToken for Verifier {
    type Output = VerifyOutput;

    /// Verifies an OAuth token. Returns `VerifyOutput` for valid tokens and a `TokenserverError`
    /// for invalid tokens.
    async fn verify(&self, token: String) -> Result<VerifyOutput, TokenserverError> {
        // We don't want to move `self` into the body of the closure here because we'd need to
        // clone it. Cloning it is only necessary if we need to verify the token remotely via FxA,
        // since that would require passing `self` to a separate thread. Passing &Self to a closure
        // gives us the flexibility to clone only when necessary.
        let verify_inner = |verifier: &Self| {
            let maybe_verify_output_string = Python::with_gil(|py| {
                let client = verifier.inner.as_ref(py);
                // `client.verify_token(token)`
                let result: &PyAny = client
                    .getattr("verify_token")?
                    .call((token,), None)
                    .map_err(|e| {
                        e.print_and_set_sys_last_vars(py);
                        e
                    })?;

                if result.is_none() {
                    Ok(None)
                } else {
                    let verify_output_python_string = result.downcast::<PyString>()?;
                    verify_output_python_string.extract::<String>().map(Some)
                }
            })
            .map_err(|e| TokenserverError {
                context: format!("pyo3 error in OAuth verifier: {}", e),
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            })?;

            match maybe_verify_output_string {
                Some(verify_output_string) => {
                    serde_json::from_str::<VerifyOutput>(&verify_output_string).map_err(|e| {
                        TokenserverError {
                            context: format!("Invalid OAuth verify output: {}", e),
                            ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                        }
                    })
                }
                None => Err(TokenserverError {
                    context: "Invalid OAuth token".to_owned(),
                    ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                }),
            }
        };

        let verifier = self.clone();

        // If the JWK is not cached or if the token is not a JWT/wasn't signed by a known key
        // type, PyFxA will make a request to FxA to retrieve it, blocking this thread. To
        // improve performance, we make the request on a thread in a threadpool specifically
        // used for blocking operations. The JWK should _always_ be cached in production to
        // maximize performance.
        let fut = web::block(move || verify_inner(&verifier));

        // The PyFxA OAuth client does not offer a way to set a request timeout, so we set one here
        // by timing out the future if the verification process blocks its thread for longer
        // than the specified number of seconds.
        time::timeout(Duration::from_secs(self.timeout), fut)
            .await
            .map_err(|_| TokenserverError {
                context: "OAuth verification timeout".to_owned(),
                ..TokenserverError::resource_unavailable()
            })?
            .map_err(|e| match e {
                BlockingError::Error(inner) => inner,
                BlockingError::Canceled => TokenserverError {
                    context: "Tokenserver threadpool operation failed".to_owned(),
                    ..TokenserverError::internal_error()
                },
            })
    }
}
