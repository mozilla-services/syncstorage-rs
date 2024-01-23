use super::VerifyToken;
use crate::crypto::OAuthVerifyError;
pub use crate::crypto::{JWTVerifier, JWTVerifierImpl};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use syncserver_common::Metrics;
use tokenserver_common::TokenserverError;
use tokenserver_settings::Settings;

const SYNC_SCOPE: &str = "https://identity.mozilla.com/apps/oldsync";

#[derive(Serialize, Deserialize, Debug)]
struct TokenClaims {
    #[serde(rename = "sub")]
    user: String,
    scope: String,
    #[serde(rename = "fxa-generation")]
    generation: Option<i64>,
}

impl TokenClaims {
    fn validate(self) -> Result<VerifyOutput, TokenserverError> {
        if !self.scope.split(',').any(|scope| scope == SYNC_SCOPE) {
            return Err(TokenserverError::invalid_credentials(
                "Unauthorized".to_string(),
            ));
        }
        Ok(self.into())
    }
}

impl From<TokenClaims> for VerifyOutput {
    fn from(value: TokenClaims) -> Self {
        Self {
            fxa_uid: value.user,
            generation: value.generation,
        }
    }
}

/// The information extracted from a valid OAuth token.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifyOutput {
    #[serde(rename = "user")]
    pub fxa_uid: String,
    pub generation: Option<i64>,
}

/// The verifier used to verify OAuth tokens.
#[derive(Clone)]
pub struct Verifier<J> {
    server_url: Url,
    jwk_verifiers: Vec<J>,
}

impl<J> Verifier<J>
where
    J: JWTVerifier,
{
    pub fn new(settings: &Settings, jwk_verifiers: Vec<J>) -> Result<Self, TokenserverError> {
        Ok(Self {
            server_url: Url::parse(&settings.fxa_oauth_server_url)
                .map_err(|_| TokenserverError::internal_error())?,
            jwk_verifiers,
        })
    }

    async fn remote_verify_token(&self, token: &str) -> Result<TokenClaims, TokenserverError> {
        #[derive(Serialize)]
        struct VerifyRequest<'a> {
            token: &'a str,
        }

        #[derive(Serialize, Deserialize)]
        struct VerifyResponse {
            user: String,
            scope: Vec<String>,
            generation: Option<i64>,
        }

        impl From<VerifyResponse> for TokenClaims {
            fn from(value: VerifyResponse) -> Self {
                Self {
                    user: value.user,
                    scope: value.scope.join(","),
                    generation: value.generation,
                }
            }
        }

        let client = reqwest::Client::new();
        let url = self
            .server_url
            .join("v1/verify")
            .map_err(|_| TokenserverError::internal_error())?;
        Ok(client
            .post(url)
            .json(&VerifyRequest { token })
            .send()
            .await
            .map_err(|_| TokenserverError::invalid_credentials("Unauthorized".to_string()))
            .and_then(|res| {
                if !res.status().is_success() {
                    Err(TokenserverError::invalid_credentials(
                        "Unauthorized".to_string(),
                    ))
                } else {
                    Ok(res)
                }
            })?
            .json::<VerifyResponse>()
            .await
            .map_err(|_| TokenserverError::invalid_credentials("Unauthorized".to_string()))?
            .into())
    }

    async fn get_remote_jwks(&self) -> Result<Vec<J>, TokenserverError> {
        #[derive(Deserialize)]
        struct KeysResponse<K> {
            keys: Vec<K>,
        }
        let client = reqwest::Client::new();
        let url = self
            .server_url
            .join("v1/jwks")
            .map_err(|_| TokenserverError::internal_error())?;
        client
            .get(url)
            .send()
            .await
            .map_err(|_| TokenserverError::internal_error())?
            .json::<KeysResponse<J::Key>>()
            .await
            .map_err(|_| TokenserverError::internal_error())?
            .keys
            .into_iter()
            .map(|key| {
                key.try_into()
                    .map_err(|_| TokenserverError::internal_error())
            })
            .collect()
    }

    fn verify_jwt_locally(
        &self,
        verifiers: &[Cow<'_, J>],
        token: &str,
    ) -> Result<TokenClaims, OAuthVerifyError> {
        if verifiers.is_empty() {
            return Err(OAuthVerifyError::InvalidKey);
        }

        verifiers
            .iter()
            .find_map(|verifier| {
                match verifier.verify::<TokenClaims>(token) {
                    // If it's an invalid signature, it means our key was well formatted,
                    // but the signature was incorrect. Lets try another key if we have any
                    Err(OAuthVerifyError::InvalidSignature) => None,
                    res => Some(res),
                }
            })
            // If there is nothing, it means all of our keys were well formatted, but none of them
            // were able to verify the signature, lets erturn a TrustError
            .ok_or(OAuthVerifyError::TrustError)?
    }
}

