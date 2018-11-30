//! API Handlers
use std::collections::HashMap;

use actix_web::{http::StatusCode, FutureResponse, HttpResponse, State};
use futures::future::{self, Future};
use serde::Serialize;

use db::{params, results::Paginated};
use error::ApiError;
use server::ServerState;
use web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HawkIdentifier,
    MetaRequest,
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
        }).map_err(From::from)
        .map(move |result| {
            HttpResponse::Ok()
                .if_true(delete_bsos, |resp| {
                    resp.header("X-Last-Modified", result.as_header());
                }).json(result)
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
    Box::new(
        fut.or_else(move |e| {
            if e.is_colllection_not_found() {
                // For b/w compat, non-existent collections must return an
                // empty list
                Ok(Paginated::default())
            } else {
                Err(e)
            }
        }).map_err(From::from)
        .and_then(|result| {
            coll.db
                .extract_resource(coll.user_id, Some(coll.collection), None)
                .map_err(From::from)
                .map(move |ts| (result, ts))
        }).map(|(result, ts)| {
            HttpResponse::build(StatusCode::OK)
                .header("X-Last-Modified", ts.as_header())
                .header("X-Weave-Records", result.items.len().to_string())
                .if_some(result.offset, |offset, resp| {
                    resp.header("X-Weave-Next-Offset", offset.to_string());
                }).json(result.items)
        }),
    )
}

pub fn post_collection(coll: CollectionPostRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        coll.db
            .post_bsos(params::PostBsos {
                user_id: coll.user_id,
                collection: coll.collection,
                bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
                failed: coll.bsos.invalid,
            }).map_err(From::from)
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Last-Modified", result.modified.as_header())
                    .json(result)
            }),
    )
}

pub fn delete_bso(bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        bso_req
            .db
            .delete_bso(params::DeleteBso {
                user_id: bso_req.user_id,
                collection: bso_req.collection,
                id: bso_req.bso,
            }).map_err(From::from)
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
            }).map_err(From::from)
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
            }).map_err(From::from)
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
