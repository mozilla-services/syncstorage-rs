//! API Handlers
use actix::{ActorResponse, Addr};
use actix_web::{
    error, AsyncResponder, Error, FromRequest, FutureResponse, HttpRequest, HttpResponse, Json,
    Path, Query, Responder, State,
};
use futures::Future;
// Hawk lib brings in some libs that don't compile at the moment for some reason
//use hawk::
use serde::de::{Deserialize, Deserializer};

use dispatcher;

/// This is the global HTTP state object that will be made available to all
/// HTTP API calls.
pub struct ServerState {
    pub db_executor: Addr<dispatcher::DBExecutor>,
}

#[derive(Debug, Deserialize)]
struct HawkHeader(String);

/// Extract a HAWK header
impl<S> FromRequest<S> for HawkHeader {
    type Config = ();
    type Result = Result<HawkHeader, Error>;

    fn from_request(req: &HttpRequest<S>, _cfg: &Self::Config) -> Self::Result {
        // TODO: Actually extract the Hawk Header
        Ok(HawkHeader("token".to_string()))
    }
}

macro_rules! endpoint {
    ($handler:ident: $dispatcher:ident ($path:ident: $path_type:ty $(, $param:ident: $type:ty)*) {$($property:ident: $value:expr),*}) => {
        pub fn $handler(
            ($path, state$(, $param)*): (Path<$path_type>, State<ServerState>$(, $type)*),
        ) -> FutureResponse<HttpResponse> {
            state
                .db_executor
                .send(dispatcher::$dispatcher {
                    $($property: $value),*
                })
                .from_err()
                .and_then(|res| match res {
                    Ok(info) => Ok(HttpResponse::Ok().json(info)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into()),
                })
                .responder()
        }
    }
}

macro_rules! info_endpoints {
    ($($handler:ident: $dispatcher:ident),+) => ($(
        endpoint! {
            $handler: $dispatcher (params: UidParam) {
                user_id: params.uid.clone()
            }
        }
    )+)
}

info_endpoints! {
    collections: Collections,
    collection_counts: CollectionCounts,
    collection_usage: CollectionUsage,
    configuration: Configuration,
    quota: Quota
}

#[derive(Deserialize)]
pub struct UidParam {
    uid: String,
}

macro_rules! collection_endpoints {
    ($($handler:ident: $dispatcher:ident ($($param:ident: $type:ty),*) {$($property:ident: $value:expr),*}),+) => ($(
        endpoint! {
            $handler: $dispatcher (params: CollectionParams $(, $param: $type)*) {
                user_id: params.uid.clone(),
                collection: params.collection.clone()
                $(, $property: $value)*
            }
        }
    )+)
}

collection_endpoints! {
    delete_collection: DeleteCollection (query: Query<DeleteCollectionQuery>) {
        bso_ids: query.ids.as_ref().map_or_else(|| Vec::new(), |ids| ids.0.clone())
    },
    get_collection: GetCollection () {},
    post_collection: PostCollection (body: Json<Vec<PostCollectionBody>>) {
        bsos: body.into_inner().into_iter().map(From::from).collect()
    }
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
    pub sortindex: Option<i64>,
    pub payload: Option<String>,
    pub ttl: Option<i64>,
}

impl From<PostCollectionBody> for dispatcher::PostCollectionBso {
    fn from(body: PostCollectionBody) -> dispatcher::PostCollectionBso {
        dispatcher::PostCollectionBso {
            bso_id: body.id.clone(),
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
    ($($handler:ident: $dispatcher:ident ($($param:ident: $type:ty),*) {$($property:ident: $value:expr),*}),+) => ($(
        endpoint! {
            $handler: $dispatcher (params: BsoParams $(, $param: $type)*) {
                user_id: params.uid.clone(),
                collection: params.collection.clone(),
                bso_id: params.bso.clone()
                $(, $property: $value)*
            }
        }
    )+)
}

bso_endpoints! {
    delete_bso: DeleteBso () {},
    get_bso: GetBso () {},
    put_bso: PutBso (body: Json<BsoBody>) {
        sortindex: body.sortindex,
        payload: body.payload.as_ref().map(|payload| payload.clone()),
        ttl: body.ttl
    }
}

#[derive(Deserialize)]
pub struct BsoParams {
    uid: String,
    collection: String,
    bso: String,
}

#[derive(Deserialize, Serialize)]
pub struct BsoBody {
    pub sortindex: Option<i64>,
    pub payload: Option<String>,
    pub ttl: Option<i64>,
}