#[async_trait]
impl<J> VerifyToken for Verifier<J>
where
    J: JWTVerifier,
{
    type Output = VerifyOutput;

    /// Verifies an OAuth token. Returns `VerifyOutput` for valid tokens and a `TokenserverError`
    /// for invalid tokens.
    async fn verify(
        &self,
        token: String,
        metrics: &Metrics,
    ) -> Result<VerifyOutput, TokenserverError> {
        let mut verifiers = self
            .jwk_verifiers
            .iter()
            .map(Cow::Borrowed)
            .collect::<Vec<_>>();
        if self.jwk_verifiers.is_empty() {
            verifiers = self
                .get_remote_jwks()
                .await
                .unwrap_or_else(|_| vec![])
                .into_iter()
                .map(Cow::Owned)
                .collect();
        }

        let claims = match self.verify_jwt_locally(&verifiers, &token) {
            Ok(res) => res,
            Err(e) => {
                if e.is_reportable_err() {
                    metrics.incr(e.metric_label())
                }
                match e {
                    OAuthVerifyError::DecodingError | OAuthVerifyError::InvalidKey => {
                        self.remote_verify_token(&token).await?
                    }
                    _ => {
                        return Err(TokenserverError::invalid_credentials(
                            "Unauthorized".to_string(),
                        ))
                    }
                }
            }
        };
        claims.validate()
    }
}

#[cfg(test)]
mod tests {
    use crate::crypto::OAuthVerifyError;
    use serde_json::json;

    use super::*;
    #[derive(Deserialize)]
    struct MockJWK {}

    macro_rules! mock_jwk_verifier {
        ($im:expr) => {
            mock_jwk_verifier!(_token, $im);
        };
        ($token:ident, $im:expr) => {
            #[derive(Clone, Debug)]
            struct MockJWTVerifier {}
            impl TryFrom<MockJWK> for MockJWTVerifier {
                type Error = OAuthVerifyError;
                fn try_from(_value: MockJWK) -> Result<Self, Self::Error> {
                    Ok(Self {})
                }
            }

            impl JWTVerifier for MockJWTVerifier {
                type Key = MockJWK;
                fn verify<T: ::serde::de::DeserializeOwned>(
                    &self,
                    $token: &str,
                ) -> Result<T, OAuthVerifyError> {
                    $im
                }
            }
        };
    }

