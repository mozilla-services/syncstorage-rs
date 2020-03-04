//! API Handlers
use std::collections::HashMap;

use actix_web::{http::StatusCode, Error, HttpRequest, HttpResponse};
use futures::future::{self, Either, Future, FutureExt, LocalBoxFuture, TryFutureExt};
use serde::Serialize;
use serde_json::{json, Value};

use crate::db::{params, results::Paginated, util::SyncTimestamp, DbError, DbErrorKind};
use crate::error::{ApiError, ApiErrorKind};
use crate::web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, ConfigRequest,
    HeartbeatRequest, MetaRequest, ReplyFormat, TestErrorRequest,
};
use crate::web::{X_LAST_MODIFIED, X_WEAVE_NEXT_OFFSET, X_WEAVE_RECORDS};

pub const ONE_KB: f64 = 1024.0;

pub fn get_collections(meta: MetaRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    meta.metrics.incr("request.get_collections");
    meta.db
        .get_collection_timestamps(meta.user_id)
        .map_err(From::from)
        .map_ok(|result| {
            HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, result.len().to_string())
                .json(result)
        })
}

pub fn get_collection_counts(
    meta: MetaRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    meta.metrics.incr("request.get_collection_counts");
    meta.db
        .get_collection_counts(meta.user_id)
        .map_err(From::from)
        .map_ok(|result| {
            HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, result.len().to_string())
                .json(result)
        })
}

pub fn get_collection_usage(
    meta: MetaRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    meta.metrics.incr("request.get_collection_usage");
    meta.db
        .get_collection_usage(meta.user_id)
        .map_err(From::from)
        .map_ok(|usage| {
            let usage: HashMap<_, _> = usage
                .into_iter()
                .map(|(coll, size)| (coll, size as f64 / ONE_KB))
                .collect();
            HttpResponse::build(StatusCode::OK)
                .header(X_WEAVE_RECORDS, usage.len().to_string())
                .json(usage)
        })
}

pub fn get_quota(meta: MetaRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    meta.metrics.incr("request.get_quota");
    meta.db
        .get_storage_usage(meta.user_id)
        .map_err(From::from)
        .map_ok(|usage| HttpResponse::Ok().json(vec![Some(usage as f64 / ONE_KB), None]))
}

pub fn delete_all(meta: MetaRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    #![allow(clippy::unit_arg)]
    meta.metrics.incr("request.delete_all");
    meta.db
        .delete_storage(meta.user_id)
        .map_err(From::from)
        .map_ok(|result| HttpResponse::Ok().json(result))
}

pub fn delete_collection(
    coll: CollectionRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    let delete_bsos = !coll.query.ids.is_empty();
    let metrics = coll.metrics.clone();
    let fut = if delete_bsos {
        metrics.incr("request.delete_bsos");
        coll.db.delete_bsos(params::DeleteBsos {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            ids: coll.query.ids.clone(),
        })
    } else {
        metrics.incr("request.delete_collection");
        coll.db.delete_collection(params::DeleteCollection {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
        })
    };

    fut.or_else(move |e| {
        if e.is_collection_not_found() || e.is_bso_not_found() {
            coll.db.get_storage_timestamp(coll.user_id)
        } else {
            Box::pin(future::err(e))
        }
    })
    .map_err(From::from)
    .map_ok(move |result| {
        HttpResponse::Ok()
            .if_true(delete_bsos, |resp| {
                resp.header(X_LAST_MODIFIED, result.as_header());
            })
            .json(result)
    })
}

pub fn get_collection(
    coll: CollectionRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    coll.metrics.clone().incr("request.get_collection");
    let params = params::GetBsos {
        user_id: coll.user_id.clone(),
        params: coll.query.clone(),
        collection: coll.collection.clone(),
    };
    if coll.query.full {
        let fut = coll.db.get_bsos(params);
        Either::Left(finish_get_collection(coll, fut))
    } else {
        // Changed to be a Paginated list of BSOs, need to extract IDs from them.
        let fut = coll.db.get_bso_ids(params);
        Either::Right(finish_get_collection(coll, fut))
    }
}

