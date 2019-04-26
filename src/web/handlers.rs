//! API Handlers
use std::collections::HashMap;

use actix_web::{http::StatusCode, FutureResponse, HttpResponse, State};
use futures::future::{self, Either, Future};
use serde::Serialize;

use db::{params, results::Paginated, DbError, DbErrorKind};
use error::ApiError;
use server::ServerState;
use web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HawkIdentifier,
    MetaRequest, ReplyFormat,
};

pub const ONE_KB: f64 = 1024.0;

pub fn get_collections(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_collection_timestamps(meta.user_id)
            .map_err(From::from)
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Weave-Records", result.len().to_string())
                    .json(result)
            }),
    )
}

pub fn get_collection_counts(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_collection_counts(meta.user_id)
            .map_err(From::from)
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Weave-Records", result.len().to_string())
                    .json(result)
            }),
    )
}

pub fn get_collection_usage(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_collection_usage(meta.user_id)
            .map_err(From::from)
            .map(|usage| {
                let usage: HashMap<_, _> = usage
                    .into_iter()
                    .map(|(coll, size)| (coll, size as f64 / ONE_KB))
                    .collect();
                HttpResponse::build(StatusCode::OK)
                    .header("X-Weave-Records", usage.len().to_string())
                    .json(usage)
            }),
    )
}

pub fn get_quota(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_storage_usage(meta.user_id)
            .map_err(From::from)
            .map(|usage| HttpResponse::Ok().json(vec![Some(usage as f64 / ONE_KB), None])),
    )
}

