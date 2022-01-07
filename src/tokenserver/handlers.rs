use std::{
    cmp,
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use actix_web::{http::StatusCode, Error, HttpResponse};
use serde::Serialize;
use serde_json::Value;

use super::db::models::Db;
use super::db::params::{GetNodeId, PostUser, PutUser, ReplaceUsers};
use super::error::TokenserverError;
use super::extractors::TokenserverRequest;
use super::support::{self, Tokenlib};
use super::NodeType;
use crate::server::metrics::Metrics;
use crate::tokenserver::support::MakeTokenPlaintext;

#[derive(Debug, Serialize)]
pub struct TokenserverResult {
    id: String,
    key: String,
    uid: i64,
    api_endpoint: String,
    duration: u64,
    hashed_fxa_uid: String,
    hashalg: &'static str,
    node_type: NodeType,
}

pub async fn get_tokenserver_result(
    req: TokenserverRequest,
    db: Box<dyn Db>,
    mut metrics: Metrics,
) -> Result<HttpResponse, Error> {
    let updates = update_user(&req, db).await?;

    let (token, derived_secret) = {
        // Get the plaintext that will be used to derive the token and secret to be returned to
        // the client
        let token_plaintext = get_token_plaintext(&req, &updates)?;

        // Derive the node-specific secret that will be used to derive the token and secret to be
        // returned to the client
        let secrets = {
            metrics.start_timer("tokenserver.node_secret_derivation", None);

            support::derive_node_secrets(vec![&hex::encode(req.shared_secret)], &req.user.node)
                .map_err(|_| {
                    error!("⚠️ Failed to derive node secret");

                    TokenserverError::internal_error()
                })?
        };

        metrics.start_timer("tokenserver.token_creation", None);
        // Get the token and secret
        Tokenlib::get_token_and_derived_secret(token_plaintext, &secrets[secrets.len() - 1])?
    };

    let result = TokenserverResult {
        id: token,
        key: derived_secret,
        uid: updates.uid,
        api_endpoint: format!("{:}/1.5/{:}", req.user.node, req.user.uid),
        duration: req.duration,
        hashed_fxa_uid: req.hashed_fxa_uid,
        hashalg: "sha256",
        node_type: req.node_type,
    };

    Ok(HttpResponse::build(StatusCode::OK).json(result))
}

fn get_token_plaintext(
    req: &TokenserverRequest,
    updates: &UserUpdates,
) -> Result<MakeTokenPlaintext, TokenserverError> {
    let fxa_kid = {
        // If decoding the hex bytes fails, it means we did something wrong when we stored the
        // client state in the database
        let client_state = hex::decode(req.client_state.clone()).map_err(|_| {
            error!("⚠️ Failed to decode client state hex");

            TokenserverError::internal_error()
        })?;
        let client_state_b64 = base64::encode_config(&client_state, base64::URL_SAFE_NO_PAD);

        format!("{:013}-{:}", updates.keys_changed_at, client_state_b64)
    };

    let expires = {
        let start = SystemTime::now();
        let current_time = start.duration_since(UNIX_EPOCH).unwrap();
        let expires = current_time + Duration::from_secs(req.duration);

        expires.as_secs()
    };

    Ok(MakeTokenPlaintext {
        node: req.user.node.to_owned(),
        fxa_kid,
        fxa_uid: req.fxa_uid.clone(),
        hashed_device_id: req.hashed_device_id.clone(),
        hashed_fxa_uid: req.hashed_fxa_uid.clone(),
        expires,
        uid: updates.uid.to_owned(),
    })
}

struct UserUpdates {
    keys_changed_at: i64,
    uid: i64,
}

async fn update_user(req: &TokenserverRequest, db: Box<dyn Db>) -> Result<UserUpdates, Error> {
    // If the keys_changed_at in the request is larger than that stored on the user record,
    // update to the value in the request.
    let keys_changed_at = if let Some(user_keys_changed_at) = req.user.keys_changed_at {
        cmp::max(req.keys_changed_at, user_keys_changed_at)
    } else {
        req.keys_changed_at
    };

    let generation = if let Some(generation) = req.generation {
        // If there's a generation on the request, choose the larger of that and the generation
        // already stored on the user record.
        cmp::max(generation, req.user.generation)
    } else if req.keys_changed_at > req.user.generation {
        // If there's not a generation on the request and the keys_changed_at on the request is
        // larger than the generation stored on the user record, set the user's generation to be
        // the keys_changed_at on the request.
        req.keys_changed_at
    } else {
        // As a fallback, set the user's generation to be 0.
        0
    };

    // If the client state changed, we need to mark the current user as "replaced" and create a
    // new user record. Otherwise, we can update the user in place.
    if req.client_state != req.user.client_state {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Create new user record with updated generation/keys_changed_at
        let post_user_params = PostUser {
            service_id: req.service_id,
            email: req.email.clone(),
            generation,
            client_state: req.client_state.clone(),
            node_id: db
                .get_node_id(GetNodeId {
                    service_id: req.service_id,
                    node: req.user.node.clone(),
                })
                .await?
                .id,
            keys_changed_at: Some(keys_changed_at),
            created_at: timestamp,
        };
        let uid = db.post_user(post_user_params).await?.id;

        // Make sure each old row is marked as replaced (they might not be, due to races in row
        // creation)
        db.replace_users(ReplaceUsers {
            email: req.email.clone(),
            service_id: req.service_id,
            replaced_at: timestamp,
        })
        .await?;

        Ok(UserUpdates {
            keys_changed_at,
            uid,
        })
    } else {
        let params = PutUser {
            email: req.email.clone(),
            service_id: req.service_id,
            generation,
            keys_changed_at: Some(keys_changed_at),
        };

        db.put_user(params).await?;

        Ok(UserUpdates {
            keys_changed_at,
            uid: req.user.uid,
        })
    }
}

pub async fn heartbeat(db: Box<dyn Db>) -> Result<HttpResponse, Error> {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );

    match db.check().await {
        Ok(result) => {
            if result {
                checklist.insert("database".to_owned(), Value::from("Ok"));
            } else {
                checklist.insert("database".to_owned(), Value::from("Err"));
                checklist.insert(
                    "database_msg".to_owned(),
                    Value::from("check failed without error"),
                );
            };
            let status = if result { "Ok" } else { "Err" };
            checklist.insert("status".to_owned(), Value::from(status));
            Ok(HttpResponse::Ok().json(checklist))
        }
        Err(e) => {
            error!("Heartbeat error: {:?}", e);
            checklist.insert("status".to_owned(), Value::from("Err"));
            checklist.insert("database".to_owned(), Value::from("Unknown"));
            Ok(HttpResponse::ServiceUnavailable().json(checklist))
        }
    }
}
