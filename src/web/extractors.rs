//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.
use actix_web::{Error, FromRequest, HttpRequest, Path, State};
use futures::{future, Future};
use serde::de::{Deserialize, Deserializer};

use db::{params, Db};
use server::ServerState;
use settings::Settings;
use web::auth::{HawkIdentifier, HawkPayload};

// XXX: Convert these to full extractors.
pub type GetCollectionRequest = (Path<CollectionParams>, HawkIdentifier, State<ServerState>);
pub type BsoRequest = (Path<BsoParams>, HawkIdentifier, State<ServerState>);

/// Request arguments needed for Information Requests
///
/// Only the database and user identifier is required for information
/// requests: https://mozilla-services.readthedocs.io/en/latest/storage/apis-1.5.html#general-info
pub struct MetaRequest {
    pub state: State<ServerState>,
    pub user_id: HawkIdentifier,
}

impl FromRequest<ServerState> for MetaRequest {
    type Config = Settings;
    type Result = Box<Future<Item = MetaRequest, Error = Error>>;

    fn from_request(req: &HttpRequest<ServerState>, cfg: &Self::Config) -> Self::Result {
        Box::new(
            <(Path<UidParam>, HawkIdentifier, State<ServerState>)>::extract(req).and_then(
                |(path, auth, state)| {
                    future::ok(MetaRequest {
                        state,
                        user_id: auth,
                    })
                },
            ),
        )
    }
}

#[derive(Deserialize)]
pub struct UidParam {
    uid: String,
}

#[derive(Deserialize)]
pub struct DeleteCollectionQuery {
    pub ids: Option<BsoIds>,
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

#[derive(Deserialize)]
pub struct CollectionParams {
    pub uid: String,
    pub collection: String,
}

#[derive(Deserialize)]
pub struct BsoParams {
    pub uid: String,
    pub collection: String,
    pub bso: String,
}

#[derive(Deserialize, Serialize)]
pub struct BsoBody {
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}
