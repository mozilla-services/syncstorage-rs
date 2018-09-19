// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! API Handlers

use actix_web::{error::ResponseError, FutureResponse, HttpResponse, Json, Path, Query, State};
use futures::future::{self, Future};
use serde::de::{Deserialize, Deserializer};

use auth::HawkPayload;
use db::{params, util::ms_since_epoch, DbError};
use server::ServerState;

#[derive(Deserialize)]
pub struct UidParam {
    uid: String,
}

#[derive(Deserialize)]
pub struct DeleteCollectionQuery {
    ids: Option<BsoIds>,
}

pub struct BsoIds(pub Vec<String>);

impl<'d> Deserialize<'d> for BsoIds {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let value: String = Deserialize::deserialize(deserializer)?;
        Ok(BsoIds(value.split(",").map(|id| id.to_string()).collect()))
    }
}

#[derive(Deserialize, Serialize)]
pub struct PostCollectionBody {
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}

impl From<PostCollectionBody> for params::PostCollectionBso {
    fn from(body: PostCollectionBody) -> params::PostCollectionBso {
        params::PostCollectionBso {
            id: body.id.clone(),
            sortindex: body.sortindex,
            payload: body.payload.as_ref().map(|payload| payload.clone()),
            ttl: body.ttl,
        }
    }
}

#[derive(Deserialize)]
pub struct CollectionParams {
    uid: String,
    collection: String,
}

#[derive(Deserialize)]
pub struct BsoParams {
    uid: String,
    collection: String,
    bso: String,
}

#[derive(Deserialize, Serialize)]
pub struct BsoBody {
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}

pub fn get_collections(
    (params, _auth, state): (Path<UidParam>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_collections(&params::GetCollections { user_id: 1 })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_counts(
    (params, _auth, state): (Path<UidParam>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_collection_counts(&params::GetCollectionCounts { user_id: 1 })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection_usage(
    (params, _auth, state): (Path<UidParam>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_collection_usage(&params::GetCollectionUsage { user_id: 1 })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_quota(
    (params, _auth, state): (Path<UidParam>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_quota(&params::GetQuota { user_id: 1 })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_all(
    (params, _auth, state): (Path<UidParam>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .delete_all(&params::DeleteAll { user_id: 1 })
            .map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_collection(
    (params, _auth, state, query): (
        Path<CollectionParams>,
        HawkPayload,
        State<ServerState>,
        Query<DeleteCollectionQuery>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .delete_collection(&params::DeleteCollection {
                user_id: 1,
                collection_id: 2,
                bso_ids: query
                    .ids
                    .as_ref()
                    .map_or_else(|| Vec::new(), |ids| ids.0.clone()),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_collection(
    (params, _auth, state): (Path<CollectionParams>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_collection(&params::GetCollection {
                user_id: 1,
                collection_id: 2,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn post_collection(
    (params, _auth, state, body): (
        Path<CollectionParams>,
        HawkPayload,
        State<ServerState>,
        Json<Vec<PostCollectionBody>>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .post_collection(&params::PostCollection {
                user_id: 1,
                collection_id: 2,
                bsos: body.into_inner().into_iter().map(From::from).collect(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn delete_bso(
    (params, _auth, state): (Path<BsoParams>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .delete_bso(&params::DeleteBso {
                user_id: 1,
                collection_id: 2,
                id: params.bso.clone(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_bso(
    (params, _auth, state): (Path<BsoParams>, HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .get_bso(&params::GetBso {
                user_id: 1,
                collection_id: 2,
                id: params.bso.clone(),
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn put_bso(
    (params, _auth, state, body): (
        Path<BsoParams>,
        HawkPayload,
        State<ServerState>,
        Json<BsoBody>,
    ),
) -> FutureResponse<HttpResponse> {
    Box::new(
        state
            .db
            .put_bso(&params::PutBso {
                user_id: 1,
                collection_id: 2,
                id: params.bso.clone(),
                modified: ms_since_epoch(),
                sortindex: body.sortindex,
                payload: body.payload.as_ref().map(|payload| payload.clone()),
                ttl: body.ttl,
            }).map_err(From::from)
            .map(|result| HttpResponse::Ok().json(result)),
    )
}

pub fn get_configuration(
    (_auth, _state): (HawkPayload, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    // TODO: populate from static config?
    Box::new(future::result(Ok(
        HttpResponse::Ok().json(Configuration::default())
    )))
}

#[derive(Debug, Default, Serialize)]
pub struct Configuration {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_post_bytes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_post_records: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_request_bytes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_total_bytes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_total_records: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_record_payload_bytes: Option<u32>,
}

impl ResponseError for DbError {}
