#![allow(non_local_definitions)]
pub mod diesel;
pub mod error;
pub mod params;
pub mod results;
pub mod util;

use std::fmt::Debug;

use async_trait::async_trait;
use lazy_static::lazy_static;
use serde::Deserialize;
use syncserver_db_common::GetPoolState;

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

    async fn init(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn get(&self) -> Result<Box<dyn Db<Error = Self::Error>>, Self::Error>;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> Result<(), Self::Error>;

    fn box_clone(&self) -> Box<dyn DbPool<Error = Self::Error>>;
}

impl<E> Clone for Box<dyn DbPool<Error = E>> {
    fn clone(&self) -> Box<dyn DbPool<Error = E>> {
        self.box_clone()
    }
}

#[async_trait(?Send)]
pub trait Db: BatchDb {
    async fn lock_for_read(&mut self, params: params::LockCollection) -> Result<(), Self::Error>;

    async fn lock_for_write(&mut self, params: params::LockCollection) -> Result<(), Self::Error>;

    async fn begin(&mut self, for_write: bool) -> Result<(), Self::Error>;

    async fn commit(&mut self) -> Result<(), Self::Error>;

    async fn rollback(&mut self) -> Result<(), Self::Error>;

    async fn get_collection_timestamps(
        &mut self,
        params: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps, Self::Error>;

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> Result<results::GetCollectionTimestamp, Self::Error>;

    async fn get_collection_counts(
        &mut self,
        params: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts, Self::Error>;

    async fn get_collection_usage(
        &mut self,
        params: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage, Self::Error>;

    async fn get_storage_timestamp(
        &mut self,
        params: params::GetStorageTimestamp,
    ) -> Result<results::GetStorageTimestamp, Self::Error>;

    async fn get_storage_usage(
        &mut self,
        params: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage, Self::Error>;

    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> Result<results::GetQuotaUsage, Self::Error>;

    async fn delete_storage(
        &mut self,
        params: params::DeleteStorage,
    ) -> Result<results::DeleteStorage, Self::Error>;

    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection, Self::Error>;

    async fn delete_bsos(
        &mut self,
        params: params::DeleteBsos,
    ) -> Result<results::DeleteBsos, Self::Error>;

    async fn get_bsos(&mut self, params: params::GetBsos) -> Result<results::GetBsos, Self::Error>;

    async fn get_bso_ids(
        &mut self,
        params: params::GetBsos,
    ) -> Result<results::GetBsoIds, Self::Error>;

    async fn post_bsos(&mut self, params: params::PostBsos) -> Result<SyncTimestamp, Self::Error>;

    async fn delete_bso(
        &mut self,
        params: params::DeleteBso,
    ) -> Result<results::DeleteBso, Self::Error>;

    async fn get_bso(
        &mut self,
        params: params::GetBso,
    ) -> Result<Option<results::GetBso>, Self::Error>;

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> Result<results::GetBsoTimestamp, Self::Error>;

    async fn put_bso(&mut self, params: params::PutBso) -> Result<results::PutBso, Self::Error>;

    async fn check(&mut self) -> Result<results::Check, Self::Error>;

    fn get_connection_info(&self) -> results::ConnectionInfo;

    /// Retrieve the timestamp for an item/collection
    async fn extract_resource(
        &mut self,
        user_id: UserIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> Result<SyncTimestamp, Self::Error> {
        match collection {
            None => {
                // No collection specified, return overall storage timestamp
                self.get_storage_timestamp(user_id).await
            }
            Some(collection) => match bso {
                None => self
                    .get_collection_timestamp(params::GetCollectionTimestamp {
                        user_id,
                        collection,
                    })
                    .await
                    .or_else(|e| {
                        if e.is_collection_not_found() {
                            Ok(SyncTimestamp::from_seconds(0f64))
                        } else {
                            Err(e)
                        }
                    }),
                Some(bso) => self
                    .get_bso_timestamp(params::GetBsoTimestamp {
                        user_id,
                        collection,
                        id: bso,
                    })
                    .await
                    .or_else(|e| {
                        if e.is_collection_not_found() {
                            Ok(SyncTimestamp::from_seconds(0f64))
                        } else {
                            Err(e)
                        }
                    }),
            },
        }
    }

    // Internal methods used by the db tests

    async fn get_collection_id(&mut self, name: &str) -> Result<i32, Self::Error>;

    async fn create_collection(&mut self, name: &str) -> Result<i32, Self::Error>;

    async fn update_collection(
        &mut self,
        params: params::UpdateCollection,
    ) -> Result<SyncTimestamp, Self::Error>;

    fn timestamp(&self) -> SyncTimestamp;

    fn set_timestamp(&mut self, timestamp: SyncTimestamp);

    async fn clear_coll_cache(&mut self) -> Result<(), Self::Error>;

    fn set_quota(&mut self, enabled: bool, limit: usize, enforce: bool);
}

#[async_trait(?Send)]
pub trait BatchDb: Debug {
    type Error: DbErrorIntrospect + 'static;

    async fn create_batch(
        &mut self,
        params: params::CreateBatch,
    ) -> Result<results::CreateBatch, Self::Error>;

    async fn validate_batch(
        &mut self,
        params: params::ValidateBatch,
    ) -> Result<results::ValidateBatch, Self::Error>;

    async fn append_to_batch(
        &mut self,
        params: params::AppendToBatch,
    ) -> Result<results::AppendToBatch, Self::Error>;

    async fn get_batch(
        &mut self,
        params: params::GetBatch,
    ) -> Result<Option<results::GetBatch>, Self::Error>;

    async fn commit_batch(
        &mut self,
        params: params::CommitBatch,
    ) -> Result<results::CommitBatch, Self::Error>;

    async fn delete_batch(&mut self, params: params::DeleteBatch) -> Result<(), Self::Error>;
}

#[derive(Debug, Default, Deserialize, Clone, PartialEq, Eq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Sorting {
    #[default]
    None,
    Newest,
    Oldest,
    Index,
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct UserIdentifier {
    /// For MySQL database backends as the primary key
    pub legacy_id: u64,
    /// For NoSQL database backends that require randomly distributed primary keys
    pub fxa_uid: String,
    pub fxa_kid: String,
    pub hashed_fxa_uid: String,
    pub hashed_device_id: String,
}
