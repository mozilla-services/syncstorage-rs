//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

use actix_web::{dev::Payload, web::Data, Error, FromRequest, HttpRequest};
use actix_web_httpauth::extractors::bearer::BearerAuth;

use futures::future::LocalBoxFuture;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

use crate::server::ServerState;
use crate::web::error::ValidationErrorKind;
use crate::web::extractors::RequestErrorLocation;

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}
pub struct TokenserverRequest {
    pub fxa_uid: String,
}

impl TokenserverRequest {
    fn get_fxa_uid(jwt: &str, rsa_modulus: String, rsa_exponent: String) -> Result<String, Error> {
        decode::<Claims>(
            &jwt,
            &DecodingKey::from_rsa_components(&rsa_modulus, &rsa_exponent),
            &Validation::new(Algorithm::RS256),
        )
        .map(|token_data| token_data.claims.sub)
        .map_err(|e| {
            ValidationErrorKind::FromDetails(
                format!("Unable to decode token JWT: {:?}", e),
                RequestErrorLocation::Header,
                Some("Bearer".to_owned()),
                label!("request.error.invalid_bearer_auth"),
            )
            .into()
        })
    }
}

/// Extracts data from the JWT in the Authorization header
impl FromRequest for TokenserverRequest {
    type Config = ();
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let mut payload = payload.take();

