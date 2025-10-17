//! Mock db implementation with methods stubbed to return default values.
#![allow(clippy::new_without_default)]
use async_trait::async_trait;
use syncserver_db_common::{GetPoolState, PoolState};
use syncstorage_db_common::{params, results, util::SyncTimestamp, BatchDb, Db, DbPool};

use crate::DbError;

#[derive(Clone, Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

#[async_trait]
impl DbPool for MockDbPool {
    type Error = DbError;

    async fn get(&self) -> Result<Box<dyn Db<Error = DbError>>, Self::Error> {
        Ok(Box::new(MockDb::new()))
    }

    fn validate_batch_id(&self, _: params::ValidateBatchId) -> Result<(), DbError> {
        Ok(())
    }

    fn box_clone(&self) -> Box<dyn DbPool<Error = DbError>> {
        Box::new(self.clone())
    }
}

impl GetPoolState for MockDbPool {
    fn state(&self) -> PoolState {
        PoolState::default()
    }
}

#[derive(Clone, Debug)]
pub struct MockDb;

impl MockDb {
    pub fn new() -> Self {
        MockDb
    }
}

#[async_trait(?Send)]
impl Db for MockDb {
    async fn commit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn begin(&mut self, _for_write: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn check(&mut self) -> Result<results::Check, Self::Error> {
        Ok(true)
    }

    async fn lock_for_read(
        &mut self,
        _params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        Ok(())
    }

    async fn lock_for_write(
        &mut self,
        _params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        Ok(())
    }

    async fn get_collection_timestamps(
        &mut self,
        _params: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps, Self::Error> {
        Ok(Default::default())
    }

    async fn get_collection_timestamp(
        &mut self,
        _params: params::GetCollectionTimestamp,
    ) -> Result<results::GetCollectionTimestamp, Self::Error> {
        Ok(Default::default())
    }

    async fn get_collection_counts(
        &mut self,
        _params: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts, Self::Error> {
        Ok(Default::default())
    }

    async fn get_collection_usage(
        &mut self,
        _params: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage, Self::Error> {
        Ok(Default::default())
    }

    async fn get_storage_timestamp(
        &mut self,
        _params: params::GetStorageTimestamp,
    ) -> Result<results::GetStorageTimestamp, Self::Error> {
        Ok(Default::default())
    }

    async fn get_storage_usage(
        &mut self,
        _params: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage, Self::Error> {
        Ok(Default::default())
    }

    async fn get_quota_usage(
        &mut self,
        _params: params::GetQuotaUsage,
    ) -> Result<results::GetQuotaUsage, Self::Error> {
        Ok(Default::default())
    }

    async fn delete_storage(
        &mut self,
        _params: params::DeleteStorage,
    ) -> Result<results::DeleteStorage, Self::Error> {
        Ok(())
    }

    async fn delete_collection(
        &mut self,
        _params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection, Self::Error> {
        Ok(Default::default())
    }

    async fn delete_bsos(
        &mut self,
        _params: params::DeleteBsos,
    ) -> Result<results::DeleteBsos, Self::Error> {
        Ok(Default::default())
    }

    async fn get_bsos(
        &mut self,
        _params: params::GetBsos,
    ) -> Result<results::GetBsos, Self::Error> {
        Ok(Default::default())
    }

    async fn get_bso_ids(
        &mut self,
        _params: params::GetBsoIds,
    ) -> Result<results::GetBsoIds, Self::Error> {
        Ok(Default::default())
    }

    async fn post_bsos(
        &mut self,
        _params: params::PostBsos,
    ) -> Result<results::PostBsos, Self::Error> {
        Ok(Default::default())
    }

    async fn delete_bso(
        &mut self,
        _params: params::DeleteBso,
    ) -> Result<results::DeleteBso, Self::Error> {
        Ok(Default::default())
    }

    async fn get_bso(
        &mut self,
        _params: params::GetBso,
    ) -> Result<Option<results::GetBso>, Self::Error> {
        Ok(Default::default())
    }

    async fn get_bso_timestamp(
        &mut self,
        _params: params::GetBsoTimestamp,
    ) -> Result<results::GetBsoTimestamp, Self::Error> {
        Ok(Default::default())
    }

    async fn put_bso(&mut self, _params: params::PutBso) -> Result<results::PutBso, Self::Error> {
        Ok(Default::default())
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        Default::default()
    }

    async fn get_collection_id(
        &mut self,
        _params: &str,
    ) -> Result<results::GetCollectionId, Self::Error> {
        Ok(Default::default())
    }

    async fn create_collection(
        &mut self,
        _params: &str,
    ) -> Result<results::CreateCollection, Self::Error> {
        Ok(Default::default())
    }

    async fn update_collection(
        &mut self,
        _params: params::UpdateCollection,
    ) -> Result<results::UpdateCollection, Self::Error> {
        Ok(Default::default())
    }

    fn timestamp(&self) -> SyncTimestamp {
        Default::default()
    }

    fn set_timestamp(&mut self, _: SyncTimestamp) {}

    async fn clear_coll_cache(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_quota(&mut self, _: bool, _: usize, _: bool) {}
}

#[async_trait(?Send)]
impl BatchDb for MockDb {
    type Error = DbError;

    async fn create_batch(
        &mut self,
        _params: params::CreateBatch,
    ) -> Result<results::CreateBatch, Self::Error> {
        Ok(Default::default())
    }

    async fn validate_batch(
        &mut self,
        _params: params::ValidateBatch,
    ) -> Result<results::ValidateBatch, Self::Error> {
        Ok(Default::default())
    }

    async fn append_to_batch(
        &mut self,
        _params: params::AppendToBatch,
    ) -> Result<results::AppendToBatch, Self::Error> {
        Ok(())
    }

    async fn get_batch(
        &mut self,
        _params: params::GetBatch,
    ) -> Result<Option<results::GetBatch>, Self::Error> {
        Ok(Default::default())
    }

    async fn commit_batch(
        &mut self,
        _params: params::CommitBatch,
    ) -> Result<results::CommitBatch, Self::Error> {
        Ok(Default::default())
    }

    async fn delete_batch(
        &mut self,
        _params: params::DeleteBatch,
    ) -> Result<results::DeleteBatch, Self::Error> {
        Ok(())
    }
}
