use async_trait::async_trait;
use jsonwebtoken::jwk::{AlgorithmParameters, Jwk, PublicKeyUse, RSAKeyParameters};
use pyo3::{
    Bound,
    ffi::c_str,
    prelude::{Py, PyAny, PyErr, PyModule, PyResult, Python},
    types::{IntoPyDict, PyAnyMethods, PyString},
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
    // verify.py's `FxaUnavailableError` class, raised when FxA errors or misbehaves
    fxa_unavailable_error: Arc<Py<PyAny>>,
    timeout: u64,
    blocking_threadpool: Arc<BlockingThreadpool>,
}

impl Verifier {
    pub fn new(
        settings: &Settings,
        blocking_threadpool: Arc<BlockingThreadpool>,
    ) -> Result<Self, TokenserverError> {
        let (inner, fxa_unavailable_error) = Python::attach(|py| -> Result<_, TokenserverError> {
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
            let fxa_unavailable_error: Py<PyAny> = module
                .getattr("FxaUnavailableError")
                .map_err(pyerr_to_tokenserver_error)?
                .into();

            Ok((object, fxa_unavailable_error))
        })?;

        Ok(Self {
            inner: Arc::new(inner),
            fxa_unavailable_error: Arc::new(fxa_unavailable_error),
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
            let maybe_verify_output_string = Python::attach(|py| {
                let client = verifier.inner.bind(py);
                let call = || -> PyResult<Option<String>> {
                    // `client.verify_token(token)`
                    let result: Bound<PyAny> = client
                        .getattr("verify_token")?
                        .call((token,), None)
                        .inspect_err(|e| e.print_and_set_sys_last_vars(py))?;
                    if result.is_none() {
                        Ok(None)
                    } else {
                        let verify_output_python_string = result.cast::<PyString>()?;
                        verify_output_python_string.extract::<String>().map(Some)
                    }
                };
                call().map_err(|e| {
                    if e.is_instance(py, verifier.fxa_unavailable_error.bind(py)) {
                        TokenserverError::oauth_fxa_unavailable(format!(
                            "FxA unavailable in OAuth verifier: {}",
                            e
                        ))
                    } else {
                        TokenserverError {
                            context: format!("pyo3 error in OAuth verifier: {}", e),
                            ..TokenserverError::internal_error()
                        }
                    }
                })
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

#[cfg(test)]
mod tests {
    use super::*;

    // A stand-in for PyFxA so these tests don't need it installed or a network call. The
    // error classes mirror the `fxa.errors` hierarchy and the client raises one per token.
    const FXA_STUB: &CStr = c_str!(
        r#"
import sys
import types

class Error(Exception): pass
class OutOfProtocolError(Error): pass
class InProtocolError(Error): pass
class ClientError(InProtocolError): pass
class ServerError(InProtocolError): pass
class TrustError(Error): pass

ERRORS = {
    "non-json": OutOfProtocolError("API responded with non-json content-type: "),
    "server-error": ServerError("Internal Server Error"),
    "client-error": ClientError("invalid token"),
    "trust-error": TrustError("invalid signature"),
    "other-error": ValueError("unexpected"),
}

class Client:
    def __init__(self, server_url=None, jwks=None):
        pass

    def verify_token(self, token, scope=None):
        if token in ERRORS:
            raise ERRORS[token]
        return {"user": "fxa_id", "client_id": "client_id", "generation": 123}

def install():
    names = ("fxa", "fxa.errors", "fxa.oauth")
    saved = {name: sys.modules.get(name) for name in names}
    fxa = types.ModuleType("fxa")
    errors = types.ModuleType("fxa.errors")
    oauth = types.ModuleType("fxa.oauth")
    for cls in (OutOfProtocolError, ServerError, ClientError, TrustError):
        setattr(errors, cls.__name__, cls)
    oauth.Client = Client
    fxa.errors = errors
    fxa.oauth = oauth
    sys.modules.update({"fxa": fxa, "fxa.errors": errors, "fxa.oauth": oauth})
    return saved

def restore(saved):
    for name, module in saved.items():
        if module is None:
            sys.modules.pop(name, None)
        else:
            sys.modules[name] = module
"#
    );

    /// Builds a `Verifier` whose verify.py is bound to the PyFxA stub.
    fn stub_verifier() -> Verifier {
        let (stub, saved) = Python::attach(|py| -> PyResult<(Py<PyAny>, Py<PyAny>)> {
            let stub =
                PyModule::from_code(py, FXA_STUB, c_str!("fxa_stub.py"), c_str!("fxa_stub"))?;
            let saved = stub.getattr("install")?.call0()?;
            Ok((stub.into_any().unbind(), saved.unbind()))
        })
        .unwrap();
        let verifier = Verifier::new(&Settings::default(), Arc::new(BlockingThreadpool::new(1)));
        // verify.py has already bound the stub's names, so put the real modules back
        Python::attach(|py| -> PyResult<()> {
            stub.bind(py).getattr("restore")?.call1((saved.bind(py),))?;
            Ok(())
        })
        .unwrap();
        verifier.unwrap()
    }

    async fn verify(token: &str) -> Result<VerifyOutput, TokenserverError> {
        stub_verifier()
            .verify(token.to_owned(), &Default::default())
            .await
    }

    #[tokio::test]
    async fn valid_token() {
        let output = verify("valid").await.unwrap();
        assert_eq!(output.fxa_uid, "fxa_id");
        assert_eq!(output.client_id, "client_id");
        assert_eq!(output.generation, Some(123));
    }

    #[tokio::test]
    async fn fxa_unavailable_is_503() {
        for token in ["non-json", "server-error"] {
            let err = verify(token).await.unwrap_err();
            assert_eq!(err.http_status.as_u16(), 503, "{token}");
            assert_eq!(
                err.tags,
                Some(vec![("reason", "oauth_fxa_unavailable".to_owned())]),
                "{token}"
            );
            assert!(
                err.context
                    .starts_with("FxA unavailable in OAuth verifier: "),
                "{token}: {}",
                err.context
            );
        }
    }

    #[tokio::test]
    async fn invalid_token_is_401() {
        for token in ["client-error", "trust-error"] {
            let err = verify(token).await.unwrap_err();
            assert_eq!(err.http_status.as_u16(), 401, "{token}");
        }
    }

    #[tokio::test]
    async fn other_python_error_is_500() {
        let err = verify("other-error").await.unwrap_err();
        assert_eq!(err.http_status.as_u16(), 500);
        assert!(
            err.context.starts_with("pyo3 error in OAuth verifier: "),
            "{}",
            err.context
        );
    }
}
