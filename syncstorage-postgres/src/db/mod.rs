#![allow(dead_code)] // XXX:
use std::{collections::HashMap, fmt, sync::Arc};

use diesel::{sql_query, sql_types::Text, OptionalExtension};
use diesel_async::{AsyncConnection, RunQueryDsl, TransactionManager};

use syncserver_common::Metrics;
use syncstorage_db_common::{util::SyncTimestamp, UserIdentifier};
use syncstorage_settings::Quota;

use super::{
    pool::{CollectionCache, Conn},
    DbResult,
};

mod batch_impl;
mod db_impl;

#[derive(Debug, Eq, PartialEq)]
enum CollectionLock {
    Read,
    Write,
}
pub struct PgDb {
    // Reference to asynchronous database connection.
    pub(super) conn: Conn,
    /// Database session struct reference.
    session: PgDbSession,
    /// Pool level cache of collection_ids and their names.
    coll_cache: Arc<CollectionCache>,
    /// Configured quota, with defined size, enabled, and enforced attributes.
    metrics: Metrics,
    /// Configured quota, with defined size, enabled, and enforced attributes.
    quota: Quota,
}

impl fmt::Debug for PgDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgDb")
            .field("session", &self.session)
            .field("coll_cache", &self.coll_cache)
            .field("metrics", &self.metrics)
            .field("quota", &self.quota)
            .finish()
    }
}

/// Per-session Db metadata.
#[derive(Debug, Default)]
struct PgDbSession {
    /// The "current time" on the server used for this session's operations.
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (HawkIdentifier, collection_id).
    coll_modified_cache: HashMap<(UserIdentifier, i32), SyncTimestamp>,
    /// Currently locked collections.
    coll_locks: HashMap<(UserIdentifier, i32), CollectionLock>,
    /// Whether a transaction was started (begin() called)
    in_transaction: bool,
    /// Boolean to identify if query in active transaction.
    in_write_transaction: bool,
    /// Whether update_collection has already been called.
    updated_collection: bool,
}

impl PgDb {
    /// Create a new instance of PgDb
    /// Fresh metrics clone and default impl of session.
    pub(super) fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: &Quota,
    ) -> Self {
        PgDb {
            conn,
            session: Default::default(),
            coll_cache,
            metrics: metrics.clone(),
            quota: *quota,
        }
    }

    /// NOTE: Will be completed with other db method task.
    pub(super) async fn get_or_create_collection_id(&mut self, _name: &str) -> DbResult<i32> {
        todo!()
    }

    async fn commit(&mut self) -> DbResult<()> {
        if self.session.in_transaction {
            <Conn as AsyncConnection>::TransactionManager::commit_transaction(&mut self.conn)
                .await?;
        }
        Ok(())
    }

    /// Utility to rollback transaction if current Db session transaction in progress.
    async fn rollback(&mut self) -> DbResult<()> {
        if self.session.in_transaction {
            <Conn as AsyncConnection>::TransactionManager::rollback_transaction(&mut self.conn)
                .await?;
        }
        Ok(())
    }

    /// Utility method to begin transaction and set current session `in_transaction` to `true`
    /// If `for_write` truthy, `in_write_transaction` sets to true.
    pub(super) async fn begin(&mut self, for_write: bool) -> DbResult<()> {
        <Conn as AsyncConnection>::TransactionManager::begin_transaction(&mut self.conn).await?;
        self.session.in_transaction = true;
        if for_write {
            self.session.in_write_transaction = true;
        }
        Ok(())
    }

    /// Simple check function to ensure database liveliness.
    async fn check(&mut self) -> DbResult<results::Check> {
        diesel::sql_query("SELECT 1")
            .execute(&mut self.conn)
            .await?;
        Ok(true)
    }

    pub(super) fn timestamp(&self) -> SyncTimestamp {
        self.session.timestamp
    }

    pub(super) async fn get_collection_id(
        &mut self,
        name: &str,
    ) -> DbResult<results::GetCollectionId> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = sql_query(
            "SELECT id
               FROM collections
              WHERE name = $1",
        )
        .bind::<Text, _>(name)
        .get_result::<results::IdResult>(&mut self.conn)
        .await
        .optional()?
        .ok_or_else(DbError::collection_not_found)?
        .id;
        if !self.session.in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }
        Ok(id)
    }
}

#[async_trait(?Send)]
impl Db for PgDb {
    async fn commit(&mut self) -> Result<(), Self::Error> {
        PgDb::commit(self).await
    }

    async fn rollback(&mut self) -> Result<(), Self::Error> {
        PgDb::rollback(self).await
    }

    async fn begin(&mut self, for_write: bool) -> Result<(), Self::Error> {
        PgDb::begin(self, for_write).await
    }

    async fn check(&mut self) -> Result<results::Check, Self::Error> {
        PgDb::check(self).await
    }

    async fn lock_for_read(
        &mut self,
        params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        PgDb::lock_for_read(self, params).await
    }

    async fn lock_for_write(
        &mut self,
        params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        PgDb::lock_for_write(self, params).await
    }

