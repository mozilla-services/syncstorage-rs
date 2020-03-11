//! Mock db implementation with methods stubbed to return default values.
#![allow(clippy::new_without_default)]
use futures::future;

use super::*;

#[derive(Clone, Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

impl DbPool for MockDbPool {
    fn get(&self) -> DbFuture<Box<dyn Db>> {
        Box::pin(future::ok(Box::new(MockDb::new()) as Box<dyn Db>))
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Debug)]
pub struct MockDb;

impl MockDb {
    pub fn new() -> Self {
        MockDb
    }
}

macro_rules! mock_db_method {
    ($name:ident, $type:ident) => {
        mock_db_method!($name, $type, results::$type);
    };
    ($name:ident, $type:ident, $result:ty) => {
        fn $name(&self, _params: params::$type) -> DbFuture<$result> {
            let result: $result = Default::default();
            Box::pin(future::ok(result))
        }
    };
}

impl Db for MockDb {
    fn commit(&self) -> DbFuture<()> {
        Box::pin(future::ok(()))
    }

    fn rollback(&self) -> DbFuture<()> {
        Box::pin(future::ok(()))
    }

    fn box_clone(&self) -> Box<dyn Db> {
        Box::new(self.clone())
    }

    fn check(&self) -> DbFuture<results::Check> {
        Box::pin(future::ok(true))
    }

    mock_db_method!(lock_for_read, LockCollection);
    mock_db_method!(lock_for_write, LockCollection);
    mock_db_method!(get_collection_timestamps, GetCollectionTimestamps);
    mock_db_method!(get_collection_timestamp, GetCollectionTimestamp);
    mock_db_method!(get_collection_counts, GetCollectionCounts);
    mock_db_method!(get_collection_usage, GetCollectionUsage);
    mock_db_method!(get_storage_timestamp, GetStorageTimestamp);
    mock_db_method!(get_storage_usage, GetStorageUsage);
    mock_db_method!(delete_storage, DeleteStorage);
    mock_db_method!(delete_collection, DeleteCollection);
    mock_db_method!(delete_bsos, DeleteBsos);
    mock_db_method!(get_bsos, GetBsos);
    mock_db_method!(get_bso_ids, GetBsoIds);
    mock_db_method!(post_bsos, PostBsos);
    mock_db_method!(delete_bso, DeleteBso);
    mock_db_method!(get_bso, GetBso, Option<results::GetBso>);
    mock_db_method!(get_bso_timestamp, GetBsoTimestamp);
    mock_db_method!(put_bso, PutBso);
    mock_db_method!(create_batch, CreateBatch);
    mock_db_method!(validate_batch, ValidateBatch);
    mock_db_method!(append_to_batch, AppendToBatch);
    mock_db_method!(get_batch, GetBatch, Option<results::GetBatch>);
    mock_db_method!(commit_batch, CommitBatch);

    fn validate_batch_id(&self, _: params::ValidateBatchId) -> Result<(), DbError> {
        Ok(())
    }

    #[cfg(test)]
    mock_db_method!(get_collection_id, GetCollectionId);
    #[cfg(test)]
    mock_db_method!(create_collection, CreateCollection);
    #[cfg(test)]
    mock_db_method!(touch_collection, TouchCollection);

    #[cfg(test)]
    fn timestamp(&self) -> SyncTimestamp {
        Default::default()
    }

    #[cfg(test)]
    fn set_timestamp(&self, _: SyncTimestamp) {}

    #[cfg(test)]
    mock_db_method!(delete_batch, DeleteBatch);

    #[cfg(test)]
    fn clear_coll_cache(&self) {}
}

unsafe impl Send for MockDb {}
