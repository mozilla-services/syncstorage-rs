use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::Error;
use actix_web::{HttpRequest, HttpResponse};
use hmac::{Hmac, Mac, NewMac};
use serde::Serialize;
use sha2::Sha256;

use super::db::{self, params::GetUser};
use super::extractors::TokenserverRequest;
use super::support::Tokenlib;
use crate::tokenserver::support::MakeTokenPlaintext;
use crate::{
    error::{ApiError, ApiErrorKind},
    server::ServerState,
};

#[derive(Debug, Serialize)]
pub struct TokenserverResult {
    id: String,
    key: String,
    uid: String,
    api_endpoint: String,
    duration: u64,
}

pub async fn get_tokenserver_result(
    tokenserver_request: TokenserverRequest,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let state = request
        .app_data::<Data<ServerState>>()
        .ok_or_else(|| internal_error("Could not load the app state"))?;
    let tokenserver_state = state.tokenserver_state.as_ref().unwrap();
    let db = {
        let db_pool = tokenserver_state.db_pool.clone();
        db_pool.get().map_err(ApiError::from)?
    };

    let user_email = format!(
        "{}@{}",
        tokenserver_request.fxa_uid, tokenserver_state.fxa_email_domain
    );
    let tokenserver_user = {
        let params = GetUser {
            email: user_email.clone(),
            service_id: db::SYNC_1_5_SERVICE_ID,
        };

        db.get_user(params).await?
    };

    let fxa_metrics_hash_secret = tokenserver_state
        .fxa_metrics_hash_secret
        .clone()
        .into_bytes();

    let hashed_fxa_uid_full =
        fxa_metrics_hash(&tokenserver_request.fxa_uid, &fxa_metrics_hash_secret)?;
    let hashed_fxa_uid = &hashed_fxa_uid_full[0..32];
    let hashed_device_id = {
        let device_id = "none".to_string();
        hash_device_id(hashed_fxa_uid, &device_id, &fxa_metrics_hash_secret)?
    };

    let fxa_kid = {
        let client_state_b64 =
            base64::encode_config(&tokenserver_user.client_state, base64::URL_SAFE_NO_PAD);

        format!(
            "{:013}-{:}",
            tokenserver_user
                .keys_changed_at
                .unwrap_or(tokenserver_request.generation),
            client_state_b64
        )
    };

    let (token, derived_secret) = {
        let shared_secret = String::from_utf8(state.secrets.master_secret.clone())
            .map_err(|_| internal_error("Failed to read master secret"))?;

        let make_token_plaintext = {
            let expires = {
                let start = SystemTime::now();
                let current_time = start.duration_since(UNIX_EPOCH).unwrap();
                let expires = current_time + Duration::new(tokenserver_request.duration, 0);

                expires.as_secs()
            };

            MakeTokenPlaintext {
                node: tokenserver_user.node.clone(),
                fxa_kid,
                fxa_uid: tokenserver_request.fxa_uid.clone(),
                hashed_device_id,
                hashed_fxa_uid: hashed_fxa_uid.to_owned(),
                expires,
                uid: tokenserver_user.uid,
            }
        };

        Tokenlib::get_token_and_derived_secret(make_token_plaintext, &shared_secret)?
    };

    let api_endpoint = format!("{:}/1.5/{:}", tokenserver_user.node, tokenserver_user.uid);

    let result = TokenserverResult {
        id: token,
        key: derived_secret,
        uid: tokenserver_request.fxa_uid,
        api_endpoint,
        duration: tokenserver_request.duration,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

fn fxa_metrics_hash(fxa_uid: &str, hmac_key: &[u8]) -> Result<String, Error> {
    let mut mac = Hmac::<Sha256>::new_from_slice(hmac_key)
        .map_err::<ApiError, _>(|err| ApiErrorKind::Internal(err.to_string()).into())?;
    mac.update(fxa_uid.as_bytes());

    let result = mac.finalize().into_bytes();
    Ok(hex::encode(result))
}

fn hash_device_id(fxa_uid: &str, device: &str, hmac_key: &[u8]) -> Result<String, Error> {
    let mut to_hash = String::from(fxa_uid);
    to_hash.push_str(device);
    let fxa_metrics_hash = fxa_metrics_hash(&to_hash, hmac_key)?;

    Ok(String::from(&fxa_metrics_hash[0..32]))
}

fn internal_error(message: &str) -> HttpResponse {
    error!("⚠️ {}", message);

    HttpResponse::InternalServerError().body("")
}