    async fn get_collection_timestamps(
        &mut self,
        params: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps, Self::Error> {
        PgDb::get_collection_timestamps(self, params).await
    }

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> Result<results::GetCollectionTimestamp, Self::Error> {
        PgDb::get_collection_timestamp(self, params).await
    }

    async fn get_collection_counts(
        &mut self,
        params: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts, Self::Error> {
        PgDb::get_collection_counts(self, params).await
    }

    async fn get_collection_usage(
        &mut self,
        params: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage, Self::Error> {
        PgDb::get_collection_usage(self, params).await
    }

    async fn get_storage_timestamp(
        &mut self,
        params: params::GetStorageTimestamp,
    ) -> Result<results::GetStorageTimestamp, Self::Error> {
        PgDb::get_storage_timestamp(self, params).await
    }

    async fn get_storage_usage(
        &mut self,
        params: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage, Self::Error> {
        PgDb::get_storage_usage(self, params).await
    }

    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> Result<results::GetQuotaUsage, Self::Error> {
        PgDb::get_quota_usage(self, params).await
    }

    async fn delete_storage(
        &mut self,
        params: params::DeleteStorage,
    ) -> Result<results::DeleteStorage, Self::Error> {
        PgDb::delete_storage(self, params).await
    }

    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection, Self::Error> {
        PgDb::delete_collection(self, params).await
    }

    async fn delete_bsos(
        &mut self,
        params: params::DeleteBsos,
    ) -> Result<results::DeleteBsos, Self::Error> {
        PgDb::delete_bsos(self, params).await
    }

    async fn get_bsos(&mut self, params: params::GetBsos) -> Result<results::GetBsos, Self::Error> {
        PgDb::get_bsos(self, params).await
    }

    async fn get_bso_ids(
        &mut self,
        params: params::GetBsoIds,
    ) -> Result<results::GetBsoIds, Self::Error> {
        PgDb::get_bso_ids(self, params).await
    }

    async fn post_bsos(
        &mut self,
        params: params::PostBsos,
    ) -> Result<results::PostBsos, Self::Error> {
        PgDb::post_bsos(self, params).await
    }

    async fn delete_bso(
        &mut self,
        params: params::DeleteBso,
    ) -> Result<results::DeleteBso, Self::Error> {
        PgDb::delete_bso(self, params).await
    }

    async fn get_bso(
        &mut self,
        params: params::GetBso,
    ) -> Result<Option<results::GetBso>, Self::Error> {
        PgDb::get_bso(self, params).await
    }

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> Result<results::GetBsoTimestamp, Self::Error> {
        PgDb::get_bso_timestamp(self, params).await
    }

    async fn put_bso(&mut self, params: params::PutBso) -> Result<results::PutBso, Self::Error> {
        PgDb::put_bso(self, params).await
    }

    async fn get_collection_id(&mut self, name: &str) -> Result<i32, Self::Error> {
        PgDb::get_collection_id(self, name).await
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    async fn create_collection(&mut self, name: &str) -> DbResult<i32> {
        self.get_or_create_collection_id(name).await
    }

    async fn update_collection(
        &mut self,
        params: params::UpdateCollection,
    ) -> Result<SyncTimestamp, Self::Error> {
        PgDb::update_collection(self, params).await
    }

    fn timestamp(&self) -> SyncTimestamp {
        PgDb::timestamp(self)
    }

    fn set_timestamp(&mut self, timestamp: SyncTimestamp) {
        self.session.timestamp = timestamp;
    }

    async fn clear_coll_cache(&mut self) -> Result<(), Self::Error> {
        self.coll_cache.clear();
        Ok(())
    }

    fn set_quota(&mut self, enabled: bool, limit: usize, enforced: bool) {
        self.quota = Quota {
            size: limit,
            enabled,
            enforced,
        }
    }
}

#[async_trait(?Send)]
impl BatchDb for PgDb {
    type Error = DbError;

    async fn create_batch(
        &mut self,
        params: params::CreateBatch,
    ) -> Result<results::CreateBatch, Self::Error> {
        PgDb::create_batch(self, params).await
    }

    async fn validate_batch(
        &mut self,
        params: params::ValidateBatch,
    ) -> Result<results::ValidateBatch, Self::Error> {
        PgDb::validate_batch(self, params).await
    }

    async fn append_to_batch(
        &mut self,
        params: params::AppendToBatch,
    ) -> Result<results::AppendToBatch, Self::Error> {
        PgDb::append_to_batch(self, params).await
    }

    async fn get_batch(
        &mut self,
        params: params::GetBatch,
    ) -> Result<Option<results::GetBatch>, Self::Error> {
        PgDb::get_batch(self, params).await
    }

    async fn commit_batch(
        &mut self,
        params: params::CommitBatch,
    ) -> Result<results::CommitBatch, Self::Error> {
        PgDb::commit_batch(self, params).await
    }

    async fn delete_batch(
        &mut self,
        params: params::DeleteBatch,
    ) -> Result<results::DeleteBatch, Self::Error> {
        PgDb::delete_batch(self, params).await
    }
}
