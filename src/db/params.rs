//! Parameter types for database methods.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::web::extractors::{BatchBsoBody, BsoQueryParams, HawkIdentifier};

macro_rules! data {
    ($name:ident {$($property:ident: $type:ty,)*}) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $property: $type,)*
        }
    }
}

macro_rules! uid_data {
    ($($name:ident,)+) => ($(
        pub type $name = HawkIdentifier;
    )+)
}

macro_rules! collection_data {
    ($($name:ident {$($property:ident: $type:ty,)*},)+) => ($(
        data! {
            $name {
                user_id: HawkIdentifier,
                collection: String,
                $($property: $type,)*
            }
        }
    )+)
}

macro_rules! bso_data {
    ($($name:ident {$($property:ident: $type:ty,)*},)+) => ($(
        data! {
            $name {
                user_id: HawkIdentifier,
                collection: String,
                id: String,
                $($property: $type,)*
            }
        }
    )+)
}

uid_data! {
    GetCollectionTimestamps,
    GetCollectionCounts,
    GetCollectionUsage,
    GetStorageTimestamp,
    GetStorageUsage,
    DeleteStorage,
}

collection_data! {
    LockCollection {},
    DeleteCollection {},
    GetCollectionTimestamp {},
    DeleteBsos {
        ids: Vec<String>,
    },
    GetBsos {
        params: BsoQueryParams,
    },
    PostBsos {
        bsos: Vec<PostCollectionBso>,
        failed: HashMap<String, String>,
    },

    CreateBatch {
        bsos: Vec<PostCollectionBso>,
    },
    ValidateBatch {
        id: String,
    },
    AppendToBatch {
        id: String,
        bsos: Vec<PostCollectionBso>,
    },
    CommitBatch {
        batch: Batch,
    },
    GetBatch {
        id: String,
    },
    DeleteBatch {
        id: String,
    },
}

pub type ValidateBatchId = String;
pub type GetBsoIds = GetBsos;

bso_data! {
    DeleteBso {},
    GetBso {},
    GetBsoTimestamp {},
}

#[derive(Debug, Default, Queryable)]
pub struct Batch {
    pub id: String,
    pub bsos: String,
    pub expiry: i64,
}

pub struct PutBso {
    pub user_id: HawkIdentifier,
    pub collection: String,
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    // ttl in seconds
    pub ttl: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostCollectionBso {
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    // ttl in seconds
    pub ttl: Option<u32>,
}

impl From<BatchBsoBody> for PostCollectionBso {
    fn from(b: BatchBsoBody) -> PostCollectionBso {
        PostCollectionBso {
            id: b.id,
            sortindex: b.sortindex,
            payload: b.payload,
            ttl: b.ttl,
        }
    }
}

#[cfg(test)]
pub type GetCollectionId = String;

#[cfg(test)]
pub type CreateCollection = String;

#[cfg(test)]
data! {
    TouchCollection {
        user_id: HawkIdentifier,
        collection_id: i32,
    }
}
