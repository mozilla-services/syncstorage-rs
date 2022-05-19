pub mod error;
pub mod params;
pub mod results;
pub mod test;
pub mod util;

use std::fmt::Debug;

use async_trait::async_trait;
use futures::future::{self, LocalBoxFuture, TryFutureExt};
use lazy_static::lazy_static;
use serde::Deserialize;

use error::DbError;
use util::SyncTimestamp;

lazy_static! {
    /// For efficiency, it's possible to use fixed pre-determined IDs for
    /// common collection names.  This is the canonical list of such
    /// names.  Non-standard collections will be allocated IDs starting
    /// from the highest ID in this collection.
    pub static ref STD_COLLS: Vec<(i32, &'static str)> = {
        vec![
        (1, "clients"),
        (2, "crypto"),
        (3, "forms"),
        (4, "history"),
        (5, "keys"),
        (6, "meta"),
        (7, "bookmarks"),
        (8, "prefs"),
        (9, "tabs"),
        (10, "passwords"),
        (11, "addons"),
        (12, "addresses"),
        (13, "creditcards"),
        ]
    };
}

/// Rough guesstimate of the maximum reasonable life span of a batch
pub const BATCH_LIFETIME: i64 = 2 * 60 * 60 * 1000; // 2 hours, in milliseconds

/// The ttl to use for rows that are never supposed to expire (in seconds)
pub const DEFAULT_BSO_TTL: u32 = 2_100_000_000;

/// Non-standard collections will be allocated IDs beginning with this value
pub const FIRST_CUSTOM_COLLECTION_ID: i32 = 101;

pub type DbResult<T> = Result<T, DbError>;
pub type DbFuture<'a, T> = LocalBoxFuture<'a, DbResult<T>>;

#[async_trait]
pub trait DbPool: Sync + Send + Debug + GetPoolState {
    async fn get(&self) -> Result<Box<dyn Db<'_>>, DbError>;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> Result<(), DbError>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}

pub trait GetPoolState {
    fn state(&self) -> PoolState;
}

impl GetPoolState for Box<dyn DbPool> {
    fn state(&self) -> PoolState {
        (**self).state()
    }
}

#[derive(Debug, Default)]
/// A mockable r2d2::State
pub struct PoolState {
    pub connections: u32,
    pub idle_connections: u32,
}

impl From<diesel::r2d2::State> for PoolState {
    fn from(state: diesel::r2d2::State) -> PoolState {
        PoolState {
            connections: state.connections,
            idle_connections: state.idle_connections,
        }
    }
}
impl From<deadpool::Status> for PoolState {
    fn from(status: deadpool::Status) -> PoolState {
        PoolState {
            connections: status.size as u32,
            idle_connections: status.available.max(0) as u32,
        }
    }
}

pub trait Db<'a>: Debug + 'a {
    fn lock_for_read(&self, params: params::LockCollection) -> DbFuture<'_, ()>;

    fn lock_for_write(&self, params: params::LockCollection) -> DbFuture<'_, ()>;

    fn begin(&self, for_write: bool) -> DbFuture<'_, ()>;

    fn commit(&self) -> DbFuture<'_, ()>;

    fn rollback(&self) -> DbFuture<'_, ()>;

    fn get_collection_timestamps(
        &self,
        params: params::GetCollectionTimestamps,
    ) -> DbFuture<'_, results::GetCollectionTimestamps>;

    fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> DbFuture<'_, results::GetCollectionTimestamp>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<'_, results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<'_, results::GetCollectionUsage>;

    fn get_storage_timestamp(
        &self,
        params: params::GetStorageTimestamp,
    ) -> DbFuture<'_, results::GetStorageTimestamp>;

    fn get_storage_usage(
        &self,
        params: params::GetStorageUsage,
    ) -> DbFuture<'_, results::GetStorageUsage>;

    fn get_quota_usage(
        &self,
        params: params::GetQuotaUsage,
    ) -> DbFuture<'_, results::GetQuotaUsage>;

    fn delete_storage(&self, params: params::DeleteStorage)
        -> DbFuture<'_, results::DeleteStorage>;

    fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> DbFuture<'_, results::DeleteCollection>;

    fn delete_bsos(&self, params: params::DeleteBsos) -> DbFuture<'_, results::DeleteBsos>;

    fn get_bsos(&self, params: params::GetBsos) -> DbFuture<'_, results::GetBsos>;

