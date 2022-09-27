use std::{
    collections::HashMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use actix_web::{http::StatusCode, Error, HttpResponse};
use serde::Serialize;
use serde_json::Value;
use tokenserver_common::{NodeType, TokenserverError};
use tokenserver_db::{
    params::{GetNodeId, PostUser, PutUser, ReplaceUsers},
    DbTrait,
};

use super::{
    auth::{MakeTokenPlaintext, Tokenlib, TokenserverOrigin},
    extractors::{DbWrapper, TokenserverRequest},
    TokenserverMetrics,
};

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
    DbWrapper(db): DbWrapper,
    TokenserverMetrics(mut metrics): TokenserverMetrics,
) -> Result<HttpResponse, TokenserverError> {
    let updates = update_user(&req, db).await?;

    let (token, derived_secret) = {
        let token_plaintext = get_token_plaintext(&req, &updates)?;

        metrics.start_timer("token_creation", None);
        // Get the token and secret
        Tokenlib::get_token_and_derived_secret(token_plaintext, &req.shared_secret)?
    };

    let result = TokenserverResult {
        id: token,
        key: derived_secret,
        uid: updates.uid,
        api_endpoint: format!("{:}/1.5/{:}", req.user.node, updates.uid),
        duration: req.duration,
        hashed_fxa_uid: req.hashed_fxa_uid,
        hashalg: "sha256",
        node_type: req.node_type,
    };

    let timestamp = {
        let start = SystemTime::now();
        start.duration_since(UNIX_EPOCH).unwrap().as_secs()
    };

    Ok(HttpResponse::build(StatusCode::OK)
        .header("X-Timestamp", timestamp.to_string())
        // This header helps to prevent cross-site scripting attacks by
        // blocking content type sniffing. It was set automatically by the
        // Pyramid cornice library used by the Python Tokenserver, so we set
        // it here for safety and consistency.
        .header("X-Content-Type-Options", "nosniff")
        .json(result))
}

fn get_token_plaintext(
    req: &TokenserverRequest,
    updates: &UserUpdates,
) -> Result<MakeTokenPlaintext, TokenserverError> {
    let fxa_kid = {
        // If decoding the hex bytes fails, it means we did something wrong when we stored the
        // client state in the database
        let client_state =
            hex::decode(req.auth_data.client_state.clone()).map_err(|e| TokenserverError {
                context: format!("Failed to decode the client state hex: {}", e),
                ..TokenserverError::internal_error()
            })?;
        let client_state_b64 = base64::encode_config(&client_state, base64::URL_SAFE_NO_PAD);

        format!(
            "{:013}-{:}",
            // We fall back to using the user's generation here, which matches FxA's behavior
            updates.keys_changed_at.unwrap_or(updates.generation),
            client_state_b64
        )
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
        fxa_uid: req.auth_data.fxa_uid.clone(),
        hashed_device_id: req.hashed_device_id.clone(),
        hashed_fxa_uid: req.hashed_fxa_uid.clone(),
        expires,
        uid: updates.uid.to_owned(),
        tokenserver_origin: TokenserverOrigin::Rust,
    })
}

struct UserUpdates {
    keys_changed_at: Option<i64>,
    generation: i64,
    uid: i64,
}

