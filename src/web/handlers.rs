//! API Handlers
use std::rc::Rc;

use actix_web::{FutureResponse, HttpResponse, State};
use futures::future::{self, Future};

use db::{params, Db};
use server::ServerState;
use web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HawkIdentifier,
    MetaRequest,
};

pub fn get_collections(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collection_modifieds(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_counts(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collection_counts(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_usage(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_collection_usage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_quota(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .get_storage_usage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(vec![Some(result), None])),
    )
}

pub fn delete_all(meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        meta.state
            .db
            .delete_storage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_collection(coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        coll.state
            .db
            .delete_collection(params::DeleteCollection {
                user_id: coll.user_id,
                collection: coll.collection,
                // XXX: handle both cases (delete_collection & delete_bsos also)
                /*
                ids: coll
                    .query
                    .ids
                    .as_ref()
                    .map_or_else(|| Vec::new(), |ids| ids.clone()),
                */
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection(coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        coll.state
            .db
            .get_collection(params::GetCollection {
                user_id: coll.user_id,
                collection: coll.collection,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn post_collection(coll: CollectionPostRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        coll.state
            .db
            .post_collection(params::PostCollection {
                user_id: coll.user_id,
                collection: coll.collection,
                bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_bso(bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        bso_req
            .state
            .db
            .delete_bso(params::DeleteBso {
                user_id: bso_req.user_id,
                collection: bso_req.collection,
                id: bso_req.bso.clone(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_bso(bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        bso_req
            .state
            .db
            .get_bso(params::GetBso {
                user_id: bso_req.user_id,
                collection: bso_req.collection,
                id: bso_req.bso.clone(),
            }).map_err(From::from)
            .map(|_result| HttpResponse::Ok().json(::db::results::GetBso::default())),
    )
}

pub fn put_bso((db, bso_req): (Rc<Box<dyn Db>>, BsoPutRequest)) -> FutureResponse<HttpResponse> {
    Box::new(
        db.put_bso(params::PutBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
            sortindex: bso_req.body.sortindex,
            payload: bso_req.body.payload,
            ttl: bso_req.body.ttl,
        }).map_err(From::from)
        .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_configuration(
    (_auth, state): (HawkIdentifier, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(future::result(Ok(HttpResponse::Ok().json(&*state.limits))))
}
