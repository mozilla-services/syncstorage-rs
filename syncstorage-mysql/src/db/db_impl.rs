use std::collections::HashMap;

use async_trait::async_trait;
use diesel::{
    delete,
    dsl::max,
    dsl::sql,
    sql_query,
    sql_types::{BigInt, Integer, Nullable, Text},
    ExpressionMethods, OptionalExtension, QueryDsl,
};
use diesel_async::{AsyncConnection, RunQueryDsl, TransactionManager};
use syncstorage_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, Db, Sorting, UserIdentifier,
    DEFAULT_BSO_TTL,
};
use syncstorage_settings::{Quota, DEFAULT_MAX_TOTAL_RECORDS};

use super::{
    diesel_ext::LockInShareModeDsl,
    schema::{bso, user_collections},
    CollectionLock, MysqlDb, COLLECTION_ID, COUNT, EXPIRY, LAST_MODIFIED, MODIFIED, TOMBSTONE,
    TOTAL_BYTES, USER_ID,
};
use crate::{pool::Conn, DbError, DbResult};

// this is the max number of records we will return.
static DEFAULT_LIMIT: u32 = DEFAULT_MAX_TOTAL_RECORDS;

#[async_trait(?Send)]
impl Db for MysqlDb {
    /// APIs for collection-level locking
    ///
    /// Explicitly lock the matching row in the user_collections table. Read
    /// locks do SELECT ... LOCK IN SHARE MODE and write locks do SELECT
    /// ... FOR UPDATE.
    ///
    /// In theory it would be possible to use serializable transactions rather
    /// than explicit locking, but our ops team have expressed concerns about
    /// the efficiency of that approach at scale.
    async fn lock_for_read(&mut self, params: params::LockCollection) -> DbResult<()> {
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

        let user_id = params.user_id.legacy_id as i64;
        let key = (params.user_id, collection_id);
        // If we already have a read or write lock then it's safe to
        // use it as-is.
        if self.session.coll_locks.contains_key(&key) {
            return Ok(());
        }

        // Lock the db
        self.begin(false).await?;
        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .lock_in_share_mode()
            .first(&mut self.conn)
            .await
            .optional()?;
        if let Some(modified) = modified {
            let modified = SyncTimestamp::from_i64(modified)?;
            self.session
                .coll_modified_cache
                .insert(key.clone(), modified);
        }
        // XXX: who's responsible for unlocking (removing the entry)
        self.session.coll_locks.insert(key, CollectionLock::Read);
        Ok(())
    }

