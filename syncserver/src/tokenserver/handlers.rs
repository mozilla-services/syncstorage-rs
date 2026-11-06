use std::{cmp, collections::HashMap, sync::Arc, time::Duration};

use actix_web::{Error, HttpResponse, http::StatusCode, web::Data};
use base64::{Engine, engine};
use chrono::{TimeDelta, Utc};
use serde::Serialize;
use serde_json::Value;
use tokio::time::timeout;
use utoipa::ToSchema;

use hawk::{Credentials, Key, RequestBuilder};
use reqwest::Url;
use syncserver_common::Metrics;
use syncserver_settings::Secrets;
use tokenserver_auth::{MakeTokenPlaintext, Tokenlib, TokenserverOrigin};
use tokenserver_common::{ErrorLocation, NodeType, TokenserverError};
use tokenserver_db::{
    Db, SYNC_SERVICE_NAME,
    params::{
        GetNodeId, GetServiceId, GetUsers, PostUser, PutUser, ReplaceUsers, RetireUser,
        UpdateUserGeneration,
    },
    results::GetRawUser,
};

use super::{
    TokenserverMetrics,
    extractors::{
        DbWrapper, DeleteAssertion, FxaWebhookToken, TokenserverRequest, fxa_metrics_hash,
        hash_device_id,
    },
};

#[derive(Debug, Serialize, ToSchema)]
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

#[utoipa::path(
    get,
    path = "/1.0/{application}/{version}",
    tag = "tokenserver",
    summary = "Get sync token",
    description = "Allocates a sync storage node and returns authentication credentials for accessing it. This is the main Tokenserver endpoint used by Firefox Sync clients.",
    params(
        ("application" = String, Path, description = "Application name (e.g., 'sync')"),
        ("version" = String, Path, description = "API version (e.g., '1.0')")
    ),
    responses(
        (status = 200, description = "Token generated successfully", body = TokenserverResult, content_type = "application/json"),
        (status = 400, description = "Bad Request - Malformed request, missing option, bad values or malformed JSON"),
        (status = 401, description = "Unauthorized - Authentication failed. Possible status strings: invalid-credentials, invalid-timestamp, invalid-generation, invalid-client-state, new-users-disabled"),
        (status = 404, description = "Not Found - Unknown URL or unsupported application"),
        (status = 405, description = "Method Not Allowed - Unsupported HTTP method"),
        (status = 406, description = "Unacceptable - Client requested an Accept type that is not supported"),
        (status = 503, description = "Service Unavailable - Server is undergoing maintenance or backends (LDAP/storage nodes) are down"),
    )
)]
pub async fn get_tokenserver_result(
    req: TokenserverRequest,
    DbWrapper(db): DbWrapper,
    TokenserverMetrics(mut metrics): TokenserverMetrics,
    #[cfg(feature = "last_seen")] state: Data<super::ServerState>,
) -> Result<HttpResponse, TokenserverError> {
    let updates = update_user(&req, db).await?;

    let (token, derived_secret) = {
        let token_plaintext = get_token_plaintext(&req, &updates)?;

        metrics.start_timer("token_creation", None);
        // Get the token and secret
        Tokenlib::get_token_and_derived_secret(token_plaintext, &req.shared_secret)?
    };

    // Record the activity.  Takes the buffer lock briefly and never waits for BigQuery.
    #[cfg(feature = "last_seen")]
    if let Some(collector) = &state.last_seen {
        collector.record(&req.auth_data.fxa_uid, &req.auth_data.client_id, Utc::now());
    }

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

    let timestamp = Utc::now().timestamp() as u64;

    Ok(HttpResponse::build(StatusCode::OK)
        .insert_header(("X-Timestamp", timestamp.to_string()))
        // This header helps to prevent cross-site scripting attacks by
        // blocking content type sniffing. It was set automatically by the
        // Pyramid cornice library used by the Python Tokenserver, so we set
        // it here for safety and consistency.
        .insert_header(("X-Content-Type-Options", "nosniff"))
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
        let client_state_b64 = engine::general_purpose::URL_SAFE_NO_PAD.encode(client_state);

        format!(
            "{:013}-{:}",
            // We fall back to using the user's generation here, which matches FxA's behavior
            updates.keys_changed_at.unwrap_or(updates.generation),
            client_state_b64
        )
    };

    let expires = (Utc::now() + TimeDelta::seconds(req.duration as i64)).timestamp() as u64;

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

/// Optionally apply a timeout when running a future
async fn apply_timeout<O, E>(
    duration: Option<Duration>,
    fut: impl std::future::Future<Output = Result<O, E>>,
) -> Result<O, TokenserverError>
where
    E: Into<TokenserverError>,
{
    match duration {
        Some(duration) => match timeout(duration, fut).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => Err(TokenserverError::elapsed()),
        },
        None => fut.await.map_err(Into::into),
    }
}