    #[tokio::test]
    async fn test_no_keys_in_verifier_fallsback_to_fxa() -> Result<(), TokenserverError> {
        // We will first ask FxA for its keys, and if that fails we'll give up and ask
        // fxa to verify for us
        let mock_jwks = mockito::mock("GET", "/v1/jwks").with_status(500).create();

        let body = json!({
            "user": "fxa_id",
            "scope": [SYNC_SCOPE],
            "generation": 123
        });
        let mock_verify = mockito::mock("POST", "/v1/verify")
            .with_header("content-type", "application/json")
            .with_status(200)
            .with_body(body.to_string())
            .create();

        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Default::default()
        };
        let verifer: Verifier<JWTVerifierImpl> = Verifier::new(&settings, vec![])?;
        let res = verifer
            .verify("a token fxa will validate".to_string(), &Default::default())
            .await?;
        mock_jwks.expect(1);
        mock_verify.expect(1);
        assert_eq!(res.generation.unwrap(), 123);
        assert_eq!(res.fxa_uid, "fxa_id");
        Ok(())
    }

    #[tokio::test]
    async fn test_expired_signature_fails() -> Result<(), TokenserverError> {
        let mock = mockito::mock("POST", "/v1/verify").create();
        mock_jwk_verifier!(Err(OAuthVerifyError::InvalidSignature));

        let jwk_verifiers = vec![MockJWTVerifier {}];
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };

        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers)?;

        let err = verifier
            .verify("An expired token".to_string(), &Default::default())
            .await
            .unwrap_err();
        // We also make sure we didn't try to hit the server
        mock.expect(0);
        assert_eq!(err.status, "invalid-credentials");
        assert_eq!(err.http_status, 401);
        assert_eq!(err.description, "Unauthorized");

        Ok(())
    }

    #[tokio::test]
    async fn test_verifier_attempts_all_keys_if_invalid_signature() -> Result<(), TokenserverError>
    {
        let mock = mockito::mock("POST", "/v1/verify").create();
        #[derive(Debug, Clone)]
        struct MockJWTVerifier {
            id: u8,
        }

        impl From<MockJWK> for MockJWTVerifier {
            fn from(_value: MockJWK) -> Self {
                Self { id: 0 }
            }
        }

        impl JWTVerifier for MockJWTVerifier {
            type Key = MockJWK;
            fn verify<T: serde::de::DeserializeOwned>(
                &self,
                token: &str,
            ) -> Result<T, OAuthVerifyError> {
                if self.id == 0 {
                    Err(OAuthVerifyError::InvalidSignature)
                } else {
                    Ok(serde_json::from_str(token).unwrap())
                }
            }
        }

        let jwk_verifiers = vec![MockJWTVerifier { id: 0 }, MockJWTVerifier { id: 1 }];
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };
        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();

        let token_claims = TokenClaims {
            user: "fxa_id".to_string(),
            scope: SYNC_SCOPE.to_string(),
            generation: Some(124),
        };

        let res = verifier
            .verify(
                serde_json::to_string(&token_claims).unwrap(),
                &Default::default(),
            )
            .await?;
        assert_eq!(res.fxa_uid, "fxa_id");
        assert_eq!(res.generation.unwrap(), 124);
        mock.expect(0); // We shouldn't have hit the server
        Ok(())
    }

    #[tokio::test]
    async fn test_verifier_all_signature_failures_fails() -> Result<(), TokenserverError> {
        let mock_verify = mockito::mock("POST", "/v1/verify").create();
        mock_jwk_verifier!(Err(OAuthVerifyError::InvalidSignature));

        let jwk_verifiers = vec![MockJWTVerifier {}, MockJWTVerifier {}];
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };
        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();
        let err = verifier
            .verify(
                "a token with an invalid signature".to_string(),
                &Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.status, "invalid-credentials");
        assert_eq!(err.http_status, 401);
        assert_eq!(err.description, "Unauthorized");

        mock_verify.expect(0);
        Ok(())
    }

    #[tokio::test]
    async fn test_verifier_fallsback_if_decode_error() -> Result<(), TokenserverError> {
        let body = json!({
            "user": "fxa_id",
            "scope": [SYNC_SCOPE],
            "generation": 123
        });
        let mock_verify = mockito::mock("POST", "/v1/verify")
            .with_header("content-type", "application/json")
            .with_status(200)
            .with_body(body.to_string())
            .create();

        mock_jwk_verifier!(Err(OAuthVerifyError::DecodingError));

        let jwk_verifiers = vec![MockJWTVerifier {}];
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };
        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();

        let res = verifier
            .verify(
                "invalid token that can't be decoded".to_string(),
                &Default::default(),
            )
            .await?;
        assert_eq!(res.fxa_uid, "fxa_id");
        assert_eq!(res.generation.unwrap(), 123);
        mock_verify.expect(1); // We would have have hit the server
        Ok(())
    }

    #[tokio::test]
    async fn test_no_sync_scope_fails() -> Result<(), TokenserverError> {
        let token_claims = TokenClaims {
            user: "fxa_id".to_string(),
            scope: "some other scope".to_string(),
            generation: Some(124),
        };
        mock_jwk_verifier!(token, Ok(serde_json::from_str(token).unwrap()));
        let jwk_verifiers = vec![MockJWTVerifier {}];
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };
        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();
        let err = verifier
            .verify(
                serde_json::to_string(&token_claims).unwrap(),
                &Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.status, "invalid-credentials");
        assert_eq!(err.http_status, 401);
        assert_eq!(err.description, "Unauthorized");

        Ok(())
    }

    #[tokio::test]
    async fn test_fxa_rejects_token_no_matter_the_body() -> Result<(), TokenserverError> {
        let body = json!({
            "user": "fxa_id",
            "scope": [SYNC_SCOPE],
            "generation": 123
        });
        let mock_verify = mockito::mock("POST", "/v1/verify")
            .with_header("content-type", "application/json")
            .with_status(401)
            // Even though the body is fine, if FxA returns a none-200, we automatically
            // return a credential error
            .with_body(body.to_string())
            .create();
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };

        mock_jwk_verifier!(Err(OAuthVerifyError::DecodingError));
        let jwk_verifiers = vec![];

        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();

        let err = verifier
            .verify(
                "A token that we will ask FxA about".to_string(),
                &Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.status, "invalid-credentials");
        assert_eq!(err.http_status, 401);
        assert_eq!(err.description, "Unauthorized");
        mock_verify.expect(1);

        Ok(())
    }

    #[tokio::test]
    async fn test_fxa_accepts_token_but_bad_body() -> Result<(), TokenserverError> {
        let body = json!({
            "bad_key": "foo",
            "scope": [SYNC_SCOPE],
            "bad_genreation": 123
        });
        let mock_verify = mockito::mock("POST", "/v1/verify")
            .with_header("content-type", "application/json")
            .with_status(200)
            // Even though the body is valid json, it doesn't match our expectation so we'll error
            // out
            .with_body(body.to_string())
            .create();
        let settings = Settings {
            fxa_oauth_server_url: mockito::server_url(),
            ..Settings::default()
        };

        mock_jwk_verifier!(Err(OAuthVerifyError::DecodingError));
        let jwk_verifiers = vec![];

        let verifier: Verifier<MockJWTVerifier> = Verifier::new(&settings, jwk_verifiers).unwrap();

        let err = verifier
            .verify(
                "A token that we will ask FxA about".to_string(),
                &Default::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(err.status, "invalid-credentials");
        assert_eq!(err.http_status, 401);
        assert_eq!(err.description, "Unauthorized");
        mock_verify.expect(1);

        Ok(())
    }
}
