//! Generic db abstration.

pub mod error;
pub mod mock;
pub mod mysql;
pub mod params;
pub mod results;
pub mod util;

use futures::future::Future;

pub use self::error::{DbError, DbErrorKind};
use self::util::SyncTimestamp;
use web::extractors::HawkIdentifier;

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

    fn get_collection_modified(
        &self,
        params: params::GetCollectionModified,
    ) -> DbFuture<results::GetCollectionModified>;

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

    fn get_bso_modified(&self, params: params::GetBsoModified)
        -> DbFuture<results::GetBsoModified>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<results::PutBso>;

    fn create_batch(&self, params: params::CreateBatch) -> DbFuture<results::CreateBatch>;

    fn validate_batch(&self, params: params::ValidateBatch) -> DbFuture<results::ValidateBatch>;

    fn append_to_batch(&self, params: params::AppendToBatch) -> DbFuture<results::AppendToBatch>;

    fn get_batch(&self, params: params::GetBatch) -> DbFuture<Option<results::GetBatch>>;

    fn delete_batch(&self, params: params::DeleteBatch) -> DbFuture<results::DeleteBatch>;

    fn box_clone(&self) -> Box<dyn Db>;

    /// Retrieve the timestamp for an item/collection
    ///
    /// Modeled on the Python `get_resource_timestamp` function.
    fn extract_resource(
        &self,
        user_id: HawkIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> DbFuture<SyncTimestamp> {
        // If there's no collection, we return the overall storage timestamp
        let collection = match collection {
            Some(collection) => collection,
            None => return Box::new(self.get_storage_modified(user_id)),
        };
        // If there's no bso, return the collection
        let bso = match bso {
            Some(bso) => bso,
            None => {
                return Box::new(
                    self.get_collection_modified(params::GetCollectionModified {
                        user_id,
                        collection,
                    }).then(|v| match v {
                        Ok(v) => Ok(v),
                        Err(e) => match e.kind() {
                            DbErrorKind::CollectionNotFound => {
                                Ok(SyncTimestamp::from_seconds(0f64))
                            }
                            _ => Err(e),
                        },
                    }),
                )
            }
        };
        Box::new(
            self.get_bso_modified(params::GetBsoModified {
                user_id,
                collection,
                id: bso,
            }).then(|v| match v {
                Ok(v) => Ok(v),
                Err(e) => match e.kind() {
                    DbErrorKind::CollectionNotFound => Ok(SyncTimestamp::from_seconds(0f64)),
                    _ => Err(e),
                },
            }),
        )
    }
}

impl Clone for Box<dyn Db> {
    fn clone(&self) -> Box<dyn Db> {
        self.box_clone()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
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
