//! API Handlers
use std::collections::HashMap;
use std::convert::Into;
use std::time::{Duration, Instant};

use crate::server::user_agent::{get_device_info, DeviceInfo};
use actix_web::{
    http::{header, StatusCode},
    web::Data,
    HttpRequest, HttpResponse, HttpResponseBuilder,
};
use serde::Serialize;
use serde_json::{json, Value};
use syncserver_common::{X_LAST_MODIFIED, X_WEAVE_NEXT_OFFSET, X_WEAVE_RECORDS};
use syncstorage_db::{
    params,
    results::{CreateBatch, Paginated},
    Db, DbError, DbErrorIntrospect,
};

use crate::{
    error::{ApiError, ApiErrorKind},
    server::ServerState,
    web::{
        extractors::{
            BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, EmitApiMetric,
            HeartbeatRequest, MetaRequest, ReplyFormat, TestErrorRequest,
        },
        transaction::DbTransactionPool,
    },
};

use glean::server_events::{EventsPing, RequestInfo, SyncstorageGetCollectionsEvent};

pub const ONE_KB: f64 = 1024.0;

pub async fn get_collections(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
    state: Data<ServerState>,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            meta.emit_api_metric("request.get_collections");
            if state.glean_enabled {
                // Values below are be passed to the Glean logic to emit metrics.
                // This is used to measure DAU (Daily Active Use) of Sync.
                let user_agent = request
                    .headers()
                    .get(header::USER_AGENT)
                    .and_then(|header| header.to_str().ok())
                    .unwrap_or("");
                let device_info: DeviceInfo = get_device_info(user_agent);

                state.glean_logger.record_events_ping(
                    &RequestInfo {
                        user_agent: user_agent.to_owned(),
                        ip_address: "".to_owned(),
                    },
                    &EventsPing {
                        syncstorage_device_family: device_info.device_family.to_string(),
                        syncstorage_hashed_device_id: meta.user_id.hashed_device_id.clone(),
                        syncstorage_hashed_fxa_uid: meta.user_id.hashed_fxa_uid.clone(),
                        syncstorage_platform: device_info.platform.to_string(),
                        event: Some(Box::new(SyncstorageGetCollectionsEvent {})),
                    },
                );
            }
            let result = db.get_collection_timestamps(meta.user_id).await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .insert_header((X_WEAVE_RECORDS, result.len().to_string()))
                .json(result))
        })
        .await
}

pub async fn get_collection_counts(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            meta.emit_api_metric("request.get_collection_counts");
            let result = db.get_collection_counts(meta.user_id).await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .insert_header((X_WEAVE_RECORDS, result.len().to_string()))
                .json(result))
        })
        .await
}

pub async fn get_collection_usage(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            meta.emit_api_metric("request.get_collection_usage");
            let usage: HashMap<_, _> = db
                .get_collection_usage(meta.user_id)
                .await?
                .into_iter()
                .map(|(coll, size)| (coll, size as f64 / ONE_KB))
                .collect();

            Ok(HttpResponse::build(StatusCode::OK)
                .insert_header((X_WEAVE_RECORDS, usage.len().to_string()))
                .json(usage))
        })
        .await
}

pub async fn get_quota(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            meta.emit_api_metric("request.get_quota");
            let usage = db.get_storage_usage(meta.user_id).await?;
            Ok(HttpResponse::Ok().json(vec![Some(usage as f64 / ONE_KB), None]))
        })
        .await
}

pub async fn delete_all(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            meta.emit_api_metric("request.delete_all");
            Ok(HttpResponse::Ok().json(db.delete_storage(meta.user_id).await?))
        })
        .await
}

pub async fn delete_collection(
    coll: CollectionRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            let delete_bsos = !coll.query.ids.is_empty();
            let timestamp = if delete_bsos {
                coll.emit_api_metric("request.delete_bsos");
                db.delete_bsos(params::DeleteBsos {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    ids: coll.query.ids.clone(),
                })
                .await
            } else {
                coll.emit_api_metric("request.delete_collection");
                db.delete_collection(params::DeleteCollection {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                })
                .await
            };

            let timestamp = match timestamp {
                Ok(timestamp) => timestamp,
                Err(e) => {
                    if e.is_collection_not_found() || e.is_bso_not_found() {
                        db.get_storage_timestamp(coll.user_id).await?
                    } else {
                        return Err(e.into());
                    }
                }
            };

            let mut resp = HttpResponse::Ok();
            if delete_bsos {
                resp.insert_header((X_LAST_MODIFIED, timestamp.as_header()));
            }
            Ok(resp.json(timestamp))
        })
        .await
}

