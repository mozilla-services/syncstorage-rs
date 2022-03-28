//! Mock db implementation with methods stubbed to return default values.
#![allow(clippy::new_without_default)]
use async_trait::async_trait;
use mockall::mock;

use super::*;

#[derive(Clone, Debug)]
pub struct MockDbPool;

#[async_trait]
impl DbPool for MockDbPool {
    async fn get(&self) -> ApiResult<Box<dyn Db>> {
        Ok(Box::new(MockDb::new()) as Box<dyn Db>)
    }

    fn state(&self) -> results::PoolState {
        results::PoolState::default()
    }

    fn validate_batch_id(&self, _: params::ValidateBatchId) -> ApiResult<()> {
        Ok(())
    }
}

mock! {
    #[derive(Debug)]
    pub Db {}

    impl Clone for Db {
        fn clone(&self) -> Self;
    }

    #[async_trait(?Send)]
    impl Db for Db {
        async fn lock_for_read(&self, params: params::LockCollection) -> ApiResult<()>;
        async fn lock_for_write(&self, params: params::LockCollection) -> ApiResult<()>;
        async fn begin(&self, for_write: bool) -> ApiResult<()>;
        async fn commit(&self) -> ApiResult<()>;
        async fn rollback(&self) -> ApiResult<()>;

        async fn get_collection_timestamps(
            &self,
            params: params::GetCollectionTimestamps,
        ) -> ApiResult<results::GetCollectionTimestamps>;

        async fn get_collection_timestamp(
            &self,
            params: params::GetCollectionTimestamp,
        ) -> ApiResult<results::GetCollectionTimestamp>;

        async fn get_collection_counts(
            &self,
            params: params::GetCollectionCounts,
        ) -> ApiResult<results::GetCollectionCounts>;

        async fn get_collection_usage(
            &self,
            params: params::GetCollectionUsage,
        ) -> ApiResult<results::GetCollectionUsage>;

        async fn get_storage_timestamp(
            &self,
            params: params::GetStorageTimestamp,
        ) -> ApiResult<results::GetStorageTimestamp>;

        async fn get_storage_usage(
            &self,
            params: params::GetStorageUsage,
        ) -> ApiResult<results::GetStorageUsage>;

        async fn get_quota_usage(
            &self,
            params: params::GetQuotaUsage,
        ) -> ApiResult<results::GetQuotaUsage>;

        async fn delete_storage(
            &self,
            params: params::DeleteStorage,
        ) -> ApiResult<results::DeleteStorage>;

        async fn delete_collection(
            &self,
            params: params::DeleteCollection,
        ) -> ApiResult<results::DeleteCollection>;

        async fn delete_bsos(&self, params: params::DeleteBsos) -> ApiResult<results::DeleteBsos>;
        async fn get_bsos(&self, params: params::GetBsos) -> ApiResult<results::GetBsos>;
        async fn get_bso_ids(&self, params: params::GetBsos) -> ApiResult<results::GetBsoIds>;
        async fn post_bsos(&self, params: params::PostBsos) -> ApiResult<results::PostBsos>;
        async fn delete_bso(&self, params: params::DeleteBso) -> ApiResult<results::DeleteBso>;
        async fn get_bso(&self, params: params::GetBso) -> ApiResult<Option<results::GetBso>>;

        async fn get_bso_timestamp(
            &self,
            params: params::GetBsoTimestamp,
        ) -> ApiResult<results::GetBsoTimestamp>;

        async fn put_bso(&self, params: params::PutBso) -> ApiResult<results::PutBso>;
        async fn create_batch(&self, params: params::CreateBatch) -> ApiResult<results::CreateBatch>;

        async fn validate_batch(
            &self,
            params: params::ValidateBatch,
        ) -> ApiResult<results::ValidateBatch>;

        async fn append_to_batch(
            &self,
            params: params::AppendToBatch,
        ) -> ApiResult<results::AppendToBatch>;

        async fn get_batch(&self, params: params::GetBatch) -> ApiResult<Option<results::GetBatch>>;
        async fn commit_batch(&self, params: params::CommitBatch) -> ApiResult<results::CommitBatch>;
        async fn check(&self) -> ApiResult<results::Check>;
        fn get_connection_info(&self) -> results::ConnectionInfo;

        async fn extract_resource(
            &self,
            user_id: HawkIdentifier,
            collection: Option<String>,
            bso: Option<String>,
        ) -> ApiResult<SyncTimestamp>;

        async fn get_collection_id(&self, name: String) -> ApiResult<i32>;

        #[cfg(test)]
        async fn create_collection(&self, name: String) -> ApiResult<i32>;

        #[cfg(test)]
        async fn update_collection(&self, params: params::UpdateCollection) -> ApiResult<SyncTimestamp>;

        #[cfg(test)]
        fn timestamp(&self) -> SyncTimestamp;

        #[cfg(test)]
        fn set_timestamp(&self, timestamp: SyncTimestamp);

        #[cfg(test)]
        async fn delete_batch(&self, params: params::DeleteBatch) -> ApiResult<()>;

        #[cfg(test)]
        async fn clear_coll_cache(&self) -> ApiResult<()>;

        #[cfg(test)]
        fn set_quota(&mut self, enabled: bool, limit: usize, enforce: bool);
    }

}
