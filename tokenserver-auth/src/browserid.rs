use async_trait::async_trait;
use reqwest::{Client as ReqwestClient, StatusCode};
use serde::{de::Deserializer, Deserialize, Serialize};
use tokenserver_common::{ErrorLocation, TokenType, TokenserverError};
use tokenserver_settings::Settings;

use super::VerifyToken;

use std::{convert::TryFrom, time::Duration};

/// The information extracted from a valid BrowserID assertion.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
pub struct VerifyOutput {
    pub device_id: Option<String>,
    pub email: String,
    pub generation: Option<i64>,
    pub keys_changed_at: Option<i64>,
}

/// The verifier used to verify BrowserID assertions.
#[derive(Clone)]
pub struct Verifier {
    audience: String,
    issuer: String,
    fxa_verifier_url: String,
    // reqwest's async client uses an `Arc` internally, so we don't need to use one here to take
    // advantage of keep-alive connections across threads.
    request_client: ReqwestClient,
}

impl TryFrom<&Settings> for Verifier {
    type Error = &'static str;

    fn try_from(settings: &Settings) -> Result<Self, Self::Error> {
        Ok(Self {
            audience: settings.fxa_browserid_audience.clone(),
            issuer: settings.fxa_browserid_issuer.clone(),
            request_client: ReqwestClient::builder()
                .timeout(Duration::from_secs(settings.fxa_browserid_request_timeout))
                .connect_timeout(Duration::from_secs(settings.fxa_browserid_connect_timeout))
                .use_rustls_tls()
                .build()
                .map_err(|_| "failed to build BrowserID reqwest client")?,
            fxa_verifier_url: settings.fxa_browserid_server_url.clone(),
        })
    }
}

#[async_trait]
impl VerifyToken for Verifier {
    type Output = VerifyOutput;

