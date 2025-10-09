use std::collections::HashMap;

use async_trait::async_trait;
use google_cloud_rust_raw::spanner::v1::{
    spanner::{BeginTransactionRequest, CommitRequest, RollbackRequest},
    transaction::{
        TransactionOptions, TransactionOptions_ReadOnly, TransactionOptions_ReadWrite,
        TransactionSelector,
    },
    type_pb::TypeCode,
};
#[allow(unused_imports)]
use protobuf::{
    well_known_types::{ListValue, Value},
    Message, RepeatedField,
};
use syncstorage_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, Db, FIRST_CUSTOM_COLLECTION_ID,
};
use syncstorage_settings::Quota;

use super::{
    support::{as_type, bso_from_row, IntoSpannerValue},
    CollectionLock, SpannerDb, TOMBSTONE,
};
use crate::{error::DbError, DbResult};

pub(super) const PRETOUCH_TS: &str = "0001-01-01T00:00:00.00Z";

#[async_trait(?Send)]
impl Db for SpannerDb {
    async fn get_collection_id(&mut self, name: &str) -> DbResult<i32> {
        if let Some(id) = self.coll_cache.get_id(name).await {
            return Ok(id);
        }
        let (sqlparams, sqlparam_types) = params! { "name" => name.to_string() };
        let result = self
            .sql(
                "SELECT collection_id
                   FROM collections
                  WHERE name = @name",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?
            .ok_or_else(DbError::collection_not_found)?;
        let id = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbError::integrity(e.to_string()))?;
        if !self.in_write_transaction() {
            self.coll_cache.put(id, name.to_owned()).await;
        }
        Ok(id)
    }

