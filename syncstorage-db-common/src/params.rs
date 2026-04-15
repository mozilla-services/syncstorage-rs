//! Parameter types for database methods.
use core::fmt;
use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

use diesel::Queryable;
use serde::{Deserialize, Serialize};

use crate::{Sorting, UserIdentifier, results, util::SyncTimestamp};

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

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct Offset {
    pub timestamp: Option<SyncTimestamp>,
    pub offset: u64,
}

impl Display for Offset {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), fmt::Error> {
        match self.timestamp {
            None => write!(fmt, "{}", self.offset),
            Some(ts) => write!(fmt, "{}:{}", ts.as_i64(), self.offset),
        }
    }
}

impl FromStr for Offset {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::Offset;
    use crate::util::SyncTimestamp;

    #[test]
    fn offset_display_without_timestamp() {
        let offset = Offset {
            timestamp: None,
            offset: 50,
        };
        assert_eq!(offset.to_string(), "50");
    }

    #[test]
    fn offset_display_with_timestamp() {
        let offset = Offset {
            timestamp: Some(SyncTimestamp::from_milliseconds(676760)),
            offset: 2,
        };
        assert_eq!(offset.to_string(), "676760:2");
    }

    #[test]
    fn offset_without_timestamp_parsed() {
        let original = Offset {
            timestamp: None,
            offset: 99,
        };
        let parsed = Offset::from_str(&original.to_string()).unwrap();
        assert_eq!(parsed.offset, original.offset);
        assert!(parsed.timestamp.is_none());
    }

    #[test]
    fn offset_with_timestamp_parsed() {
        let original = Offset {
            timestamp: Some(SyncTimestamp::from_milliseconds(71138383830)),
            offset: 3,
        };
        let parsed = Offset::from_str(&original.to_string()).unwrap();
        assert_eq!(parsed.offset, original.offset);
        assert_eq!(parsed.timestamp, original.timestamp);
    }

    #[test]
    fn offset_fromstr_malformed_returns_error() {
        assert!(Offset::from_str("quux").is_err());
        assert!(Offset::from_str("wibble:buzz").is_err());
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

data! {
    UpdateCollection {
        user_id: UserIdentifier,
        collection_id: i32,
        collection: String,
    }
}
