use async_trait::async_trait;
use jsonwebtoken::jwk::{AlgorithmParameters, Jwk, PublicKeyUse, RSAKeyParameters};
use pyo3::{
    ffi::c_str,
    prelude::{Py, PyAny, PyErr, PyModule, Python},
    types::{IntoPyDict, PyAnyMethods, PyString},
    Bound,
};
use serde_json;
use std::ffi::CStr;
use syncserver_common::{BlockingThreadpool, Metrics};
use tokenserver_common::TokenserverError;
use tokenserver_settings::Settings;
use tokio::time;

use super::VerifyOutput;
use crate::VerifyToken;

use std::{sync::Arc, time::Duration};

/// The verifier used to verify OAuth tokens.
#[derive(Clone)]
pub struct Verifier {
    // pointer
    inner: Arc<Py<PyAny>>,
    timeout: u64,
    blocking_threadpool: Arc<BlockingThreadpool>,
}

impl Verifier {
    pub fn new(
        settings: &Settings,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> Result<Self, TokenserverError> {
        let inner: Py<PyAny> = Python::with_gil::<_, Result<Py<PyAny>, TokenserverError>>(|py| {
            let code: &CStr = c_str!(include_str!("verify.py"));
            let module = PyModule::from_code(py, code, c_str!("verify.py"), c_str!("verify.py"))
                .map_err(pyerr_to_tokenserver_error)?;
            let kwargs = {
                let dict = [("server_url", &settings.fxa_oauth_server_url)].into_py_dict(py)?;
                let parse_jwk = |jwk: &Jwk| {
                    let (n, e) = match &jwk.algorithm {
                        AlgorithmParameters::RSA(RSAKeyParameters { key_type: _, n, e }) => (n, e),
                        _ => return Err(TokenserverError::internal_error()),
                    };
                    let alg = jwk
                        .common
                        .key_algorithm
                        .ok_or_else(TokenserverError::internal_error)?
                        .to_string();
                    let kid = jwk
                        .common
                        .key_id
                        .as_ref()
                        .ok_or_else(TokenserverError::internal_error)?;
                    if !matches!(
                        jwk.common
                            .public_key_use
                            .as_ref()
                            .ok_or_else(TokenserverError::internal_error)?,
                        PublicKeyUse::Signature
                    ) {
                        return Err(TokenserverError::internal_error());
                    }

                    let dict = [
                        ("kty", "RSA"),
                        ("alg", &alg),
                        ("kid", kid),
                        ("use", "sig"),
                        ("n", n),
                        ("e", e),
                    ]
                    .into_py_dict(py)?;
                    Ok(dict)
                };

                let jwks = match (
                    &settings.fxa_oauth_primary_jwk,
                    &settings.fxa_oauth_secondary_jwk,
                ) {
                    (Some(primary_jwk), Some(secondary_jwk)) => {
                        Some(vec![parse_jwk(primary_jwk)?, parse_jwk(secondary_jwk)?])
                    }
                    (Some(jwk), None) | (None, Some(jwk)) => Some(vec![parse_jwk(jwk)?]),
                    (None, None) => None,
                };
                dict.set_item("jwks", jwks)?;
                dict
            };
            let object: Py<PyAny> = module
                .getattr("FxaOAuthClient")
                .map_err(pyerr_to_tokenserver_error)?
                .call((), Some(&kwargs))
                .map_err(|e| {
                    e.print_and_set_sys_last_vars(py);
                    pyerr_to_tokenserver_error(e)
                })?
                .into();

            Ok(object)
        })?;

        Ok(Self {
            inner: Arc::new(inner),
            timeout: settings.fxa_oauth_request_timeout,
            blocking_threadpool,
        })
    }
}

#[async_trait]
impl VerifyToken for Verifier {
    type Output = VerifyOutput;

    /// Verifies an OAuth token. Returns `VerifyOutput` for valid tokens and a `TokenserverError`
    /// for invalid tokens.
    async fn verify(
        &self,
        token: String,
        _metrics: &Metrics,
    ) -> Result<VerifyOutput, TokenserverError> {
        // We don't want to move `self` into the body of the closure here because we'd need to
        // clone it. Cloning it is only necessary if we need to verify the token remotely via FxA,
        // since that would require passing `self` to a separate thread. Passing &Self to a closure
        // gives us the flexibility to clone only when necessary.
        let verify_inner = |verifier: &Self| {
            let maybe_verify_output_string = Python::with_gil(|py| {
                let client = verifier.inner.bind(py);
                // `client.verify_token(token)`
                let result: Bound<PyAny> = client
                    .getattr("verify_token")?
                    .call((token,), None)
                    .inspect_err(|e| e.print_and_set_sys_last_vars(py))?;
                if result.is_none() {
                    Ok(None)
                } else {
                    let verify_output_python_string = result.downcast::<PyString>()?;
                    verify_output_python_string.extract::<String>().map(Some)
                }
            })
            .map_err(|e| TokenserverError {
                context: format!("pyo3 error in OAuth verifier: {}", e),
                ..TokenserverError::internal_error()
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
        let fut = self
            .blocking_threadpool
            .spawn(move || verify_inner(&verifier));

        // The PyFxA OAuth client does not offer a way to set a request timeout, so we set one here
        // by timing out the future if the verification process blocks this thread for longer
        // than the specified number of seconds.
        time::timeout(Duration::from_secs(self.timeout), fut)
            .await
            .map_err(|_| TokenserverError::oauth_timeout())?
    }
}

fn pyerr_to_tokenserver_error(e: PyErr) -> TokenserverError {
    TokenserverError {
        context: e.to_string(),
        ..TokenserverError::internal_error()
    }
}
