//! Mock db implementation with methods stubbed to return default values.

use futures::future;

use super::*;

#[derive(Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

impl DbPool for MockDbPool {
    fn get(&self) -> DbFuture<Box<dyn Db>> {
        Box::new(future::ok(Box::new(MockDb::new()) as Box<dyn Db>))
    }
}

#[derive(Debug)]
pub struct MockDb;

impl MockDb {
    pub fn new() -> Self {
        MockDb
    }
}

macro_rules! mock_db_method {
    ($name:ident, $type:ident) => {
        fn $name(&self, _params: &params::$type) -> DbFuture<results::$type> {
            Box::new(future::ok(results::$type::default()))
        }
    }
}

// XXX: temporary: takes ownership of params vs mock_db_method taking
// a reference
macro_rules! mock_db_method2 {
    ($name:ident, $type:ident) => {
        fn $name(&self, _params: params::$type) -> DbFuture<results::$type> {
            Box::new(future::ok(results::$type::default()))
        }
    }
}

impl Db for MockDb {
    fn commit(&self) -> DbFuture<()> {
        Box::new(future::ok(()))
    }

    fn rollback(&self) -> DbFuture<()> {
        Box::new(future::ok(()))
    }

    mock_db_method2!(lock_for_read, LockCollection);
    mock_db_method2!(lock_for_write, LockCollection);

    mock_db_method!(get_collection_id, GetCollectionId);
    mock_db_method!(get_collections, GetCollections);
    mock_db_method!(get_collection_counts, GetCollectionCounts);
    mock_db_method!(get_collection_usage, GetCollectionUsage);
    mock_db_method!(get_storage_usage, GetStorageUsage);
    mock_db_method!(delete_all, DeleteAll);
    mock_db_method!(delete_collection, DeleteCollection);
    mock_db_method!(get_collection, GetCollection);
    mock_db_method!(post_collection, PostCollection);
    mock_db_method!(delete_bso, DeleteBso);
    mock_db_method!(get_bso, GetBso);

    mock_db_method2!(put_bso, PutBso);
}

unsafe impl Send for MockDb {}
