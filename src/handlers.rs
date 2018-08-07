//! API Handlers
use actix::{ActorResponse, Addr};
use actix_web::{
    error, AsyncResponder, Error, FromRequest, FutureResponse, HttpRequest, HttpResponse, Json,
    Path, Responder, State,
};
use futures::Future;
// Hawk lib brings in some libs that don't compile at the moment for some reason
//use hawk::

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

macro_rules! info_endpoints {
    ($($handler:ident: $dispatcher:ident),+) => ($(
        pub fn $handler(
            (params, state): (Path<UidParam>, State<ServerState>),
        ) -> FutureResponse<HttpResponse> {
            state
                .db_executor
                .send(dispatcher::$dispatcher {
                    user_id: params.uid.clone(),
                })
                .from_err()
                .and_then(|res| match res {
                    Ok(info) => Ok(HttpResponse::Ok().json(info)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into()),
                })
                .responder()
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

macro_rules! bso_endpoints {
    ($($handler:ident: $dispatcher:ident ($($param:ident: $type:ty),*) {$($property:ident: $value:expr),*}),+) => ($(
        pub fn $handler(
            (params, state$(, $param),*): (Path<BsoParams>, State<ServerState>$(, $type),*),
        ) -> FutureResponse<HttpResponse> {
            state
                .db_executor
                .send(dispatcher::$dispatcher {
                    user_id: params.uid.clone(),
                    collection: params.collection.clone(),
                    bso_id: params.bso.clone(),
                    $($property: $value),*
                })
                .from_err()
                .and_then(|res| match res {
                    Ok(info) => Ok(HttpResponse::Ok().json(info)),
                    Err(_) => Ok(HttpResponse::InternalServerError().into()),
                })
                .responder()
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
