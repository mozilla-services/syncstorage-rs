use actix_web::Error;
use async_trait::async_trait;
use futures::TryFutureExt;
use pyo3::{
    prelude::{Py, PyAny, PyErr, PyModule, Python},
    types::{IntoPyDict, PyString},
};
use serde::{Deserialize, Serialize};
use serde_json;
use tokenserver_common::error::TokenserverError;
use tokio::{task, time};

use super::VerifyToken;
use crate::tokenserver::settings::Settings;

use core::time::Duration;
use std::convert::TryFrom;

/// The information extracted from a valid OAuth token.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct VerifyOutput {
    #[serde(rename = "user")]
    pub fxa_uid: String,
    pub generation: Option<i64>,
}

/// The verifier used to verify OAuth tokens.
#[derive(Clone)]
pub struct RemoteVerifier {
    // Note that we do not need to use an Arc here, since Py is already a reference-counted
    // pointer
    inner: Py<PyAny>,
    timeout: u64,
}

impl RemoteVerifier {
    const FILENAME: &'static str = "verify.py";
}

impl TryFrom<&Settings> for RemoteVerifier {
    type Error = Error;

    fn try_from(settings: &Settings) -> Result<Self, Error> {
        let inner: Py<PyAny> = Python::with_gil::<_, Result<Py<PyAny>, PyErr>>(|py| {
            let code = include_str!("verify.py");
            let module = PyModule::from_code(py, code, Self::FILENAME, Self::FILENAME)?;
            let kwargs = [("server_url", &settings.fxa_oauth_server_url)].into_py_dict(py);
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
        .map_err(super::pyerr_to_actix_error)?;

        Ok(Self {
            inner,
            timeout: settings.fxa_oauth_request_timeout,
        })
    }
}

#[async_trait]
impl VerifyToken for RemoteVerifier {
    type Output = VerifyOutput;

    /// Verifies an OAuth token. Returns `VerifyOutput` for valid tokens and a `TokenserverError`
    /// for invalid tokens.
    async fn verify(&self, token: String) -> Result<VerifyOutput, TokenserverError> {
        let verifier = self.clone();

        let fut = task::spawn_blocking(move || {
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
                ..TokenserverError::invalid_credentials("Unauthorized")
            })?;

            match maybe_verify_output_string {
                Some(verify_output_string) => {
                    serde_json::from_str::<VerifyOutput>(&verify_output_string).map_err(|e| {
                        TokenserverError {
                            context: format!("Invalid OAuth verify output: {}", e),
                            ..TokenserverError::invalid_credentials("Unauthorized")
                        }
                    })
                }
                None => Err(TokenserverError {
                    context: "Invalid OAuth token".to_owned(),
                    ..TokenserverError::invalid_credentials("Unauthorized")
                }),
            }
        })
        .map_err(|err| {
            let context = if err.is_cancelled() {
                "Tokenserver threadpool operation cancelled"
            } else if err.is_panic() {
                "Tokenserver threadpool operation panicked"
            } else {
                "Tokenserver threadpool operation failed for unknown reason"
            };

            TokenserverError {
                context: context.to_owned(),
                ..TokenserverError::internal_error()
            }
        });

        // The PyFxA OAuth client does not offer a way to set a request timeout, so we set one here
        // by timing out the future if the verification process blocks this thread for longer
        // than the specified number of seconds.
        time::timeout(Duration::from_secs(self.timeout), fut)
            .await
            .map_err(|_| TokenserverError {
                context: "OAuth verification timeout".to_owned(),
                ..TokenserverError::resource_unavailable()
            })?
            .map_err(|_| TokenserverError::resource_unavailable())?
    }
}
