use async_trait::async_trait;

use std::{self, cell::RefCell, collections::HashMap, fmt, future::Future, ops::Deref, sync::Arc};

use diesel::{
    connection::TransactionManager,
    delete,
    dsl::max,
    expression::sql_literal::sql,
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, PooledConnection},
    sql_query,
    sql_types::{BigInt, Integer, Nullable, Text},
    Connection, ExpressionMethods, GroupByDsl, QueryDsl,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;

use super::{
    batch,
    diesel_ext::{LockInShareModeDsl, OptionalExtension, RunAsyncQueryDsl},
    pool::CollectionCache,
    schema::{bso, collections, user_collections},
};
use crate::db::{
    self,
    error::{DbError, DbErrorKind},
    params, results,
    util::SyncTimestamp,
    Db, Sorting,
};
use crate::error::{ApiError, ApiResult};
use crate::server::metrics::Metrics;
use crate::settings::{Quota, DEFAULT_MAX_TOTAL_RECORDS};
use crate::web::extractors::{BsoQueryParams, HawkIdentifier};
use crate::web::tags::Tags;

pub type Conn = PooledConnection<ConnectionManager<MysqlConnection>>;

/// The ttl to use for rows that are never supposed to expire (in seconds)
/// We store the TTL as a SyncTimestamp, which is milliseconds, so remember
/// to multiply this by 1000.
pub const DEFAULT_BSO_TTL: u32 = 2_100_000_000;
// this is the max number of records we will return.
pub static DEFAULT_LIMIT: u32 = DEFAULT_MAX_TOTAL_RECORDS;

pub const TOMBSTONE: i32 = 0;
/// SQL Variable remapping
/// These names are the legacy values mapped to the new names.
pub const COLLECTION_ID: &str = "collection";
pub const USER_ID: &str = "userid";
pub const MODIFIED: &str = "modified";
pub const EXPIRY: &str = "ttl";
pub const LAST_MODIFIED: &str = "last_modified";
pub const COUNT: &str = "count";
pub const TOTAL_BYTES: &str = "total_bytes";

#[derive(Debug)]
pub enum CollectionLock {
    Read,
    Write,
}

/// Per session Db metadata
#[derive(Debug, Default)]
struct MysqlDbSession {
    /// The "current time" on the server used for this session's operations
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (user_id, collection_id)
    coll_modified_cache: HashMap<(u32, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(u32, i32), CollectionLock>,
    /// Whether a transaction was started (begin() called)
    in_transaction: bool,
    in_write_transaction: bool,
}

#[derive(Clone, Debug)]
pub struct MysqlDb {
    /// Synchronous Diesel calls are executed in actix_web::web::block to satisfy
    /// the Db trait's asynchronous interface.
    ///
    /// Arc<MysqlDbInner> provides a Clone impl utilized for safely moving to
    /// the thread pool but does not provide Send as the underlying db
    /// conn. structs are !Sync (Arc requires both for Send). See the Send impl
    /// below.
    pub(super) inner: Arc<MysqlDbInner>,

    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    pub metrics: Metrics,
    pub quota: Quota,
}

/// Despite the db conn structs being !Sync (see Arc<MysqlDbInner> above) we
/// don't spawn multiple MysqlDb calls at a time in the thread pool. Calls are
/// queued to the thread pool via Futures, naturally serialized.
unsafe impl Send for MysqlDb {}

pub struct MysqlDbInner {
    #[cfg(not(test))]
    pub(super) conn: Conn,
    #[cfg(test)]
    pub(super) conn: LoggingConnection<Conn>, // display SQL when RUST_LOG="diesel_logger=trace"

    session: RefCell<MysqlDbSession>,
}

impl fmt::Debug for MysqlDbInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MysqlDbInner {{ session: {:?} }}", self.session)
    }
}

impl Deref for MysqlDb {
    type Target = MysqlDbInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl MysqlDb {
    pub fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: &Quota,
    ) -> Self {
        let inner = MysqlDbInner {
            #[cfg(not(test))]
            conn,
            #[cfg(test)]
            conn: LoggingConnection::new(conn),
            session: RefCell::new(Default::default()),
        };
        MysqlDb {
            inner: Arc::new(inner),
            coll_cache,
            metrics: metrics.clone(),
            quota: *quota,
        }
    }

