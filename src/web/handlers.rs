//! API Handlers
use actix_web::{FutureResponse, HttpResponse, State};
use futures::future::{self, Future};

use db::{params, Db, DbErrorKind, Sorting};
use server::ServerState;
use web::extractors::{
    BsoPutRequest, BsoRequest, CollectionPostRequest, CollectionRequest, HawkIdentifier,
    MetaRequest,
};

pub fn get_collections(db: Box<dyn Db>, meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.get_collection_modifieds(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_counts(db: Box<dyn Db>, meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.get_collection_counts(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_usage(db: Box<dyn Db>, meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.get_collection_usage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_quota(db: Box<dyn Db>, meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.get_storage_usage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(vec![Some(result), None])),
    )
}

pub fn delete_all(db: Box<dyn Db>, meta: MetaRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.delete_storage(meta.user_id)
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_collection(db: Box<dyn Db>, coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.delete_collection(params::DeleteCollection {
            user_id: coll.user_id.clone(),
            collection: coll.collection.clone(),
            // XXX: handle both cases (delete_collection & delete_bsos also)
                /*
                ids: coll
                    .query
                    .ids
                    .as_ref()
                    .map_or_else(|| Vec::new(), |ids| ids.clone()),
                */
        }).or_else(move |e| match e.kind() {
            DbErrorKind::CollectionNotFound | DbErrorKind::ItemNotFound => {
                db.get_storage_modified(coll.user_id)
            }
            _ => Box::new(future::err(e)),
        }).map_err(From::from)
        .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection(db: Box<dyn Db>, coll: CollectionRequest) -> FutureResponse<HttpResponse> {
    // XXX: it may make more sense for Db to take BsoQuery params as Options
    Box::new(
        db.get_bsos(params::GetBsos {
            user_id: coll.user_id,
            collection: coll.collection,
            ids: vec!["foo".to_owned()],
            older: 0,
            newer: 0,
            sort: Sorting::Newest,
            limit: 3,
            offset: 0,
        }).map_err(From::from)
        .map(|result| HttpResponse::Ok().json(result.bsos)),
    )
}

pub fn post_collection(
    db: Box<dyn Db>,
    coll: CollectionPostRequest,
) -> FutureResponse<HttpResponse> {
    Box::new(
        db.post_bsos(params::PostBsos {
            user_id: coll.user_id,
            collection: coll.collection,
            bsos: coll.bsos.valid.into_iter().map(From::from).collect(),
        }).map_err(From::from)
        .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_bso(db: Box<dyn Db>, bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.delete_bso(params::DeleteBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
        }).map_err(From::from)
        .map(|result| HttpResponse::Ok().json(json!({ "modified": result }))),
    )
}

pub fn get_bso(db: Box<dyn Db>, bso_req: BsoRequest) -> FutureResponse<HttpResponse> {
    Box::new(
        db.get_bso(params::GetBso {
            user_id: bso_req.user_id,
            collection: bso_req.collection,
            id: bso_req.bso,
        }).map_err(From::from)
        .map(|_result| HttpResponse::Ok().json(::db::results::GetBso::default())),
    )
}

pub fn put_bso((db, bso_req): (Box<dyn Db>, BsoPutRequest)) -> FutureResponse<HttpResponse> {
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
