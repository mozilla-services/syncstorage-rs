//! Mock db implementation with methods stubbed to return default values.
#![allow(clippy::new_without_default)]
use async_trait::async_trait;
use futures::future;
use syncserver_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, Db, DbPool,
};

type DbFuture<'a, T> = syncserver_db_common::DbFuture<'a, T, DbError>;

pub struct DbError;

impl DbErrorIntrospect for DbError {
    fn is_batch_not_found(&self) -> bool {
        false
    }

    fn is_bso_not_found(&self) -> bool {
        false
    }

    fn is_collection_not_found(&self) -> bool {
        false
    }

    fn is_conflict(&self) -> bool {
        false
    }

    fn is_quota(&self) -> bool {
        false
    }

    fn is_sentry_event(&self) -> bool {
        false
    }

    fn metric_label(&self) -> Option<String> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

#[async_trait]
impl<'a> DbPool for MockDbPool {
    type Db = MockDb;
    type Error = DbError;

    async fn get(&self) -> Result<Self::Db, Self::Error> {
        Ok(MockDb::new())
    }

    fn validate_batch_id(&self, _: params::ValidateBatchId) -> Result<(), DbError> {
        Ok(())
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
        fn $name(&self, _params: params::$type) -> DbFuture<'_, $result> {
            let result: $result = Default::default();
            Box::pin(future::ok(result))
        }
    };
}

impl Db for MockDb {
    type Error = DbError;

    fn commit(&self) -> DbFuture<'_, ()> {
        Box::pin(future::ok(()))
    }

    fn rollback(&self) -> DbFuture<'_, ()> {
        Box::pin(future::ok(()))
    }

    fn begin(&self, _for_write: bool) -> DbFuture<'_, ()> {
        Box::pin(future::ok(()))
    }

    fn check(&self) -> DbFuture<'_, results::Check> {
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
    mock_db_method!(get_quota_usage, GetQuotaUsage);
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

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    mock_db_method!(get_collection_id, GetCollectionId);
    mock_db_method!(create_collection, CreateCollection);
    mock_db_method!(update_collection, UpdateCollection);

    fn timestamp(&self) -> SyncTimestamp {
        Default::default()
    }

    fn set_timestamp(&self, _: SyncTimestamp) {}

    mock_db_method!(delete_batch, DeleteBatch);

    fn clear_coll_cache(&self) -> DbFuture<'_, ()> {
        Box::pin(future::ok(()))
    }

    fn set_quota(&mut self, _: bool, _: usize, _: bool) {}
}

unsafe impl Send for MockDb {}