    pub(super) async fn get_or_create_collection_id(&self, name: String) -> ApiResult<i32> {
        if let Some(id) = self.coll_cache.get_id(&name)? {
            return Ok(id);
        }

        let id = {
            let name = name.clone();

            self.transaction(|| async move {
                diesel::insert_or_ignore_into(collections::table)
                    .values(collections::name.eq(name.clone()))
                    .execute(self.clone())
                    .await
                    .map_err(ApiError::from)?;

                collections::table
                    .select(collections::id)
                    .filter(collections::name.eq(name.clone()))
                    .first(self.clone())
                    .await
                    .map_err(Into::into)
            })
            .await?
        };

        if !self.session.borrow().in_write_transaction {
            self.coll_cache.put(id, name)?;
        }

        Ok(id)
    }

    async fn erect_tombstone(&self, user_id: i32) -> ApiResult<()> {
        sql_query(format!(
            r#"INSERT INTO user_collections ({user_id}, {collection_id}, {modified})
            VALUES (?, ?, ?)
                ON DUPLICATE KEY UPDATE
                    {modified} = VALUES({modified})"#,
            user_id = USER_ID,
            collection_id = COLLECTION_ID,
            modified = LAST_MODIFIED
        ))
        .bind::<BigInt, _>(user_id as i64)
        .bind::<Integer, _>(TOMBSTONE)
        .bind::<BigInt, _>(self.timestamp().as_i64())
        .execute(self.clone())
        .await
        .map(|_| ())
        .map_err(Into::into)
    }

    async fn _get_collection_name(&self, id: i32) -> ApiResult<String> {
        let name = if let Some(name) = self.coll_cache.get_name(id)? {
            name
        } else {
            sql_query(
                "SELECT name
                FROM collections
                WHERE id = ?",
            )
            .bind::<Integer, _>(id)
            .get_result::<NameResult>(self.clone())
            .await
            .optional()?
            .ok_or(DbErrorKind::CollectionNotFound)?
            .name
        };
        Ok(name)
    }

