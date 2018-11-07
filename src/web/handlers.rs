//! API Handlers
use actix_web::{http::StatusCode, FutureResponse, HttpResponse, State};
use futures::future::{self, Future};

use db::{params, DbErrorKind};
use server::ServerState;
use web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HawkIdentifier,
    MetaRequest,
};

pub fn get_collections(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_collection_modifieds(meta.user_id)
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
            .map(|result| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Weave-Records", result.len().to_string())
                    .json(result)
            }),
    )
}

pub fn get_quota(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.db
            .get_storage_usage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(vec![Some(result), None])),
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
    Box::new(
        coll.db
            .delete_collection(params::DeleteCollection {
                user_id: coll.user_id.clone(),
                collection: coll.collection.clone(),
                // XXX: handle both cases (delete_collection & delete_bsos also)
                // XXX: If we are passed id's to delete bso's, also set X-Last-Modified
                /*
                ids: coll
                    .query
                    .ids
                    .as_ref()
                    .map_or_else(|| Vec::new(), |ids| ids.clone()),
                */
            }).or_else(move |e| match e.kind() {
                DbErrorKind::CollectionNotFound | DbErrorKind::ItemNotFound => {
                    coll.db.get_storage_modified(coll.user_id)
                }
                _ => Box::new(future::err(e)),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection(coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    // XXX: it may make more sense for Db to take BsoQuery params as Options
    // XXX: Pagination will require setting the X-Weave-Next-Offset header
    Box::new(
        coll.db
            .get_bsos(params::GetBsos {
                user_id: coll.user_id.clone(),
                collection: coll.collection.clone(),
                params: coll.query.clone(),
            }).map_err(From::from)
            .and_then(|result| {
                coll.db
                    .extract_resource(coll.user_id, Some(coll.collection), None)
                    .map_err(From::from)
                    .map(move |ts| (result, ts))
            }).map(|(result, ts)| {
                HttpResponse::build(StatusCode::OK)
                    .header("X-Last-Modified", ts.as_header())
                    .json(result.bsos)
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
            .map(|_result| HttpResponse::Ok().json(::db::results::GetBso::default())),
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
