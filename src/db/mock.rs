//! Mock db implementation with methods stubbed to return default values.

use futures::future;

use super::*;

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
            Box::new(future::result(Ok(results::$type::default())))
        }
    }
}

impl Db for MockDb {
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
    mock_db_method!(put_bso, PutBso);
}

unsafe impl Send for MockDb {}