    async fn map_collection_names<T>(
        &self,
        by_id: HashMap<i32, T>,
    ) -> ApiResult<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys()).await?;
        by_id
            .into_iter()
            .map(|(id, value)| {
                names.remove(&id).map(|name| (name, value)).ok_or_else(|| {
                    ApiError::from(DbError::internal(
                        "load_collection_names unknown collection id",
                    ))
                })
            })
            .collect()
    }

    async fn load_collection_names<'a>(
        &self,
        collection_ids: impl Iterator<Item = &'a i32>,
    ) -> ApiResult<HashMap<i32, String>> {
        let mut names = HashMap::new();
        let mut uncached = Vec::new();
        for &id in collection_ids {
            if let Some(name) = self.coll_cache.get_name(id)? {
                names.insert(id, name);
            } else {
                uncached.push(id);
            }
        }

        if !uncached.is_empty() {
            let result = collections::table
                .select((collections::id, collections::name))
                .filter(collections::id.eq_any(uncached))
                .load::<(i32, String)>(self.clone())
                .await?;

            for (id, name) in result {
                names.insert(id, name.clone());
                if !self.session.borrow().in_write_transaction {
                    self.coll_cache.put(id, name)?;
                }
            }
        }

        Ok(names)
    }

    // perform a heavier weight quota calculation
    async fn calc_quota_usage(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> ApiResult<results::GetQuotaUsage> {
        let (total_bytes, count): (i64, i32) = bso::table
            .select((
                sql::<BigInt>(r#"COALESCE(SUM(LENGTH(COALESCE(payload, ""))),0)"#),
                sql::<Integer>("COALESCE(COUNT(*),0)"),
            ))
            .filter(bso::user_id.eq(user_id as i64))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .filter(bso::collection_id.eq(collection_id))
            .get_result(self.clone())
            .await
            .optional()?
            .unwrap_or_default();

        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }

    pub(super) async fn update_collection(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> ApiResult<SyncTimestamp> {
        let quota = if self.quota.enabled {
            self.calc_quota_usage(user_id, collection_id).await?
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
        sql_query(upsert)
            .bind::<BigInt, _>(user_id as i64)
            .bind::<Integer, _>(collection_id)
            .bind::<BigInt, _>(self.timestamp().as_i64())
            .bind::<BigInt, _>(total_bytes)
            .bind::<Integer, _>(quota.count)
            .bind::<BigInt, _>(self.timestamp().as_i64())
            .bind::<BigInt, _>(total_bytes)
            .bind::<Integer, _>(quota.count)
            .execute(self.clone())
            .await?;
        Ok(self.timestamp())
    }

    pub fn timestamp(&self) -> SyncTimestamp {
        self.session.borrow().timestamp
    }

    async fn transaction<T, F, R>(&self, f: F) -> ApiResult<T>
    where
        F: FnOnce() -> R,
        R: Future<Output = ApiResult<T>>,
    {
        self.begin(true).await?;
        match f().await {
            Ok(value) => {
                self.commit().await?;
                Ok(value)
            }
            Err(e) => {
                self.rollback().await?;
                Err(e)
            }
        }
    }
}

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
    async fn lock_for_read(&self, params: params::LockCollection) -> ApiResult<()> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self
            .get_collection_id(params.collection)
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
            .borrow()
            .coll_locks
            .get(&(user_id as u32, collection_id))
            .is_some()
        {
            return Ok(());
        }

        // Lock the db
        self.begin(false).await?;
        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .lock_in_share_mode()
            .first(self.clone())
            .await
            .optional()?;

        if let Some(modified) = modified {
            let modified = SyncTimestamp::from_i64(modified)?;
            self.session
                .borrow_mut()
                .coll_modified_cache
                .insert((user_id as u32, collection_id), modified); // why does it still expect a u32 int?
        }
        // XXX: who's responsible for unlocking (removing the entry)
        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id as u32, collection_id), CollectionLock::Read);

        Ok(())
    }

    async fn lock_for_write(&self, params: params::LockCollection) -> ApiResult<()> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(params.collection).await?;
        if let Some(CollectionLock::Read) = self
            .session
            .borrow()
            .coll_locks
            .get(&(user_id as u32, collection_id))
        {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }

        // Lock the db
        self.begin(true).await?;
        let modified = {
            user_collections::table
                .select(user_collections::modified)
                .filter(user_collections::user_id.eq(user_id))
                .filter(user_collections::collection_id.eq(collection_id))
                .for_update()
                .first(self.clone())
                .await
                .optional()?
        };
        if let Some(modified) = modified {
            let modified = SyncTimestamp::from_i64(modified)?;
            // Forbid the write if it would not properly incr the timestamp
            if modified >= self.timestamp() {
                Err(DbErrorKind::Conflict)?
            }
            self.session
                .borrow_mut()
                .coll_modified_cache
                .insert((user_id as u32, collection_id), modified);
        }
        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id as u32, collection_id), CollectionLock::Write);
        Ok(())
    }

    async fn begin(&self, for_write: bool) -> ApiResult<()> {
        let db = self.clone();
        db::blocking_thread(move || db.conn.transaction_manager().begin_transaction(&db.conn))
            .await?;
        self.session.borrow_mut().in_transaction = true;
        if for_write {
            self.session.borrow_mut().in_write_transaction = true;
        }
        Ok(())
    }

    async fn commit(&self) -> ApiResult<()> {
        if self.session.borrow().in_transaction {
            let db = self.clone();
            db::blocking_thread(move || db.conn.transaction_manager().commit_transaction(&db.conn))
                .await?;
        }
        Ok(())
    }

    async fn rollback(&self) -> ApiResult<()> {
        if self.session.borrow().in_transaction {
            let db = self.clone();
            db::blocking_thread(move || {
                db.conn.transaction_manager().rollback_transaction(&db.conn)
            })
            .await?;
        }
        Ok(())
    }

    async fn delete_storage(&self, user_id: HawkIdentifier) -> ApiResult<()> {
        let user_id = user_id.legacy_id as i64;
        // Delete user data.
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .execute(self.clone())
            .await?;
        // Delete user collections.
        delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .execute(self.clone())
            .await?;
        Ok(())
    }

    // Deleting the collection should result in:
    //  - collection does not appear in /info/collections
    //  - X-Last-Modified timestamp at the storage level changing
    async fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> ApiResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        let mut count = delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .execute(self.clone())
            .await?;
        count += delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .execute(self.clone())
            .await?;

        if count == 0 {
            Err(DbErrorKind::CollectionNotFound)?
        } else {
            self.erect_tombstone(user_id as i32).await?;
        }
        self.get_storage_timestamp(params.user_id).await
    }

    async fn get_collection_id(&self, name: String) -> ApiResult<i32> {
        if let Some(id) = self.coll_cache.get_id(&name)? {
            return Ok(id);
        }

        let id = sql_query(
            "SELECT id
               FROM collections
              WHERE name = ?",
        )
        .bind::<Text, _>(name.clone())
        .get_result::<IdResult>(self.clone())
        .await
        .optional()?
        .ok_or(DbErrorKind::CollectionNotFound)?
        .id;
        if !self.session.borrow().in_write_transaction {
            self.coll_cache.put(id, name)?;
        }
        Ok(id)
    }

    async fn put_bso(&self, bso: params::PutBso) -> ApiResult<results::PutBso> {
        /*
        if bso.payload.is_none() && bso.sortindex.is_none() && bso.ttl.is_none() {
            // XXX: go returns an error here (ErrNothingToDo), and is treated
            // as other errors
            return Ok(());
        }
        */

        let collection_id = self
            .get_or_create_collection_id(bso.collection.clone())
            .await?;
        let user_id: u64 = bso.user_id.legacy_id;
        let timestamp = self.timestamp().as_i64();
        if self.quota.enabled {
            let usage = self
                .get_quota_usage(params::GetQuotaUsage {
                    user_id: HawkIdentifier::new_legacy(user_id),
                    collection: bso.collection.clone(),
                    collection_id,
                })
                .await?;
            if usage.total_bytes >= self.quota.size as usize {
                let mut tags = Tags::default();
                tags.tags
                    .insert("collection".to_owned(), bso.collection.clone());
                self.metrics
                    .incr_with_tags("storage.quota.at_limit", Some(tags));
                if self.quota.enforced {
                    return Err(DbErrorKind::Quota.into());
                } else {
                    warn!("Quota at limit for user's collection ({} bytes)", usage.total_bytes; "collection"=>bso.collection.clone());
                }
            }
        }

        self.transaction(|| async move {
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
                .bind::<Integer, _>(collection_id)
                .bind::<Text, _>(bso.id)
                .bind::<Nullable<Integer>, _>(sortindex)
                .bind::<Text, _>(payload.to_owned())
                .bind::<BigInt, _>(timestamp)
                .bind::<BigInt, _>(timestamp + (i64::from(ttl) * 1000)) // remember: this is in millis
                .execute(self.clone())
                .await?;
            self.update_collection(user_id as u32, collection_id).await
        }).await
    }

    async fn get_bsos(&self, params: params::GetBsos) -> ApiResult<results::GetBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection).await?;
        let BsoQueryParams {
            newer,
            older,
            sort,
            limit,
            offset,
            ids,
            ..
        } = params.params;

        let db = self.clone();
        // Diesel's boxed queries aren't `Send`, so we have to construct the query inside the new
        // thread and invoke it synchronously.
        db::blocking_thread(move || {
            let now = db.timestamp().as_i64();
            let mut query = bso::table
                .select((
                    bso::id,
                    bso::modified,
                    bso::payload,
                    bso::sortindex,
                    bso::expiry,
                ))
                .filter(bso::user_id.eq(user_id))
                .filter(bso::collection_id.eq(collection_id as i32)) // XXX:
                .filter(bso::expiry.gt(now))
                .into_boxed();

            if let Some(older) = older {
                query = query.filter(bso::modified.lt(older.as_i64()));
            }
            if let Some(newer) = newer {
                query = query.filter(bso::modified.gt(newer.as_i64()));
            }

            if !ids.is_empty() {
                query = query.filter(bso::id.eq_any(ids));
            }

            // it's possible for two BSOs to be inserted with the same `modified` date,
            // since there's no guarantee of order when doing a get, pagination can return
            // an error. We "fudge" a bit here by taking the id order as a secondary, since
            // that is guaranteed to be unique by the client.
            query = match sort {
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

            let limit = limit.map(i64::from).unwrap_or(DEFAULT_LIMIT as i64).max(0);
            // fetch an extra row to detect if there are more rows that
            // match the query conditions
            query = query.limit(if limit > 0 { limit + 1 } else { limit });

            let numeric_offset = offset.map_or(0, |offset| offset.offset as i64);

            if numeric_offset > 0 {
                // XXX: copy over this optimization:
                // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
                query = query.offset(numeric_offset);
            }
            let mut bsos = diesel::RunQueryDsl::load::<results::GetBso>(query, &db.conn)?;

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
                    Some(0u8.to_string())
                } else {
                    None
                }
            };

            Ok(results::GetBsos {
                items: bsos,
                offset: next_offset,
            })
        })
        .await
        .map_err(Into::into)
    }

    async fn get_bso_ids(&self, params: params::GetBsos) -> ApiResult<results::GetBsoIds> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection).await?;
        let BsoQueryParams {
            newer,
            older,
            sort,
            limit,
            offset,
            ids,
            ..
        } = params.params;

        let db = self.clone();
        // Diesel's boxed queries aren't `Send`, so we have to construct the query inside the new
        // thread and invoke it synchronously.
        db::blocking_thread(move || {
            let mut query = bso::table
                .select(bso::id)
                .filter(bso::user_id.eq(user_id))
                .filter(bso::collection_id.eq(collection_id as i32)) // XXX:
                .filter(bso::expiry.gt(db.timestamp().as_i64()))
                .into_boxed();

            if let Some(older) = older {
                query = query.filter(bso::modified.lt(older.as_i64()));
            }
            if let Some(newer) = newer {
                query = query.filter(bso::modified.gt(newer.as_i64()));
            }

            if !ids.is_empty() {
                query = query.filter(bso::id.eq_any(ids));
            }

            query = match sort {
                Sorting::Index => query.order(bso::sortindex.desc()),
                Sorting::Newest => query.order(bso::modified.desc()),
                Sorting::Oldest => query.order(bso::modified.asc()),
                _ => query,
            };

            // negative limits are no longer allowed by mysql.
            let limit = limit.map(i64::from).unwrap_or(DEFAULT_LIMIT as i64).max(0);
            // fetch an extra row to detect if there are more rows that
            // match the query conditions. Negative limits will cause an error.
            query = query.limit(if limit == 0 { limit } else { limit + 1 });
            let numeric_offset = offset.map_or(0, |offset| offset.offset as i64);
            if numeric_offset != 0 {
                // XXX: copy over this optimization:
                // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
                query = query.offset(numeric_offset);
            }
            let mut ids = diesel::RunQueryDsl::load::<String>(query, &db.conn)?;

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
        })
        .await
        .map_err(Into::into)
    }

    async fn get_bso(&self, params: params::GetBso) -> ApiResult<Option<results::GetBso>> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection).await?;
        Ok(bso::table
            .select((
                bso::id,
                bso::modified,
                bso::payload,
                bso::sortindex,
                bso::expiry,
            ))
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::id.eq(params.id))
            .filter(bso::expiry.ge(self.timestamp().as_i64()))
            .get_result::<results::GetBso>(self.clone())
            .await
            .optional()?)
    }

    async fn delete_bso(&self, params: params::DeleteBso) -> ApiResult<results::DeleteBso> {
        let user_id = params.user_id.legacy_id;
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        let affected_rows = delete(bso::table)
            .filter(bso::user_id.eq(user_id as i64))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::id.eq(params.id))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .execute(self.clone())
            .await?;
        if affected_rows == 0 {
            Err(DbErrorKind::BsoNotFound)?
        }
        self.update_collection(user_id as u32, collection_id).await
    }

    async fn delete_bsos(&self, params: params::DeleteBsos) -> ApiResult<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::id.eq_any(params.ids))
            .execute(self.clone())
            .await?;
        self.update_collection(user_id as u32, collection_id).await
    }

    async fn post_bsos(&self, params: params::PostBsos) -> ApiResult<results::PostBsos> {
        let collection_id = self
            .get_or_create_collection_id(params.collection.clone())
            .await?;
        let mut result = results::PostBsos {
            modified: self.timestamp(),
            success: Default::default(),
            failed: params.failed,
        };

        for pbso in params.bsos {
            let id = pbso.id;
            let put_result = self
                .put_bso(params::PutBso {
                    user_id: params.user_id.clone(),
                    collection: params.collection.clone(),
                    id: id.clone(),
                    payload: pbso.payload,
                    sortindex: pbso.sortindex,
                    ttl: pbso.ttl,
                })
                .await;
            // XXX: python version doesn't report failures from db
            // layer.. (wouldn't db failures abort the entire transaction
            // anyway?)
            // XXX: sanitize to.to_string()?
            match put_result {
                Ok(_) => result.success.push(id),
                Err(e) => {
                    result.failed.insert(id, e.to_string());
                }
            }
        }
        self.update_collection(params.user_id.legacy_id as u32, collection_id)
            .await?;
        Ok(result)
    }

    async fn get_storage_timestamp(&self, user_id: HawkIdentifier) -> ApiResult<SyncTimestamp> {
        let user_id = user_id.legacy_id as i64;
        let modified = user_collections::table
            .select(max(user_collections::modified))
            .filter(user_collections::user_id.eq(user_id))
            .first::<Option<i64>>(self.clone())
            .await?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified).map_err(Into::into)
    }

    async fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> ApiResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_collection_id(params.collection).await?;
        if let Some(modified) = self
            .session
            .borrow()
            .coll_modified_cache
            .get(&(user_id, collection_id))
        {
            return Ok(*modified);
        }
        user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id as i64))
            .filter(user_collections::collection_id.eq(collection_id))
            .first(self.clone())
            .await
            .optional()?
            .ok_or_else(|| DbErrorKind::CollectionNotFound.into())
    }

    async fn get_bso_timestamp(&self, params: params::GetBsoTimestamp) -> ApiResult<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        let modified = bso::table
            .select(bso::modified)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id))
            .filter(bso::id.eq(params.id))
            .first::<i64>(self.clone())
            .await
            .optional()?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified).map_err(Into::into)
    }

    async fn get_collection_timestamps(
        &self,
        user_id: HawkIdentifier,
    ) -> ApiResult<results::GetCollectionTimestamps> {
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
        .load::<UserCollectionsResult>(self.clone())
        .await?
        .into_iter()
        .map(|cr| SyncTimestamp::from_i64(cr.last_modified).map(|ts| (cr.collection, ts)))
        .collect::<Result<HashMap<_, _>, DbError>>()
        .map_err(DbError::from)?;
        self.map_collection_names(modifieds).await
    }

    async fn check(&self) -> ApiResult<results::Check> {
        // has the database been up for more than 0 seconds?
        let result = sql_query("SHOW STATUS LIKE \"Uptime\"")
            .execute(self.clone())
            .await?;
        Ok(result as u64 > 0)
    }

    #[cfg(test)]
    async fn update_collection(
        &self,
        params: params::UpdateCollection,
    ) -> ApiResult<SyncTimestamp> {
        self.update_collection(params.user_id.legacy_id as u32, params.collection_id)
            .await
    }

    // Perform a lighter weight "read only" storage size check
    async fn get_storage_usage(
        &self,
        user_id: HawkIdentifier,
    ) -> ApiResult<results::GetStorageUsage> {
        let uid = user_id.legacy_id as i64;
        let total_bytes = bso::table
            .select(sql::<Nullable<BigInt>>("SUM(LENGTH(payload))"))
            .filter(bso::user_id.eq(uid))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .get_result::<Option<i64>>(self.clone())
            .await?;
        Ok(total_bytes.unwrap_or_default() as u64)
    }

    // Perform a lighter weight "read only" quota storage check
    async fn get_quota_usage(
        &self,
        params: params::GetQuotaUsage,
    ) -> ApiResult<results::GetQuotaUsage> {
        let uid = params.user_id.legacy_id as i64;
        let (total_bytes, count): (i64, i32) = user_collections::table
            .select((
                sql::<BigInt>("COALESCE(SUM(COALESCE(total_bytes, 0)), 0)"),
                sql::<Integer>("COALESCE(SUM(COALESCE(count, 0)), 0)"),
            ))
            .filter(user_collections::user_id.eq(uid))
            .filter(user_collections::collection_id.eq(params.collection_id))
            .get_result(self.clone())
            .await
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }

    async fn get_collection_usage(
        &self,
        user_id: HawkIdentifier,
    ) -> ApiResult<results::GetCollectionUsage> {
        let counts = bso::table
            .select((bso::collection_id, sql::<BigInt>("SUM(LENGTH(payload))")))
            .filter(bso::user_id.eq(user_id.legacy_id as i64))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .group_by(bso::collection_id)
            .load(self.clone())
            .await?
            .into_iter()
            .collect();
        self.map_collection_names(counts).await
    }

    async fn get_collection_counts(
        &self,
        user_id: HawkIdentifier,
    ) -> ApiResult<results::GetCollectionCounts> {
        let counts = bso::table
            .select((
                bso::collection_id,
                sql::<BigInt>(&format!(
                    "COUNT({collection_id})",
                    collection_id = COLLECTION_ID
                )),
            ))
            .filter(bso::user_id.eq(user_id.legacy_id as i64))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .group_by(bso::collection_id)
            .load(self.clone())
            .await?
            .into_iter()
            .collect();
        self.map_collection_names(counts).await
    }

    async fn create_batch(&self, params: params::CreateBatch) -> ApiResult<results::CreateBatch> {
        batch::create(self, params).await
    }

    async fn validate_batch(
        &self,
        params: params::ValidateBatch,
    ) -> ApiResult<results::ValidateBatch> {
        batch::validate(self, params).await
    }

    async fn append_to_batch(
        &self,
        params: params::AppendToBatch,
    ) -> ApiResult<results::AppendToBatch> {
        batch::append(self, params).await
    }

    async fn commit_batch(&self, params: params::CommitBatch) -> ApiResult<results::CommitBatch> {
        batch::commit(self, params).await
    }

    async fn get_batch(&self, params: params::GetBatch) -> ApiResult<Option<results::GetBatch>> {
        batch::get(self, params).await
    }

    #[cfg(test)]
    async fn delete_batch(&self, params: params::DeleteBatch) -> ApiResult<()> {
        batch::delete(self, params).await
    }

    #[cfg(test)]
    fn timestamp(&self) -> SyncTimestamp {
        self.timestamp()
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    #[cfg(test)]
    async fn create_collection(&self, name: String) -> ApiResult<i32> {
        self.get_or_create_collection_id(name).await
    }

    #[cfg(test)]
    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = timestamp;
    }

    #[cfg(test)]
    async fn clear_coll_cache(&self) -> ApiResult<()> {
        let db = self.clone();
        db::blocking_thread(move || {
            db.coll_cache.clear();
            Ok::<(), diesel::result::Error>(())
        })
        .await?;

        Ok(())
    }

    #[cfg(test)]
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
    #[sql_type = "Integer"]
    id: i32,
}

#[allow(dead_code)] // Not really dead, Rust can't see the use above
#[derive(Debug, QueryableByName)]
struct NameResult {
    #[sql_type = "Text"]
    name: String,
}

#[derive(Debug, QueryableByName)]
struct UserCollectionsResult {
    // Can't substitute column names here.
    #[sql_type = "Integer"]
    collection: i32, // COLLECTION_ID
    #[sql_type = "BigInt"]
    last_modified: i64, // LAST_MODIFIED
}