pub async fn get_collection(
    coll: CollectionRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            coll.emit_api_metric("request.get_collection");
            let params = params::GetBsos {
                user_id: coll.user_id.clone(),
                newer: coll.query.newer,
                older: coll.query.older,
                sort: coll.query.sort,
                limit: coll.query.limit,
                offset: coll.query.offset.map(Into::into),
                ids: coll.query.ids.clone(),
                full: coll.query.full,
                collection: coll.collection.clone(),
            };
            let response = if coll.query.full {
                let result = db.get_bsos(params).await;
                finish_get_collection(&coll, db, result).await?
            } else {
                // Changed to be a Paginated list of BSOs, need to extract IDs from them.
                let result = db.get_bso_ids(params).await;
                finish_get_collection(&coll, db, result).await?
            };
            Ok(response)
        })
        .await
}

async fn finish_get_collection<T>(
    coll: &CollectionRequest,
    db: &mut dyn Db<Error = DbError>,
    result: Result<Paginated<T>, DbError>,
) -> Result<HttpResponse, DbError>
where
    T: Serialize + Default + 'static,
{
    let result = result.or_else(|e| {
        if e.is_collection_not_found() {
            // For b/w compat, non-existent collections must return an
            // empty list
            Ok(Paginated::default())
        } else {
            Err(e)
        }
    })?;

    let ts = db
        .extract_resource(coll.user_id.clone(), Some(coll.collection.clone()), None)
        .await?;

    let mut builder = HttpResponse::build(StatusCode::OK);
    let resp = builder
        .insert_header((X_LAST_MODIFIED, ts.as_header()))
        .insert_header((X_WEAVE_RECORDS, result.items.len().to_string()));

    if let Some(offset) = result.offset {
        resp.insert_header((X_WEAVE_NEXT_OFFSET, offset));
    }

    match coll.reply {
        ReplyFormat::Json => Ok(resp.json(result.items)),
        ReplyFormat::Newlines => {
            let items: String = result
                .items
                .into_iter()
                .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "".to_string()))
                .filter(|v| !v.is_empty())
                .map(|v| v.replace('\n', "\\u000a") + "\n")
                .collect();

            Ok(resp
                .insert_header(("Content-Type", "application/newlines"))
                .insert_header(("Content-Length", format!("{}", items.len())))
                .body(items))
        }
    }
}

pub async fn post_collection(
    coll: CollectionPostRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            coll.emit_api_metric("request.post_collection");
            trace!("Collection: Post");

            // batches are a conceptual, singular update, so we should handle
            // them separately.
            if let Some(ref batch) = coll.batch {
                // Optimization: specifying ?batch=true&commit=true
                // (batch.id.is_none() && batch.commit) is equivalent to a
                // simpler post_bsos call. Fallthrough in that case, instead of
                // incurring post_collection_batch's overhead
                if !(batch.id.is_none() && batch.commit) {
                    return post_collection_batch(coll, db).await;
                }
            }

            let (success_ids, bsos): (Vec<_>, Vec<_>) = coll
                .bsos
                .valid
                .into_iter()
                .map(|x| (x.id.clone(), x.into()))
                .unzip();

            let modified = db
                .post_bsos(params::PostBsos {
                    user_id: coll.user_id,
                    collection: coll.collection,
                    bsos,
                    for_batch: false,
                })
                .await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .insert_header((X_LAST_MODIFIED, modified.as_header()))
                .json(json!({
                    "modified": modified,
                    "success": success_ids,
                    "failed": coll.bsos.invalid,
                })))
        })
        .await
}

