//! API Handlers
use actix::{ActorResponse, Addr};
use actix_web::{
    error, Error, AsyncResponder, FromRequest, FutureResponse, HttpRequest, HttpResponse, Json,
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

/// HTTP API methods
pub fn collection_info(state: State<ServerState>) -> FutureResponse<HttpResponse> {
    let user_id = "dummyval".to_string();
    state
        .db_executor
        .send(dispatcher::CollectionInfo { user_id: user_id })
        .from_err()
        .and_then(|res| match res {
            Ok(info) => Ok(HttpResponse::Ok().json(info)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

pub fn get_bso(
    (params, state): (Path<BsoParams>, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    state
        .db_executor
        .send(dispatcher::GetBso {
            user_id: params.uid.clone(),
            collection: params.collection.clone(),
            bso_id: params.bso.clone(),
        })
        .from_err()
        .and_then(|res| match res {
            Ok(info) => Ok(HttpResponse::Ok().json(info)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
}

pub fn put_bso(
    (params, body, state): (Path<BsoParams>, Json<BsoBody>, State<ServerState>),
) -> FutureResponse<HttpResponse> {
    state
        .db_executor
        .send(dispatcher::PutBso {
            user_id: params.uid.clone(),
            collection: params.collection.clone(),
            bso_id: params.bso.clone(),
            sortindex: body.sortindex,
            payload: body.payload.as_ref().map(|payload| payload.clone()),
            ttl: body.ttl,
        })
        .from_err()
        .and_then(|res| match res {
            Ok(info) => Ok(HttpResponse::Ok().json(info)),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
        .responder()
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