    /// Verifies a BrowserID assertion. Returns `VerifyOutput` for valid assertions and a
    /// `TokenserverError` for invalid assertions.
    async fn verify(&self, assertion: String) -> Result<VerifyOutput, TokenserverError> {
        let response = self
            .request_client
            .post(&self.fxa_verifier_url)
            .json(&VerifyRequest {
                assertion,
                audience: self.audience.clone(),
                trusted_issuers: [self.issuer.clone()],
            })
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    // If we are unable to reach the FxA server or if FxA responds with an HTTP
                    // status other than 200, report a 503 to the client
                    TokenserverError {
                        context: format!(
                            "Request error occurred during BrowserID request to FxA: {}",
                            e
                        ),
                        token_type: TokenType::BrowserId,
                        ..TokenserverError::resource_unavailable()
                    }
                } else {
                    // If any other error occurs during the request, report a 401 to the client
                    TokenserverError {
                        context: format!(
                            "Unknown error occurred during BrowserID request to FxA: {}",
                            e
                        ),
                        token_type: TokenType::BrowserId,
                        ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
                    }
                }
            })?;

        if response.status() != StatusCode::OK {
            return Err(TokenserverError {
                context: format!(
                    "FxA returned a status code other than 200 ({})",
                    response.status().as_u16()
                ),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            });
        }

        // If FxA responds with an invalid response body, report a 503 to the client
        let response_body =
            response
                .json::<VerifyResponse>()
                .await
                .map_err(|e| TokenserverError {
                    context: format!(
                        "Invalid BrowserID verification response received from FxA: {}",
                        e
                    ),
                    token_type: TokenType::BrowserId,
                    ..TokenserverError::resource_unavailable()
                })?;

        match response_body {
            VerifyResponse::Failure {
                reason: Some(reason),
            } if reason.contains("expired") || reason.contains("issued later than") => {
                Err(TokenserverError {
                    status: "invalid-timestamp",
                    location: ErrorLocation::Body,
                    context: "Expired BrowserID assertion".to_owned(),
                    token_type: TokenType::BrowserId,
                    ..Default::default()
                })
            }
            VerifyResponse::Failure {
                reason: Some(reason),
            } => Err(TokenserverError {
                context: format!("BrowserID verification error: {}", reason),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            }),
            VerifyResponse::Failure { .. } => Err(TokenserverError {
                context: "Unknown BrowserID verification error".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            }),
            VerifyResponse::Okay { issuer, .. } if issuer != self.issuer => Err(TokenserverError {
                context: "BrowserID issuer mismatch".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            }),
            VerifyResponse::Okay {
                idp_claims: Some(claims),
                ..
            } if !claims.token_verified() => Err(TokenserverError {
                context: "BrowserID assertion not verified".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            }),
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
    /// The nested `Option`s are necessary to distinguish between a `null` value and a missing key
    /// altogether: `Some(None)` translates to a `null` value and `None` translates to a missing
    /// key.
    #[serde(
        default,
        rename = "fxa-generation",
        deserialize_with = "strict_deserialize"
    )]
    generation: Option<Option<i64>>,
    /// The nested `Option`s are necessary to distinguish between a `null` value and a missing key
    /// altogether: `Some(None)` translates to a `null` value and `None` translates to a missing
    /// key.
    #[serde(
        default,
        rename = "fxa-keysChangedAt",
        deserialize_with = "strict_deserialize"
    )]
    keys_changed_at: Option<Option<i64>>,
    /// The nested `Option`s are necessary to distinguish between a `null` value and a missing key
    /// altogether: `Some(None)` translates to a `null` value and `None` translates to a missing
    /// key.
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
            Some(None) => Err(TokenserverError {
                context: "null fxa-generation claim in BrowserID assertion".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_generation()
            }),
        }
    }

    fn keys_changed_at(&self) -> Result<Option<i64>, TokenserverError> {
        match self.keys_changed_at {
            // If the fxa-keysChangedAt claim is present, return its value. If it's missing, return None.
            Some(Some(_)) | None => Ok(self.keys_changed_at.flatten()),
            // If the fxa-keysChangedAt claim is null, return an error.
            Some(None) => Err(TokenserverError {
                description: "invalid keysChangedAt".to_owned(),
                status: "invalid-credentials",
                location: ErrorLocation::Body,
                context: "null fxa-keysChangedAt claim in BrowserID assertion".to_owned(),
                token_type: TokenType::BrowserId,
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
/// This function is used to deserialize JSON fields that may or may not be present. If the field
/// is present, its value is enclosed in `Some`. This results in types of the form
/// `Option<Option<T>>`. If the outer `Option` is `None`, the field wasn't present in the JSON, and
/// if the inner `Option` is `None`, the field was present with a `null` value.
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
    use serde_json::json;

    #[tokio::test]
    async fn test_browserid_verifier_success() {
        let body = json!({
            "status": "okay",
            "email": "test@example.com",
            "audience": "https://test.com",
            "issuer": "accounts.firefox.com",
            "idpClaims": {
                "fxa-deviceId": "test_device_id",
                "fxa-generation": 1234,
                "fxa-keysChangedAt": 5678
            }
        });
        let mock = mockito::mock("POST", "/v2")
            .with_header("content-type", "application/json")
            .with_body(body.to_string())
            .create();
        let verifier = Verifier::try_from(&Settings {
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

    #[tokio::test]
    async fn test_browserid_verifier_failure_cases() {
        const AUDIENCE: &str = "https://test.com";

        let verifier = Verifier::try_from(&Settings {
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

            let expected_error = TokenserverError {
                context: "FxA returned a status code other than 200 (500)".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
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

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: expected value at line 1 column 1".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
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

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: unknown variant `error`, expected `okay` or `failure` at line 1 column 18".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
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

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: unknown variant `potato`, expected `okay` or `failure` at line 1 column 19".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
            assert_eq!(expected_error, error);
        }

        // {"status": "failure"} in body with random reason
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"failure\", \"reason\": \"something broke\"}")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError {
                context: "BrowserID verification error: something broke".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            };
            assert_eq!(expected_error, error);
        }
        // {"status": "failure"} in body with no reason
        {
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body("{\"status\": \"failure\"}")
                .create();

            let error = verifier.verify(assertion.to_owned()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError {
                context: "Unknown BrowserID verification error".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
            };
            assert_eq!(expected_error, error);
        }
    }

    #[tokio::test]
    async fn test_browserid_verifier_rejects_unissuers() {
        const AUDIENCE: &str = "https://test.com";
        const ISSUER: &str = "accounts.firefox.com";

        fn mock(issuer: &'static str) -> Mock {
            let body = json!({
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": issuer
            });

            mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body.to_string())
                .create()
        }

        let expected_error = TokenserverError {
            context: "BrowserID issuer mismatch".to_owned(),
            token_type: TokenType::BrowserId,
            ..TokenserverError::invalid_credentials("Unauthorized".to_owned())
        };
        let verifier = Verifier::try_from(&Settings {
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

        {
            let body = json!({
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": 42,
            });
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body.to_string())
                .create();
            let error = verifier.verify(assertion.clone()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: invalid type: integer `42`, expected a string".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
            assert_eq!(expected_error, error);
        }

        {
            let body = json!({
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
                "issuer": None::<()>,
            });
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body.to_string())
                .create();
            let error = verifier.verify(assertion.clone()).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: invalid type: null, expected a string".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
            assert_eq!(expected_error, error);
        }

        {
            let body = json!({
                "status": "okay",
                "email": "test@example.com",
                "audience": "https://testmytoken.com",
            });
            let mock = mockito::mock("POST", "/v2")
                .with_header("content-type", "application/json")
                .with_body(body.to_string())
                .create();
            let error = verifier.verify(assertion).await.unwrap_err();
            mock.assert();

            let expected_error = TokenserverError {
                context: "Invalid BrowserID verification response received from FxA: error decoding response body: missing field `issuer`".to_owned(),
                token_type: TokenType::BrowserId,
                ..TokenserverError::resource_unavailable()
            };
            assert_eq!(expected_error, error);
        }
    }
}
