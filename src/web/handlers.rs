//! API Handlers

use actix_web::{error::ResponseError, FutureResponse, HttpResponse, Json, Path, Query, State};
use futures::future::{self, Future};

use db::{params, DbError};
use server::ServerState;
use web::auth::{HawkIdentifier, HawkPayload};
use web::extractors::{
    BsoBody, BsoParams, BsoQueryParams, CollectionParams, GetCollectionRequest, MetaRequest,
};

pub fn get_collections(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collections(&params::GetCollections {
                user_id: meta.user_id,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_counts(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collection_counts(&params::GetCollectionCounts {
                user_id: meta.user_id,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_usage(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collection_usage(&params::GetCollectionUsage {
                user_id: meta.user_id,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_quota(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_storage_usage(&params::GetStorageUsage {
                user_id: meta.user_id,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_all(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .delete_all(&params::DeleteAll {
                user_id: meta.user_id,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_collection(
    (_params, auth, state, query): (
        Path<CollectionParams>,
        HawkIdentifier,
        State<ServerState>,
        Query<BsoQueryParams>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .delete_collection(&params::DeleteCollection {
                user_id: auth,
                collection_id: 2,
                bso_ids: query
                    .ids
                    .as_ref()
                    .map_or_else(|| Vec::new(), |ids| ids.clone()),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection(
    (_params, auth, state): GetCollectionRequest,
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_collection(&params::GetCollection {
                user_id: auth,
                collection_id: 2,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn post_collection(
    (_params, auth, state, body): (
        Path<CollectionParams>,
        HawkIdentifier,
        State<ServerState>,
        Json<Vec<params::PostCollectionBso>>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .post_collection(&params::PostCollection {
                user_id: auth,
                collection_id: 2,
                bsos: body.into_inner().into_iter().map(From::from).collect(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_bso(
    (params, auth, state): (Path<BsoParams>, HawkIdentifier, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .delete_bso(&params::DeleteBso {
                user_id: auth,
                collection_id: 2,
                id: params.bso.clone(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_bso(
    (params, auth, state): (Path<BsoParams>, HawkIdentifier, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_bso(&params::GetBso {
                user_id: auth,
                collection_id: 2,
                id: params.bso.clone(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn put_bso(
    (params, auth, state, body): (
        Path<BsoParams>,
        HawkIdentifier,
        State<ServerState>,
        Json<BsoBody>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .put_bso(&params::PutBso {
                user_id: auth,
                collection_id: 2,
                id: params.bso.clone(),
                sortindex: body.sortindex,
                payload: body.payload.as_ref().map(|payload| payload.into()),
                ttl: body.ttl,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_configuration(
    (_auth, state): (HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(future::result(Ok(HttpResponse::Ok().json(&*state.limits))))
}

impl ResponseError for DbError {}