    async fn lock_for_write(&mut self, params: params::LockCollection) -> DbResult<()> {
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;
        let user_id = params.user_id.legacy_id as i64;
        let key = (params.user_id, collection_id);

        if let Some(CollectionLock::Read) = self.session.coll_locks.get(&key) {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_owned(),
            ));
        }

        // Lock the db
        self.begin(true).await?;
        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .for_update()
            .first(&mut self.conn)
            .await
            .optional()?;
        if let Some(modified) = modified {
            let modified = SyncTimestamp::from_i64(modified)?;
            // Forbid the write if it would not properly incr the timestamp
            if modified >= self.timestamp() {
                return Err(DbError::conflict());
            }
            self.session
                .coll_modified_cache
                .insert(key.clone(), modified);
        }
        self.session.coll_locks.insert(key, CollectionLock::Write);
        Ok(())
    }

    async fn begin(&mut self, for_write: bool) -> DbResult<()> {
        <Conn as AsyncConnection>::TransactionManager::begin_transaction(&mut self.conn).await?;
        self.session.in_transaction = true;
        if for_write {
            self.session.in_write_transaction = true;
        }
        Ok(())
    }

    async fn commit(&mut self) -> DbResult<()> {
        if self.session.in_transaction {
            <Conn as AsyncConnection>::TransactionManager::commit_transaction(&mut self.conn)
                .await?;
        }
        Ok(())
    }

    async fn rollback(&mut self) -> DbResult<()> {
        if self.session.in_transaction {
            <Conn as AsyncConnection>::TransactionManager::rollback_transaction(&mut self.conn)
                .await?;
        }
        Ok(())
    }

    async fn delete_storage(&mut self, user_id: UserIdentifier) -> DbResult<()> {
        let user_id = user_id.legacy_id as i64;
        // Delete user data.
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .execute(&mut self.conn)
            .await?;
        // Delete user collections.
        delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    // Deleting the collection should result in:
    //  - collection does not appear in /info/collections
    //  - X-Last-Modified timestamp at the storage level changing
    async fn delete_collection(
        &mut self,
        params: params::DeleteCollection,
    ) -> DbResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let mut count = delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .execute(&mut self.conn)
            .await?;
        count += delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(&collection_id))
            .execute(&mut self.conn)
            .await?;
        if count == 0 {
            return Err(DbError::collection_not_found());
        } else {
            self.erect_tombstone(user_id as i32).await?;
        }
        self.get_storage_timestamp(params.user_id).await
    }

    async fn get_collection_id(&mut self, name: &str) -> DbResult<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = sql_query(
            "SELECT id
               FROM collections
              WHERE name = ?",
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

    async fn put_bso(&mut self, bso: params::PutBso) -> DbResult<results::PutBso> {
        /*
        if bso.payload.is_none() && bso.sortindex.is_none() && bso.ttl.is_none() {
            // XXX: go returns an error here (ErrNothingToDo), and is treated
            // as other errors
            return Ok(());
        }
        */

        let collection_id = self.get_or_create_collection_id(&bso.collection).await?;
        let user_id: u64 = bso.user_id.legacy_id;
        let timestamp = self.timestamp().as_i64();
        if self.quota.enabled {
            let usage = self
                .get_quota_usage(params::GetQuotaUsage {
                    user_id: bso.user_id.clone(),
                    collection: bso.collection.clone(),
                    collection_id,
                })
                .await?;
            if usage.total_bytes >= self.quota.size {
                let mut tags = HashMap::default();
                tags.insert("collection".to_owned(), bso.collection.clone());
                self.metrics.incr_with_tags("storage.quota.at_limit", tags);
                if self.quota.enforced {
                    return Err(DbError::quota());
                } else {
                    warn!("Quota at limit for user's collection ({} bytes)", usage.total_bytes; "collection"=>bso.collection.clone());
                }
            }
        }

        let payload = bso.payload.as_deref().unwrap_or_default();
        let sortindex = bso.sortindex;
        let ttl = bso.ttl.map_or(DEFAULT_BSO_TTL, |ttl| ttl);
        let q = format!(
            r#"
            INSERT INTO bso ({user_id}, {collection_id}, id, sortindex, payload, {modified}, {expiry})
            VALUES (?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                   {user_id} = VALUES({user_id}),
                   {collection_id} = VALUES({collection_id}),
                   id = VALUES(id)
            "#,
            user_id = USER_ID,
            modified = MODIFIED,
            collection_id = COLLECTION_ID,
            expiry = EXPIRY
        );
        let q = format!(
            "{}{}",
            q,
            if bso.sortindex.is_some() {
                ", sortindex = VALUES(sortindex)"
            } else {
                ""
            },
        );
        let q = format!(
            "{}{}",
            q,
            if bso.payload.is_some() {
                ", payload = VALUES(payload)"
            } else {
                ""
            },
        );
        let q = format!(
            "{}{}",
            q,
            if bso.ttl.is_some() {
                format!(", {expiry} = VALUES({expiry})", expiry = EXPIRY)
            } else {
                "".to_owned()
            },
        );
        let q = format!(
            "{}{}",
            q,
            if bso.payload.is_some() || bso.sortindex.is_some() {
                format!(", {modified} = VALUES({modified})", modified = MODIFIED)
            } else {
                "".to_owned()
            },
        );
        sql_query(q)
            .bind::<BigInt, _>(user_id as i64) // XXX:
            .bind::<Integer, _>(&collection_id)
            .bind::<Text, _>(&bso.id)
            .bind::<Nullable<Integer>, _>(sortindex)
            .bind::<Text, _>(payload)
            .bind::<BigInt, _>(timestamp)
            .bind::<BigInt, _>(timestamp + (i64::from(ttl) * 1000)) // remember: this is in millis
            .execute(&mut self.conn)
            .await?;
        self.update_collection(params::UpdateCollection {
            user_id: bso.user_id,
            collection_id,
            collection: bso.collection,
        })
        .await
    }

    async fn get_bsos(&mut self, params: params::GetBsos) -> DbResult<results::GetBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let now = self.timestamp().as_i64();
        let mut query = bso::table
            .select((
                bso::id,
                bso::modified,
                bso::payload,
                bso::sortindex,
                bso::expiry,
            ))
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::expiry.gt(now))
            .into_boxed();

        if let Some(older) = params.older {
            query = query.filter(bso::modified.lt(older.as_i64()));
        }
        if let Some(newer) = params.newer {
            query = query.filter(bso::modified.gt(newer.as_i64()));
        }

        if !params.ids.is_empty() {
            query = query.filter(bso::id.eq_any(params.ids));
        }

        // it's possible for two BSOs to be inserted with the same `modified` date,
        // since there's no guarantee of order when doing a get, pagination can return
        // an error. We "fudge" a bit here by taking the id order as a secondary, since
        // that is guaranteed to be unique by the client.
        query = match params.sort {
            // issue559: Revert to previous sorting
            /*
            Sorting::Index => query.order(bso::id.desc()).order(bso::sortindex.desc()),
            Sorting::Newest | Sorting::None => {
                query.order(bso::id.desc()).order(bso::modified.desc())
            }
            Sorting::Oldest => query.order(bso::id.asc()).order(bso::modified.asc()),
            */
            Sorting::Index => query.order(bso::sortindex.desc()),
            Sorting::Newest => query.order((bso::modified.desc(), bso::id.desc())),
            Sorting::Oldest => query.order((bso::modified.asc(), bso::id.asc())),
            _ => query,
        };

        let limit = params
            .limit
            .map(i64::from)
            .unwrap_or(DEFAULT_LIMIT as i64)
            .max(0);
        // fetch an extra row to detect if there are more rows that
        // match the query conditions
        query = query.limit(if limit > 0 { limit + 1 } else { limit });

        let numeric_offset = params.offset.map_or(0, |offset| offset.offset as i64);

        if numeric_offset > 0 {
            // XXX: copy over this optimization:
            // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
            query = query.offset(numeric_offset);
        }
        let mut bsos = query.load::<results::GetBso>(&mut self.conn).await?;

        // XXX: an additional get_collection_timestamp is done here in
        // python to trigger potential CollectionNotFoundErrors
        //if bsos.len() == 0 {
        //}

        let next_offset = if limit >= 0 && bsos.len() > limit as usize {
            bsos.pop();
            Some((limit + numeric_offset).to_string())
        } else {
            // if an explicit "limit=0" is sent, return the offset of "0"
            // Otherwise, this would break at least the db::tests::db::get_bsos_limit_offset
            // unit test.
            if limit == 0 {
                Some(0.to_string())
            } else {
                None
            }
        };

        Ok(results::GetBsos {
            items: bsos,
            offset: next_offset,
        })
    }

    async fn get_bso_ids(&mut self, params: params::GetBsos) -> DbResult<results::GetBsoIds> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let mut query = bso::table
            .select(bso::id)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .into_boxed();

        if let Some(older) = params.older {
            query = query.filter(bso::modified.lt(older.as_i64()));
        }
        if let Some(newer) = params.newer {
            query = query.filter(bso::modified.gt(newer.as_i64()));
        }

        if !params.ids.is_empty() {
            query = query.filter(bso::id.eq_any(params.ids));
        }

        query = match params.sort {
            Sorting::Index => query.order(bso::sortindex.desc()),
            Sorting::Newest => query.order(bso::modified.desc()),
            Sorting::Oldest => query.order(bso::modified.asc()),
            _ => query,
        };

        // negative limits are no longer allowed by mysql.
        let limit = params
            .limit
            .map(i64::from)
            .unwrap_or(DEFAULT_LIMIT as i64)
            .max(0);
        // fetch an extra row to detect if there are more rows that
        // match the query conditions. Negative limits will cause an error.
        query = query.limit(if limit == 0 { limit } else { limit + 1 });
        let numeric_offset = params.offset.map_or(0, |offset| offset.offset as i64);
        if numeric_offset != 0 {
            // XXX: copy over this optimization:
            // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
            query = query.offset(numeric_offset);
        }
        let mut ids = query.load::<String>(&mut self.conn).await?;

        // XXX: an additional get_collection_timestamp is done here in
        // python to trigger potential CollectionNotFoundErrors
        //if bsos.len() == 0 {
        //}

        let next_offset = if limit >= 0 && ids.len() > limit as usize {
            ids.pop();
            Some((limit + numeric_offset).to_string())
        } else {
            None
        };

        Ok(results::GetBsoIds {
            items: ids,
            offset: next_offset,
        })
    }

    async fn get_bso(&mut self, params: params::GetBso) -> DbResult<Option<results::GetBso>> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        Ok(bso::table
            .select((
                bso::id,
                bso::modified,
                bso::payload,
                bso::sortindex,
                bso::expiry,
            ))
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(&params.id))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .get_result::<results::GetBso>(&mut self.conn)
            .await
            .optional()?)
    }

    async fn delete_bso(&mut self, params: params::DeleteBso) -> DbResult<results::DeleteBso> {
        let user_id = params.user_id.legacy_id;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let affected_rows = delete(bso::table)
            .filter(bso::user_id.eq(user_id as i64))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(params.id))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
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

    async fn delete_bsos(&mut self, params: params::DeleteBsos) -> DbResult<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq_any(params.ids))
            .execute(&mut self.conn)
            .await?;
        self.update_collection(params::UpdateCollection {
            user_id: params.user_id,
            collection_id,
            collection: params.collection,
        })
        .await
    }

    async fn post_bsos(&mut self, input: params::PostBsos) -> DbResult<SyncTimestamp> {
        let collection_id = self.get_or_create_collection_id(&input.collection).await?;
        let modified = self.timestamp();

        for pbso in input.bsos {
            self.put_bso(params::PutBso {
                user_id: input.user_id.clone(),
                collection: input.collection.clone(),
                id: pbso.id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            })
            .await?;
        }
        self.update_collection(params::UpdateCollection {
            user_id: input.user_id,
            collection_id,
            collection: input.collection,
        })
        .await?;

        Ok(modified)
    }

    async fn get_storage_timestamp(&mut self, user_id: UserIdentifier) -> DbResult<SyncTimestamp> {
        let user_id = user_id.legacy_id as i64;
        let modified = user_collections::table
            .select(max(user_collections::modified))
            .filter(user_collections::user_id.eq(user_id))
            .first::<Option<i64>>(&mut self.conn)
            .await?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified).map_err(Into::into)
    }

    async fn get_collection_timestamp(
        &mut self,
        params: params::GetCollectionTimestamp,
    ) -> DbResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
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
            .filter(user_collections::user_id.eq(user_id as i64))
            .filter(user_collections::collection_id.eq(collection_id))
            .first(&mut self.conn)
            .await
            .optional()?
            .ok_or_else(DbError::collection_not_found)
    }

    async fn get_bso_timestamp(
        &mut self,
        params: params::GetBsoTimestamp,
    ) -> DbResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        let modified = bso::table
            .select(bso::modified)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(&params.id))
            .first::<i64>(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified).map_err(Into::into)
    }

    async fn get_collection_timestamps(
        &mut self,
        user_id: UserIdentifier,
    ) -> DbResult<results::GetCollectionTimestamps> {
        let modifieds = sql_query(format!(
            "SELECT {collection_id}, {modified}
               FROM user_collections
              WHERE {user_id} = ?
               AND {collection_id} != ?",
            collection_id = COLLECTION_ID,
            user_id = USER_ID,
            modified = LAST_MODIFIED
        ))
        .bind::<BigInt, _>(user_id.legacy_id as i64)
        .bind::<Integer, _>(TOMBSTONE)
        .load::<UserCollectionsResult>(&mut self.conn)
        .await?
        .into_iter()
        .map(|cr| {
            SyncTimestamp::from_i64(cr.last_modified)
                .map(|ts| (cr.collection, ts))
                .map_err(Into::into)
        })
        .collect::<DbResult<HashMap<_, _>>>()?;
        self.map_collection_names(modifieds).await
    }

    async fn check(&mut self) -> DbResult<results::Check> {
        sql_query("SELECT 1").execute(&mut self.conn).await?;
        Ok(true)
    }

    async fn update_collection(
        &mut self,
        params: params::UpdateCollection,
    ) -> DbResult<SyncTimestamp> {
        let quota = if self.quota.enabled {
            self.calc_quota_usage(params.user_id.legacy_id as i64, params.collection_id)
                .await?
        } else {
            results::GetQuotaUsage {
                count: 0,
                total_bytes: 0,
            }
        };
        let upsert = format!(
            r#"
                INSERT INTO user_collections ({user_id}, {collection_id}, {modified}, {total_bytes}, {count})
                VALUES (?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                       {modified} = ?,
                       {total_bytes} = ?,
                       {count} = ?
        "#,
            user_id = USER_ID,
            collection_id = COLLECTION_ID,
            modified = LAST_MODIFIED,
            count = COUNT,
            total_bytes = TOTAL_BYTES,
        );
        let total_bytes = quota.total_bytes as i64;
        let timestamp = self.timestamp().as_i64();
        sql_query(upsert)
            .bind::<BigInt, _>(params.user_id.legacy_id as i64)
            .bind::<Integer, _>(&params.collection_id)
            .bind::<BigInt, _>(&timestamp)
            .bind::<BigInt, _>(&total_bytes)
            .bind::<Integer, _>(&quota.count)
            .bind::<BigInt, _>(&timestamp)
            .bind::<BigInt, _>(&total_bytes)
            .bind::<Integer, _>(&quota.count)
            .execute(&mut self.conn)
            .await?;
        Ok(self.timestamp())
    }

    // Perform a lighter weight "read only" storage size check
    async fn get_storage_usage(
        &mut self,
        user_id: UserIdentifier,
    ) -> DbResult<results::GetStorageUsage> {
        let uid = user_id.legacy_id as i64;
        let total_bytes = bso::table
            .select(sql::<Nullable<BigInt>>("SUM(LENGTH(payload))"))
            .filter(bso::user_id.eq(uid))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .get_result::<Option<i64>>(&mut self.conn)
            .await?;
        Ok(total_bytes.unwrap_or_default() as u64)
    }

    // Perform a lighter weight "read only" quota storage check
    async fn get_quota_usage(
        &mut self,
        params: params::GetQuotaUsage,
    ) -> DbResult<results::GetQuotaUsage> {
        let uid = params.user_id.legacy_id as i64;
        let (total_bytes, count): (i64, i32) = user_collections::table
            .select((
                sql::<BigInt>("COALESCE(SUM(COALESCE(total_bytes, 0)), 0)"),
                sql::<Integer>("COALESCE(SUM(COALESCE(count, 0)), 0)"),
            ))
            .filter(user_collections::user_id.eq(uid))
            .filter(user_collections::collection_id.eq(params.collection_id))
            .get_result(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }

    async fn get_collection_usage(
        &mut self,
        user_id: UserIdentifier,
    ) -> DbResult<results::GetCollectionUsage> {
        let counts = bso::table
            .select((bso::collection_id, sql::<BigInt>("SUM(LENGTH(payload))")))
            .filter(bso::user_id.eq(user_id.legacy_id as i64))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .group_by(bso::collection_id)
            .load(&mut self.conn)
            .await?
            .into_iter()
            .collect();
        self.map_collection_names(counts).await
    }

    async fn get_collection_counts(
        &mut self,
        user_id: UserIdentifier,
    ) -> DbResult<results::GetCollectionCounts> {
        let counts = bso::table
            .select((
                bso::collection_id,
                sql::<BigInt>(&format!(
                    "COUNT({collection_id})",
                    collection_id = COLLECTION_ID
                )),
            ))
            .filter(bso::user_id.eq(user_id.legacy_id as i64))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .group_by(bso::collection_id)
            .load(&mut self.conn)
            .await?
            .into_iter()
            .collect();
        self.map_collection_names(counts).await
    }

    fn timestamp(&self) -> SyncTimestamp {
        self.session.timestamp
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    async fn create_collection(&mut self, name: &str) -> Result<i32, Self::Error> {
        self.get_or_create_collection_id(name).await
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

#[derive(Debug, QueryableByName)]
struct UserCollectionsResult {
    // Can't substitute column names here.
    #[diesel(sql_type = Integer)]
    collection: i32, // COLLECTION_ID
    #[diesel(sql_type = BigInt)]
    last_modified: i64, // LAST_MODIFIED
}
