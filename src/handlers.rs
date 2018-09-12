// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! API Handlers

use actix_web::{error::ResponseError, FutureResponse, HttpResponse, Json, Path, Query, State};
use futures::future::{self, Future};
use serde::de::{Deserialize, Deserializer};

use auth::HawkPayload;
use db::{params, util::ms_since_epoch, Db, DbError};

/// This is the global HTTP state object that will be made available to all
/// HTTP API calls.
pub struct ServerState {
    pub db: Box<Db>,
}

macro_rules! db_endpoint {
    ($handler:ident: $data:ident ($path:ident: $path_type:ty $(, $param:ident: $type:ty)*) {$($property:ident: $value:expr,)*}) => {
        pub fn $handler(
            ($path, _auth, state$(, $param)*): (Path<$path_type>, HawkPayload, State<ServerState>$(, $type)*),
        ) -> FutureResponse<HttpResponse> {
            Box::new(
                state.db.$handler(&params::$data {
                    $($property: $value,)*
                })
                .map_err(From::from)
                .map(|result| HttpResponse::Ok().json(result))
            )
        }
    }
}

macro_rules! info_endpoints {
    ($($handler:ident: $data:ident,)+) => ($(
        db_endpoint! {
            $handler: $data (params: UidParam) {
                // XXX: -> HawkPayload::uid
                user_id: 1,
            }
        }
    )+)
}

info_endpoints! {
    get_collections: GetCollections,
    get_collection_counts: GetCollectionCounts,
    get_collection_usage: GetCollectionUsage,
    get_quota: GetQuota,
    delete_all: DeleteAll,
}

#[derive(Deserialize)]
pub struct UidParam {
    uid: String,
}

macro_rules! collection_endpoints {
    ($($handler:ident: $data:ident ($($param:ident: $type:ty),*) {$($property:ident: $value:expr,)*},)+) => ($(
        db_endpoint! {
            $handler: $data (params: CollectionParams $(, $param: $type)*) {
                // XXX: -> HawkPayload::uid
                user_id: 1,
                collection_id: 2, // XXX: get_collection_id(&params.collection)
                $($property: $value,)*
            }
        }
    )+)
}

collection_endpoints! {
    delete_collection: DeleteCollection (query: Query<DeleteCollectionQuery>) {
        bso_ids: query.ids.as_ref().map_or_else(|| Vec::new(), |ids| ids.0.clone()),
    },
    get_collection: GetCollection () {},
    post_collection: PostCollection (body: Json<Vec<PostCollectionBody>>) {
        bsos: body.into_inner().into_iter().map(From::from).collect(),
    },
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

macro_rules! bso_endpoints {
    ($($handler:ident: $data:ident ($($param:ident: $type:ty),*) {$($property:ident: $value:expr,)*},)+) => ($(
        db_endpoint! {
            $handler: $data (params: BsoParams $(, $param: $type)*) {
                // XXX: -> HawkPayload::uid
                user_id: 1,
                collection_id: 2, // XXX: get_collection_id(&params.collection)
                id: params.bso.clone(),
                $($property: $value,)*
            }
        }
    )+)
}

bso_endpoints! {
    delete_bso: DeleteBso () {},
    get_bso: GetBso () {},
    put_bso: PutBso (body: Json<BsoBody>) {
        modified: ms_since_epoch(),
        sortindex: body.sortindex,
        payload: body.payload.as_ref().map(|payload| payload.clone()),
        ttl: body.ttl,
    },
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
