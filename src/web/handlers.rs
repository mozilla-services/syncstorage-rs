//! API Handlers
use std::collections::HashMap;

use actix_web::{http::StatusCode, web::Data, Error, HttpRequest, HttpResponse};
use serde::Serialize;
use serde_json::{json, Value};

use crate::{
    db::{
        params,
        results::{CreateBatch, Paginated},
        transaction::DbTransactionPool,
        util::SyncTimestamp,
        Db, DbError, DbErrorKind,
    },
    error::{ApiError, ApiErrorKind, ApiResult},
    server::ServerState,
    web::{
        extractors::{
            BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HeartbeatRequest,
            MetaRequest, ReplyFormat, TestErrorRequest,
        },
        X_LAST_MODIFIED, X_WEAVE_NEXT_OFFSET, X_WEAVE_RECORDS,
    },
};

pub const ONE_KB: f64 = 1024.0;

pub async fn get_collections(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            meta.metrics.incr("request.get_collections");
            let result = db.get_collection_timestamps(meta.user_id).await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, result.len().to_string())
                .json(result))
        })
        .await
}

pub async fn get_collection_counts(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            meta.metrics.incr("request.get_collection_counts");
            let result = db.get_collection_counts(meta.user_id).await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, result.len().to_string())
                .json(result))
        })
        .await
}

pub async fn get_collection_usage(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            meta.metrics.incr("request.get_collection_usage");
            let usage: HashMap<_, _> = db
                .get_collection_usage(meta.user_id)
                .await?
                .into_iter()
                .map(|(coll, size)| (coll, size as f64 / ONE_KB))
                .collect();

            Ok(HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, usage.len().to_string())
                .json(usage))
        })
        .await
}

pub async fn get_quota(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            meta.metrics.incr("request.get_quota");
            let usage = db.get_storage_usage(meta.user_id).await?;
            Ok(HttpResponse::Ok().json(vec![Some(usage as f64 / ONE_KB), None]))
        })
        .await
}

pub async fn delete_all(
    meta: MetaRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            meta.metrics.incr("request.delete_all");
            Ok(HttpResponse::Ok().json(db.delete_storage(meta.user_id).await?))
        })
        .await
}

pub async fn delete_collection(
    coll: CollectionRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            let delete_bsos = !coll.query.ids.is_empty();
            let metrics = coll.metrics.clone();
            let timestamp: ApiResult<SyncTimestamp> = if delete_bsos {
                metrics.incr("request.delete_bsos");
                db.delete_bsos(params::DeleteBsos {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    ids: coll.query.ids.clone(),
                })
                .await
            } else {
                metrics.incr("request.delete_collection");
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

            Ok(HttpResponse::Ok()
                .if_true(delete_bsos, |resp| {
                    resp.header(X_LAST_MODIFIED, timestamp.as_header());
                })
                .json(timestamp))
        })
        .await
}

pub async fn get_collection(
    coll: CollectionRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            coll.metrics.clone().incr("request.get_collection");
            let params = params::GetBsos {
                user_id: coll.user_id.clone(),
                params: coll.query.clone(),
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
    db: Box<dyn Db<'_> + '_>,
    result: Result<Paginated<T>, ApiError>,
) -> Result<HttpResponse, Error>
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
        .header(X_LAST_MODIFIED, ts.as_header())
        .header(X_WEAVE_RECORDS, result.items.len().to_string())
        .if_some(result.offset, |offset, resp| {
            resp.header(X_WEAVE_NEXT_OFFSET, offset);
        });

    match coll.reply {
        ReplyFormat::Json => Ok(resp.json(result.items)),
        ReplyFormat::Newlines => {
            let items: String = result
                .items
                .into_iter()
                .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "".to_string()))
                .filter(|v| !v.is_empty())
                .map(|v| v.replace("\n", "\\u000a") + "\n")
                .collect();

            Ok(resp
                .header("Content-Type", "application/newlines")
                .header("Content-Length", format!("{}", items.len()))
                .body(items))
        }
    }
}

pub async fn post_collection(
    coll: CollectionPostRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            coll.metrics.clone().incr("request.post_collection");

            if coll.batch.is_some() {
                return post_collection_batch(coll, db).await;
            }

            let result = db
                .post_bsos(params::PostBsos {
                    user_id: coll.user_id,
                    collection: coll.collection,
                    bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                    failed: coll.bsos.invalid,
                })
                .await?;

            Ok(HttpResponse::build(StatusCode::OK)
                .header(X_LAST_MODIFIED, result.modified.as_header())
                .json(result))
        })
        .await
}