fn finish_get_collection<F, T>(
    coll: CollectionRequest,
    fut: F,
) -> LocalBoxFuture<'static, Result<HttpResponse, Error>>
where
    F: Future<Output = Result<Paginated<T>, ApiError>> + 'static,
    T: Serialize + Default + 'static,
{
    let reply_format = coll.reply;
    Box::pin(
        fut.or_else(move |e| {
            if e.is_collection_not_found() {
                // For b/w compat, non-existent collections must return an
                // empty list
                future::ok(Paginated::default())
            } else {
                future::err(e)
            }
        })
        .map_err(From::from)
        .and_then(|result| {
            coll.db
                .extract_resource(coll.user_id, Some(coll.collection), None)
                .map_err(From::from)
                .map_ok(move |ts| (result, ts))
        })
        .map_ok(move |(result, ts): (Paginated<T>, SyncTimestamp)| {
            let mut builder = HttpResponse::build(StatusCode::OK);
            let resp = builder
                .header(X_LAST_MODIFIED, ts.as_header())
                .header(X_WEAVE_RECORDS, result.items.len().to_string())
                .if_some(result.offset, |offset, resp| {
                    resp.header(X_WEAVE_NEXT_OFFSET, offset);
                });
            match reply_format {
                ReplyFormat::Json => resp.json(result.items),
                ReplyFormat::Newlines => {
                    let items: String = result
                        .items
                        .into_iter()
                        .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "".to_string()))
                        .filter(|v| !v.is_empty())
                        .map(|v| v.replace("\n", "\\u000a") + "\n")
                        .collect();
                    resp.header("Content-Type", "application/newlines")
                        .header("Content-Length", format!("{}", items.len()))
                        .body(items)
                }
            }
        }),
    )
}

pub fn post_collection(
    coll: CollectionPostRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    coll.metrics.clone().incr("request.post_collection");
    if coll.batch.is_some() {
        return Either::Left(post_collection_batch(coll));
    }
    Either::Right(
        coll.db
            .post_bsos(params::PostBsos {
                user_id: coll.user_id,
                collection: coll.collection,
                bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                failed: coll.bsos.invalid,
            })
            .map_err(From::from)
            .map_ok(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header(X_LAST_MODIFIED, result.modified.as_header())
                    .json(result)
            }),
    )
}

pub fn post_collection_batch(
    coll: CollectionPostRequest,
) -> impl Future<Output = Result<HttpResponse, Error>> {
    coll.metrics.clone().incr("request.post_collection_batch");
    // Bail early if we have nonsensical arguments
    let breq = match coll.batch.clone() {
        Some(breq) => breq,
        None => {
            let err: DbError = DbErrorKind::BatchNotFound.into();
            let err: ApiError = err.into();
            return Either::Left(future::err(err.into()));
        }
    };

    let fut = if let Some(id) = breq.id.clone() {
        // Validate the batch before attempting a full append (for efficiency)
        Either::Left(
            coll.db
                .validate_batch(params::ValidateBatch {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    id: id.clone(),
                })
                .and_then(move |is_valid| {
                    if is_valid {
                        future::ok(id)
                    } else {
                        let err: DbError = DbErrorKind::BatchNotFound.into();
                        future::err(err.into())
                    }
                }),
        )
    } else {
        Either::Right(coll.db.create_batch(params::CreateBatch {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            bsos: vec![],
        }))
    };

    let commit = breq.commit;
    let db = coll.db.clone();
    let user_id = coll.user_id.clone();
    let collection = coll.collection.clone();

    Either::Right(
        fut.and_then(move |id| {
            let mut success = vec![];
            let mut failed = coll.bsos.invalid.clone();
            let bso_ids: Vec<_> = coll.bsos.valid.iter().map(|bso| bso.id.clone()).collect();

            if commit && !coll.bsos.valid.is_empty() {
                // There's pending items to append to the batch but since we're
                // committing, write them to bsos immediately. Otherwise under
                // Spanner we would pay twice the mutations for those pending
                // items (once writing them to to batch_bsos, then again
                // writing them to bsos)
                Either::Left(
                    coll.db
                        .post_bsos(params::PostBsos {
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
                        .and_then(|_| future::ok(())),
                )
            } else {
                Either::Right(coll.db.append_to_batch(params::AppendToBatch {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    id: id.clone(),
                    bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                }))
            }
            .then(move |result| {
                match result {
                    Ok(_) => success.extend(bso_ids),
                    Err(e) if e.is_conflict() => return future::err(e),
                    Err(_) => {
                        failed.extend(bso_ids.into_iter().map(|id| (id, "db error".to_owned())))
                    }
                };
                future::ok((id, success, failed))
            })
        })
        .map_err(From::from)
        .and_then(move |(id, success, failed)| {
            let mut resp = json!({
                "success": success,
                "failed": failed,
            });

            if !breq.commit {
                resp["batch"] = json!(&id);
                return Either::Left(future::ok(HttpResponse::Accepted().json(resp)));
            }

            let fut = db
                .get_batch(params::GetBatch {
                    user_id: user_id.clone(),
                    collection: collection.clone(),
                    id,
                })
                .and_then(move |batch| {
                    // TODO: validate *actual* sizes of the batch items
                    // (max_total_records, max_total_bytes)
                    if let Some(batch) = batch {
                        db.commit_batch(params::CommitBatch {
                            user_id: user_id.clone(),
                            collection: collection.clone(),
                            batch,
                        })
                    } else {
                        let err: DbError = DbErrorKind::BatchNotFound.into();
                        Box::pin(future::err(err.into()))
                    }
                })
                .map_err(From::from)
                .map_ok(|result| {
                    resp["modified"] = json!(result.modified);
                    HttpResponse::build(StatusCode::OK)
                        .header(X_LAST_MODIFIED, result.modified.as_header())
                        .json(resp)
                });
            Either::Right(fut)
        }),
    )
}

pub fn delete_bso(bso_req: BsoRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    bso_req.metrics.incr("request.delete_bso");
    bso_req
        .db
        .delete_bso(params::DeleteBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
        })
        .map_err(From::from)
        .map(|result: Result<SyncTimestamp, Error>| match result {
            Ok(result) => Ok(HttpResponse::Ok().json(json!({ "modified": result }))),
            Err(_e) => Ok(HttpResponse::NotFound().finish()),
        })
}

pub fn get_bso(bso_req: BsoRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    bso_req.metrics.incr("request.get_bso");
    bso_req
        .db
        .get_bso(params::GetBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
        })
        .map_err(From::from)
        .map_ok(|result| {
            result.map_or_else(
                || HttpResponse::NotFound().finish(),
                |bso| HttpResponse::Ok().json(bso),
            )
        })
}

