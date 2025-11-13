#![allow(unused_variables)] // XXX:
use async_trait::async_trait;
use diesel::{
    sql_query,
    sql_types::{Integer, Text},
    OptionalExtension,
};
use diesel_async::{AsyncConnection, RunQueryDsl, TransactionManager};
use syncstorage_db_common::{params, results, util::SyncTimestamp, Db};
use syncstorage_settings::Quota;

use super::PgDb;
use crate::{pool::Conn, DbError, DbResult};

#[async_trait(?Send)]
impl Db for PgDb {
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
    async fn begin(&mut self, for_write: bool) -> DbResult<()> {
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

    async fn lock_for_read(
        &mut self,
        params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        todo!()
    }

    async fn lock_for_write(
        &mut self,
        params: params::LockCollection,
    ) -> Result<results::LockCollection, Self::Error> {
        todo!()
    }

    async fn get_collection_timestamps(
        &mut self,
        params: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps, Self::Error> {
        todo!()
    }

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> Result<results::GetCollectionTimestamp, Self::Error> {
        todo!()
    }

    async fn get_collection_counts(
        &mut self,
        params: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts, Self::Error> {
        todo!()
    }

    async fn get_collection_usage(
        &mut self,
        params: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage, Self::Error> {
        todo!()
    }

    async fn get_storage_timestamp(
        &mut self,
        params: params::GetStorageTimestamp,
    ) -> Result<results::GetStorageTimestamp, Self::Error> {
        todo!()
    }

    async fn get_storage_usage(
        &mut self,
        params: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage, Self::Error> {
        todo!()
    }

    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> Result<results::GetQuotaUsage, Self::Error> {
        todo!()
    }

    async fn delete_storage(
        &mut self,
        params: params::DeleteStorage,
    ) -> Result<results::DeleteStorage, Self::Error> {
        todo!()
    }

    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection, Self::Error> {
        todo!()
    }

    async fn delete_bsos(
        &mut self,
        params: params::DeleteBsos,
    ) -> Result<results::DeleteBsos, Self::Error> {
        todo!()
    }

    async fn get_bsos(&mut self, params: params::GetBsos) -> Result<results::GetBsos, Self::Error> {
        todo!()
    }

    async fn get_bso_ids(
        &mut self,
        params: params::GetBsoIds,
    ) -> Result<results::GetBsoIds, Self::Error> {
        todo!()
    }

    async fn post_bsos(
        &mut self,
        params: params::PostBsos,
    ) -> Result<results::PostBsos, Self::Error> {
        todo!()
    }

    async fn delete_bso(
        &mut self,
        params: params::DeleteBso,
    ) -> Result<results::DeleteBso, Self::Error> {
        todo!()
    }

    async fn get_bso(
        &mut self,
        params: params::GetBso,
    ) -> Result<Option<results::GetBso>, Self::Error> {
        todo!()
    }

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> Result<results::GetBsoTimestamp, Self::Error> {
        todo!()
    }

    async fn put_bso(&mut self, params: params::PutBso) -> Result<results::PutBso, Self::Error> {
        todo!()
    }

    async fn get_collection_id(&mut self, name: &str) -> DbResult<results::GetCollectionId> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = sql_query(
            "SELECT id
               FROM collections
              WHERE name = $1",
        )
        .bind::<Text, _>(name)
        .get_result::<IdResult>(&mut self.conn)
        .await
        .optional()?
        .ok_or_else(DbError::collection_not_found)?
        .id;
        if !self.session.in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }
        Ok(id)
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
        todo!()
    }

    fn timestamp(&self) -> SyncTimestamp {
        self.session.timestamp
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

#[derive(Debug, QueryableByName)]
struct IdResult {
    #[diesel(sql_type = Integer)]
    id: i32,
}
