use std::borrow::Cow;

use super::VerifyToken;
use async_trait::async_trait;
use jsonwebtoken::{errors::ErrorKind, jwk::Jwk, Algorithm, DecodingKey, Validation};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use tokenserver_common::TokenserverError;
use tokenserver_settings::Settings;

const SYNC_SCOPE: &str = "https://identity.mozilla.com/apps/oldsync";

#[derive(Deserialize, Debug)]
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

#[derive(Debug, thiserror::Error)]
enum OAuthVerifyError {
    #[error("Untrusted token")]
    TrustError,
    #[error("Invalid Key")]
    InvalidKey,
    #[error("Error decoding JWT")]
    DecodingError,
    #[error("No keys were provided")]
    NoKeys,
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
pub struct Verifier {
    server_url: Url,
    jwks: Vec<Jwk>,
    validation: Validation,
}

impl Verifier {
    pub fn new(settings: &Settings) -> Result<Self, TokenserverError> {
        let mut validation = Validation::new(Algorithm::RS256);
        // The FxA OAuth ecosystem currently doesn't make good use of aud, and
        // instead relies on scope for restricting which services can accept
        // which tokens. So there's no value in checking it here, and in fact if
        // we check it here, it fails because the right audience isn't being
        // requested.
        validation.validate_aud = false;
        let mut jwks = Vec::new();
        if let Some(primary) = &settings.fxa_oauth_primary_jwk {
            jwks.push(primary.clone())
        }
        if let Some(secondary) = &settings.fxa_oauth_secondary_jwk {
            jwks.push(secondary.clone())
        }
        Ok(Self {
            server_url: Url::parse(&settings.fxa_oauth_server_url)
                .map_err(|_| TokenserverError::internal_error())?,
            jwks,
            validation,
        })
    }

    async fn remote_verify_token(&self, token: &str) -> Result<TokenClaims, TokenserverError> {
        #[derive(Serialize)]
        struct VerifyRequest<'a> {
            token: &'a str,
        }

        #[derive(Deserialize)]
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
            .map_err(|_| TokenserverError::invalid_credentials("Unauthorized".to_string()))?
            .json::<VerifyResponse>()
            .await
            .map_err(|_| TokenserverError::invalid_credentials("Unauthorized".to_string()))?
            .into())
    }

    async fn get_remote_jwks(&self) -> Result<Vec<Jwk>, TokenserverError> {
        #[derive(Deserialize)]
        struct KeysResponse {
            keys: Vec<Jwk>,
        }
        let client = reqwest::Client::new();
        let url = self
            .server_url
            .join("v1/jwks")
            .map_err(|_| TokenserverError::internal_error())?;
        Ok(client
            .get(url)
            .send()
            .await
            .map_err(|_| TokenserverError::internal_error())?
            .json::<KeysResponse>()
            .await
            .map_err(|_| TokenserverError::internal_error())?
            .keys)
    }

    fn verify_jwt_locally(
        &self,
        keys: &[Cow<'_, Jwk>],
        token: &str,
    ) -> Result<TokenClaims, OAuthVerifyError> {
        if keys.is_empty() {
            return Err(OAuthVerifyError::NoKeys);
        }

        for key in keys {
            let decoding_key =
                DecodingKey::from_jwk(key).map_err(|_| OAuthVerifyError::InvalidKey)?;

            let token_data =
                match jsonwebtoken::decode::<TokenClaims>(token, &decoding_key, &self.validation) {
                    Ok(res) => res,
                    Err(e) => match e.kind() {
                        // Invalid signature, lets try the other keys if we have any
                        ErrorKind::InvalidSignature => continue,

                        // If the signature is expired, we return right away, hitting the FxA
                        // server won't change anything
                        ErrorKind::ExpiredSignature => return Err(OAuthVerifyError::TrustError),
                        // If the token shape is invalid, and weren't able to decode it into the shape we think the tokens should have,
                        // we want to be able to fallback to asking FxA to
                        // verify
                        _ => return Err(OAuthVerifyError::DecodingError),
                    },
                };
            token_data
                .header
                .typ
                .ok_or(OAuthVerifyError::TrustError)
                .and_then(|typ| {
                    // Ref https://tools.ietf.org/html/rfc7515#section-4.1.9 the `typ` header
                    // is lowercase and has an implicit default `application/` prefix.
                    let typ = if !typ.contains('/') {
                        format!("application/{}", typ)
                    } else {
                        typ
                    };
                    if typ.to_lowercase() != "application/at+jwt" {
                        return Err(OAuthVerifyError::TrustError);
                    }
                    Ok(typ)
                })?;
            return Ok(token_data.claims);
        }
        // All the keys were well formatted, but we weren't able to verify the token
        // we return a TrustError
        Err(OAuthVerifyError::TrustError)
    }
}

#[async_trait]
impl VerifyToken for Verifier {
    type Output = VerifyOutput;

    /// Verifies an OAuth token. Returns `VerifyOutput` for valid tokens and a `TokenserverError`
    /// for invalid tokens.
    async fn verify(&self, token: String) -> Result<VerifyOutput, TokenserverError> {
        let mut keys: Vec<Cow<'_, Jwk>> = self.jwks.iter().map(Cow::Borrowed).collect();
        if keys.is_empty() {
            keys = self
                .get_remote_jwks()
                .await
                .unwrap_or_else(|_| vec![])
                .into_iter()
                .map(Cow::Owned)
                .collect();
        }

        let claims = match self.verify_jwt_locally(&keys, &token) {
            Ok(res) => res,
            Err(OAuthVerifyError::DecodingError)
            | Err(OAuthVerifyError::InvalidKey)
            | Err(OAuthVerifyError::NoKeys) => self.remote_verify_token(&token).await?,
            _ => {
                return Err(TokenserverError::invalid_credentials(
                    "Unauthorized".to_string(),
                ))
            }
        };
        claims.validate()
    }
}