async fn update_user(
    req: &TokenserverRequest,
    db: Box<dyn DbTrait>,
) -> Result<UserUpdates, TokenserverError> {
    let keys_changed_at = match (req.auth_data.keys_changed_at, req.user.keys_changed_at) {
        // If the keys_changed_at in the request is larger than that stored on the user record,
        // update to the value in the request.
        (Some(request_keys_changed_at), Some(user_keys_changed_at))
            if request_keys_changed_at >= user_keys_changed_at =>
        {
            Some(request_keys_changed_at)
        }
        // If there is a keys_changed_at in the request and it's smaller than that stored on the
        // user record, we've already returned an error at this point.
        (Some(_request_keys_changed_at), Some(_user_keys_changed_at)) => unreachable!(),
        // If there is a keys_changed_at on the request but not one on the user record, this is the
        // first time the client reported it, so we assign the new value.
        (Some(request_keys_changed_at), None) => Some(request_keys_changed_at),
        // At this point, we've already validated that, if there is a keys_changed_at already
        // stored on the user record, there must be one in the request. If that isn't the case,
        // we've already returned an error.
        (None, Some(user_keys_changed_at)) if user_keys_changed_at != 0 => unreachable!(),
        // If there's no keys_changed_at in the request and the keys_changed_at on the user record
        // is 0, keep the value as 0.
        (None, Some(_user_keys_changed_at)) => Some(0),
        // If there is no keys_changed_at on the user record or in the request, we want to leave
        // the value unset.
        (None, None) => None,
    };

    let generation = match req.auth_data.generation {
        // If there's a generation in the request and it's greater than or equal to that stored on
        // the user record, update to the value in the request.
        Some(request_generation) if request_generation >= req.user.generation => request_generation,
        // If there's a generation in the request and it's smaller than that stored on the user
        // record, we've already returned an error.
        Some(_request_generation) => unreachable!(),
        None => match (req.auth_data.keys_changed_at, req.user.keys_changed_at) {
            // If there's not a generation on the request but the keys_changed_at on the request
            // is greater than the user's current generation AND the keys_changed_at on the request
            // is greater than the user's current keys_changed_at, set the user's generation to
            // the new keys_changed_at.
            (Some(request_keys_changed_at), Some(user_keys_changed_at))
                if request_keys_changed_at > user_keys_changed_at
                    && request_keys_changed_at > req.user.generation =>
            {
                request_keys_changed_at
            }
            // If there's not a generation on the request but the keys_changed_at on the request
            // is greater than the user's current generation AND there is a keys_changed_at on the
            // request but not currently on the user record, set the user's generation to the new
            // keys_changed_at.
            (Some(request_keys_changed_at), None)
                if request_keys_changed_at > req.user.generation =>
            {
                request_keys_changed_at
            }
            // If the request has a keys_changed_at but the above conditions don't hold OR if the
            // request doesn't have a keys_changed_at, just keep the same generation.
            (_, _) => req.user.generation,
        },
    };

    // If the client state changed, we need to mark the current user as "replaced" and create a
    // new user record. Otherwise, we can update the user in place.
    if req.auth_data.client_state != req.user.client_state {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Create new user record with updated generation/keys_changed_at
        let post_user_params = PostUser {
            service_id: req.service_id,
            email: req.auth_data.email.clone(),
            generation,
            client_state: req.auth_data.client_state.clone(),
            node_id: db
                .get_node_id(GetNodeId {
                    service_id: req.service_id,
                    node: req.user.node.clone(),
                })
                .await?
                .id,
            keys_changed_at,
            created_at: timestamp,
        };
        let uid = db.post_user(post_user_params).await?.id;

        // Make sure each old row is marked as replaced (they might not be, due to races in row
        // creation)
        db.replace_users(ReplaceUsers {
            email: req.auth_data.email.clone(),
            service_id: req.service_id,
            replaced_at: timestamp,
        })
        .await?;

        Ok(UserUpdates {
            keys_changed_at,
            generation,
            uid,
        })
    } else {
        if generation != req.user.generation || keys_changed_at != req.user.keys_changed_at {
            let params = PutUser {
                email: req.auth_data.email.clone(),
                service_id: req.service_id,
                generation,
                keys_changed_at,
            };

            db.put_user(params).await?;
        }

        Ok(UserUpdates {
            keys_changed_at,
            generation,
            uid: req.user.uid,
        })
    }
}

pub async fn heartbeat(DbWrapper(db): DbWrapper) -> Result<HttpResponse, Error> {
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

/// Generates an error to test the Sentry integration
pub async fn test_error() -> Result<HttpResponse, TokenserverError> {
    error!("Test Error");
    Err(TokenserverError {
        context: "Test error for Sentry".to_owned(),
        description: "Test error for Sentry".to_owned(),
        ..TokenserverError::internal_error()
    })
}