pub fn delete_all(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .delete_storage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_collection(coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    let delete_bsos = !coll.query.ids.is_empty();
    let fut = if delete_bsos {
        coll.db.delete_bsos(params::DeleteBsos {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            ids: coll.query.ids.clone(),
        })
    } else {
        coll.db.delete_collection(params::DeleteCollection {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
        })
    };

    Box::new(
        fut.or_else(move |e| {
            if e.is_colllection_not_found() || e.is_bso_not_found() {
                coll.db.get_storage_timestamp(coll.user_id)
            } else {
                Box::new(future::err(e))
            }
        })
        .map_err(From::from)
        .map(move |result| {
            HttpResponse::Ok()
                .if_true(delete_bsos, |resp| {
                    resp.header("X-Last-Modified", result.as_header());
                })
                .json(result)
        }),
    )
}

pub fn get_collection(coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    let params = params::GetBsos {
        user_id: coll.user_id.clone(),
        collection: coll.collection.clone(),
        params: coll.query.clone(),
    };
    if coll.query.full {
        let fut = coll.db.get_bsos(params);
        finish_get_collection(coll, fut)
    } else {
        let fut = coll.db.get_bso_ids(params);
        finish_get_collection(coll, fut)
    }
}

fn finish_get_collection<F, T>(coll: CollectionRequest, fut: F) -> FutureResponse<HttpResponse>
where
    F: Future<Item = Paginated<T>, Error = ApiError> + 'static,
    T: Serialize + Default + 'static,
{
    let reply_format = coll.reply;
    Box::new(
        fut.or_else(move |e| {
            if e.is_colllection_not_found() {
                // For b/w compat, non-existent collections must return an
                // empty list
                Ok(Paginated::default())
            } else {
                Err(e)
            }
        })
        .map_err(From::from)
        .and_then(|result| {
            coll.db
                .extract_resource(coll.user_id, Some(coll.collection), None)
                .map_err(From::from)
                .map(move |ts| (result, ts))
        })
        .map(move |(result, ts)| {
            let mut builder = HttpResponse::build(StatusCode::OK);
            let resp = builder
                .header("X-Last-Modified", ts.as_header())
                .header("X-Weave-Records", result.items.len().to_string())
                .if_some(result.offset, |offset, resp| {
                    resp.header("X-Weave-Next-Offset", offset.to_string());
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

pub fn post_collection(coll: CollectionPostRequest) -> FutureResponse<HttpResponse> {
    if coll.batch.is_some() {
        return post_collection_batch(coll);
    }
    Box::new(
        coll.db
            .post_bsos(params::PostBsos {
                user_id: coll.user_id,
                collection: coll.collection,
                bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                failed: coll.bsos.invalid,
            })
            .map_err(From::from)
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Last-Modified", result.modified.as_header())
                    .json(result)
            }),
    )
}

pub fn post_collection_batch(coll: CollectionPostRequest) -> FutureResponse<HttpResponse> {
    // Bail early if we have nonsensical arguments
    let breq = match coll.batch.clone() {
        Some(breq) => breq,
        None => {
            let err: DbError = DbErrorKind::BatchNotFound.into();
            let err: ApiError = err.into();
            return Box::new(future::err(err.into()));
        }
    };

    let fut = if let Some(id) = breq.id {
        // Validate the batch before attempting a full append (for efficiency)
        Either::A(
            coll.db
                .validate_batch(params::ValidateBatch {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    id,
                })
                .and_then(move |is_valid| {
                    if is_valid {
                        Box::new(future::ok(id))
                    } else {
                        let err: DbError = DbErrorKind::BatchNotFound.into();
                        Box::new(future::err(err.into()))
                    }
                }),
        )
    } else {
        Either::B(coll.db.create_batch(params::CreateBatch {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            bsos: vec![],
        }))
    };

    let db = coll.db.clone();
    let user_id = coll.user_id.clone();
    let collection = coll.collection.clone();

    let fut = fut
        .and_then(move |id| {
            let mut success = vec![];
            let mut failed = coll.bsos.invalid.clone();
            let bso_ids: Vec<_> = coll.bsos.valid.iter().map(|bso| bso.id.clone()).collect();

            coll.db
                .append_to_batch(params::AppendToBatch {
                    user_id: coll.user_id.clone(),
                    collection: coll.collection.clone(),
                    id,
                    bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                })
                .then(move |result| {
                    match result {
                        Ok(_) => success.extend(bso_ids),
                        Err(e) => {
                            // NLL: not a guard as: (E0008) "moves value into
                            // pattern guard"
                            if e.is_conflict() {
                                return future::err(e);
                            }
                            failed.extend(bso_ids.into_iter().map(|id| (id, "db error".to_owned())))
                        }
                    };
                    future::ok((id, success, failed))
                })
        })
        .map_err(From::from);

    Box::new(fut.and_then(move |(id, success, failed)| {
        let mut resp = json!({
            "success": success,
            "failed": failed,
        });

        if !breq.commit {
            resp["batch"] = json!(base64::encode(&id.to_string()));
            return Either::A(future::ok(HttpResponse::Accepted().json(resp)));
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
                    Box::new(future::err(err.into()))
                }
            })
            .map_err(From::from)
            .map(|result| {
                resp["modified"] = json!(result.modified);
                HttpResponse::build(StatusCode::OK)
                    .header("X-Last-Modified", result.modified.as_header())
                    .json(resp)
            });
        Either::B(fut)
    }))
}

pub fn delete_bso(bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        bso_req
            .db
            .delete_bso(params::DeleteBso {
                user_id: bso_req.user_id,
                collection: bso_req.collection,
                id: bso_req.bso,
            })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(json!({ "modified": result }))),
    )
}

pub fn get_bso(bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        bso_req
            .db
            .get_bso(params::GetBso {
                user_id: bso_req.user_id,
                collection: bso_req.collection,
                id: bso_req.bso,
            })
            .map_err(From::from)
            .map(|result| {
                result.map_or_else(
                    || HttpResponse::NotFound().finish(),
                    |bso| HttpResponse::Ok().json(bso),
                )
            }),
    )
}

pub fn put_bso(bso_req: BsoPutRequest) -> FutureResponse<HttpResponse> {
    Box::new(
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
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Last-Modified", result.as_header())
                    .json(result)
            }),
    )
}

pub fn get_configuration(
    (_auth, state): (HawkIdentifier, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(future::result(Ok(HttpResponse::Ok().json(&*state.limits))))
}