pub async fn post_collection_batch(
    coll: CollectionPostRequest,
    db: Box<dyn Db<'_> + '_>,
) -> Result<HttpResponse, Error> {
    coll.metrics.clone().incr("request.post_collection_batch");
    // Bail early if we have nonsensical arguments
    let breq = match coll.batch.clone() {
        Some(breq) => breq,
        None => {
            let err: DbError = DbErrorKind::BatchNotFound.into();
            let err: ApiError = err.into();
            return Err(err.into());
        }
    };

    let new_batch = if let Some(id) = breq.id.clone() {
        // Validate the batch before attempting a full append (for efficiency)
        let is_valid = db
            .validate_batch(params::ValidateBatch {
                user_id: coll.user_id.clone(),
                collection: coll.collection.clone(),
                id: id.clone(),
            })
            .await?;

        if is_valid {
            let collection_id = db.get_collection_id(coll.collection.clone()).await?;
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
                    Some(usage.total_bytes as usize)
                } else {
                    None
                },
            }
        } else {
            let err: DbError = DbErrorKind::BatchNotFound.into();
            return Err(ApiError::from(err).into());
        }
    } else {
        db.create_batch(params::CreateBatch {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            bsos: vec![],
        })
        .await?
    };

    let commit = breq.commit;
    let user_id = coll.user_id.clone();
    let collection = coll.collection.clone();

    let mut success = vec![];
    let mut failed = coll.bsos.invalid;
    let bso_ids: Vec<_> = coll.bsos.valid.iter().map(|bso| bso.id.clone()).collect();

    let result = if commit && !coll.bsos.valid.is_empty() {
        // There's pending items to append to the batch but since we're
        // committing, write them to bsos immediately. Otherwise under
        // Spanner we would pay twice the mutations for those pending
        // items (once writing them to to batch_bsos, then again
        // writing them to bsos)
        db.post_bsos(params::PostBsos {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            // XXX: why does BatchBsoBody exist (it's the same struct
            // as PostCollectionBso)?
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
            failed: Default::default(),
        })
        .await
        .map(|_| ())
    } else {
        db.append_to_batch(params::AppendToBatch {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            batch: new_batch.clone(),
            bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
        })
        .await
    };

    match result {
        Ok(_) => success.extend(bso_ids),
        Err(e) if e.is_conflict() => return Err(e.into()),
        Err(_) => failed.extend(bso_ids.into_iter().map(|id| (id, "db error".to_owned()))),
    };

    let mut resp = json!({
        "success": success,
        "failed": failed,
    });

    if !breq.commit {
        resp["batch"] = json!(&new_batch.id);
        return Ok(HttpResponse::Accepted().json(resp));
    }

    let batch = db
        .get_batch(params::GetBatch {
            user_id: user_id.clone(),
            collection: collection.clone(),
            id: new_batch.id,
        })
        .await?;

    // TODO: validate *actual* sizes of the batch items
    // (max_total_records, max_total_bytes)
    let result = if let Some(batch) = batch {
        db.commit_batch(params::CommitBatch {
            user_id: user_id.clone(),
            collection: collection.clone(),
            batch,
        })
        .await?
    } else {
        let err: DbError = DbErrorKind::BatchNotFound.into();
        return Err(ApiError::from(err).into());
    };

    resp["modified"] = json!(result.modified);
    Ok(HttpResponse::build(StatusCode::OK)
        .header(X_LAST_MODIFIED, result.modified.as_header())
        .json(resp))
}

pub async fn delete_bso(
    bso_req: BsoRequest,
    db_pool: DbTransactionPool,
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            bso_req.metrics.incr("request.delete_bso");
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
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            bso_req.metrics.incr("request.get_bso");
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
) -> Result<HttpResponse, Error> {
    db_pool
        .transaction_http(|db| async move {
            bso_req.metrics.incr("request.put_bso");
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
                .header(X_LAST_MODIFIED, result.as_header())
                .json(result))
        })
        .await
}

pub fn get_configuration(state: Data<ServerState>) -> HttpResponse {
    // With no DbConnection (via a `transaction_http` call) needed here, we
    // miss out on a couple things it does:
    // 1. Ensuring an X-Last-Modified (always 0.00) is returned
    // 2. Handling precondition checks
    // The precondition checks don't make sense against hardcoded to the
    // service limits data + a 0.00 timestamp, so just ensure #1 is handled
    HttpResponse::Ok()
        .header(X_LAST_MODIFIED, "0.00")
        .content_type("application/json")
        .body(&state.limits_json)
}

/** Returns a status message indicating the state of the current server
 *
 */
pub async fn heartbeat(hb: HeartbeatRequest) -> Result<HttpResponse, Error> {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );
    let db = hb.db_pool.get().await?;

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

// try returning an API error
pub async fn test_error(
    _req: HttpRequest,
    ter: TestErrorRequest,
) -> Result<HttpResponse, ApiError> {
    // generate an error for sentry.

    /*  The various error log macros only can take a string.
        Content of Tags struct can be logged as KV (key value) pairs after a `;`.
        e.g.
        ```
        error!("Something Bad {:?}", err; wtags)
        ```

        TODO: find some way to transform Tags into error::KV
    */
    error!("Test Error: {:?}", &ter.tags);

    // ApiError will call the middleware layer to auto-append the tags.
    let err = ApiError::from(ApiErrorKind::Internal("Oh Noes!".to_owned()));

    Err(err)
}
