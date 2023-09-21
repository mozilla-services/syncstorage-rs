//! Result types for database methods.
use std::collections::HashMap;

use diesel::{
    sql_types::{BigInt, Integer, Nullable, Text},
    Queryable, QueryableByName,
};
use serde::{Deserialize, Serialize};

use super::params;
use crate::util::SyncTimestamp;

pub type LockCollection = ();
pub type GetBsoTimestamp = SyncTimestamp;
pub type GetCollectionTimestamps = HashMap<String, SyncTimestamp>;
pub type GetCollectionTimestamp = SyncTimestamp;
pub type GetCollectionCounts = HashMap<String, i64>;
pub type GetCollectionUsage = HashMap<String, i64>;
pub type GetStorageTimestamp = SyncTimestamp;
pub type GetStorageUsage = u64;
pub type DeleteStorage = ();
pub type DeleteCollection = SyncTimestamp;
pub type DeleteBsos = SyncTimestamp;
pub type DeleteBso = SyncTimestamp;
pub type PutBso = SyncTimestamp;

#[derive(Debug, Default, Clone)]
pub struct CreateBatch {
    pub id: String,
    pub size: Option<usize>,
}

pub type ValidateBatch = bool;
pub type AppendToBatch = ();
pub type GetBatch = params::Batch;
pub type DeleteBatch = ();
pub type CommitBatch = SyncTimestamp;
pub type ValidateBatchId = ();
pub type Check = bool;

#[derive(Debug, Default)]
pub struct GetQuotaUsage {
    pub total_bytes: usize,
    pub count: i32,
}

#[derive(Debug, Default, Deserialize, Queryable, QueryableByName, Serialize)]
pub struct GetBso {
    #[sql_type = "Text"]
    pub id: String,
    #[sql_type = "BigInt"]
    pub modified: SyncTimestamp,
    #[sql_type = "Text"]
    pub payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[sql_type = "Nullable<Integer>"]
    pub sortindex: Option<i32>,
    // NOTE: expiry (ttl) is never rendered to clients and only loaded for
    // tests: this and its associated queries/loading could be wrapped in
    // #[cfg(test)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[sql_type = "BigInt"]
    pub expiry: i64,
}

#[derive(Debug, Default)]
pub struct Paginated<T>
where
    T: Serialize,
{
    pub items: Vec<T>,
    pub offset: Option<String>,
}

pub type GetBsos = Paginated<GetBso>;
pub type GetBsoIds = Paginated<String>;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PostBsos {
    pub modified: SyncTimestamp,
    pub success: Vec<String>,
    pub failed: HashMap<String, String>,
}

#[derive(Debug, Default)]
pub struct ConnectionInfo {
    pub age: i64,
    pub spanner_age: i64,
    pub spanner_idle: i64,
}

pub type GetCollectionId = i32;

pub type CreateCollection = i32;

pub type UpdateCollection = SyncTimestamp;