// Append additional collection items into the given Batch, optionally commiting
// the entire, accumulated if the `commit` flag is set.
pub async fn post_collection_batch(
    coll: CollectionPostRequest,
    db: &mut dyn Db<Error = DbError>,
) -> Result<HttpResponse, ApiError> {
    coll.emit_api_metric("request.post_collection_batch");
    trace!("Batch: Post collection batch");
    // Bail early if we have nonsensical arguments
    // TODO: issue932 may make these multi-level transforms easier
    let breq = coll
        .batch
        .clone()
        .ok_or_else(|| -> ApiError { ApiErrorKind::Db(DbError::batch_not_found()).into() })?;

    let new_batch = if let Some(id) = breq.id.clone() {
        trace!("Batch: Validating {}", &id);
        // Validate the batch before attempting a full append (for efficiency)
        let is_valid = db
            .validate_batch(params::ValidateBatch {
                user_id: coll.user_id.clone(),
                collection: coll.collection.clone(),
                id: id.clone(),
            })
            .await?;

        if is_valid {
            let collection_id = db.get_collection_id(&coll.collection).await?;
            let usage = db
                .get_quota_usage(params::GetQuotaUsage {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    collection_id,
                })
                .await?;
            CreateBatch {
                id: id.clone(),
                size: if coll.quota_enabled {
                    Some(usage.total_bytes)
                } else {
                    None
                },
            }
        } else {
            return Err(ApiErrorKind::Db(DbError::batch_not_found()).into());
        }
    } else {
        trace!("Batch: Creating new batch");
        db.create_batch(params::CreateBatch {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            bsos: vec![],
        })
        .await?
    };

    let user_id = coll.user_id.clone();
    let collection = coll.collection.clone();

    let mut success = vec![];
    let mut failed = coll.bsos.invalid;
    let bso_ids: Vec<_> = coll.bsos.valid.iter().map(|bso| bso.id.clone()).collect();

    let mut resp: Value = json!({});

    macro_rules! handle_result {
        // collect up the successful and failed bso_ids into a response.
        ( $r: expr) => {
            match $r {
                Ok(_) => success.extend(bso_ids.clone()),
                Err(e) if e.is_conflict() || e.is_quota() => return Err(e.into()),
                _ => failed.extend(
                    bso_ids
                        .clone()
                        .into_iter()
                        .map(|id| (id, "db error".to_owned())),
                ),
            };
        };
    }

    // If we're not committing the current set of records yet.
    if !breq.commit {
        // and there are bsos included in this message.
        if !coll.bsos.valid.is_empty() {
            // Append the data to the requested batch.
            let result = {
                trace!("Batch: Appending to {}", &new_batch.id);
                db.append_to_batch(params::AppendToBatch {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    batch: new_batch.clone(),
                    bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                })
                .await
            };
            handle_result!(result);
        }

        // Return the batch append response without committing the current
        // batch to the BSO table.
        resp["success"] = json!(success);
        resp["failed"] = json!(failed);

        resp["batch"] = json!(&new_batch.id);
        return Ok(HttpResponse::Accepted().json(resp));
    }

    // We've been asked to commit the accumulated data, so get to it!
    let batch = db
        .get_batch(params::GetBatch {
            user_id: user_id.clone(),
            collection: collection.clone(),
            id: new_batch.id,
        })
        .await?;

    // TODO: validate *actual* sizes of the batch items
    // (max_total_records, max_total_bytes)
    //
    // First, write the pending batch BSO data into the BSO table.
    let modified = if let Some(batch) = batch {
        db.commit_batch(params::CommitBatch {
            user_id: user_id.clone(),
            collection: collection.clone(),
            batch,
        })
        .await?
    } else {
        return Err(ApiErrorKind::Db(DbError::batch_not_found()).into());
    };

    // Then, write the BSOs contained in the commit request into the BSO table.
    // (This presumes that the BSOs contained in the final "commit" message are
    // newer, and thus more "correct", than any prior BSO info that may have been
    // included in the prior batch creation messages. The client shouldn't really
    // be including BSOs with the commit message, however it does and we should
    // handle that case.)
    if !coll.bsos.valid.is_empty() {
        trace!("Batch: writing commit message bsos");
        let result = db
            .post_bsos(params::PostBsos {
                user_id: coll.user_id.clone(),
                collection: coll.collection.clone(),
                bsos: coll
                    .bsos
                    .valid
                    .into_iter()
                    .map(|batch_bso| params::PostCollectionBso {
                        id: batch_bso.id,
                        sortindex: batch_bso.sortindex,
                        payload: batch_bso.payload,
                        ttl: batch_bso.ttl,
                    })
                    .collect(),
                for_batch: false,
            })
            .await
            .map(|_| ());

        handle_result!(result);
    }

    // Always return success, failed, & modified
    resp["success"] = json!(success);
    resp["failed"] = json!(failed);
    resp["modified"] = json!(modified);
    trace!("Batch: Returning result: {}", &resp);
    Ok(HttpResponse::build(StatusCode::OK)
        .insert_header((X_LAST_MODIFIED, modified.as_header()))
        .json(resp))
}

pub async fn delete_bso(
    bso_req: BsoRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            bso_req.emit_api_metric("request.delete_bso");
            let result = db
                .delete_bso(params::DeleteBso {
                    user_id: bso_req.user_id,
                    collection: bso_req.collection,
                    id: bso_req.bso,
                })
                .await?;
            Ok(HttpResponse::Ok().json(json!({ "modified": result })))
        })
        .await
}

pub async fn get_bso(
    bso_req: BsoRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            bso_req.emit_api_metric("request.get_bso");
            let result = db
                .get_bso(params::GetBso {
                    user_id: bso_req.user_id,
                    collection: bso_req.collection,
                    id: bso_req.bso,
                })
                .await?;

            Ok(result.map_or_else(
                || HttpResponse::NotFound().finish(),
                |bso| HttpResponse::Ok().json(bso),
            ))
        })
        .await
}

