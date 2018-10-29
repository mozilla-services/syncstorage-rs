//! Generic db abstration.

pub mod error;
pub mod mock;
pub mod mysql;
pub mod params;
pub mod results;
pub mod util;

use futures::future::Future;

pub use self::error::{DbError, DbErrorKind};

lazy_static! {
    /// For efficiency, it's possible to use fixed pre-determined IDs for
    /// common collection names.  This is the canonical list of such
    /// names.  Non-standard collections will be allocated IDs starting
    /// from the highest ID in this collection.
    static ref STD_COLLS: Vec<(i32, &'static str)> = {
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

type DbFuture<T> = Box<Future<Item = T, Error = DbError>>;

pub trait DbPool: Sync {
    fn get(&self) -> DbFuture<Box<dyn Db>>;
}

pub trait Db: Send {
    fn lock_for_read(&self, params: params::LockCollection) -> DbFuture<()>;

    fn lock_for_write(&self, params: params::LockCollection) -> DbFuture<()>;

    fn commit(&self) -> DbFuture<()>;

    fn rollback(&self) -> DbFuture<()>;

    fn get_collection_modifieds(
        &self,
        params: params::GetCollectionModifieds,
    ) -> DbFuture<results::GetCollectionModifieds>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<results::GetCollectionUsage>;

    fn get_storage_modified(
        &self,
        params: params::GetStorageModified,
    ) -> DbFuture<results::GetStorageModified>;

    fn get_storage_usage(
        &self,
        params: params::GetStorageUsage,
    ) -> DbFuture<results::GetStorageUsage>;

    fn delete_storage(&self, params: params::DeleteStorage) -> DbFuture<results::DeleteStorage>;

    fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> DbFuture<results::DeleteCollection>;

    fn delete_bsos(&self, params: params::DeleteBsos) -> DbFuture<results::DeleteBsos>;

    fn get_bsos(&self, params: params::GetBsos) -> DbFuture<results::GetBsos>;

    fn post_bsos(&self, params: params::PostBsos) -> DbFuture<results::PostBsos>;

    fn delete_bso(&self, params: params::DeleteBso) -> DbFuture<results::DeleteBso>;

    fn get_bso(&self, params: params::GetBso) -> DbFuture<Option<results::GetBso>>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<results::PutBso>;

    fn box_clone(&self) -> Box<dyn Db>;
}

impl Clone for Box<dyn Db> {
    fn clone(&self) -> Box<dyn Db> {
        self.box_clone()
    }
}

#[derive(Debug)]
pub enum Sorting {
    None,
    Newest,
    Oldest,
    Index,
}
