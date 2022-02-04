use actix_web::{web, Error};
use async_trait::async_trait;
use reqwest::{blocking::Client as ReqwestClient, StatusCode};
use serde::{de::Deserializer, Deserialize, Serialize};

use super::VerifyToken;
use crate::tokenserver::{
    error::{ErrorLocation, TokenserverError},
    settings::Settings,
};

use core::time::Duration;
use std::{convert::TryFrom, sync::Arc};

/// The information extracted from a valid BrowserID assertion.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct VerifyOutput {
    pub device_id: Option<String>,
    pub email: String,
    pub generation: Option<i64>,
    pub keys_changed_at: Option<i64>,
}

/// The verifier used to verify BrowserID assertions.
#[derive(Clone)]
pub struct RemoteVerifier {
    audience: String,
    issuer: String,
    fxa_verifier_url: String,
    // We need to use an Arc here because reqwest's blocking client doesn't use one internally,
    // and we want to share keep-alive connections across threads
    request_client: Arc<ReqwestClient>,
}

impl TryFrom<&Settings> for RemoteVerifier {
    type Error = Error;

    fn try_from(settings: &Settings) -> Result<Self, Error> {
        Ok(Self {
            audience: settings.fxa_browserid_audience.clone(),
            issuer: settings.fxa_browserid_issuer.clone(),
            request_client: Arc::new(
                ReqwestClient::builder()
                    .timeout(Duration::new(settings.fxa_browserid_request_timeout, 0))
                    .build()
                    .map_err(|_| Error::from(()))?,
            ),
            fxa_verifier_url: settings.fxa_browserid_server_url.clone(),
        })
    }
}

#[async_trait]
impl VerifyToken for RemoteVerifier {
    type Output = VerifyOutput;

    /// Verifies a BrowserID assertion. Returns `VerifyOutput` for valid assertions and a
    /// `TokenserverError` for invalid assertions.
    async fn verify(&self, assertion: String) -> Result<VerifyOutput, TokenserverError> {
        let verifier = self.clone();

        web::block(move || {
            let response = verifier
                .request_client
                .post(&verifier.fxa_verifier_url)
                .json(&VerifyRequest {
                    assertion,
                    audience: verifier.audience.clone(),
                    trusted_issuers: [verifier.issuer.clone()],
                })
                .send()
                .map_err(|e| {
                    if e.is_connect() {
                        // If we are unable to reach the FxA server, report a 503 to the client
                        TokenserverError::resource_unavailable()
                    } else {
                        // If any other error occurs during the request, report a 401 to the client
                        TokenserverError::invalid_credentials("Unauthorized")
                    }
                })?;

            // If FxA responds with an HTTP status other than 200, report a 503 to the client
            if response.status() != StatusCode::OK {
                return Err(TokenserverError::resource_unavailable());
            }

            // If FxA responds with an invalid response body, report a 503 to the client
            let response_body = response
                .json::<VerifyResponse>()
                .map_err(|_| TokenserverError::resource_unavailable())?;

            match response_body {
                VerifyResponse::Failure {
                    reason: Some(reason),
                } if reason.contains("expired") || reason.contains("issued later than") => {
                    Err(TokenserverError {
                        status: "invalid-timestamp",
                        location: ErrorLocation::Body,
                        ..Default::default()
                    })
                }
                VerifyResponse::Failure { .. } => {
                    Err(TokenserverError::invalid_credentials("Unauthorized"))
                }
                VerifyResponse::Okay { issuer, .. } if issuer != verifier.issuer => {
                    Err(TokenserverError::invalid_credentials("Unauthorized"))
                }
                VerifyResponse::Okay {
                    idp_claims: Some(claims),
                    ..
                } if !claims.token_verified() => {
                    Err(TokenserverError::invalid_credentials("Unauthorized"))
                }
                VerifyResponse::Okay {
                    email,
                    idp_claims: Some(claims),
                    ..
                } => Ok(VerifyOutput {
                    device_id: claims.device_id.clone(),
                    email,
                    generation: claims.generation()?,
                    keys_changed_at: claims.keys_changed_at()?,
                }),
                VerifyResponse::Okay { email, .. } => Ok(VerifyOutput {
                    device_id: None,
                    email,
                    generation: None,
                    keys_changed_at: None,
                }),
            }
        })
        .await
        .map_err(Into::into)
    }
}

/// The request sent to the FxA BrowserID verifier for token verification.
#[derive(Serialize)]
struct VerifyRequest {
    assertion: String,
    audience: String,
    #[serde(rename(serialize = "trustedIssuers"))]
    trusted_issuers: [String; 1],
}

/// The response returned by the FxA BrowserID verifier for a token verification request.
#[derive(Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum VerifyResponse {
    Okay {
        email: String,
        #[serde(rename = "idpClaims")]
        idp_claims: Option<IdpClaims>,
        issuer: String,
    },
    Failure {
        reason: Option<String>,
    },
}

/// The claims extracted from a valid BrowserID assertion.
#[derive(Deserialize, Serialize)]
struct IdpClaims {
    #[serde(rename = "fxa-deviceId")]
    pub device_id: Option<String>,
    #[serde(
        default,
        rename = "fxa-generation",
        deserialize_with = "strict_deserialize"
    )]
    generation: Option<Option<i64>>,
    #[serde(
        default,
        rename = "fxa-keysChangedAt",
        deserialize_with = "strict_deserialize"
    )]
    keys_changed_at: Option<Option<i64>>,
    #[serde(
        default,
        rename = "fxa-tokenVerified",
        deserialize_with = "strict_deserialize"
    )]
    token_verified: Option<Option<bool>>,
}