pub async fn put_bso(
    bso_req: BsoPutRequest,
    db_pool: DbTransactionPool,
    request: HttpRequest,
) -> Result<HttpResponse, ApiError> {
    db_pool
        .transaction_http(&request, async |db| {
            bso_req.emit_api_metric("request.put_bso");
            let result = db
                .put_bso(params::PutBso {
                    user_id: bso_req.user_id,
                    collection: bso_req.collection,
                    id: bso_req.bso,
                    sortindex: bso_req.body.sortindex,
                    payload: bso_req.body.payload,
                    ttl: bso_req.body.ttl,
                })
                .await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .insert_header((X_LAST_MODIFIED, result.as_header()))
                .json(result))
        })
        .await
}

pub async fn get_configuration(state: Data<ServerState>) -> HttpResponse {
    // With no DbConnection (via a `transaction_http` call) needed here, we
    // miss out on a couple things it does:
    // 1. Ensuring an X-Last-Modified (always 0.00) is returned
    // 2. Handling precondition checks
    // The precondition checks don't make sense against hardcoded to the
    // service limits data + a 0.00 timestamp, so just ensure #1 is handled
    HttpResponse::Ok()
        .insert_header((X_LAST_MODIFIED, "0.00"))
        .content_type("application/json")
        .body(state.limits_json.clone())
}

/** Returns a status message indicating the state of the current server
 *
 */
pub async fn heartbeat(hb: HeartbeatRequest) -> Result<HttpResponse, ApiError> {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );
    let mut db = hb.db_pool.get().await?;

    checklist.insert("quota".to_owned(), serde_json::to_value(hb.quota)?);

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

pub async fn lbheartbeat(req: HttpRequest) -> Result<HttpResponse, ApiError> {
    let mut resp: HashMap<String, Value> = HashMap::new();

    let state = match req.app_data::<Data<ServerState>>() {
        Some(s) => s,
        None => {
            error!("⚠️ Could not load the app state");
            return Ok(HttpResponse::InternalServerError().finish());
        }
    };

    let deadarc = state.deadman.clone();
    let mut deadman = *deadarc.read().await;
    if matches!(deadman.expiry, Some(expiry) if expiry <= Instant::now()) {
        // We're set to report a failed health check after a certain time (to
        // evict this instance and start a fresh one)
        return Ok(HttpResponseBuilder::new(StatusCode::INTERNAL_SERVER_ERROR).json(resp));
    }

    let db_state = if cfg!(test) {
        use actix_web::http::header::HeaderValue;
        use std::str::FromStr;
        use syncstorage_db::PoolState;

        let test_pool = PoolState {
            connections: u32::from_str(
                req.headers()
                    .get("TEST_CONNECTIONS")
                    .unwrap_or(&HeaderValue::from_static("0"))
                    .to_str()
                    .unwrap_or("0"),
            )
            .unwrap_or_default(),
            idle_connections: u32::from_str(
                req.headers()
                    .get("TEST_IDLES")
                    .unwrap_or(&HeaderValue::from_static("0"))
                    .to_str()
                    .unwrap_or("0"),
            )
            .unwrap_or_default(),
        };
        test_pool
    } else {
        state.db_pool.clone().state()
    };

    let active = db_state.connections - db_state.idle_connections;
    let mut status_code = StatusCode::OK;

    if active >= deadman.max_size && db_state.idle_connections == 0 {
        if deadman.clock_start.is_none() {
            deadman.clock_start = Some(Instant::now());
        }
        status_code = StatusCode::INTERNAL_SERVER_ERROR;
    } else if deadman.clock_start.is_some() {
        deadman.clock_start = None
    }
    deadman.previous_count = db_state.idle_connections as usize;
    {
        *deadarc.write().await = deadman;
    }
    resp.insert("active_connections".to_string(), Value::from(active));
    resp.insert(
        "idle_connections".to_string(),
        Value::from(db_state.idle_connections),
    );
    if let Some(clock) = deadman.clock_start {
        let duration: Duration = Instant::now() - clock;
        resp.insert("duration_ms".to_string(), Value::from(duration.as_millis()));
    };

    Ok(HttpResponseBuilder::new(status_code).json(json!(resp)))
}

// try returning an API error
pub async fn test_error(
    _req: HttpRequest,
    _ter: TestErrorRequest,
) -> Result<HttpResponse, ApiError> {
    // generate an error for sentry.

    // ApiError will call the middleware layer to auto-append the tags.
    error!("Test Error");
    let err = ApiError::from(ApiErrorKind::Internal("Oh Noes!".to_owned()));

    Err(err)
}
