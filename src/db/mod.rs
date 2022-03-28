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

use actix_web::web;
use async_trait::async_trait;
use cadence::{Gauged, StatsdClient};
use diesel::result::QueryResult;
use dyn_clone::DynClone;
use lazy_static::lazy_static;
use mockall::automock;
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

#[automock]
#[async_trait]
pub trait DbPool: Sync + Send + Debug {
    async fn get(&self) -> ApiResult<Box<dyn Db>>;

    fn state(&self) -> results::PoolState;

    fn validate_batch_id(&self, params: params::ValidateBatchId) -> ApiResult<()>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}

#[async_trait(?Send)]
pub trait Db: Debug + DynClone {
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

    #[cfg(test)]
    async fn update_collection(&self, params: params::UpdateCollection)
        -> ApiResult<SyncTimestamp>;

    fn get_connection_info(&self) -> results::ConnectionInfo;

    /// Retrieve the timestamp for an item/collection
    ///
    /// Modeled on the Python `get_resource_timestamp` function.
    async fn extract_resource(
        &self,
        user_id: HawkIdentifier,
        collection: Option<String>,
        bso: Option<String>,
    ) -> ApiResult<SyncTimestamp> {
        // If there's no collection, we return the overall storage timestamp
        let collection = match collection {
            Some(collection) => collection,
            None => return self.get_storage_timestamp(user_id).await,
        };
        // If there's no bso, return the collection
        let bso = match bso {
            Some(bso) => bso,
            None => {
                return self
                    .get_collection_timestamp(params::GetCollectionTimestamp {
                        user_id,
                        collection,
                    })
                    .await
                    .or_else(|e| {
                        if e.is_collection_not_found() {
                            Ok(SyncTimestamp::from_seconds(0f64))
                        } else {
                            Err(e)
                        }
                    })
            }
        };

        self.get_bso_timestamp(params::GetBsoTimestamp {
            user_id,
            collection,
            id: bso,
        })
        .await
        .or_else(|e| {
            if e.is_collection_not_found() {
                Ok(SyncTimestamp::from_seconds(0f64))
            } else {
                Err(e)
            }
        })
    }

    /// Internal methods used by the db tests

    async fn get_collection_id(&self, name: String) -> ApiResult<i32>;

    #[cfg(test)]
    async fn create_collection(&self, name: String) -> ApiResult<i32>;

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

dyn_clone::clone_trait_object!(Db);

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
) -> ApiResult<Box<dyn DbPool>> {
    let url =
        Url::parse(&settings.database_url).map_err(|e| DbErrorKind::InvalidUrl(e.to_string()))?;
    Ok(match url.scheme() {
        "mysql" => Box::new(mysql::pool::MysqlDbPool::new(settings, metrics)?),
        "spanner" => Box::new(spanner::pool::SpannerDbPool::new(settings, metrics).await?),
        _ => return Err(DbErrorKind::InvalidUrl(settings.database_url.to_owned()).into()),
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

async fn blocking_thread<F, I>(f: F) -> ApiResult<I>
where
    F: FnOnce() -> QueryResult<I> + Send + 'static,
    I: Send + 'static,
{
    web::block(move || f().map_err(ApiError::from))
        .await
        .map_err(ApiError::from)
}
