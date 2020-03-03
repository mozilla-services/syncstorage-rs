//! Generic db abstration.

pub mod error;
pub mod mock;
pub mod mysql;
pub mod params;
pub mod results;
pub mod spanner;
#[cfg(test)]
mod tests;
pub mod util;

use std::fmt::Debug;

use futures::future::{self, LocalBoxFuture, TryFutureExt};
use lazy_static::lazy_static;
use serde::Deserialize;
use url::Url;

pub use self::error::{DbError, DbErrorKind};
use self::util::SyncTimestamp;
use crate::error::ApiError;
use crate::server::metrics::Metrics;
use crate::settings::Settings;
use crate::web::extractors::HawkIdentifier;

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

/// Non-standard collections will be allocated IDs beginning with this value
pub const FIRST_CUSTOM_COLLECTION_ID: i32 = 101;

/// Rough guesstimate of the maximum reasonable life span of a batch
pub const BATCH_LIFETIME: i64 = 2 * 60 * 60 * 1000; // 2 hours, in milliseconds

/// DbPools' worker ThreadPool size
pub const DB_THREAD_POOL_SIZE: usize = 50;

type DbFuture<T> = LocalBoxFuture<'static, Result<T, ApiError>>;

pub trait DbPool: Sync + Send + Debug {
    fn get(&self) -> DbFuture<Box<dyn Db>>;
    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}

pub trait Db: Send + Debug {
    fn lock_for_read(&self, params: params::LockCollection) -> DbFuture<()>;

    fn lock_for_write(&self, params: params::LockCollection) -> DbFuture<()>;

    fn commit(&self) -> DbFuture<()>;

    fn rollback(&self) -> DbFuture<()>;

    fn get_collection_timestamps(
        &self,
        params: params::GetCollectionTimestamps,
    ) -> DbFuture<results::GetCollectionTimestamps>;

    fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> DbFuture<results::GetCollectionTimestamp>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<results::GetCollectionUsage>;

    fn get_storage_timestamp(
        &self,
        params: params::GetStorageTimestamp,
    ) -> DbFuture<results::GetStorageTimestamp>;

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

    fn get_bso_ids(&self, params: params::GetBsos) -> DbFuture<results::GetBsoIds>;

    fn post_bsos(&self, params: params::PostBsos) -> DbFuture<results::PostBsos>;

    fn delete_bso(&self, params: params::DeleteBso) -> DbFuture<results::DeleteBso>;

    fn get_bso(&self, params: params::GetBso) -> DbFuture<Option<results::GetBso>>;

    fn get_bso_timestamp(
        &self,
        params: params::GetBsoTimestamp,
    ) -> DbFuture<results::GetBsoTimestamp>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<results::PutBso>;

    fn create_batch(&self, params: params::CreateBatch) -> DbFuture<results::CreateBatch>;

    fn validate_batch(&self, params: params::ValidateBatch) -> DbFuture<results::ValidateBatch>;

    fn append_to_batch(&self, params: params::AppendToBatch) -> DbFuture<results::AppendToBatch>;

    fn get_batch(&self, params: params::GetBatch) -> DbFuture<Option<results::GetBatch>>;

    fn commit_batch(&self, params: params::CommitBatch) -> DbFuture<results::CommitBatch>;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> Result<(), DbError>;

    fn box_clone(&self) -> Box<dyn Db>;

    fn check(&self) -> DbFuture<results::Check>;

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
            None => return Box::pin(self.get_storage_timestamp(user_id)),
        };
        // If there's no bso, return the collection
        let bso = match bso {
            Some(bso) => bso,
            None => {
                return Box::pin(
                    self.get_collection_timestamp(params::GetCollectionTimestamp {
                        user_id,
                        collection,
                    })
                    .or_else(|e| {
                        if e.is_collection_not_found() {
                            future::ok(SyncTimestamp::from_seconds(0f64))
                        } else {
                            future::err(e)
                        }
                    }),
                )
            }
        };
        Box::pin(
            self.get_bso_timestamp(params::GetBsoTimestamp {
                user_id,
                collection,
                id: bso,
            })
            .or_else(|e| {
                if e.is_collection_not_found() {
                    future::ok(SyncTimestamp::from_seconds(0f64))
                } else {
                    future::err(e)
                }
            }),
        )
    }

    /// Internal methods used by the db tests

    #[cfg(test)]
    fn get_collection_id(&self, name: String) -> DbFuture<i32>;

    #[cfg(test)]
    fn create_collection(&self, name: String) -> DbFuture<i32>;

    #[cfg(test)]
    fn touch_collection(&self, params: params::TouchCollection) -> DbFuture<SyncTimestamp>;

    #[cfg(test)]
    fn timestamp(&self) -> SyncTimestamp;

    #[cfg(test)]
    fn set_timestamp(&self, timestamp: SyncTimestamp);

    #[cfg(test)]
    fn delete_batch(&self, params: params::DeleteBatch) -> DbFuture<()>;

    #[cfg(test)]
    fn clear_coll_cache(&self);
}

impl Clone for Box<dyn Db> {
    fn clone(&self) -> Box<dyn Db> {
        self.box_clone()
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq, Copy)]
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

/// Create/initialize a pool of managed Db connections
// XXX: should likely return a Future?
pub fn pool_from_settings(
    settings: &Settings,
    metrics: &Metrics,
) -> Result<Box<dyn DbPool>, DbError> {
    let url =
        Url::parse(&settings.database_url).map_err(|e| DbErrorKind::InvalidUrl(e.to_string()))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(&settings, &metrics)?),
        "spanner" => Box::new(spanner::pool::SpannerDbPool::new(&settings, &metrics)?),
        _ => Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?,
    })
}
