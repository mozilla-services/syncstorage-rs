use async_trait::async_trait;
use chrono::{offset::Utc, DateTime, TimeDelta};
use diesel::{
    delete,
    dsl::{count, max, now, sql},
    sql_types::{Array, BigInt, Integer, Nullable, Timestamptz},
    upsert::excluded,
    ExpressionMethods, IntoSql, OptionalExtension, QueryDsl, SelectableHelper,
};
use diesel_async::{AsyncConnection, RunQueryDsl, TransactionManager};
use futures::TryStreamExt;
use syncstorage_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, Db, Sorting, DEFAULT_BSO_TTL,
};

use super::{PgDb, TOMBSTONE};
use crate::{
    bsos_query,
    db::{CollectionLock, PRETOUCH_DT},
    orm_models::{sql_types::PostBso, BsoChangeset},
    pool::Conn,
    schema::{bsos, collections, user_collections},
    DbError, DbResult,
};

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

    /// Explicitly lock the matching row in the user_collections table. Read
    /// locks do `SELECT ... LOCK IN SHARE MODE` and write locks do `SELECT
    /// ... FOR UPDATE`.
    ///
    /// In theory it would be possible to use serializable transactions rather
    /// than explicit locking, but our ops team have expressed concerns about
    /// the efficiency of that approach at scale.
    async fn lock_for_read(
        &mut self,
        params: params::LockCollection,
    ) -> DbResult<results::LockCollection> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self
            .get_collection_id(&params.collection)
            .await
            .or_else(|e| {
                if e.is_collection_not_found() {
                    // If the collection doesn't exist, we still want to start a
                    // transaction, so it will continue to not exist.
                    Ok(0)
                } else {
                    Err(e)
                }
            })?;

        let key = (params.user_id, collection_id);
        // If we already have a read or write lock then it's safe to
        // use it as-is.
        if self.session.coll_locks.contains_key(&key) {
            return Ok(());
        }

        // `FOR SHARE`
        // Obtains shared lock, allowing multiple transactions to read rows simultaneously.
        self.begin(false).await?;

        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .for_share()
            .first(&mut self.conn)
            .await
            .optional()?;

        if let Some(modified) = modified {
            self.session
                .coll_modified_cache
                .insert(key.clone(), modified);
        }
        self.session.coll_locks.insert(key, CollectionLock::Read);
        Ok(())
    }

    async fn lock_for_write(&mut self, params: params::LockCollection) -> DbResult<()> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;
        let key = (params.user_id, collection_id);

        if let Some(CollectionLock::Read) = self.session.coll_locks.get(&key) {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_string(),
            ));
        }

        // `FOR UPDATE`
        // Acquires exclusive lock on select rows, prohibits other transactions from modifying
        // until complete.
        let nowtz = now.into_sql::<Timestamptz>();
        self.begin(true).await?;
        let row = user_collections::table
            .select((user_collections::modified, nowtz))
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .filter(user_collections::modified.gt(PRETOUCH_DT))
            .for_update()
            .first(&mut self.conn)
            .await
            .optional()?;

        let timestamp = if let Some((modified, timestamp)) = row {
            // Do not allow write if it would incorrectly increment timestamp.
            if modified >= timestamp {
                return Err(DbError::conflict());
            }
            self.session
                .coll_modified_cache
                .insert(key.clone(), modified);
            timestamp
        } else {
            diesel::select(nowtz).get_result(&mut self.conn).await?
        };
        self.session.timestamp = Some(timestamp);

        self.session.coll_locks.insert(key, CollectionLock::Write);
        Ok(())
    }

    async fn get_collection_timestamps(
        &mut self,
        params: params::GetCollectionTimestamps,
    ) -> DbResult<results::GetCollectionTimestamps> {
        let modifieds = user_collections::table
            .select((user_collections::collection_id, user_collections::modified))
            .filter(user_collections::user_id.eq(params.legacy_id as i64))
            .filter(user_collections::collection_id.ne(TOMBSTONE))
            .filter(user_collections::modified.gt(PRETOUCH_DT))
            .load_stream::<(_, SyncTimestamp)>(&mut self.conn)
            .await?
            .try_collect()
            .await?;
        self.map_collection_names(modifieds).await
    }

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> DbResult<results::GetCollectionTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        if let Some(modified) = self
            .session
            .coll_modified_cache
            .get(&(params.user_id, collection_id))
        {
            return Ok(*modified);
        }
        user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .filter(user_collections::modified.gt(PRETOUCH_DT))
            .first(&mut self.conn)
            .await
            .optional()?
            .ok_or_else(DbError::collection_not_found)
    }

    async fn get_collection_counts(
        &mut self,
        params: params::GetCollectionCounts,
    ) -> DbResult<results::GetCollectionCounts> {
        let counts = bsos::table
            .group_by(bsos::collection_id)
            .select((bsos::collection_id, count(bsos::collection_id)))
            .filter(bsos::user_id.eq(params.legacy_id as i64))
            .filter(bsos::expiry.gt(now))
            .load_stream::<(_, i64)>(&mut self.conn)
            .await?
            .try_collect()
            .await?;
        self.map_collection_names(counts).await
    }

    async fn get_collection_usage(
        &mut self,
        params: params::GetCollectionUsage,
    ) -> DbResult<results::GetCollectionUsage> {
        let counts = bsos::table
            .group_by(bsos::collection_id)
            .select((bsos::collection_id, sql::<BigInt>("SUM(LENGTH(payload))")))
            .filter(bsos::user_id.eq(params.legacy_id as i64))
            .filter(bsos::expiry.gt(now))
            .load_stream::<(_, i64)>(&mut self.conn)
            .await?
            .try_collect()
            .await?;
        self.map_collection_names(counts).await
    }

    async fn get_storage_timestamp(
        &mut self,
        params: params::GetStorageTimestamp,
    ) -> DbResult<results::GetStorageTimestamp> {
        let modified = user_collections::table
            .select(max(user_collections::modified))
            .filter(user_collections::user_id.eq(params.legacy_id as i64))
            .first::<Option<_>>(&mut self.conn)
            .await?
            .unwrap_or_else(SyncTimestamp::zero);
        Ok(modified)
    }

    async fn get_storage_usage(
        &mut self,
        params: params::GetStorageUsage,
    ) -> DbResult<results::GetStorageUsage> {
        let total_bytes = bsos::table
            .select(sql::<Nullable<BigInt>>("SUM(LENGTH(payload))"))
            .filter(bsos::user_id.eq(params.legacy_id as i64))
            .filter(bsos::expiry.gt(now))
            .get_result::<Option<i64>>(&mut self.conn)
            .await?;
        Ok(total_bytes.unwrap_or_default() as u64)
    }

    /// Performs a light-weight "read only" quota storage check.
    /// Currently used by `put_bso`
    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> DbResult<results::GetQuotaUsage> {
        let (total_bytes, count): (i64, i64) = user_collections::table
            .select((
                sql::<BigInt>("COALESCE(SUM(COALESCE(total_bytes, 0)), 0)::BIGINT"),
                sql::<BigInt>("COALESCE(SUM(COALESCE(count, 0)), 0)::BIGINT"),
            ))
            .filter(user_collections::user_id.eq(params.user_id.legacy_id as i64))
            .filter(user_collections::collection_id.eq(params.collection_id))
            .get_result(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count: count as i32,
        })
    }

    async fn delete_storage(
        &mut self,
        params: params::DeleteStorage,
    ) -> DbResult<results::DeleteStorage> {
        let user_id = params.legacy_id as i64;
        delete(bsos::table)
            .filter(bsos::user_id.eq(user_id))
            .execute(&mut self.conn)
            .await?;
        delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> DbResult<results::DeleteCollection> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let mut count = delete(bsos::table)
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::collection_id.eq(&collection_id))
            .execute(&mut self.conn)
            .await?;
        count += delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(&collection_id))
            .filter(user_collections::modified.gt(PRETOUCH_DT))
            .execute(&mut self.conn)
            .await?;
        if count == 0 {
            return Err(DbError::collection_not_found());
        } else {
            self.erect_tombstone(user_id).await?;
        }
        self.get_storage_timestamp(params.user_id).await
    }

    async fn delete_bsos(&mut self, params: params::DeleteBsos) -> DbResult<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        delete(bsos::table)
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::collection_id.eq(&collection_id))
            .filter(bsos::bso_id.eq_any(params.ids))
            .execute(&mut self.conn)
            .await?;
        self.update_collection(params::UpdateCollection {
            user_id: params.user_id,
            collection_id,
            collection: params.collection,
        })
        .await
    }

    async fn get_bsos(&mut self, params: params::GetBsos) -> DbResult<results::GetBsos> {
        let (bsos, offset) = bsos_query!(self, params, GetBso::as_select());
        let items = bsos
            .into_iter()
            .map(TryInto::try_into)
            .collect::<DbResult<_>>()?;
        Ok(results::GetBsos { items, offset })
    }

    async fn get_bso_ids(&mut self, params: params::GetBsoIds) -> DbResult<results::GetBsoIds> {
        let (items, offset) = bsos_query!(self, params, bsos::bso_id);
        Ok(results::GetBsoIds { items, offset })
    }

    async fn delete_bso(&mut self, params: params::DeleteBso) -> DbResult<results::DeleteBso> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let affected_rows = delete(bsos::table)
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::collection_id.eq(&collection_id))
            .filter(bsos::bso_id.eq(params.id))
            .filter(bsos::expiry.gt(now))
            .execute(&mut self.conn)
            .await?;
        if affected_rows == 0 {
            return Err(DbError::bso_not_found());
        }
        self.update_collection(params::UpdateCollection {
            user_id: params.user_id,
            collection_id,
            collection: params.collection,
        })
        .await
    }

    async fn get_bso(&mut self, params: params::GetBso) -> DbResult<Option<results::GetBso>> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let bso = bsos::table
            .select(GetBso::as_select())
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::collection_id.eq(collection_id))
            .filter(bsos::bso_id.eq(&params.id))
            .filter(bsos::expiry.gt(now))
            .get_result(&mut self.conn)
            .await
            .optional()?
            .map(TryInto::try_into)
            .transpose()?;
        Ok(bso)
    }

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> DbResult<results::GetBsoTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let modified = bsos::table
            .select(bsos::modified)
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::collection_id.eq(collection_id))
            .filter(bsos::bso_id.eq(&params.id))
            .first(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_else(SyncTimestamp::zero);
        Ok(modified)
    }

    async fn put_bso(&mut self, bso: params::PutBso) -> DbResult<results::PutBso> {
        let user_id = bso.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&bso.collection).await?;

        self.check_quota(&bso.user_id, &bso.collection, collection_id)
            .await?;

        let payload = bso.payload.as_deref().unwrap_or_default();
        let sortindex = bso.sortindex;
        let ttl = bso.ttl.unwrap_or(DEFAULT_BSO_TTL);

        let modified = self.checked_timestamp()?.as_datetime()?;
        // Expiry originally required millisecond conversion
        let expiry = modified + TimeDelta::seconds(ttl as i64);
        // The changeset utilizes Diesel's `AsChangeset` trait.
        // This allows selective updates of fields if and only if they are `Some(<T>)`
        let changeset = BsoChangeset {
            sortindex: bso.sortindex,
            payload: bso.payload.as_deref(),
            modified: (bso.payload.is_some() || bso.sortindex.is_some()).then_some(modified),
            expiry: bso.ttl.map(|_| expiry),
        };
        self.ensure_user_collection(user_id, collection_id).await?;
        diesel::insert_into(bsos::table)
            .values((
                bsos::user_id.eq(user_id),
                bsos::collection_id.eq(&collection_id),
                bsos::bso_id.eq(&bso.id),
                bsos::sortindex.eq(sortindex),
                bsos::payload.eq(payload),
                bsos::modified.eq(modified),
                bsos::expiry.eq(expiry),
            ))
            .on_conflict((bsos::user_id, bsos::collection_id, bsos::bso_id))
            .do_update()
            .set(changeset)
            .execute(&mut self.conn)
            .await?;

        self.update_collection(params::UpdateCollection {
            user_id: bso.user_id,
            collection_id,
            collection: bso.collection,
        })
        .await
    }

    async fn post_bsos(&mut self, params: params::PostBsos) -> DbResult<results::PostBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;
        self.check_quota(&params.user_id, &params.collection, collection_id)
            .await?;
        self.ensure_user_collection(user_id, collection_id).await?;

        // Rendering a VALUES statement for MERGE INTO here is painful so we
        // pass the bsos in a single bind param as an Array of a named
        // composite type (post_bso[]). The composite type must be explicitly
        // named/declared as postgres disallows unnamed/anonymous composite
        // types to be used as bind parameters
        diesel::sql_query(include_str!("post_bsos.sql"))
            .bind::<BigInt, _>(user_id)
            .bind::<Integer, _>(collection_id)
            .bind::<Array<PostBso>, _>(params.bsos)
            .bind::<Timestamptz, _>(self.checked_timestamp()?.as_datetime()?)
            .bind::<BigInt, _>(DEFAULT_BSO_TTL as i64)
            .execute(&mut self.conn)
            .await?;

        self.update_collection(params::UpdateCollection {
            user_id: params.user_id,
            collection_id,
            collection: params.collection,
        })
        .await
    }

    async fn get_collection_id(&mut self, name: &str) -> DbResult<results::GetCollectionId> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = collections::table
            .select(collections::collection_id)
            .filter(collections::name.eq(name))
            .first::<i32>(&mut self.conn)
            .await
            .optional()?
            .ok_or_else(DbError::collection_not_found)?;

        if !self.session.in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }
        Ok(id)
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    /// Updates a given collection entry, when provided the `user_id`, `collection_id`,
    /// and `collection` string. This is an insertion operation should the
    /// `user_id` and `collection_id` keys not exist, but will update with the Postgres
    /// `INSERT...ON CONFLICT` statement.
    async fn update_collection(
        &mut self,
        params: params::UpdateCollection,
    ) -> DbResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let quota = if self.quota.enabled {
            self.calc_quota_usage(user_id, params.collection_id).await?
        } else {
            results::GetQuotaUsage {
                count: 0,
                total_bytes: 0,
            }
        };
        let total_bytes = quota.total_bytes as i64;
        let modified = self.checked_timestamp()?;

        diesel::insert_into(user_collections::table)
            .values((
                user_collections::user_id.eq(user_id),
                user_collections::collection_id.eq(params.collection_id),
                user_collections::modified.eq(modified.as_datetime()?),
                user_collections::count.eq(quota.count as i64),
                user_collections::total_bytes.eq(total_bytes),
            ))
            .on_conflict((user_collections::user_id, user_collections::collection_id))
            .do_update()
            .set((
                user_collections::modified.eq(excluded(user_collections::modified)),
                user_collections::count.eq(excluded(user_collections::count)),
                user_collections::total_bytes.eq(excluded(user_collections::total_bytes)),
            ))
            .execute(&mut self.conn)
            .await?;
        Ok(modified)
    }

    #[cfg(debug_assertions)]
    async fn create_collection(&mut self, name: &str) -> DbResult<i32> {
        self.get_or_create_collection_id(name).await
    }

    #[cfg(debug_assertions)]
    fn timestamp(&self) -> SyncTimestamp {
        self.session
            .timestamp
            .expect("set_timestamp() not called yet for PgDb")
    }

    #[cfg(debug_assertions)]
    fn set_timestamp(&mut self, timestamp: SyncTimestamp) {
        self.session.timestamp = Some(timestamp);
    }

    #[cfg(debug_assertions)]
    async fn clear_coll_cache(&mut self) -> DbResult<()> {
        self.coll_cache.clear();
        Ok(())
    }

    #[cfg(debug_assertions)]
    fn set_quota(&mut self, enabled: bool, limit: usize, enforced: bool) {
        self.quota = syncstorage_settings::Quota {
            size: limit,
            enabled,
            enforced,
        }
    }
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = bsos)]
pub struct GetBso {
    #[diesel(sql_type = Text)]
    pub bso_id: String,
    #[diesel(sql_type = Nullable<Integer>)]
    pub sortindex: Option<i32>,
    #[diesel(sql_type = Text)]
    pub payload: String,
    #[diesel(sql_type = Timestamptz)]
    pub modified: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    pub expiry: DateTime<Utc>,
}

impl TryFrom<GetBso> for results::GetBso {
    type Error = DbError;

    fn try_from(pg: GetBso) -> DbResult<Self> {
        Ok(Self {
            id: pg.bso_id,
            sortindex: pg.sortindex,
            payload: pg.payload,
            modified: SyncTimestamp::from_datetime(pg.modified)?,
            expiry: pg.expiry.timestamp_millis(),
        })
    }
}