        Box::pin(async move {
            let state = match req.app_data::<Data<ServerState>>() {
                Some(s) => s,
                None => {
                    error!("⚠️ Could not load the app state");
                    return Err(ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                    .into());
                }
            };
            let auth = BearerAuth::from_request(&req, &mut payload).await?;
            let fxa_uid = {
                let rsa_modulus = state.tokenserver_jwks_rsa_modulus.clone().ok_or_else(|| {
                    error!("⚠️ Tokenserver JWK RSA modulus not set");
                    ValidationErrorKind::FromDetails(
                        "Internal error".to_owned(),
                        RequestErrorLocation::Unknown,
                        Some("app_data".to_owned()),
                        None,
                    )
                })?;
                let rsa_exponent =
                    state.tokenserver_jwks_rsa_exponent.clone().ok_or_else(|| {
                        error!("⚠️ Tokenserver JWK RSA exponent not set");
                        ValidationErrorKind::FromDetails(
                            "Internal error".to_owned(),
                            RequestErrorLocation::Unknown,
                            Some("app_data".to_owned()),
                            None,
                        )
                    })?;
                Self::get_fxa_uid(auth.token(), rsa_modulus, rsa_exponent)?
            };

            Ok(Self { fxa_uid })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{http::Method, test::TestRequest, HttpResponse};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use lazy_static::lazy_static;
    use tokio::sync::RwLock;

    use crate::db::mock::MockDbPool;
    use crate::server::{metrics, ServerState};
    use crate::settings::{Deadman, Secrets, ServerLimits, Settings};

    use std::sync::Arc;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    lazy_static! {
        static ref SECRETS: Arc<Secrets> = Arc::new(Secrets::new("Ted Koppel is a robot").unwrap());
        static ref SERVER_LIMITS: Arc<ServerLimits> = Arc::new(ServerLimits::default());
    }

    const RSA_PRIVATE_KEY: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
    MIIEowIBAAKCAQEArRWWL6xF8f34ykDqFLg6O2ehmRqHonEWeruYJ8i4OPn5DwAj\n\
    fCaCNu/A/6JCUEtNJXZ6CwVub0a6kDENdW9vkzGJPfz3EjvzxbSTCekiDrXYHFRn\n\
    hNhgXDoeOE4NQ0Ob69BdDc7Zwyu+pIgTvCjDsuZiDm+bZdzwgWspK/Wn1qCfdkRo\n\
    J0AV81pWUtcyRBJpQ/3hM9BbwBAWpjXNDaHHxvp/lUyJY8dbw1YxHSQ3eoNPmRz3\n\
    ioSU1x7zDcWJzZ/RowrFqqBku+UQakxp7kq72Bv1kHcD4Cye2366sh9aLQjR6o87\n\
    b1owiv382qaRqT0/gJP7lVRGpRnVs0orV2SxjQIDAQABAoIBAA8xmup6a/VvPvy6\n\
    MBI7jdkTIstm2cs3tCp390Zex1UxFFztvS+zzbB24XFPVBTqV05XlSUMiAI6qjvo\n\
    Im9RpfC843hOkX3HR4HudQ3kqjmyWtM50ZCG0gamj2qP53glIjXUJ6cDpngMigK1\n\
    c04MIgm9UZRE1dZeS7qApq+WM/KSKBg6CtiJAU0UcXAXsrNqv315QUhqVWjnvti5\n\
    gt+U/5oaE2J7/WYfYUC44+OqS0ItuwWToKiv1w6wwY7cNEVr0Su3aDiMa1X/m2/2\n\
    Ykn0dPHpQpTMbpeJMFBki7Iah9Gn8XMB0OsMAd4DjGkfkn1dKDw7oewqQbFbmGx4\n\
    6131ON0CgYEA0/uC/FHr7N251oAUA9jlJTimMNS/zBUEpIhNQTAd86T9uKwLrUWK\n\
    KSY50ubYBwhULrZrXQBMjavokAi4WRmvK9SxxccgmDDFOpdB3DzN1wbM2r5qay2/\n\
    guchlpM1H/D9ceLm6IZRs9KGxPV+eydmrXTSnM3fzvLHk+hmM1geH8sCgYEA0QZX\n\
    YMWgncF92z5xW5Rcy1q9PtNmoQ5yj1TZH40+zaGrAvWQEojpb04enDfkMxhMLVTZ\n\
    G527Q/mEEfXjWxUIKTse7olFsGbcT8T81jX4pg7uKkJGHZ2Q7+ttBev/onx9JzUf\n\
    ieqjb1NYt8xqiptOmdDYXnoFAU2bu9lWVkuFGQcCgYEAxhpgGOl+L8guahUbn1TN\n\
    IHHGbhAEhfaGdjSi7e7HrvBb5H90EiPQsA/3Le9pp3jTIyx7PViQMj2bgy+DCFGG\n\
    cNG+qPQks9WwG8dLV0TDoNXMEAivbyY7uVvC+fLsTMNsN0gzPs54ADMYm2xJHVJ/\n\
    FE7+nGeRZtdgSAuBpy4MSO0CgYBEyUByATdVEvrW7pqhV5ad+TNz/F+2uqlqj7KQ\n\
    FoxHYV+ErskFwHaJgXzDTgVT5zgSZuy3kNWyjecvfeqe67Hu15zbRONhJMh1m87U\n\
    s5grFZi84WhvkI3E1oXfQAW1NCB/iZTibwvvs87rVWLuUCOyrK63kJIbFq4cSG6I\n\
    IXwgewKBgBV63Cd87I2hb+IIFwIjDmGw4aqa16fJB25GCWYDL3Annxe3JKi0UpJU\n\
    ejg5O4GsIRARaOFzZJ2Lzcwv+C/RMyJKcrXVsflSrSFRswlXVDCoNLBpoX6FqAvh\n\
    qQFiwEtArcfLQEC1hLaq2sWcaZ/zPVGu7wl7hSSaZa997fYiHQkt\n\
    -----END RSA PRIVATE KEY-----";
    const RSA_MODULUS: &str = "AK0Vli-sRfH9-MpA6hS4OjtnoZkah6JxFnq7mCfIuDj5-Q8AI3wmgjbvwP-iQlBLTSV2egsFbm9GupAxDXVvb5MxiT389xI788W0kwnpIg612BxUZ4TYYFw6HjhODUNDm-vQXQ3O2cMrvqSIE7wow7LmYg5vm2Xc8IFrKSv1p9agn3ZEaCdAFfNaVlLXMkQSaUP94TPQW8AQFqY1zQ2hx8b6f5VMiWPHW8NWMR0kN3qDT5kc94qElNce8w3Fic2f0aMKxaqgZLvlEGpMae5Ku9gb9ZB3A-Asntt-urIfWi0I0eqPO29aMIr9_Nqmkak9P4CT-5VURqUZ1bNKK1dksY0";
    const RSA_PUBLIC_EXPONENT: &str = "AQAB";
    const SECONDS_IN_A_YEAR: u64 = 60 * 60 * 24 * 365;

    #[actix_rt::test]
    async fn test_valid_tokenserver_request() {
        let state = make_state();
        let uri = "/1.0/sync/1.5";
        let fxa_uid = "test123";
        let bearer_token = {
            let fxa_uid = "test123";
            let start = SystemTime::now();
            let current_time = start.duration_since(UNIX_EPOCH).unwrap();
            let exp_duration = current_time + Duration::new(SECONDS_IN_A_YEAR, 0);
            let claims = Claims {
                sub: fxa_uid.to_owned(),
                iat: current_time.as_secs() as i64,
                exp: exp_duration.as_secs() as i64,
            };

            encode::<Claims>(
                &Header::new(Algorithm::RS256),
                &claims,
                &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
            )
            .unwrap()
        };

        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", format!("Bearer {}", bearer_token))
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload)
            .await
            .unwrap();

        assert_eq!(result.fxa_uid, fxa_uid);
    }

    #[actix_rt::test]
    async fn test_invalid_tokenserver_request() {
        let state = make_state();
        let uri = "/1.0/sync/1.5";
        let bearer_token = "I am not a valid token";

        let req = TestRequest::with_uri(&uri)
            .data(state)
            .header("authorization", format!("Bearer {}", bearer_token))
            .header("accept", "application/json,text/plain:q=0.5")
            .method(Method::GET)
            .to_http_request();

        let mut payload = Payload::None;
        let result = TokenserverRequest::from_request(&req, &mut payload).await;
        assert!(result.is_err());

        let response: HttpResponse = result.err().unwrap().into();
        assert_eq!(response.status(), 400);
    }

    fn make_state() -> ServerState {
        let settings = Settings::default();
        ServerState {
            db_pool: Box::new(MockDbPool::new()),
            limits: Arc::clone(&SERVER_LIMITS),
            limits_json: serde_json::to_string(&**SERVER_LIMITS).unwrap(),
            secrets: Arc::clone(&SECRETS),
            tokenserver_database_url: None,
            tokenserver_jwks_rsa_modulus: Some(RSA_MODULUS.to_owned()),
            tokenserver_jwks_rsa_exponent: Some(RSA_PUBLIC_EXPONENT.to_owned()),
            fxa_metrics_hash_secret: None,
            port: 8000,
            metrics: Box::new(metrics::metrics_from_opts(&settings).unwrap()),
            quota_enabled: settings.enable_quota,
            deadman: Arc::new(RwLock::new(Deadman::default())),
        }
    }
}