    fn get_bso_ids(&self, params: params::GetBsos) -> DbFuture<'_, results::GetBsoIds>;

    fn post_bsos(&self, params: params::PostBsos) -> DbFuture<'_, results::PostBsos>;

    fn delete_bso(&self, params: params::DeleteBso) -> DbFuture<'_, results::DeleteBso>;

    fn get_bso(&self, params: params::GetBso) -> DbFuture<'_, Option<results::GetBso>>;

    fn get_bso_timestamp(
        &self,
        params: params::GetBsoTimestamp,
    ) -> DbFuture<'_, results::GetBsoTimestamp>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<'_, results::PutBso>;

    fn create_batch(&self, params: params::CreateBatch) -> DbFuture<'_, results::CreateBatch>;

    fn validate_batch(&self, params: params::ValidateBatch)
        -> DbFuture<'_, results::ValidateBatch>;

    fn append_to_batch(
        &self,
        params: params::AppendToBatch,
    ) -> DbFuture<'_, results::AppendToBatch>;

    fn get_batch(&self, params: params::GetBatch) -> DbFuture<'_, Option<results::GetBatch>>;

    fn commit_batch(&self, params: params::CommitBatch) -> DbFuture<'_, results::CommitBatch>;

    fn box_clone(&self) -> Box<dyn Db<'a>>;

    fn check(&self) -> DbFuture<'_, results::Check>;

    fn get_connection_info(&self) -> results::ConnectionInfo;

    /// Retrieve the timestamp for an item/collection
    ///
    /// Modeled on the Python `get_resource_timestamp` function.
    fn extract_resource(
        &self,
        user_id: UserIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> DbFuture<'_, SyncTimestamp> {
        // If there's no collection, we return the overall storage timestamp
        let collection = match collection {
            Some(collection) => collection,
            None => return Box::pin(self.get_storage_timestamp(user_id)),
        };
        // If there's no bso, return the collection
        let bso = match bso {
            Some(bso) => bso,
            None => {
                return Box::pin(
                    self.get_collection_timestamp(params::GetCollectionTimestamp {
                        user_id,
                        collection,
                    })
                    .or_else(|e| {
                        if e.is_collection_not_found() {
                            future::ok(SyncTimestamp::from_seconds(0f64))
                        } else {
                            future::err(e)
                        }
                    }),
                )
            }
        };
        Box::pin(
            self.get_bso_timestamp(params::GetBsoTimestamp {
                user_id,
                collection,
                id: bso,
            })
            .or_else(|e| {
                if e.is_collection_not_found() {
                    future::ok(SyncTimestamp::from_seconds(0f64))
                } else {
                    future::err(e)
                }
            }),
        )
    }

    /// Internal methods used by the db tests

    fn get_collection_id(&self, name: String) -> DbFuture<'_, i32>;

    fn create_collection(&self, name: String) -> DbFuture<'_, i32>;

    fn update_collection(&self, params: params::UpdateCollection) -> DbFuture<'_, SyncTimestamp>;

    fn timestamp(&self) -> SyncTimestamp;

    fn set_timestamp(&self, timestamp: SyncTimestamp);

    fn delete_batch(&self, params: params::DeleteBatch) -> DbFuture<'_, ()>;

    fn clear_coll_cache(&self) -> DbFuture<'_, ()>;

    fn set_quota(&mut self, enabled: bool, limit: usize, enforce: bool);
}

impl<'a> Clone for Box<dyn Db<'a>> {
    fn clone(&self) -> Box<dyn Db<'a>> {
        self.box_clone()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Sorting {
    None,
    Newest,
    Oldest,
    Index,
}

impl Default for Sorting {
    fn default() -> Self {
        Sorting::None
    }
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct UserIdentifier {
    /// For MySQL database backends as the primary key
    pub legacy_id: u64,
    /// For NoSQL database backends that require randomly distributed primary keys
    pub fxa_uid: String,
    pub fxa_kid: String,
}

impl UserIdentifier {
    /// Create a new legacy id user identifier
    pub fn new_legacy(user_id: u64) -> Self {
        Self {
            legacy_id: user_id,
            ..Default::default()
        }
    }
}

impl From<u32> for UserIdentifier {
    fn from(val: u32) -> Self {
        Self {
            legacy_id: val.into(),
            ..Default::default()
        }
    }
}
