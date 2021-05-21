use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_web::http::StatusCode;
use actix_web::web::Data;
use actix_web::Error;
use actix_web::{HttpRequest, HttpResponse};
use hmac::{Hmac, Mac, NewMac};
use pyo3::prelude::*;
use pyo3::types::{IntoPyDict, PyDict};
use sha2::Sha256;

use super::db::models::get_tokenserver_user_sync;
use super::extractors::TokenserverRequest;
use crate::{error::ApiError, server::ServerState};

const DEFAULT_TOKEN_DURATION: u64 = 5 * 60;
const FXA_EMAIL_DOMAIN: &str = "api.accounts.firefox.com";

pub struct Tokenlib<'a> {
    py: Python<'a>,
    inner: &'a PyModule,
}

impl<'a> Tokenlib<'a> {
    pub fn new(py: Python<'a>) -> Result<Self, PyErr> {
        let inner = PyModule::import(py, "tokenlib").map_err(|e| {
            e.print_and_set_sys_last_vars(py);
            e
        })?;

        Ok(Self { py, inner })
    }

    pub fn make_token(&self, plaintext: &PyDict, shared_secret: &str) -> Result<String, PyErr> {
        let kwargs = PyDict::new(self.py);
        kwargs.set_item("secret", shared_secret)?;

        match self.inner.call("make_token", (plaintext,), Some(kwargs)) {
            Err(e) => {
                e.print_and_set_sys_last_vars(self.py);
                Err(e)
            }
            Ok(x) => Ok(x.extract::<String>().unwrap()),
        }
    }

    pub fn get_derived_secret(
        &self,
        plaintext: &str,
        shared_secret: &str,
    ) -> Result<String, PyErr> {
        let kwargs = PyDict::new(self.py);
        kwargs.set_item("secret", shared_secret)?;

        match self
            .inner
            .call("get_derived_secret", (plaintext,), Some(kwargs))
        {
            Err(e) => {
                e.print_and_set_sys_last_vars(self.py);
                Err(e)
            }
            Ok(x) => Ok(x.extract::<String>().unwrap()),
        }
    }
}

#[derive(serde::Serialize)]
pub struct TokenserverResult {
    id: String,
    key: String,
    uid: String,
    api_endpoint: String,
    duration: String,
}

// TODO can we split up this function to make it shorter and more easily digestible?
pub async fn get_tokenserver_result(
    tokenserver_request: TokenserverRequest,
    request: HttpRequest,
) -> Result<HttpResponse, Error> {
    let state = request
        .app_data::<Data<ServerState>>()
        .ok_or_else(|| internal_error("Could not load the app state"))?;
    let user_email = format!("{}@{}", tokenserver_request.fxa_uid, FXA_EMAIL_DOMAIN);
    let tokenserver_user = {
        let database_url = state
            .tokenserver_database_url
            .clone()
            .ok_or_else(|| internal_error("Could not load the app state"))?;
        get_tokenserver_user_sync(&user_email, &database_url).map_err(ApiError::from)?
    };

    // Update generation and keys_changed_at


    // TODO: maybe keeping local vars scoped like this is an improvement?
    // though I still find it difficult to follow what's going on
    // Build the token and the derived secret
    let (token, derived_secret) = {
        let fxa_metrics_hash_secret = state
            .fxa_metrics_hash_secret
            .clone()
            .ok_or_else(|| internal_error("Failed to read FxA metrics hash secret"))?
            .into_bytes();

        let hashed_fxa_uid = fxa_metrics_hash(&user_email, &fxa_metrics_hash_secret);
        let hashed_device_id = {
            let device_id = "none".to_string();
            hash_device_id(&hashed_fxa_uid, &device_id, &fxa_metrics_hash_secret)
        };

        let fxa_kid = {
            let client_state_b64 =
                base64::encode_config(&tokenserver_user.client_state, base64::URL_SAFE_NO_PAD);

            format!(
                "{:013}-{:}",
                tokenserver_user.keys_changed_at.unwrap_or(0),
                client_state_b64
            )
        };

        let shared_secret = String::from_utf8(state.secrets.master_secret.clone())
            .map_err(|_| internal_error("Failed to read master secret"))?;
        Python::with_gil(|py| -> Result<(String, String), PyErr> {
            let dict = [
                ("node", &tokenserver_user.node),
                ("fxa_kid", &fxa_kid),
                ("fxa_uid", &tokenserver_request.fxa_uid),
                ("hashed_device_id", &hashed_device_id),
                ("hashed_fxa_uid", &hashed_fxa_uid),
            ]
            .into_py_dict(py);

            let expires = {
                let start = SystemTime::now();
                let current_time = start.duration_since(UNIX_EPOCH).unwrap();
                current_time + Duration::new(DEFAULT_TOKEN_DURATION, 0)
            };

            // These need to be set separately since they aren't strings, and
            // Rust doesn't support heterogeneous arrays
            dict.set_item("expires", expires.as_secs_f64()).unwrap();
            dict.set_item("uid", tokenserver_user.uid).unwrap();

            let tokenlib = Tokenlib::new(py)?;
            let token = tokenlib.make_token(dict, &shared_secret)?;
            let derived_secret = tokenlib.get_derived_secret(&token, &shared_secret)?;
            Ok((token, derived_secret))
        })
        .unwrap()
    };

    let api_endpoint = format!("{:}/1.5/{:}", tokenserver_user.node, tokenserver_user.uid);

    let result = TokenserverResult {
        id: token,
        key: derived_secret,
        uid: tokenserver_request.fxa_uid,
        api_endpoint,
        duration: DEFAULT_TOKEN_DURATION.to_string(),
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

fn fxa_metrics_hash(value: &str, hmac_key: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(hmac_key).unwrap();
    let v = value.split('@').next().unwrap();
    mac.update(v.as_bytes());

    let result = mac.finalize().into_bytes();
    hex::encode(result)
}

fn hash_device_id(fxa_uid: &str, device: &str, hmac_key: &[u8]) -> String {
    let mut to_hash = String::from(&fxa_uid[0..32]);
    to_hash.push_str(device);

    String::from(&fxa_metrics_hash(&to_hash, hmac_key)[0..32])
}

fn internal_error(message: &str) -> HttpResponse {
    error!("⚠️ {}", message);

    HttpResponse::InternalServerError().body("")
}