impl IdpClaims {
    fn generation(&self) -> Result<Option<i64>, TokenserverError> {
        match self.generation {
            // If the fxa-generation claim is present, return its value. If it's missing, return None.
            Some(Some(_)) | None => Ok(self.generation.flatten()),
            // If the fxa-generation claim is null, return an error.
            Some(None) => Err(TokenserverError::invalid_generation()),
        }
    }

    fn keys_changed_at(&self) -> Result<Option<i64>, TokenserverError> {
        match self.keys_changed_at {
            // If the fxa-keysChangedAt claim is present, return its value. If it's missing, return None.
            Some(Some(_)) | None => Ok(self.keys_changed_at.flatten()),
            // If the fxa-keysChangedAt claim is null, return an error.
            Some(None) => Err(TokenserverError {
                description: "invalid keysChangedAt",
                status: "invalid-credentials",
                location: ErrorLocation::Body,
                ..Default::default()
            }),
        }
    }

    fn token_verified(&self) -> bool {
        match self.token_verified {
            // If the fxa-tokenVerified claim is true or missing, return true.
            Some(Some(true)) | None => true,
            // If the fxa-tokenVerified claim is false or null, return false.
            Some(Some(false)) | Some(None) => false,
        }
    }
}

// Approach inspired by: https://github.com/serde-rs/serde/issues/984#issuecomment-314143738
fn strict_deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    use mockito::{self, Mock};

    #[actix_rt::test]
    async fn test_browserid_verifier_success() {
        let body = r#"{
            "status": "okay",
            "email": "test@example.com",
            "audience": "https://test.com",
            "issuer": "accounts.firefox.com",
            "idpClaims": {
                "fxa-deviceId": "test_device_id",
                "fxa-generation": 1234,
                "fxa-keysChangedAt": 5678
            }
        }"#;
        let mock = mockito::mock("POST", "/v2")
            .with_header("content-type", "application/json")
            .with_body(body)
            .create();
        let verifier = RemoteVerifier::try_from(&Settings {
            fxa_browserid_audience: "https://test.com".to_owned(),
            fxa_browserid_issuer: "accounts.firefox.com".to_owned(),
            fxa_browserid_server_url: format!("{}/v2", mockito::server_url()),
            ..Default::default()
        })
        .unwrap();

        let result = verifier.verify("test".to_owned()).await.unwrap();
        mock.assert();

        let expected_result = VerifyOutput {
            device_id: Some("test_device_id".to_owned()),
            email: "test@example.com".to_owned(),
            generation: Some(1234),
            keys_changed_at: Some(5678),
        };

        assert_eq!(expected_result, result);
    }

    #[actix_rt::test]
    async fn test_browserid_verifier_failure_cases() {
        const AUDIENCE: &str = "https://test.com";

        let verifier = RemoteVerifier::try_from(&Settings {
            fxa_browserid_audience: AUDIENCE.to_owned(),
            fxa_browserid_server_url: format!("{}/v2", mockito::server_url()),
            ..Default::default()
        })
        .unwrap();
        let assertion = "test";

        // Verifier returns 500
        {
            let mock = mockito::mock("POST", "/v2")
                .with_status(500)
                .with_header("content-type", "application/json")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError::resource_unavailable();
            assert_eq!(expected_error, error);
        }

        // "Server Error" in body
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("<h1>Server Error</h1>")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError::resource_unavailable();
            assert_eq!(expected_error, error);
        }

        // {"status": "error"}
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"error\"}")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError::resource_unavailable();
            assert_eq!(expected_error, error);
        }

        // {"status": "potato"} in body
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"potato\"}")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError::resource_unavailable();
            assert_eq!(expected_error, error);
        }

        // {"status": "error"} in body with random reason
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"failure\", \"reason\": \"something broke\"}")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError::invalid_credentials("Unauthorized");
            assert_eq!(expected_error, error);
        }
    }

    #[actix_rt::test]
    async fn test_browserid_verifier_rejects_unissuers() {
        const AUDIENCE: &str = "https://test.com";
        const ISSUER: &str = "accounts.firefox.com";

        fn mock(issuer: &'static str) -> Mock {
            let body = format!(
                r#"{{
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": "{}"
            }}"#,
                issuer
            );

            mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body)
                .create()
        }

        let expected_error = TokenserverError::invalid_credentials("Unauthorized");
        let verifier = RemoteVerifier::try_from(&Settings {
            fxa_browserid_audience: AUDIENCE.to_owned(),
            fxa_browserid_issuer: ISSUER.to_owned(),
            fxa_browserid_server_url: format!("{}/v2", mockito::server_url()),
            ..Default::default()
        })
        .unwrap();
        let assertion = "test".to_owned();

        {
            let mock = mock("login.persona.org");
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        {
            let mock = mock(ISSUER);
            let result = verifier.verify(assertion.clone()).await.unwrap();
            let expected_result = VerifyOutput {
                device_id: None,
                email: "test@example.com".to_owned(),
                generation: None,
                keys_changed_at: None,
            };

            mock.assert();
            assert_eq!(expected_result, result);
        }

        {
            let mock = mock("accounts.firefox.org");
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        {
            let mock = mock("http://accounts.firefox.com");
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        {
            let mock = mock("accounts.firefox.co");
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        let expected_error = TokenserverError::resource_unavailable();

        {
            let body = r#"{{
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": 42
            }}"#;
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body)
                .create();
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        {
            let body = r#"{{
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": null
            }}"#;
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body)
                .create();
            let error = verifier.verify(assertion.clone()).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }

        {
            let body = r#"{{
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com"
            }}"#;
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body)
                .create();
            let error = verifier.verify(assertion).await.unwrap_err();

            mock.assert();
            assert_eq!(expected_error, error);
        }
    }
}