async fn update_user(
    req: &TokenserverRequest,
    mut db: Box<dyn Db>,
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
        let timestamp = Utc::now().timestamp_millis();

        // Create new user record with updated generation/keys_changed_at
        let post_user_params = PostUser {
            service_id: req.service_id,
            email: req.auth_data.email.clone(),
            generation,
            client_state: req.auth_data.client_state.clone(),
            node_id: apply_timeout(
                db.timeout(),
                db.get_node_id(GetNodeId {
                    service_id: req.service_id,
                    node: req.user.node.clone(),
                }),
            )
            .await?
            .id,
            keys_changed_at,
            created_at: timestamp,
        };
        let uid = apply_timeout(db.timeout(), db.post_user(post_user_params))
            .await?
            .uid;

        // Make sure each old row is marked as replaced (they might not be, due to races in row
        // creation)
        apply_timeout(
            db.timeout(),
            db.replace_users(ReplaceUsers {
                email: req.auth_data.email.clone(),
                service_id: req.service_id,
                replaced_at: timestamp,
            }),
        )
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

            apply_timeout(db.timeout(), db.put_user(params)).await?;
        }

        Ok(UserUpdates {
            keys_changed_at,
            generation,
            uid: req.user.uid,
        })
    }
}

// statsd counter names
const METRIC_EVENTS_RECEIVED: &str = "webhook.events_received";
const METRIC_EVENTS_PROCESSED: &str = "webhook.events_processed";

// statsd names for the FxA-triggered data deletion endpoint.
const METRIC_DELETE_REQUESTS: &str = "fxa_delete_data.requests";
const METRIC_DELETE_NODE: &str = "fxa_delete_data.node_delete";
const METRIC_DELETE_SKIPPED: &str = "fxa_delete_data.skipped";
const METRIC_DELETE_TIMER: &str = "fxa_delete_data";

/// How long the HAWK credential minted for the storage node stays valid.
///
/// It is used within milliseconds; a short life limits the damage if a signed
/// header ever reaches a log. Note the node only exempts
/// `/1.5/{uid}/info/collections` from the expiry check, so a DELETE genuinely
/// needs this in the future.
const NODE_TOKEN_TTL_SECS: i64 = 120;

/// Upper bound on the whole delete, independent of the per-request client
/// timeout, so a slow node cannot pin a worker indefinitely.
const DELETE_OVERALL_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteUserDataResult {
    /// Whether a storage node was asked to delete, and confirmed.
    ///
    /// `false` means the user's current record has no node assignment, so
    /// there was no node to ask and no data location to derive. That is a
    /// success — there is nothing to delete — but it is worth distinguishing
    /// from a delete that actually happened.
    deleted: bool,
}

