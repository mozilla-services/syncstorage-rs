pub mod error;
pub mod params;
pub mod results;
pub mod util;

use std::fmt::Debug;

use async_trait::async_trait;
use futures::{future, TryFutureExt};
use lazy_static::lazy_static;
use serde::Deserialize;
use syncserver_db_common::{DbFuture, GetPoolState};

use error::DbErrorIntrospect;
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

#[async_trait]
pub trait DbPool: Sync + Send + Debug + GetPoolState {
    type Error;

    async fn get(&self) -> Result<Box<dyn Db<Error = Self::Error>>, Self::Error>;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> Result<(), Self::Error>;

    fn box_clone(&self) -> Box<dyn DbPool<Error = Self::Error>>;
}

impl<E> Clone for Box<dyn DbPool<Error = E>> {
    fn clone(&self) -> Box<dyn DbPool<Error = E>> {
        self.box_clone()
    }
}

pub trait Db: Debug {
    type Error: DbErrorIntrospect + 'static;

    fn lock_for_read(&self, params: params::LockCollection) -> DbFuture<'_, (), Self::Error>;

    fn lock_for_write(&self, params: params::LockCollection) -> DbFuture<'_, (), Self::Error>;

    fn begin(&self, for_write: bool) -> DbFuture<'_, (), Self::Error>;

    fn commit(&self) -> DbFuture<'_, (), Self::Error>;

    fn rollback(&self) -> DbFuture<'_, (), Self::Error>;

    fn get_collection_timestamps(
        &self,
        params: params::GetCollectionTimestamps,
    ) -> DbFuture<'_, results::GetCollectionTimestamps, Self::Error>;

    fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> DbFuture<'_, results::GetCollectionTimestamp, Self::Error>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<'_, results::GetCollectionCounts, Self::Error>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<'_, results::GetCollectionUsage, Self::Error>;

    fn get_storage_timestamp(
        &self,
        params: params::GetStorageTimestamp,
    ) -> DbFuture<'_, results::GetStorageTimestamp, Self::Error>;

    fn get_storage_usage(
        &self,
        params: params::GetStorageUsage,
    ) -> DbFuture<'_, results::GetStorageUsage, Self::Error>;

    fn get_quota_usage(
        &self,
        params: params::GetQuotaUsage,
    ) -> DbFuture<'_, results::GetQuotaUsage, Self::Error>;

    fn delete_storage(
        &self,
        params: params::DeleteStorage,
    ) -> DbFuture<'_, results::DeleteStorage, Self::Error>;

    fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> DbFuture<'_, results::DeleteCollection, Self::Error>;

    fn delete_bsos(
        &self,
        params: params::DeleteBsos,
    ) -> DbFuture<'_, results::DeleteBsos, Self::Error>;

    fn get_bsos(&self, params: params::GetBsos) -> DbFuture<'_, results::GetBsos, Self::Error>;

    fn get_bso_ids(&self, params: params::GetBsos)
        -> DbFuture<'_, results::GetBsoIds, Self::Error>;

    fn post_bsos(&self, params: params::PostBsos) -> DbFuture<'_, results::PostBsos, Self::Error>;

    fn delete_bso(
        &self,
        params: params::DeleteBso,
    ) -> DbFuture<'_, results::DeleteBso, Self::Error>;

    fn get_bso(&self, params: params::GetBso)
        -> DbFuture<'_, Option<results::GetBso>, Self::Error>;

    fn get_bso_timestamp(
        &self,
        params: params::GetBsoTimestamp,
    ) -> DbFuture<'_, results::GetBsoTimestamp, Self::Error>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<'_, results::PutBso, Self::Error>;

    fn create_batch(
        &self,
        params: params::CreateBatch,
    ) -> DbFuture<'_, results::CreateBatch, Self::Error>;

    fn validate_batch(
        &self,
        params: params::ValidateBatch,
    ) -> DbFuture<'_, results::ValidateBatch, Self::Error>;

    fn append_to_batch(
        &self,
        params: params::AppendToBatch,
    ) -> DbFuture<'_, results::AppendToBatch, Self::Error>;

    fn get_batch(
        &self,
        params: params::GetBatch,
    ) -> DbFuture<'_, Option<results::GetBatch>, Self::Error>;

    fn commit_batch(
        &self,
        params: params::CommitBatch,
    ) -> DbFuture<'_, results::CommitBatch, Self::Error>;

    fn box_clone(&self) -> Box<dyn Db<Error = Self::Error>>;

    fn check(&self) -> DbFuture<'_, results::Check, Self::Error>;

    fn get_connection_info(&self) -> results::ConnectionInfo;

    /// Retrieve the timestamp for an item/collection
    ///
    /// Modeled on the Python `get_resource_timestamp` function.
    fn extract_resource(
        &self,
        user_id: UserIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> DbFuture<'_, SyncTimestamp, Self::Error> {
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

    fn get_collection_id(&self, name: String) -> DbFuture<'_, i32, Self::Error>;

    fn create_collection(&self, name: String) -> DbFuture<'_, i32, Self::Error>;

    fn update_collection(
        &self,
        params: params::UpdateCollection,
    ) -> DbFuture<'_, SyncTimestamp, Self::Error>;

    fn timestamp(&self) -> SyncTimestamp;

    fn set_timestamp(&self, timestamp: SyncTimestamp);

    fn delete_batch(&self, params: params::DeleteBatch) -> DbFuture<'_, (), Self::Error>;

    fn clear_coll_cache(&self) -> DbFuture<'_, (), Self::Error>;

    fn set_quota(&mut self, enabled: bool, limit: usize, enforce: bool);
}

impl<E> Clone for Box<dyn Db<Error = E>>
where
    E: DbErrorIntrospect + 'static,
{
    fn clone(&self) -> Box<dyn Db<Error = E>> {
        self.box_clone()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Copy)]
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
