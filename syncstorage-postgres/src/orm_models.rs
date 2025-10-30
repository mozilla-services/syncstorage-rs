use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::schema::{batch_bsos, batches, bsos, collections, user_collections};
use diesel::{Identifiable, Insertable, Queryable};

#[allow(unused)]
#[allow(clippy::all)]
#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id))]
pub struct BatchBso {
    pub fxa_uid: Uuid,
    pub fxa_kid: String,
    pub collection_id: i64,
    pub batch_id: String,
    pub batch_bso_id: String,
    pub sortindex: Option<i64>,
    pub payload: Option<Vec<u8>>,
    pub ttl: Option<i64>,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(fxa_uid, fxa_kid, collection_id, batch_id))]
#[diesel(table_name=batches)]
pub struct Batch {
    pub fxa_uid: Uuid,
    pub fxa_kid: String,
    pub collection_id: i64,
    pub batch_id: String,
    pub expiry: NaiveDateTime,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(fxa_uid, fxa_kid, collection_id, bso_id))]
pub struct Bso {
    pub fxa_uid: Uuid,
    pub fxa_kid: String,
    pub collection_id: i64,
    pub bso_id: String,
    pub sortindex: Option<i64>,
    pub payload: Vec<u8>,
    pub modified: NaiveDateTime,
    pub expiry: NaiveDateTime,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(collection_id))]
pub struct Collection {
    pub collection_id: i64,
    pub name: String,
}

#[derive(Queryable, Debug, Identifiable)]
#[diesel(primary_key(fxa_uid, fxa_kid, collection_id))]
pub struct UserCollection {
    pub fxa_uid: Uuid,
    pub fxa_kid: String,
    pub collection_id: i64,
    pub modified: NaiveDateTime,
    pub count: Option<i64>,
    pub total_bytes: Option<i64>,
}