    async fn create_collection(&mut self, name: &str) -> DbResult<i32> {
        // This should always run within a r/w transaction, so that: "If a
        // transaction successfully commits, then no other writer modified the
        // data that was read in the transaction after it was read."
        if !cfg!(debug_assertions) && !self.in_write_transaction() {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_owned(),
            ));
        }
        let result = self
            .sql(
                "SELECT COALESCE(MAX(collection_id), 1)
                   FROM collections",
            )
            .await?
            .execute(&self.conn)?
            .one()
            .await?;
        let max = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbError::integrity(e.to_string()))?;
        let id = FIRST_CUSTOM_COLLECTION_ID.max(max + 1);
        let (sqlparams, sqlparam_types) = params! {
            "name" => name.to_string(),
            "collection_id" => id,
        };

        self.sql(
            "INSERT INTO collections (collection_id, name)
             VALUES (@collection_id, @name)",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        Ok(id)
    }

    async fn lock_for_read(&mut self, params: params::LockCollection) -> DbResult<()> {
        // Begin a transaction
        self.begin(false).await?;

        let collection_id = self
            .get_collection_id(&params.collection)
            .await
            .or_else(|e| {
                if e.is_collection_not_found() {
                    // If the collection doesn't exist, we still want to start a
                    // transaction so it will continue to not exist.
                    Ok(0)
                } else {
                    Err(e)
                }
            })?;
        // If we already have a read or write lock then it's safe to
        // use it as-is.
        if self
            .session
            .coll_locks
            .contains_key(&(params.user_id.clone(), collection_id))
        {
            return Ok(());
        }

        self.session
            .coll_locks
            .insert((params.user_id, collection_id), CollectionLock::Read);

        Ok(())
    }

    async fn lock_for_write(&mut self, params: params::LockCollection) -> DbResult<()> {
        // Begin a transaction
        self.begin(true).await?;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;
        if let Some(CollectionLock::Read) = self
            .session
            .coll_locks
            .get(&(params.user_id.clone(), collection_id))
        {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_owned(),
            ));
        }
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "pretouch_ts" => PRETOUCH_TS.to_owned(),
        };
        sqlparam_types.insert("pretouch_ts".to_owned(), as_type(TypeCode::TIMESTAMP));

        let result = self
            .sql(
                "SELECT CURRENT_TIMESTAMP(), modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?;

        let timestamp = if let Some(result) = result {
            let modified = sync_timestamp_from_rfc3339(result[1].get_string_value())?;
            let now = sync_timestamp_from_rfc3339(result[0].get_string_value())?;
            // Forbid the write if it would not properly incr the modified
            // timestamp
            if modified >= now {
                return Err(DbError::conflict());
            }
            self.session
                .coll_modified_cache
                .insert((params.user_id.clone(), collection_id), modified);
            now
        } else {
            let result = self
                .sql("SELECT CURRENT_TIMESTAMP()")
                .await?
                .execute(&self.conn)?
                .one()
                .await?;
            sync_timestamp_from_rfc3339(result[0].get_string_value())?
        };
        self.set_timestamp(timestamp);

        self.session
            .coll_locks
            .insert((params.user_id, collection_id), CollectionLock::Write);

        Ok(())
    }

    fn set_timestamp(&mut self, timestamp: SyncTimestamp) {
        self.session.timestamp = Some(timestamp);
    }

    async fn begin(&mut self, for_write: bool) -> DbResult<()> {
        let spanner = &self.conn;
        let mut options = TransactionOptions::new();
        if for_write {
            options.set_read_write(TransactionOptions_ReadWrite::new());
            self.session.in_write_transaction = true;
        } else {
            options.set_read_only(TransactionOptions_ReadOnly::new());
        }
        let mut req = BeginTransactionRequest::new();
        req.set_session(spanner.session.get_name().to_owned());
        req.set_options(options);
        let mut transaction = spanner
            .client
            .begin_transaction_async_opt(&req, spanner.session_opt()?)?
            .await?;

        let mut ts = TransactionSelector::new();
        ts.set_id(transaction.take_id());
        self.session.transaction = Some(ts);
        Ok(())
    }

    async fn commit(&mut self) -> DbResult<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if cfg!(debug_assertions) && self.conn.settings.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction().await? {
            let spanner = &self.conn;
            let mut req = CommitRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            if let Some(mutations) = self.session.mutations.take() {
                req.set_mutations(RepeatedField::from_vec(mutations));
            }
            spanner
                .client
                .commit_async_opt(&req, spanner.session_opt()?)?
                .await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit".to_owned()))
        }
    }

    async fn rollback(&mut self) -> DbResult<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction().await? {
            let spanner = &self.conn;
            let mut req = RollbackRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner
                .client
                .rollback_async_opt(&req, spanner.session_opt()?)?
                .await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback".to_owned()))
        }
    }

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> DbResult<SyncTimestamp> {
        let collection_id = self.get_collection_id(&params.collection).await?;
        if let Some(modified) = self
            .session
            .coll_modified_cache
            .get(&(params.user_id.clone(), collection_id))
        {
            return Ok(*modified);
        }
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id,
            "pretouch_ts" => PRETOUCH_TS.to_owned(),
        };
        sqlparam_types.insert("pretouch_ts".to_owned(), as_type(TypeCode::TIMESTAMP));

        let result = self
            .sql(
                "SELECT modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?
            .ok_or_else(DbError::collection_not_found)?;
        let modified = sync_timestamp_from_rfc3339(result[0].get_string_value())?;
        Ok(modified)
    }

    async fn get_collection_timestamps(
        &mut self,
        user_id: params::GetCollectionTimestamps,
    ) -> DbResult<results::GetCollectionTimestamps> {
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
            "collection_id" => TOMBSTONE,
            "pretouch_ts" => PRETOUCH_TS.to_owned(),
        };
        sqlparam_types.insert("pretouch_ts".to_owned(), as_type(TypeCode::TIMESTAMP));
        let mut streaming = self
            .sql(
                "SELECT collection_id, modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id != @collection_id
                    AND modified > @pretouch_ts",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?;
        let mut results = HashMap::new();
        while let Some(row) = streaming.try_next().await? {
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            let modified = sync_timestamp_from_rfc3339(row[1].get_string_value())?;
            results.insert(collection_id, modified);
        }
        self.map_collection_names(results).await
    }

    async fn get_collection_counts(
        &mut self,
        user_id: params::GetCollectionCounts,
    ) -> DbResult<results::GetCollectionCounts> {
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
        };
        let mut streaming = self
            .sql(
                "SELECT collection_id, COUNT(collection_id)
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY collection_id",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?;
        let mut counts = HashMap::new();
        while let Some(row) = streaming.try_next().await? {
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            let count = row[1]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            counts.insert(collection_id, count);
        }
        self.map_collection_names(counts).await
    }

    async fn get_collection_usage(
        &mut self,
        user_id: params::GetCollectionUsage,
    ) -> DbResult<results::GetCollectionUsage> {
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid
        };
        let mut streaming = self
            .sql(
                "SELECT collection_id, SUM(BYTE_LENGTH(payload))
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY collection_id",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?;
        let mut usages = HashMap::new();
        while let Some(row) = streaming.try_next().await? {
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            let usage = row[1]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            usages.insert(collection_id, usage);
        }
        self.map_collection_names(usages).await
    }

    async fn get_storage_timestamp(
        &mut self,
        user_id: params::GetStorageTimestamp,
    ) -> DbResult<SyncTimestamp> {
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
            "pretouch_ts" => PRETOUCH_TS.to_owned(),
        };
        sqlparam_types.insert("pretouch_ts".to_owned(), as_type(TypeCode::TIMESTAMP));
        let row = self
            .sql(
                "SELECT MAX(modified)
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND modified > @pretouch_ts",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one()
            .await?;
        if row[0].has_null_value() {
            SyncTimestamp::from_i64(0).map_err(|e| DbError::integrity(e.to_string()))
        } else {
            sync_timestamp_from_rfc3339(row[0].get_string_value())
        }
    }

    async fn get_storage_usage(
        &mut self,
        user_id: params::GetStorageUsage,
    ) -> DbResult<results::GetStorageUsage> {
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid
        };
        let result = self
            .sql(
                "SELECT SUM(BYTE_LENGTH(payload))
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY fxa_uid",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            let usage = result[0]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            Ok(usage as u64)
        } else {
            Ok(0)
        }
    }

    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> DbResult<results::GetQuotaUsage> {
        if !self.quota.enabled {
            return Ok(results::GetQuotaUsage::default());
        }
        let check_sql = "SELECT COALESCE(total_bytes,0), COALESCE(count,0)
            FROM user_collections
           WHERE fxa_uid = @fxa_uid
             AND fxa_kid = @fxa_kid
             AND collection_id = @collection_id";
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => params.collection_id,
        };
        let result = self
            .sql(check_sql)
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            let total_bytes = if self.quota.enabled {
                result[0]
                    .get_string_value()
                    .parse::<usize>()
                    .map_err(|e| DbError::integrity(e.to_string()))?
            } else {
                0
            };
            let count = result[1]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbError::integrity(e.to_string()))?;
            Ok(results::GetQuotaUsage { total_bytes, count })
        } else {
            Ok(results::GetQuotaUsage::default())
        }
    }

    async fn delete_storage(&mut self, user_id: params::DeleteStorage) -> DbResult<()> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid
        };
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        Ok(())
    }

    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> DbResult<results::DeleteCollection> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        let collection_id = self.get_collection_id(&params.collection).await?;
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "pretouch_ts" => PRETOUCH_TS.to_owned(),
        };
        sqlparam_types.insert("pretouch_ts".to_owned(), as_type(TypeCode::TIMESTAMP));
        let affected_rows = self
            .sql(
                "DELETE FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml(&self.conn)
            .await?;
        if affected_rows > 0 {
            let mut tags = HashMap::default();
            tags.insert("collection".to_string(), params.collection);
            self.metrics
                .incr_with_tags("storage.spanner.delete_collection", tags);
            self.erect_tombstone(&params.user_id).await
        } else {
            self.get_storage_timestamp(params.user_id).await
        }
    }

    async fn update_collection(
        &mut self,
        params: params::UpdateCollection,
    ) -> DbResult<SyncTimestamp> {
        // NOTE: Spanner supports upserts via its InsertOrUpdate mutation but
        // lacks a SQL equivalent. This call could be 1 InsertOrUpdate instead
        // of 2 queries but would require put/post_bsos to also use mutations.
        // Due to case of when no parent row exists (in user_collections)
        // before writing to bsos. Spanner requires a parent table row exist
        // before child table rows are written.
        // Mutations don't run in the same order as ExecuteSql calls, they are
        // buffered on the client side and only issued to Spanner in the final
        // transaction Commit.
        let timestamp = self.checked_timestamp()?;
        if !cfg!(debug_assertions) && self.session.updated_collection {
            // No need to touch it again (except during tests where we
            // currently reuse Dbs for multiple requests)
            return Ok(timestamp);
        }

        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => params.collection_id,
            "modified" => timestamp.as_rfc3339()?,
        };
        sqlparam_types.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));
        let result = self
            .sql(
                "SELECT 1
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id",
            )
            .await?
            .params(sqlparams.clone())
            .param_types(sqlparam_types.clone())
            .execute(&self.conn)?
            .one_or_none()
            .await?;
        if result.is_some() {
            let sql = "UPDATE user_collections
                    SET modified = @modified
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id";
            self.sql(sql)
                .await?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_dml(&self.conn)
                .await?;
        } else {
            let mut tags = HashMap::default();
            tags.insert("collection".to_owned(), params.collection);
            self.metrics
                .clone()
                .start_timer("storage.quota.init_totals", Some(tags));
            let update_sql = if self.quota.enabled {
                "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified, count, total_bytes)
                VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified, 0, 0)"
            } else {
                "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
                VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)"
            };
            self.sql(update_sql)
                .await?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_dml(&self.conn)
                .await?;
        }
        self.session.updated_collection = true;
        Ok(timestamp)
    }

    async fn delete_bso(&mut self, params: params::DeleteBso) -> DbResult<results::DeleteBso> {
        let collection_id = self.get_collection_id(&params.collection).await?;
        let user_id = params.user_id.clone();
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id,
            "bso_id" => params.id,
        };
        let affected_rows = self
            .sql(
                "DELETE FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml(&self.conn)
            .await?;
        if affected_rows == 0 {
            Err(DbError::bso_not_found())
        } else {
            self.metrics.incr("storage.spanner.delete_bso");
            Ok(self
                .update_user_collection_quotas(&user_id, collection_id)
                .await?)
        }
    }

    async fn delete_bsos(&mut self, params: params::DeleteBsos) -> DbResult<results::DeleteBsos> {
        let user_id = params.user_id.clone();
        let collection_id = self.get_collection_id(&params.collection).await?;

        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
            "collection_id" => collection_id,
            "ids" => params.ids,
        };
        self.sql(
            "DELETE FROM bsos
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND bso_id IN UNNEST(@ids)",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        let mut tags = HashMap::default();
        tags.insert("collection".to_string(), params.collection.clone());
        self.metrics
            .incr_with_tags("self.storage.delete_bsos", tags);
        self.update_user_collection_quotas(&params.user_id, collection_id)
            .await
    }

    async fn get_bsos(&mut self, params: params::GetBsos) -> DbResult<results::GetBsos> {
        let query = "\
            SELECT bso_id, sortindex, payload, modified, expiry
              FROM bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND expiry > CURRENT_TIMESTAMP()";
        let limit = params.limit.map(i64::from).unwrap_or(-1);
        let params::Offset { offset, timestamp } = params.offset.clone().unwrap_or_default();
        let sort = params.sort;

        let mut streaming = self.bsos_query(query, params).await?;
        let mut bsos = vec![];
        while let Some(row) = streaming.try_next().await? {
            bsos.push(bso_from_row(row)?);
        }

        // NOTE: when bsos.len() == 0, server-syncstorage (the Python impl)
        // makes an additional call to get_collection_timestamp to potentially
        // trigger CollectionNotFound errors.  However it ultimately eats the
        // CollectionNotFound and returns empty anyway, for the sake of
        // backwards compat.:
        // https://bugzilla.mozilla.org/show_bug.cgi?id=963332

        let next_offset = if limit >= 0 && bsos.len() > limit as usize {
            bsos.pop();
            let modifieds: Vec<i64> = bsos.iter().map(|r| r.modified.as_i64()).collect();
            self.encode_next_offset(sort, offset, timestamp.map(|t| t.as_i64()), modifieds)
        } else {
            None
        };

        Ok(results::GetBsos {
            items: bsos,
            offset: next_offset,
        })
    }

    async fn get_bso_ids(&mut self, params: params::GetBsos) -> DbResult<results::GetBsoIds> {
        let limit = params.limit.map(i64::from).unwrap_or(-1);
        let params::Offset { offset, timestamp } = params.offset.clone().unwrap_or_default();
        let sort = params.sort;

        let query = "\
            SELECT bso_id, modified
              FROM bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND expiry > CURRENT_TIMESTAMP()";
        let mut stream = self.bsos_query(query, params).await?;

        let mut ids = vec![];
        let mut modifieds = vec![];
        while let Some(mut row) = stream.try_next().await? {
            ids.push(row[0].take_string_value());
            modifieds.push(sync_timestamp_from_rfc3339(row[1].get_string_value())?.as_i64());
        }
        // NOTE: when bsos.len() == 0, server-syncstorage (the Python impl)
        // makes an additional call to get_collection_timestamp to potentially
        // trigger CollectionNotFound errors.  However it ultimately eats the
        // CollectionNotFound and returns empty anyway, for the sake of
        // backwards compat.:
        // https://bugzilla.mozilla.org/show_bug.cgi?id=963332

        let next_offset = if limit >= 0 && ids.len() > limit as usize {
            ids.pop();
            modifieds.pop();
            self.encode_next_offset(sort, offset, timestamp.map(|t| t.as_i64()), modifieds)
        } else {
            None
        };

        Ok(results::GetBsoIds {
            items: ids,
            offset: next_offset,
        })
    }

    async fn get_bso(&mut self, params: params::GetBso) -> DbResult<Option<results::GetBso>> {
        let collection_id = self.get_collection_id(&params.collection).await?;
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id,
            "bso_id" => params.id,
        };
        self.sql(
            "SELECT bso_id, sortindex, payload, modified, expiry
               FROM bsos
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND bso_id = @bso_id
                AND expiry > CURRENT_TIMESTAMP()",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute(&self.conn)?
        .one_or_none()
        .await?
        .map(bso_from_row)
        .transpose()
    }

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> DbResult<SyncTimestamp> {
        let collection_id = self.get_collection_id(&params.collection).await?;
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id,
            "bso_id" => params.id,
        };

        let result = self
            .sql(
                "SELECT modified
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id
                    AND expiry > CURRENT_TIMESTAMP()",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            sync_timestamp_from_rfc3339(result[0].get_string_value())
        } else {
            SyncTimestamp::from_i64(0).map_err(|e| DbError::integrity(e.to_string()))
        }
    }

    async fn put_bso(&mut self, params: params::PutBso) -> DbResult<results::PutBso> {
        if self.conn.settings.use_mutations {
            self.put_bso_with_mutations(params).await
        } else {
            self.put_bso_without_mutations(params).await
        }
    }

    async fn post_bsos(&mut self, params: params::PostBsos) -> DbResult<SyncTimestamp> {
        if self.conn.settings.use_mutations {
            self.post_bsos_with_mutations(params).await
        } else {
            self.post_bsos_without_mutations(params).await
        }
    }

    async fn check(&mut self) -> DbResult<results::Check> {
        // TODO: is there a better check than just fetching UTC?
        self.sql("SELECT CURRENT_TIMESTAMP()")
            .await?
            .execute(&self.conn)?
            .one()
            .await?;
        Ok(true)
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        let session = self.conn.session.clone();
        let now = crate::now();
        results::ConnectionInfo {
            spanner_age: session
                .create_time
                .into_option()
                .map(|time| now - time.seconds)
                .unwrap_or_default(),
            spanner_idle: session
                .approximate_last_use_time
                .into_option()
                .map(|time| now - time.seconds)
                .unwrap_or_default(),
            age: now - self.conn.create_time,
        }
    }

    fn timestamp(&self) -> SyncTimestamp {
        self.checked_timestamp()
            .expect("set_timestamp() not called yet for SpannerDb")
    }

    async fn clear_coll_cache(&mut self) -> Result<(), Self::Error> {
        self.coll_cache.clear().await;
        Ok(())
    }

    fn set_quota(&mut self, enabled: bool, limit: usize, enforced: bool) {
        self.quota = Quota {
            size: limit,
            enabled,
            enforced,
        };
    }
}

fn sync_timestamp_from_rfc3339(val: &str) -> Result<SyncTimestamp, DbError> {
    SyncTimestamp::from_rfc3339(val).map_err(|e| DbError::integrity(e.to_string()))
}