pub async fn handle_fxa_events(
    FxaWebhookToken(claims): FxaWebhookToken,
    DbWrapper(mut db): DbWrapper,
    state: Data<super::ServerState>,
) -> Result<HttpResponse, TokenserverError> {
    let metrics = Metrics::from(&state.metrics);

    let Some(events) = claims.events.as_object() else {
        return Ok(HttpResponse::Ok().finish());
    };

    // Count every event received.
    metrics.count(METRIC_EVENTS_RECEIVED, events.len() as i64);

    if state.fxa_webhook_metrics_only {
        return Ok(HttpResponse::Ok().finish());
    }

    let service_id = db
        .get_service_id(tokenserver_db::params::GetServiceId {
            service: SYNC_SERVICE_NAME.to_owned(),
        })
        .await?
        .id;

    for event_type in events.keys() {
        let email = format!("{}@{}", claims.sub, state.fxa_email_domain);
        match event_type.as_str() {
            "https://schemas.accounts.firefox.com/event/delete-user" => {
                info!("Processing account delete for {}", email);
                db.retire_user(RetireUser { service_id, email }).await?;
                metrics.incr_with_tag(METRIC_EVENTS_PROCESSED, "event_type", "delete_user");
            }
            "https://schemas.accounts.firefox.com/event/password-change" => {
                if let Some(change_time_ms) = events[event_type]
                    .get("changeTime")
                    .and_then(|t| t.as_i64())
                {
                    info!("Processing password change for {}", email);
                    db.update_user_generation(UpdateUserGeneration {
                        service_id,
                        email,
                        generation: Some(change_time_ms - 1),
                        keys_changed_at: None,
                    })
                    .await?;
                    metrics.incr_with_tag(METRIC_EVENTS_PROCESSED, "event_type", "password_change");
                }
            }
            _ => {
                info!(
                    "Dropping unhandled event type {:?} for {}",
                    event_type, email
                );
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[utoipa::path(
    delete,
    path = "/1.0/users/{fxa_uid}/storage",
    tag = "tokenserver",
    summary = "Delete a user's syncstorage data",
    description = "Deletes all syncstorage data for the user's current node assignment, \
                   leaving the Tokenserver user record intact so the account can keep \
                   syncing. Intended for the FxA Auth Server; authenticated with an \
                   FxA-signed assertion naming the target user in `sub`, not with an \
                   end-user credential.",
    params(
        ("fxa_uid" = String, Path, description = "The user's FxA account id")
    ),
    responses(
        (status = 200, description = "Deleted, or the user has no node assignment", body = DeleteUserDataResult, content_type = "application/json"),
        (status = 401, description = "Unauthorized - credential missing, malformed, expired, wrongly typed, or its `sub` does not match the path uid"),
        (status = 404, description = "Not Found - no Tokenserver record for this fxa_uid"),
        (status = 500, description = "Internal Server Error"),
        (status = 503, description = "Service Unavailable - the storage node could not be reached"),
    )
)]
/// Deletes the user's syncstorage data, leaving their Tokenserver record
/// alone.
pub async fn delete_user_syncstorage_data(
    DeleteAssertion(claims): DeleteAssertion,
    DbWrapper(mut db): DbWrapper,
    state: Data<super::ServerState>,
    secrets: Data<Arc<Secrets>>,
    TokenserverMetrics(mut metrics): TokenserverMetrics,
) -> Result<HttpResponse, TokenserverError> {
    let fxa_uid = claims.sub;

    // Same secret the node derives its token key from; see Secrets::new.
    let shared_secret =
        String::from_utf8(secrets.master_secret.clone()).map_err(|e| TokenserverError {
            context: format!("Failed to read the master secret: {}", e),
            ..TokenserverError::internal_error()
        })?;
    metrics.start_timer(METRIC_DELETE_TIMER, None);

    let email = format!("{}@{}", fxa_uid, state.fxa_email_domain);
    let hashed_fxa_uid = {
        let full = fxa_metrics_hash(&fxa_uid, state.fxa_metrics_hash_secret.as_bytes());
        full[0..32].to_owned()
    };

    let service_id = db
        .get_service_id(GetServiceId {
            service: SYNC_SERVICE_NAME.to_owned(),
        })
        .await?
        .id;

    let mut rows =
        apply_timeout(db.timeout(), db.get_users(GetUsers { email, service_id })).await?;

    // Release the pooled connection before any network I/O
    drop(db);

    if rows.is_empty() {
        metrics.incr_with_tag(METRIC_DELETE_REQUESTS, "result", "not_found");
        return Err(TokenserverError {
            context: format!("No tokenserver user records for {}", hashed_fxa_uid),
            description: "Unknown user".to_owned(),
            http_status: http::StatusCode::NOT_FOUND,
            location: ErrorLocation::Url,
            ..TokenserverError::default()
        });
    }

    // Same "current user" rule as `Db::get_or_create_user`: greatest
    // (generation, created_at) wins. Older rows belong to superseded key epochs
    // and are left to purge_old_records.py.
    rows.sort_by_key(|row| cmp::Reverse((row.generation, row.created_at)));
    let row = &rows[0];

    let Some(node) = &row.node else {
        // No node assignment means no location to derive and nothing to ask.
        info!(
            "No node assigned, nothing to delete";
            "metrics_uid" => &hashed_fxa_uid
        );
        metrics.incr_with_tag(METRIC_DELETE_SKIPPED, "reason", "no_node");
        metrics.incr_with_tag(METRIC_DELETE_REQUESTS, "result", "ok");
        return Ok(HttpResponse::Ok().json(DeleteUserDataResult { deleted: false }));
    };

    info!(
        "Deleting syncstorage data";
        "metrics_uid" => &hashed_fxa_uid,
        "uid" => row.uid
    );

    apply_timeout(
        Some(Duration::from_secs(DELETE_OVERALL_TIMEOUT_SECS)),
        delete_node_data(&state, &fxa_uid, &hashed_fxa_uid, node, row, &shared_secret),
    )
    .await
    .inspect_err(|_| metrics.incr_with_tag(METRIC_DELETE_REQUESTS, "result", "failed"))?;

    metrics.incr_with_tag(METRIC_DELETE_REQUESTS, "result", "ok");
    Ok(HttpResponse::Ok().json(DeleteUserDataResult { deleted: true }))
}

/// Derive the `fxa_kid` the user's syncstorage data is filed under.
fn node_fxa_kid(row: &GetRawUser) -> Result<String, TokenserverError> {
    // NOTE: `purge_old_records.py` does `user.keys_changed_at or
    // user.generation`, where 0 is falsy, so it gets this wrong for
    // the same rows. Match the token, not the script.
    let client_state = hex::decode(&row.client_state).map_err(|e| TokenserverError {
        context: format!("Failed to decode the client state hex: {}", e),
        ..TokenserverError::internal_error()
    })?;

    Ok(format!(
        "{:013}-{:}",
        row.keys_changed_at.unwrap_or(row.generation),
        engine::general_purpose::URL_SAFE_NO_PAD.encode(client_state)
    ))
}

/// Mint a tokenlib token for `plaintext` and return the `Authorization` value
/// authenticating a DELETE of `url`.
fn delete_auth_header(
    url: &Url,
    plaintext: MakeTokenPlaintext,
    shared_secret: &str,
) -> Result<String, TokenserverError> {
    let (token, derived_secret) = Tokenlib::get_token_and_derived_secret(plaintext, shared_secret)?;

    let credentials = Credentials {
        id: token,
        key: Key::new(derived_secret.as_bytes(), hawk::SHA256).map_err(|e| TokenserverError {
            context: format!("Failed to build the hawk key: {}", e),
            ..TokenserverError::internal_error()
        })?,
    };
    let header = RequestBuilder::from_url("DELETE", url)
        .map_err(|e| TokenserverError {
            context: format!("Failed to build the hawk request: {}", e),
            ..TokenserverError::internal_error()
        })?
        .request()
        .make_header(&credentials)
        .map_err(|e| TokenserverError {
            context: format!("Failed to sign the hawk request: {}", e),
            ..TokenserverError::internal_error()
        })?;

    Ok(format!("Hawk {}", header))
}

/// Mint a HAWK credential for `row` and ask its storage node to delete the
/// user's data, mirroring `delete_service_data()` in
/// tools/tokenserver/purge_old_records.py.
async fn delete_node_data(
    state: &super::ServerState,
    fxa_uid: &str,
    hashed_fxa_uid: &str,
    node: &str,
    row: &GetRawUser,
    shared_secret: &str,
) -> Result<(), TokenserverError> {
    let metrics = Metrics::from(&state.metrics);
    let node_result = |result: &str| metrics.incr_with_tag(METRIC_DELETE_NODE, "result", result);

    let fxa_kid = node_fxa_kid(row).map_err(|e| {
        node_result("bad_client_state");
        TokenserverError {
            context: format!("{} for {}", e.context, hashed_fxa_uid),
            ..e
        }
    })?;

    let plaintext = MakeTokenPlaintext {
        node: node.to_owned(),
        fxa_kid,
        fxa_uid: fxa_uid.to_owned(),
        hashed_device_id: hash_device_id(hashed_fxa_uid, state.fxa_metrics_hash_secret.as_bytes()),
        hashed_fxa_uid: hashed_fxa_uid.to_owned(),
        expires: (Utc::now() + TimeDelta::seconds(NODE_TOKEN_TTL_SECS)).timestamp() as u64,
        uid: row.uid,
        tokenserver_origin: TokenserverOrigin::Rust,
    };

    let url =
        Url::parse(&format!("{}/1.5/{}", node.trim_end_matches('/'), row.uid)).map_err(|e| {
            TokenserverError {
                context: format!("Invalid node url {:?}: {}", node, e),
                ..TokenserverError::internal_error()
            }
        })?;

    let auth = delete_auth_header(&url, plaintext, shared_secret)?;

    let response = state
        .storage_client
        .delete(url)
        .header(reqwest::header::AUTHORIZATION, auth)
        .send()
        .await
        .map_err(|e| {
            node_result(if e.is_timeout() { "timeout" } else { "connect" });
            TokenserverError {
                context: format!("Storage node request failed for {}", hashed_fxa_uid),
                tags: Some(vec![("reason", "node_unreachable".to_owned())]),
                ..TokenserverError::resource_unavailable()
            }
        })?;

    let status = response.status();
    // 404 is success, matching purge_old_records.py
    if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
        node_result(if status == reqwest::StatusCode::NOT_FOUND {
            "not_found"
        } else {
            "ok"
        });
        return Ok(());
    }

    if status == reqwest::StatusCode::UNAUTHORIZED {
        // We minted this token ourselves, so a rejection is our bug
        node_result("unauthorized");
        return Err(TokenserverError {
            context: format!(
                "Storage node rejected our minted token for {}",
                hashed_fxa_uid
            ),
            ..TokenserverError::internal_error()
        });
    }

    node_result(if status.is_server_error() {
        "http_5xx"
    } else if status.is_client_error() {
        "http_4xx"
    } else {
        "http_other"
    });
    Err(TokenserverError {
        context: format!(
            "Storage node returned {} for {}",
            status.as_u16(),
            hashed_fxa_uid
        ),
        tags: Some(vec![("reason", "node_error".to_owned())]),
        ..TokenserverError::resource_unavailable()
    })
}

#[utoipa::path(
    get,
    path = "/__heartbeat__",
    tag = "tokenserver",
    summary = "Tokenserver health check",
    description = "Returns health status of the Tokenserver including database connectivity.",
    responses(
        (status = 200, description = "Service is healthy", content_type = "application/json"),
        (status = 503, description = "Service is unhealthy", content_type = "application/json"),
    )
)]
pub async fn heartbeat(DbWrapper(mut db): DbWrapper) -> Result<HttpResponse, Error> {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );

    match apply_timeout(db.timeout(), db.check()).await {
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::{SystemTime, UNIX_EPOCH};

    use actix_web::middleware::ErrorHandlers;
    use actix_web::test::{self, TestRequest};
    use actix_web::{
        App, HttpRequest, HttpResponse, http::StatusCode, http::header::LOCATION,
        middleware::from_fn, web, web::Data,
    };
    use base64::{Engine as _, engine};
    use serde_json::json;
    use utoipa::OpenApi;
    use utoipa_swagger_ui::SwaggerUi;

    use syncserver_common::middleware::{request_metrics::request_metrics, sentry::SentryWrapper};
    use syncserver_settings::Settings;
    use tokenserver_auth::test_utils::{
        DeleteAssertionBuilder, OTHER_PRIVATE_KEY_PEM, TEST_PRIVATE_KEY_PEM, make_set,
        other_test_jwk, test_jwk,
    };
    use tokenserver_auth::{FxaDeleteAssertionVerifier, MockVerifier, SETVerifierImpl, oauth};
    use tokenserver_db::mock::MockDbPool;
    use tokenserver_db::results::GetRawUser;
    use tokenserver_settings::Settings as TokenserverSettings;

    use crate::build_app_without_syncstorage;
    use crate::error::ApiError;
    use crate::server::{ApiDoc, TOKENSERVER_DOCS_URL};
    use crate::tokenserver::handlers::node_fxa_kid;
    use crate::tokenserver::{self, ServerState};
    use crate::web::middleware;

    fn make_state(set_verifiers: Vec<SETVerifierImpl>) -> ServerState {
        make_state_with_db_pool(set_verifiers, Box::new(MockDbPool::new()))
    }

    #[cfg(feature = "last_seen")]
    #[actix_web::test]
    async fn test_last_seen_records_only_successful_token_exchanges() {
        use crate::tokenserver::last_seen::Collector;

        let mut state = make_state(vec![]);
        let collector = Arc::new(Collector::new(Arc::clone(&state.metrics)));
        state.last_seen = Some(Arc::clone(&collector));
        state.oauth_verifier = Box::new(MockVerifier {
            valid: true,
            verify_output: oauth::VerifyOutput {
                fxa_uid: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                client_id: "bbbbbbbbbbbbbbbb".to_owned(),
                generation: Some(1234),
            },
        });
        let mut failed_state = state.clone();
        failed_state.oauth_verifier = Box::new(MockVerifier::<oauth::VerifyOutput> {
            valid: false,
            ..Default::default()
        });

        let app = make_app_from_state(state).await;
        for _ in 0..2 {
            let request = TestRequest::get()
                .uri("/1.0/sync/1.5")
                .insert_header(("authorization", "Bearer test-oauth-token"))
                .insert_header(("x-keyid", "0000000001234-qqo"))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), StatusCode::OK);
        }
        let batch = collector.take_all();
        assert_eq!(batch.len(), 1);
        assert!(batch.contains_key(&(
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            "bbbbbbbbbbbbbbbb".to_owned(),
        )));

        let app = make_app_from_state(failed_state).await;
        let request = TestRequest::get()
            .uri("/1.0/sync/1.5")
            .insert_header(("authorization", "Bearer rejected-token"))
            .insert_header(("x-keyid", "0000000001234-qqo"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(collector.is_empty());
    }

    async fn make_app(
        set_verifiers: Vec<SETVerifierImpl>,
    ) -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
    > {
        let state = make_state(set_verifiers);
        make_app_from_state(state).await
    }

    fn make_state_with_db_pool(
        set_verifiers: Vec<SETVerifierImpl>,
        db_pool: Box<dyn tokenserver_db::DbPool>,
    ) -> ServerState {
        let syncserver_settings = Settings::default();
        let tokenserver_settings = TokenserverSettings::default();
        ServerState {
            fxa_email_domain: "api.accounts.firefox.com".to_owned(),
            fxa_metrics_hash_secret: "topsecretz".to_owned(),
            oauth_verifier: Box::new(MockVerifier::<oauth::VerifyOutput>::default()),
            db_pool,
            node_capacity_release_rate: None,
            node_type: Default::default(),
            metrics: syncserver_common::metrics_from_opts(
                &tokenserver_settings.statsd_label,
                syncserver_settings.statsd_host.as_deref(),
                syncserver_settings.statsd_port,
            )
            .unwrap(),
            token_duration: 3600,
            set_verifiers,
            fxa_webhook_enabled: true,
            fxa_webhook_metrics_only: false,
            fxa_delete_data_enabled: false,
            delete_assertion_verifiers: vec![],
            storage_client: reqwest::Client::new(),
            allow_new_users: true,
            #[cfg(feature = "last_seen")]
            last_seen: None,
        }
    }

    async fn make_app_from_state(
        state: ServerState,
    ) -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
        Error = actix_web::Error,
    > {
        let secrets = Arc::new(syncserver_settings::Secrets::new("secret").unwrap());
        let metrics = state.metrics.clone();
        test::init_service(build_app_without_syncstorage!(
            state,
            secrets,
            actix_cors::Cors::default(),
            metrics
        ))
        .await
    }

    #[actix_web::test]
    async fn test_missing_auth_header() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let app = make_app(vec![verifier]).await;
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_wrong_signing_key() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let app = make_app(vec![verifier]).await;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            OTHER_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_wrong_audience() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let app = make_app(vec![verifier]).await;
        let token = make_set(
            "quux",
            "some-other-RP",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_no_verifiers_configured() {
        let app = make_app(vec![]).await;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
    }

    #[actix_web::test]
    async fn test_delete_user_event() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let (pool, call_log) = MockDbPool::with_capture();
        let app =
            make_app_from_state(make_state_with_db_pool(vec![verifier], Box::new(pool))).await;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let retire_user_calls = call_log.retire_user.lock().unwrap();
        assert_eq!(retire_user_calls.len(), 1);
        assert_eq!(retire_user_calls[0].email, "quux@api.accounts.firefox.com");
    }

    #[actix_web::test]
    async fn test_metrics_only_mode_skips_processing() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let (pool, call_log) = MockDbPool::with_capture();
        let mut state = make_state_with_db_pool(vec![verifier], Box::new(pool));
        state.fxa_webhook_metrics_only = true;
        let app = make_app_from_state(state).await;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/delete-user": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        // no database mutation in metrics-only mode
        assert_eq!(resp.status(), 200);
        assert_eq!(call_log.retire_user.lock().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_password_change_event() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let (pool, call_log) = MockDbPool::with_capture();
        let app =
            make_app_from_state(make_state_with_db_pool(vec![verifier], Box::new(pool))).await;
        let change_time_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/password-change": {"changeTime": change_time_ms}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);
        let update_user_generation_calls = call_log.update_user_generation.lock().unwrap();
        assert_eq!(update_user_generation_calls.len(), 1);
        assert_eq!(
            update_user_generation_calls[0].email,
            "quux@api.accounts.firefox.com"
        );
        assert_eq!(
            update_user_generation_calls[0].generation,
            Some(change_time_ms - 1)
        );
        assert_eq!(update_user_generation_calls[0].keys_changed_at, None);

        assert_eq!(call_log.retire_user.lock().unwrap().len(), 0);
    }

    #[actix_web::test]
    async fn test_unknown_event() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "testo", "https://accounts.firefox.com/").unwrap();
        let (pool, call_log) = MockDbPool::with_capture();
        let app =
            make_app_from_state(make_state_with_db_pool(vec![verifier], Box::new(pool))).await;
        let token = make_set(
            "quux",
            "testo",
            json!({"https://schemas.accounts.firefox.com/event/unknown-event": {}}),
            TEST_PRIVATE_KEY_PEM,
        );
        let req = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 200);

        assert_eq!(call_log.update_user_generation.lock().unwrap().len(), 0);
        assert_eq!(call_log.retire_user.lock().unwrap().len(), 0);
    }

    // ---- FxA-triggered syncstorage data deletion ----

    const TEST_DELETE_AUD: &str = "urn:mozilla:fxa:tokenserver:sync-data-delete";
    const TEST_DELETE_ISS: &str = "https://accounts.firefox.com";
    const TEST_FXA_UID: &str = "testuid";

    /// Mint the assertion the endpoint expects, the way FxA will.
    fn assertion(sub: &str) -> DeleteAssertionBuilder {
        DeleteAssertionBuilder::new(sub, TEST_DELETE_AUD, TEST_DELETE_ISS)
    }

    fn valid_delete_token(fxa_uid: &str) -> String {
        assertion(fxa_uid).build()
    }

    fn user_row(uid: i64, node: Option<String>) -> GetRawUser {
        GetRawUser {
            uid,
            client_state: "aaaa".to_owned(),
            generation: 1234,
            node,
            keys_changed_at: Some(1234),
            created_at: 1,
            replaced_at: None,
        }
    }

    async fn make_delete_app(
        users: Vec<GetRawUser>,
    ) -> (
        impl actix_web::dev::Service<
            actix_http::Request,
            Response = actix_web::dev::ServiceResponse<impl actix_web::body::MessageBody>,
            Error = actix_web::Error,
        >,
        tokenserver_db::mock::CallLog,
    ) {
        let (pool, call_log) = tokenserver_db::mock::MockDbPool::with_users(users);
        let mut state = make_state_with_db_pool(vec![], Box::new(pool));
        state.fxa_delete_data_enabled = true;
        state.delete_assertion_verifiers = vec![
            FxaDeleteAssertionVerifier::new(&test_jwk(), TEST_DELETE_AUD, TEST_DELETE_ISS).unwrap(),
        ];
        (make_app_from_state(state).await, call_log)
    }

    fn delete_request(fxa_uid: &str, auth: Option<&str>) -> actix_http::Request {
        let mut req = TestRequest::delete().uri(&format!("/1.0/users/{}/storage", fxa_uid));
        if let Some(auth) = auth {
            req = req.insert_header(("authorization", format!("Bearer {}", auth)));
        }
        req.to_request()
    }

    /// Requests a [`fake_node`] received, as `(path, authorization)`.
    type NodeRequests = Arc<Mutex<Vec<(String, String)>>>;

    /// httptest has no request-capture facility, so record from a matcher that
    /// always matches.
    struct Capture(NodeRequests);

    impl httptest::matchers::Matcher<httptest::http::Request<httptest::bytes::Bytes>> for Capture {
        fn matches(
            &mut self,
            req: &httptest::http::Request<httptest::bytes::Bytes>,
            _ctx: &mut httptest::matchers::ExecutionContext,
        ) -> bool {
            let auth = req
                .headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_owned();
            self.0
                .lock()
                .unwrap()
                .push((req.uri().path().to_owned(), auth));
            true
        }

        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Capture")
        }
    }

    /// A storage node that answers `status` and records what it was asked.
    fn fake_node(status: u16) -> (httptest::Server, NodeRequests) {
        use httptest::{Expectation, matchers::*, responders::*};

        let seen: NodeRequests = Arc::new(Mutex::new(vec![]));
        let server = httptest::Server::run();
        server.expect(
            Expectation::matching(all_of![request::method("DELETE"), Capture(seen.clone()),])
                .times(..)
                .respond_with(status_code(status)),
        );
        (server, seen)
    }

    #[test]
    fn test_node_fxa_kid() {
        let kid = |keys_changed_at, generation, client_state: &str| {
            node_fxa_kid(&GetRawUser {
                keys_changed_at,
                generation,
                client_state: client_state.to_owned(),
                ..Default::default()
            })
        };

        // keys_changed_at wins when set.
        assert_eq!(kid(Some(5000), 1234, "aaaa").unwrap(), "0000000005000-qqo");

        // Only an absent keys_changed_at falls back to generation.
        assert_eq!(kid(None, 1234, "aaaa").unwrap(), "0000000001234-qqo");

        // A stored 0 is a real value, not "unset". `update_user` keeps it as
        // `Some(0)`, so the user's own token addresses
        // `0000000000000-<client_state>` and the delete must target that.
        // Treating it as unset here would address an empty keyspace and
        // silently delete nothing.
        assert_eq!(kid(Some(0), 1234, "aaaa").unwrap(), "0000000000000-qqo");

        // Legacy rows carry no client state; the trailing dash is correct.
        assert_eq!(kid(Some(1), 1, "").unwrap(), "0000000000001-");

        // Non-hex client_state is corruption on our side, not a bad request.
        let err = kid(Some(1), 1, "zzzz").unwrap_err();
        assert_eq!(err.http_status, http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[actix_web::test]
    async fn test_delete_data_happy_path() {
        let (node, seen) = fake_node(200);
        let (app, call_log) = make_delete_app(vec![user_row(42, Some(node.url_str("")))]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        let status = response.status();
        let body = test::read_body(response).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "body: {:?}",
            String::from_utf8_lossy(&body)
        );

        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["deleted"], true);

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1);
        assert_eq!(seen[0].0, "/1.5/42");
        assert!(seen[0].1.starts_with("Hawk "), "got {:?}", seen[0].1);

        // The user record must survive: this is reset-sync-data, not account
        // deletion.
        assert!(call_log.retire_user.lock().unwrap().is_empty());
        assert!(call_log.update_user_generation.lock().unwrap().is_empty());

        // And it must look the user up by the derived FxA email.
        let lookups = call_log.get_users.lock().unwrap();
        assert_eq!(lookups.len(), 1);
        assert_eq!(lookups[0].email, "testuid@api.accounts.firefox.com");
    }

    #[actix_web::test]
    async fn test_delete_data_uses_newest_row_only() {
        let (node, seen) = fake_node(200);
        let url = node.url_str("");
        // Lowest generation first, so a naive implementation picks the wrong one.
        let mut old = user_row(1, Some(url.clone()));
        old.generation = 1;
        old.created_at = 1;
        let mut newest = user_row(99, Some(url));
        newest.generation = 5000;
        newest.created_at = 2;
        let (app, _) = make_delete_app(vec![old, newest]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 1, "only the current row should be deleted");
        assert_eq!(seen[0].0, "/1.5/99");
    }

    #[actix_web::test]
    async fn test_delete_data_no_node_assignment() {
        let (app, _) = make_delete_app(vec![user_row(42, None)]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let body = test::read_body(response).await;
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // No node assigned: a success, but nothing was deleted.
        assert_eq!(body["deleted"], false);
    }

    #[actix_web::test]
    async fn test_delete_data_unknown_user_is_404() {
        let (app, _) = make_delete_app(vec![]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_delete_data_node_404_is_success() {
        let (node, _) = fake_node(404);
        let (app, _) = make_delete_app(vec![user_row(42, Some(node.url_str("")))]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_delete_data_node_401_is_500() {
        // Our own minted token being rejected is our bug, not the caller's.
        let (node, _) = fake_node(401);
        let (app, _) = make_delete_app(vec![user_row(42, Some(node.url_str("")))]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Key rotation is the main argument for this scheme over a shared secret,
    /// so prove it works: an assertion signed with either configured JWK is
    /// accepted, in either position.
    #[actix_web::test]
    async fn test_delete_data_accepts_either_configured_jwk() {
        let (node, _) = fake_node(200);
        let other_jwk = other_test_jwk();

        for (name, jwks) in [
            ("incoming key second", vec![other_jwk.clone(), test_jwk()]),
            ("incoming key first", vec![test_jwk(), other_jwk.clone()]),
        ] {
            let (pool, _) = tokenserver_db::mock::MockDbPool::with_users(vec![user_row(
                42,
                Some(node.url_str("")),
            )]);
            let mut state = make_state_with_db_pool(vec![], Box::new(pool));
            state.fxa_delete_data_enabled = true;
            state.delete_assertion_verifiers = jwks
                .iter()
                .map(|jwk| {
                    FxaDeleteAssertionVerifier::new(jwk, TEST_DELETE_AUD, TEST_DELETE_ISS).unwrap()
                })
                .collect();
            let app = make_app_from_state(state).await;

            let response = test::call_service(
                &app,
                delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK, "{name}");
        }
    }

    #[actix_web::test]
    async fn test_delete_data_rejects_bad_credentials() {
        let (node, seen) = fake_node(200);
        let url = node.url_str("");

        let cases: Vec<(&str, Option<String>)> = vec![
            ("missing header", None),
            ("not a jwt", Some("garbage".to_owned())),
            (
                "wrong signing key",
                Some(assertion(TEST_FXA_UID).key(OTHER_PRIVATE_KEY_PEM).build()),
            ),
            ("expired", Some(assertion(TEST_FXA_UID).exp_in(-60).build())),
            // Minted far enough ahead that it would never go stale, which the
            // lifetime cap exists to refuse.
            (
                "over the lifetime cap",
                Some(assertion(TEST_FXA_UID).exp_in(86400).build()),
            ),
            (
                "wrong audience",
                Some(assertion(TEST_FXA_UID).aud("urn:mozilla:fxa:other").build()),
            ),
            (
                "wrong issuer",
                Some(assertion(TEST_FXA_UID).iss("https://evil.example").build()),
            ),
            (
                "no audience claim",
                Some(assertion(TEST_FXA_UID).without_aud().build()),
            ),
            (
                "wrong typ",
                Some(assertion(TEST_FXA_UID).typ("at+jwt").build()),
            ),
            (
                "no jti",
                Some(assertion(TEST_FXA_UID).without_jti().build()),
            ),
            // Validly signed, but naming a different user than the path.
            (
                "sub does not match the path",
                Some(assertion("otheruid").build()),
            ),
            // A webhook SET must not authorize a deletion.
            (
                "webhook SET",
                Some(make_set(
                    TEST_FXA_UID,
                    TEST_DELETE_AUD,
                    json!({}),
                    TEST_PRIVATE_KEY_PEM,
                )),
            ),
        ];

        for (name, auth) in cases {
            let (app, call_log) = make_delete_app(vec![user_row(42, Some(url.clone()))]).await;
            let response =
                test::call_service(&app, delete_request(TEST_FXA_UID, auth.as_deref())).await;
            assert_eq!(
                response.status(),
                StatusCode::UNAUTHORIZED,
                "expected 401 for {name}"
            );
            // Authentication must fail before the user is even looked up.
            assert!(
                call_log.get_users.lock().unwrap().is_empty(),
                "{name} reached the database"
            );
        }

        assert!(
            seen.lock().unwrap().is_empty(),
            "no storage node request may be made for a rejected credential"
        );
    }

    /// The reverse confusion direction. A delete assertion must not drive the
    /// webhook, and in particular must not retire the account — the one thing
    /// this endpoint exists to avoid doing.
    #[actix_web::test]
    async fn test_delete_assertion_cannot_drive_the_webhook() {
        let verifier =
            SETVerifierImpl::new(&test_jwk(), "fxa-webhook-client-id", TEST_DELETE_ISS).unwrap();
        let (pool, call_log) = tokenserver_db::mock::MockDbPool::with_capture();
        let mut state = make_state_with_db_pool(vec![verifier], Box::new(pool));
        state.fxa_webhook_enabled = true;
        let app = make_app_from_state(state).await;

        let request = TestRequest::post()
            .uri("/1.0/webhooks/fxa/events")
            .insert_header((
                "authorization",
                format!("Bearer {}", assertion(TEST_FXA_UID).build()),
            ))
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(
            call_log.retire_user.lock().unwrap().is_empty(),
            "a delete assertion must never retire an account"
        );
    }

    #[actix_web::test]
    async fn test_delete_data_zero_keys_changed_at_is_preserved() {
        // End-to-end counterpart of the node_fxa_kid unit test: prove the kid
        // for a stored `keys_changed_at` of 0 reaches the wire as
        // 0000000000000-, which is where such a user's own sync token files
        // their data. Treating the 0 as unset would address an empty keyspace
        // and the delete would report success having removed nothing.
        let (node, seen) = fake_node(200);
        let mut row = user_row(42, Some(node.url_str("")));
        row.keys_changed_at = Some(0);
        row.generation = 1234;
        let (app, _) = make_delete_app(vec![row]).await;

        let response = test::call_service(
            &app,
            delete_request(TEST_FXA_UID, Some(&valid_delete_token(TEST_FXA_UID))),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let seen = seen.lock().unwrap();
        let auth = &seen[0].1;
        let id = auth
            .split("id=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("hawk id");
        let decoded = engine::general_purpose::URL_SAFE.decode(id).unwrap();
        // The token is the JSON payload followed by a 32 byte HMAC.
        let payload: serde_json::Value =
            serde_json::from_slice(&decoded[..decoded.len() - 32]).unwrap();
        assert_eq!(payload["fxa_kid"], "0000000000000-qqo");
    }
}
