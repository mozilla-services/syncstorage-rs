//! Result types for database methods.

#![allow(proc_macro_derive_resolution_fallback)]

use std::collections::HashMap;

use diesel::sql_types::{BigInt, Integer, Nullable, Text};

pub type LockCollection = ();
pub type GetCollectionModifieds = HashMap<String, i64>;
pub type GetCollectionCounts = HashMap<String, i64>;
pub type GetCollectionUsage = HashMap<String, i64>;
pub type GetStorageUsage = u64;
pub type DeleteStorage = ();
pub type GetCollection = Vec<GetCollectionItem>;
pub type DeleteCollection = i64;
pub type DeleteBsos = i64;
pub type DeleteBso = i64;
pub type PutBso = u64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetCollectionItem {
    Id(String),
    Bso(GetBso),
}

#[derive(Debug, Default, Deserialize, Queryable, QueryableByName, Serialize)]
pub struct GetBso {
    #[sql_type = "Text"]
    pub id: String,
    #[sql_type = "BigInt"]
    pub modified: i64,
    #[sql_type = "Text"]
    pub payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sql_type = "Nullable<Integer>"]
    pub sortindex: Option<i32>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[sql_type = "BigInt"]
    pub expiry: i64,
}

#[derive(Debug, Default)]
pub struct BSOs {
    pub bsos: Vec<GetBso>, // XXX: naming
    pub more: bool,
    pub offset: i64, // XXX: i64?
}

// XXX: ideally only used by the handlers (could use json! instead?)
/*
#[derive(Debug, Default, Serialize)]
pub struct DeleteBso {
    modified: u64,
}
*/

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PostCollection {
    pub modified: u64,
    pub success: Vec<String>,
    pub failed: HashMap<String, String>,
}