pub fn put_bso(bso_req: BsoPutRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    bso_req.metrics.incr("request.put_bso");
    bso_req
        .db
        .put_bso(params::PutBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
            sortindex: bso_req.body.sortindex,
            payload: bso_req.body.payload,
            ttl: bso_req.body.ttl,
        })
        .map_err(From::from)
        .map_ok(|result| {
            HttpResponse::build(StatusCode::OK)
                .header(X_LAST_MODIFIED, result.as_header())
                .json(result)
        })
}

pub fn get_configuration(creq: ConfigRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    future::ready(Ok(HttpResponse::Ok().json(creq.limits)))
}

/** Returns a status message indicating the state of the current server
 *
 */
pub fn heartbeat(hb: HeartbeatRequest) -> impl Future<Output = Result<HttpResponse, Error>> {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );
    hb.db.check().then(|response| match response {
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
            HttpResponse::Ok().json(checklist)
        }
        Err(e) => {
            error!("Heartbeat error: {:?}", e);
            checklist.insert("status".to_owned(), Value::from("Err"));
            checklist.insert("database".to_owned(), Value::from("Unknown"));
            HttpResponse::ServiceUnavailable().json(checklist)
        }
    })
}

pub fn test_error(
    _req: HttpRequest,
    ter: TestErrorRequest,
) -> impl Future<Output = Result<HttpResponse, ApiError>> {
    // generate an error for sentry.

    /*  The various error log macros only can take a string.
        Additional values can be expressed as KV (key value) after a `;`
        e.g.
        ```
        error!("Something Bad {:?}", err;
                "ua.os.family" => wtags.get("ua.os.family"),
                "ua.browser.family" => wtags.get("ua.browser.family"),
                "ua.name" => wtags.get("ua.name"),
                "ua.os.ver" => wtags.get("ua.os.ver"),
                "ua.browser.ver" => wtags.get("ua.browser.ver"))
        ```
        TODO: find some way to transform Tags into error::KV
    */
    error!("Test Error: {:?}", &ter.tags);

    // ApiError will call the middleware layer to auto-append the tags.
    let err = ApiError::from(ApiErrorKind::Internal("Oh Noes!".to_owned()));

    future::ready(Err(err))
}
