//! Parameter types for database methods.
use std::{collections::HashMap, num::ParseIntError, str::FromStr};

use diesel::Queryable;
use serde::{Deserialize, Serialize};

use crate::{results, util::SyncTimestamp, Sorting, UserIdentifier};

macro_rules! data {
    ($name:ident {$($property:ident: $type:ty,)*}) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            $(pub $property: $type,)*
        }
    }
}

macro_rules! uid_data {
    ($($name:ident,)+) => ($(
        pub type $name = UserIdentifier;
    )+)
}

macro_rules! collection_data {
    ($($name:ident {$($property:ident: $type:ty,)*},)+) => ($(
        data! {
            $name {
                user_id: UserIdentifier,
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
                user_id: UserIdentifier,
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

#[derive(Debug, Default, Clone)]
pub struct Offset {
    pub timestamp: Option<SyncTimestamp>,
    pub offset: u64,
}

impl ToString for Offset {
    fn to_string(&self) -> String {
        // issue559: Disable ':' support for now.
        self.offset.to_string()
        /*
        match self.timestamp {
            None => self.offset.to_string(),
            Some(ts) => format!("{}:{}", ts.as_i64(), self.offset),
        }
        */
    }
}

impl FromStr for Offset {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // issue559: Disable ':' support for now: simply parse as i64 as
        // previously (it was u64 previously but i64's close enough)
        let result = Offset {
            timestamp: None,
            offset: s.parse::<u64>()?,
        };
        /*
        let result = match s.chars().position(|c| c == ':') {
            None => Offset {
                timestamp: None,
                offset: s.parse::<u64>()?,
            },
            Some(_colon_position) => {
                let mut parts = s.split(':');
                let timestamp_string = parts.next().unwrap_or("0");
                let timestamp = SyncTimestamp::from_milliseconds(timestamp_string.parse::<u64>()?);
                let offset = parts.next().unwrap_or("0").parse::<u64>()?;
                Offset {
                    timestamp: Some(timestamp),
                    offset,
                }
            }
        };
        */
        Ok(result)
    }
}

collection_data! {
    LockCollection {},
    DeleteCollection {},
    GetCollectionTimestamp {},
    DeleteBsos {
        ids: Vec<String>,
    },
    GetBsos {
        newer: Option<SyncTimestamp>,
        older: Option<SyncTimestamp>,
        sort: Sorting,
        limit: Option<u32>,
        offset: Option<Offset>,
        ids: Vec<String>,
        full: bool,
    },
    PostBsos {
        bsos: Vec<PostCollectionBso>,
        for_batch: bool,
        failed: HashMap<String, String>,
    },

    CreateBatch {
        bsos: Vec<PostCollectionBso>,
    },
    ValidateBatch {
        id: String,
    },
    AppendToBatch {
        batch: results::CreateBatch,
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
    GetQuotaUsage {
        collection_id: i32,
    },
}

impl From<ValidateBatch> for GetBatch {
    fn from(v: ValidateBatch) -> Self {
        Self {
            id: v.id,
            user_id: v.user_id,
            collection: v.collection,
        }
    }
}

pub type ValidateBatchId = String;
pub type GetBsoIds = GetBsos;

bso_data! {
    DeleteBso {},
    GetBso {},
    GetBsoTimestamp {},
}

#[derive(Clone, Debug, Default, Queryable)]
pub struct Batch {
    pub id: String,
}

pub struct PutBso {
    pub user_id: UserIdentifier,
    pub collection: String,
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    // ttl in seconds
    pub ttl: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PostCollectionBso {
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    // ttl in seconds
    pub ttl: Option<u32>,
}

pub type GetCollectionId = String;

pub type CreateCollection = String;

data! {
    UpdateCollection {
        user_id: UserIdentifier,
        collection_id: i32,
        collection: String,
    }
}
