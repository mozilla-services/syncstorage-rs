//! Generic db abstration.

pub mod error;
pub mod mock;
pub mod mysql;
pub mod params;
pub mod results;
pub mod spanner;
#[cfg(test)]
mod tests;
pub mod transaction;
pub mod util;

use std::{fmt::Debug, time::Duration};

use async_trait::async_trait;
use cadence::{Gauged, StatsdClient};
use futures::future::{self, LocalBoxFuture, TryFutureExt};
use lazy_static::lazy_static;
use serde::Deserialize;
use url::Url;

pub use self::error::{DbError, DbErrorKind};
use self::util::SyncTimestamp;
use crate::error::{ApiError, ApiResult};
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

type DbFuture<'a, T> = LocalBoxFuture<'a, Result<T, ApiError>>;

#[async_trait(?Send)]
pub trait DbPool: Sync + Send + Debug {
    async fn get(&self) -> ApiResult<Box<dyn Db<'_>>>;

    fn state(&self) -> results::PoolState;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> Result<(), DbError>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}

pub trait Db<'a>: Debug + 'a {
    fn lock_for_read(&self, params: params::LockCollection) -> DbFuture<'_, ()>;

    fn lock_for_write(&self, params: params::LockCollection) -> DbFuture<'_, ()>;

    fn begin(&self, for_write: bool) -> DbFuture<'_, ()>;

    fn commit(&self) -> DbFuture<'_, ()>;

    fn rollback(&self) -> DbFuture<'_, ()>;

    fn get_collection_timestamps(
        &self,
        params: params::GetCollectionTimestamps,
    ) -> DbFuture<'_, results::GetCollectionTimestamps>;

    fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> DbFuture<'_, results::GetCollectionTimestamp>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<'_, results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<'_, results::GetCollectionUsage>;

    fn get_storage_timestamp(
        &self,
        params: params::GetStorageTimestamp,
    ) -> DbFuture<'_, results::GetStorageTimestamp>;

    fn get_storage_usage(
        &self,
        params: params::GetStorageUsage,
    ) -> DbFuture<'_, results::GetStorageUsage>;

    fn get_quota_usage(
        &self,
        params: params::GetQuotaUsage,
    ) -> DbFuture<'_, results::GetQuotaUsage>;

    fn delete_storage(&self, params: params::DeleteStorage)
        -> DbFuture<'_, results::DeleteStorage>;

    fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> DbFuture<'_, results::DeleteCollection>;

    fn delete_bsos(&self, params: params::DeleteBsos) -> DbFuture<'_, results::DeleteBsos>;

    fn get_bsos(&self, params: params::GetBsos) -> DbFuture<'_, results::GetBsos>;

    fn get_bso_ids(&self, params: params::GetBsos) -> DbFuture<'_, results::GetBsoIds>;

    fn post_bsos(&self, params: params::PostBsos) -> DbFuture<'_, results::PostBsos>;

    fn delete_bso(&self, params: params::DeleteBso) -> DbFuture<'_, results::DeleteBso>;

    fn get_bso(&self, params: params::GetBso) -> DbFuture<'_, Option<results::GetBso>>;

    fn get_bso_timestamp(
        &self,
        params: params::GetBsoTimestamp,
    ) -> DbFuture<'_, results::GetBsoTimestamp>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<'_, results::PutBso>;

    fn create_batch(&self, params: params::CreateBatch) -> DbFuture<'_, results::CreateBatch>;

    fn validate_batch(&self, params: params::ValidateBatch)
        -> DbFuture<'_, results::ValidateBatch>;

    fn append_to_batch(
        &self,
        params: params::AppendToBatch,
    ) -> DbFuture<'_, results::AppendToBatch>;

    fn get_batch(&self, params: params::GetBatch) -> DbFuture<'_, Option<results::GetBatch>>;

    fn commit_batch(&self, params: params::CommitBatch) -> DbFuture<'_, results::CommitBatch>;

    fn box_clone(&self) -> Box<dyn Db<'a>>;

    fn check(&self) -> DbFuture<'_, results::Check>;

    /// Retrieve the timestamp for an item/collection
    ///
    /// Modeled on the Python `get_resource_timestamp` function.
    fn extract_resource(
        &self,
        user_id: HawkIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> DbFuture<'_, SyncTimestamp> {
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

    fn get_collection_id(&self, name: String) -> DbFuture<'_, i32>;

    #[cfg(test)]
    fn create_collection(&self, name: String) -> DbFuture<'_, i32>;

    #[cfg(test)]
    fn update_collection(&self, params: params::UpdateCollection) -> DbFuture<'_, SyncTimestamp>;

    #[cfg(test)]
    fn timestamp(&self) -> SyncTimestamp;

    #[cfg(test)]
    fn set_timestamp(&self, timestamp: SyncTimestamp);

    #[cfg(test)]
    fn delete_batch(&self, params: params::DeleteBatch) -> DbFuture<'_, ()>;

    #[cfg(test)]
    fn clear_coll_cache(&self);

    #[cfg(test)]
    fn set_quota(&mut self, enabled: bool, limit: usize);
}

impl<'a> Clone for Box<dyn Db<'a>> {
    fn clone(&self) -> Box<dyn Db<'a>> {
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
pub async fn pool_from_settings(
    settings: &Settings,
    metrics: &Metrics,
) -> Result<Box<dyn DbPool>, DbError> {
    let url =
        Url::parse(&settings.database_url).map_err(|e| DbErrorKind::InvalidUrl(e.to_string()))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(&settings, &metrics)?),
        "spanner" => Box::new(spanner::pool::SpannerDbPool::new(&settings, &metrics).await?),
        _ => Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()))?,
    })
}

/// Emit DbPool metrics periodically
pub fn spawn_pool_periodic_reporter(
    interval: Duration,
    metrics: StatsdClient,
    pool: Box<dyn DbPool>,
) -> Result<(), DbError> {
    let hostname = hostname::get()
        .expect("Couldn't get hostname")
        .into_string()
        .expect("Couldn't get hostname");
    actix_rt::spawn(async move {
        loop {
            let results::PoolState {
                connections,
                idle_connections,
            } = pool.state();
            metrics
                .gauge_with_tags(
                    "storage.pool.connections.active",
                    (connections - idle_connections) as u64,
                )
                .with_tag("hostname", &hostname)
                .send();
            metrics
                .gauge_with_tags("storage.pool.connections.idle", idle_connections as u64)
                .with_tag("hostname", &hostname)
                .send();
            actix_rt::time::delay_for(interval).await;
        }
    });
    Ok(())
}
