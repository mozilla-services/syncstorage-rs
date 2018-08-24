// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Mock db implementation with methods stubbed to return default values.

use futures::future;

use super::*;

#[derive(Debug)]
pub struct MockDb;

macro_rules! mock_db_method {
    ($name:ident, $type:ident) => {
        fn $name(&self, _params: params::$type) -> DbFuture<results::$type> {
            Box::new(future::result(Ok(results::$type::default())))
        }
    }
}

impl Db for MockDb {
    fn new() -> Box<Db> {
        Box::new(MockDb)
    }

    mock_db_method!(get_collections, GetCollections);
    mock_db_method!(get_collection_counts, GetCollectionCounts);
    mock_db_method!(get_collection_usage, GetCollectionUsage);
    mock_db_method!(get_quota, GetQuota);
    mock_db_method!(delete_all, DeleteAll);
    mock_db_method!(delete_collection, DeleteCollection);
    mock_db_method!(get_collection, GetCollection);
    mock_db_method!(post_collection, PostCollection);
    mock_db_method!(delete_bso, DeleteBso);
    mock_db_method!(get_bso, GetBso);
    mock_db_method!(put_bso, PutBso);
}

unsafe impl Send for MockDb {}
