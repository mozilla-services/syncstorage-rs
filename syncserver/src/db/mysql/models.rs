use futures::future::TryFutureExt;

use std::{self, cell::RefCell, collections::HashMap, fmt, ops::Deref, sync::Arc};

use diesel::{
    connection::TransactionManager,
    delete,
    dsl::max,
    expression::sql_literal::sql,
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, PooledConnection},
    sql_query,
    sql_types::{BigInt, Integer, Nullable, Text},
    Connection, ExpressionMethods, GroupByDsl, OptionalExtension, QueryDsl, RunQueryDsl,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use syncstorage_db_common::{
    error::{DbError, DbErrorKind},
    params, results,
    util::SyncTimestamp,
    Db, DbFuture, Sorting, UserIdentifier, DEFAULT_BSO_TTL,
};
use syncstorage_settings::{Quota, DEFAULT_MAX_TOTAL_RECORDS};

use super::{
    batch,
    diesel_ext::LockInShareModeDsl,
    pool::CollectionCache,
    schema::{bso, collections, user_collections},
};
use crate::db;
use crate::server::metrics::Metrics;
use crate::web::tags::Tags;

pub type Result<T> = std::result::Result<T, DbError>;
type Conn = PooledConnection<ConnectionManager<MysqlConnection>>;

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
    /// Synchronous Diesel calls are executed in tokio::task::spawn_blocking to satisfy
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

    /// APIs for collection-level locking
    ///
    /// Explicitly lock the matching row in the user_collections table. Read
    /// locks do SELECT ... LOCK IN SHARE MODE and write locks do SELECT
    /// ... FOR UPDATE.
    ///
    /// In theory it would be possible to use serializable transactions rather
    /// than explicit locking, but our ops team have expressed concerns about
    /// the efficiency of that approach at scale.
    pub fn lock_for_read_sync(&self, params: params::LockCollection) -> Result<()> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).or_else(|e| {
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
        self.begin(false)?;
        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .lock_in_share_mode()
            .first(&self.conn)
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

    pub fn lock_for_write_sync(&self, params: params::LockCollection) -> Result<()> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection)?;
        if let Some(CollectionLock::Read) = self
            .session
            .borrow()
            .coll_locks
            .get(&(user_id as u32, collection_id))
        {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }

        // Lock the db
        self.begin(true)?;
        let modified = user_collections::table
            .select(user_collections::modified)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(collection_id))
            .for_update()
            .first(&self.conn)
            .optional()?;
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

    pub(super) fn begin(&self, for_write: bool) -> Result<()> {
        self.conn
            .transaction_manager()
            .begin_transaction(&self.conn)?;
        self.session.borrow_mut().in_transaction = true;
        if for_write {
            self.session.borrow_mut().in_write_transaction = true;
        }
        Ok(())
    }

    pub async fn begin_async(&self, for_write: bool) -> Result<()> {
        self.begin(for_write)
    }

    pub fn commit_sync(&self) -> Result<()> {
        if self.session.borrow().in_transaction {
            self.conn
                .transaction_manager()
                .commit_transaction(&self.conn)?;
        }
        Ok(())
    }

    pub fn rollback_sync(&self) -> Result<()> {
        if self.session.borrow().in_transaction {
            self.conn
                .transaction_manager()
                .rollback_transaction(&self.conn)?;
        }
        Ok(())
    }

    fn erect_tombstone(&self, user_id: i32) -> Result<()> {
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
        .execute(&self.conn)?;
        Ok(())
    }

    pub fn delete_storage_sync(&self, user_id: UserIdentifier) -> Result<()> {
        let user_id = user_id.legacy_id as i64;
        // Delete user data.
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .execute(&self.conn)?;
        // Delete user collections.
        delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .execute(&self.conn)?;
        Ok(())
    }

    // Deleting the collection should result in:
    //  - collection does not appear in /info/collections
    //  - X-Last-Modified timestamp at the storage level changing
    pub fn delete_collection_sync(
        &self,
        params: params::DeleteCollection,
    ) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
        let mut count = delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .execute(&self.conn)?;
        count += delete(user_collections::table)
            .filter(user_collections::user_id.eq(user_id))
            .filter(user_collections::collection_id.eq(&collection_id))
            .execute(&self.conn)?;
        if count == 0 {
            Err(DbErrorKind::CollectionNotFound)?
        } else {
            self.erect_tombstone(user_id as i32)?;
        }
        self.get_storage_timestamp_sync(params.user_id)
    }

    pub(super) fn get_or_create_collection_id(&self, name: &str) -> Result<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = self.conn.transaction(|| {
            diesel::insert_or_ignore_into(collections::table)
                .values(collections::name.eq(name))
                .execute(&self.conn)?;

            collections::table
                .select(collections::id)
                .filter(collections::name.eq(name))
                .first(&self.conn)
        })?;

        if !self.session.borrow().in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }

        Ok(id)
    }

    pub(super) fn get_collection_id(&self, name: &str) -> Result<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let id = sql_query(
            "SELECT id
               FROM collections
              WHERE name = ?",
        )
        .bind::<Text, _>(name)
        .get_result::<IdResult>(&self.conn)
        .optional()?
        .ok_or(DbErrorKind::CollectionNotFound)?
        .id;
        if !self.session.borrow().in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }
        Ok(id)
    }

    fn _get_collection_name(&self, id: i32) -> Result<String> {
        let name = if let Some(name) = self.coll_cache.get_name(id)? {
            name
        } else {
            sql_query(
                "SELECT name
                   FROM collections
                  WHERE id = ?",
            )
            .bind::<Integer, _>(&id)
            .get_result::<NameResult>(&self.conn)
            .optional()?
            .ok_or(DbErrorKind::CollectionNotFound)?
            .name
        };
        Ok(name)
    }

    pub fn put_bso_sync(&self, bso: params::PutBso) -> Result<results::PutBso> {
        /*
        if bso.payload.is_none() && bso.sortindex.is_none() && bso.ttl.is_none() {
            // XXX: go returns an error here (ErrNothingToDo), and is treated
            // as other errors
            return Ok(());
        }
        */

        let collection_id = self.get_or_create_collection_id(&bso.collection)?;
        let user_id: u64 = bso.user_id.legacy_id;
        let timestamp = self.timestamp().as_i64();
        if self.quota.enabled {
            let usage = self.get_quota_usage_sync(params::GetQuotaUsage {
                user_id: UserIdentifier::new_legacy(user_id),
                collection: bso.collection.clone(),
                collection_id,
            })?;
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

        self.conn.transaction(|| {
            let payload = bso.payload.as_deref().unwrap_or_default();
            let sortindex = bso.sortindex;
            let ttl = bso.ttl.map_or(DEFAULT_BSO_TTL, |ttl| ttl);
            let q = format!(r#"
            INSERT INTO bso ({user_id}, {collection_id}, id, sortindex, payload, {modified}, {expiry})
            VALUES (?, ?, ?, ?, ?, ?, ?)
                ON DUPLICATE KEY UPDATE
                   {user_id} = VALUES({user_id}),
                   {collection_id} = VALUES({collection_id}),
                   id = VALUES(id)
            "#, user_id=USER_ID, modified=MODIFIED, collection_id=COLLECTION_ID, expiry=EXPIRY);
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
                    format!(", {expiry} = VALUES({expiry})", expiry=EXPIRY)
                } else {
                    "".to_owned()
                },
            );
            let q = format!(
                "{}{}",
                q,
                if bso.payload.is_some() || bso.sortindex.is_some() {
                    format!(", {modified} = VALUES({modified})", modified=MODIFIED)
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
                .execute(&self.conn)?;
            self.update_collection(user_id as u32, collection_id)
        })
    }

    pub fn get_bsos_sync(&self, params: params::GetBsos) -> Result<results::GetBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
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
            .filter(bso::collection_id.eq(collection_id as i32)) // XXX:
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
        let mut bsos = query.load::<results::GetBso>(&self.conn)?;

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

    pub fn get_bso_ids_sync(&self, params: params::GetBsos) -> Result<results::GetBsoIds> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
        let mut query = bso::table
            .select(bso::id)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(collection_id as i32)) // XXX:
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
        let mut ids = query.load::<String>(&self.conn)?;

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

    pub fn get_bso_sync(&self, params: params::GetBso) -> Result<Option<results::GetBso>> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
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
            .filter(bso::expiry.ge(self.timestamp().as_i64()))
            .get_result::<results::GetBso>(&self.conn)
            .optional()?)
    }

    pub fn delete_bso_sync(&self, params: params::DeleteBso) -> Result<results::DeleteBso> {
        let user_id = params.user_id.legacy_id;
        let collection_id = self.get_collection_id(&params.collection)?;
        let affected_rows = delete(bso::table)
            .filter(bso::user_id.eq(user_id as i64))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(params.id))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .execute(&self.conn)?;
        if affected_rows == 0 {
            Err(DbErrorKind::BsoNotFound)?
        }
        self.update_collection(user_id as u32, collection_id)
    }

    pub fn delete_bsos_sync(&self, params: params::DeleteBsos) -> Result<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
        delete(bso::table)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq_any(params.ids))
            .execute(&self.conn)?;
        self.update_collection(user_id as u32, collection_id)
    }

    pub fn post_bsos_sync(&self, input: params::PostBsos) -> Result<results::PostBsos> {
        let collection_id = self.get_or_create_collection_id(&input.collection)?;
        let mut result = results::PostBsos {
            modified: self.timestamp(),
            success: Default::default(),
            failed: input.failed,
        };

        for pbso in input.bsos {
            let id = pbso.id;
            let put_result = self.put_bso_sync(params::PutBso {
                user_id: input.user_id.clone(),
                collection: input.collection.clone(),
                id: id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            });
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
        self.update_collection(input.user_id.legacy_id as u32, collection_id)?;
        Ok(result)
    }

    pub fn get_storage_timestamp_sync(&self, user_id: UserIdentifier) -> Result<SyncTimestamp> {
        let user_id = user_id.legacy_id as i64;
        let modified = user_collections::table
            .select(max(user_collections::modified))
            .filter(user_collections::user_id.eq(user_id))
            .first::<Option<i64>>(&self.conn)?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified)
    }

    pub fn get_collection_timestamp_sync(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_collection_id(&params.collection)?;
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
            .first(&self.conn)
            .optional()?
            .ok_or_else(|| DbErrorKind::CollectionNotFound.into())
    }

    pub fn get_bso_timestamp_sync(&self, params: params::GetBsoTimestamp) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection)?;
        let modified = bso::table
            .select(bso::modified)
            .filter(bso::user_id.eq(user_id))
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(&params.id))
            .first::<i64>(&self.conn)
            .optional()?
            .unwrap_or_default();
        SyncTimestamp::from_i64(modified)
    }

    pub fn get_collection_timestamps_sync(
        &self,
        user_id: UserIdentifier,
    ) -> Result<results::GetCollectionTimestamps> {
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
        .load::<UserCollectionsResult>(&self.conn)?
        .into_iter()
        .map(|cr| SyncTimestamp::from_i64(cr.last_modified).map(|ts| (cr.collection, ts)))
        .collect::<Result<HashMap<_, _>>>()?;
        self.map_collection_names(modifieds)
    }

    fn check_sync(&self) -> Result<results::Check> {
        // has the database been up for more than 0 seconds?
        let result = sql_query("SHOW STATUS LIKE \"Uptime\"").execute(&self.conn)?;
        Ok(result as u64 > 0)
    }

    fn map_collection_names<T>(&self, by_id: HashMap<i32, T>) -> Result<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys())?;
        by_id
            .into_iter()
            .map(|(id, value)| {
                names
                    .remove(&id)
                    .map(|name| (name, value))
                    .ok_or_else(|| DbError::internal("load_collection_names unknown collection id"))
            })
            .collect()
    }

    fn load_collection_names<'a>(
        &self,
        collection_ids: impl Iterator<Item = &'a i32>,
    ) -> Result<HashMap<i32, String>> {
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
                .load::<(i32, String)>(&self.conn)?;

            for (id, name) in result {
                names.insert(id, name.clone());
                if !self.session.borrow().in_write_transaction {
                    self.coll_cache.put(id, name)?;
                }
            }
        }

        Ok(names)
    }

    pub(super) fn update_collection(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> Result<SyncTimestamp> {
        let quota = if self.quota.enabled {
            self.calc_quota_usage_sync(user_id, collection_id)?
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
            .bind::<Integer, _>(&collection_id)
            .bind::<BigInt, _>(&self.timestamp().as_i64())
            .bind::<BigInt, _>(&total_bytes)
            .bind::<Integer, _>(&quota.count)
            .bind::<BigInt, _>(&self.timestamp().as_i64())
            .bind::<BigInt, _>(&total_bytes)
            .bind::<Integer, _>(&quota.count)
            .execute(&self.conn)?;
        Ok(self.timestamp())
    }

    // Perform a lighter weight "read only" storage size check
    pub fn get_storage_usage_sync(
        &self,
        user_id: UserIdentifier,
    ) -> Result<results::GetStorageUsage> {
        let uid = user_id.legacy_id as i64;
        let total_bytes = bso::table
            .select(sql::<Nullable<BigInt>>("SUM(LENGTH(payload))"))
            .filter(bso::user_id.eq(uid))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .get_result::<Option<i64>>(&self.conn)?;
        Ok(total_bytes.unwrap_or_default() as u64)
    }

    // Perform a lighter weight "read only" quota storage check
    pub fn get_quota_usage_sync(
        &self,
        params: params::GetQuotaUsage,
    ) -> Result<results::GetQuotaUsage> {
        let uid = params.user_id.legacy_id as i64;
        let (total_bytes, count): (i64, i32) = user_collections::table
            .select((
                sql::<BigInt>("COALESCE(SUM(COALESCE(total_bytes, 0)), 0)"),
                sql::<Integer>("COALESCE(SUM(COALESCE(count, 0)), 0)"),
            ))
            .filter(user_collections::user_id.eq(uid))
            .filter(user_collections::collection_id.eq(params.collection_id))
            .get_result(&self.conn)
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }

    // perform a heavier weight quota calculation
    pub fn calc_quota_usage_sync(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> Result<results::GetQuotaUsage> {
        let (total_bytes, count): (i64, i32) = bso::table
            .select((
                sql::<BigInt>(r#"COALESCE(SUM(LENGTH(COALESCE(payload, ""))),0)"#),
                sql::<Integer>("COALESCE(COUNT(*),0)"),
            ))
            .filter(bso::user_id.eq(user_id as i64))
            .filter(bso::expiry.gt(self.timestamp().as_i64()))
            .filter(bso::collection_id.eq(collection_id))
            .get_result(&self.conn)
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }

    pub fn get_collection_usage_sync(
        &self,
        user_id: UserIdentifier,
    ) -> Result<results::GetCollectionUsage> {
        let counts = bso::table
            .select((bso::collection_id, sql::<BigInt>("SUM(LENGTH(payload))")))
            .filter(bso::user_id.eq(user_id.legacy_id as i64))
            .filter(bso::expiry.gt(&self.timestamp().as_i64()))
            .group_by(bso::collection_id)
            .load(&self.conn)?
            .into_iter()
            .collect();
        self.map_collection_names(counts)
    }

    pub fn get_collection_counts_sync(
        &self,
        user_id: UserIdentifier,
    ) -> Result<results::GetCollectionCounts> {
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
            .load(&self.conn)?
            .into_iter()
            .collect();
        self.map_collection_names(counts)
    }

    batch_db_method!(create_batch_sync, create, CreateBatch);
    batch_db_method!(validate_batch_sync, validate, ValidateBatch);
    batch_db_method!(append_to_batch_sync, append, AppendToBatch);
    batch_db_method!(commit_batch_sync, commit, CommitBatch);
    batch_db_method!(delete_batch_sync, delete, DeleteBatch);

    pub fn get_batch_sync(&self, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
        batch::get(self, params)
    }

    pub fn timestamp(&self) -> SyncTimestamp {
        self.session.borrow().timestamp
    }
}
#[macro_export]
macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result> {
            let db = self.clone();
            Box::pin(db::run_on_blocking_threadpool(move || {
                db.$sync_name(params)
            }))
        }
    };
}

impl<'a> Db<'a> for MysqlDb {
    fn commit(&self) -> DbFuture<'_, ()> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || db.commit_sync()))
    }

    fn rollback(&self) -> DbFuture<'_, ()> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || db.rollback_sync()))
    }

    fn begin(&self, for_write: bool) -> DbFuture<'_, ()> {
        let db = self.clone();
        Box::pin(async move { db.begin_async(for_write).map_err(Into::into).await })
    }

    fn box_clone(&self) -> Box<dyn Db<'a>> {
        Box::new(self.clone())
    }

    fn check(&self) -> DbFuture<'_, results::Check> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || db.check_sync()))
    }

    sync_db_method!(lock_for_read, lock_for_read_sync, LockCollection);
    sync_db_method!(lock_for_write, lock_for_write_sync, LockCollection);
    sync_db_method!(
        get_collection_timestamps,
        get_collection_timestamps_sync,
        GetCollectionTimestamps
    );
    sync_db_method!(
        get_collection_timestamp,
        get_collection_timestamp_sync,
        GetCollectionTimestamp
    );
    sync_db_method!(
        get_collection_counts,
        get_collection_counts_sync,
        GetCollectionCounts
    );
    sync_db_method!(
        get_collection_usage,
        get_collection_usage_sync,
        GetCollectionUsage
    );
    sync_db_method!(
        get_storage_timestamp,
        get_storage_timestamp_sync,
        GetStorageTimestamp
    );
    sync_db_method!(get_storage_usage, get_storage_usage_sync, GetStorageUsage);
    sync_db_method!(get_quota_usage, get_quota_usage_sync, GetQuotaUsage);
    sync_db_method!(delete_storage, delete_storage_sync, DeleteStorage);
    sync_db_method!(delete_collection, delete_collection_sync, DeleteCollection);
    sync_db_method!(delete_bsos, delete_bsos_sync, DeleteBsos);
    sync_db_method!(get_bsos, get_bsos_sync, GetBsos);
    sync_db_method!(get_bso_ids, get_bso_ids_sync, GetBsoIds);
    sync_db_method!(post_bsos, post_bsos_sync, PostBsos);
    sync_db_method!(delete_bso, delete_bso_sync, DeleteBso);
    sync_db_method!(get_bso, get_bso_sync, GetBso, Option<results::GetBso>);
    sync_db_method!(
        get_bso_timestamp,
        get_bso_timestamp_sync,
        GetBsoTimestamp,
        results::GetBsoTimestamp
    );
    sync_db_method!(put_bso, put_bso_sync, PutBso);
    sync_db_method!(create_batch, create_batch_sync, CreateBatch);
    sync_db_method!(validate_batch, validate_batch_sync, ValidateBatch);
    sync_db_method!(append_to_batch, append_to_batch_sync, AppendToBatch);
    sync_db_method!(
        get_batch,
        get_batch_sync,
        GetBatch,
        Option<results::GetBatch>
    );
    sync_db_method!(commit_batch, commit_batch_sync, CommitBatch);

    fn get_collection_id(&self, name: String) -> DbFuture<'_, i32> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || {
            db.get_collection_id(&name)
        }))
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        results::ConnectionInfo::default()
    }

    fn create_collection(&self, name: String) -> DbFuture<'_, i32> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || {
            db.get_or_create_collection_id(&name)
        }))
    }

    fn update_collection(&self, param: params::UpdateCollection) -> DbFuture<'_, SyncTimestamp> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || {
            db.update_collection(param.user_id.legacy_id as u32, param.collection_id)
        }))
    }

    fn timestamp(&self) -> SyncTimestamp {
        self.timestamp()
    }

    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = timestamp;
    }

    sync_db_method!(delete_batch, delete_batch_sync, DeleteBatch);

    fn clear_coll_cache(&self) -> DbFuture<'_, ()> {
        let db = self.clone();
        Box::pin(db::run_on_blocking_threadpool(move || {
            db.coll_cache.clear();
            Ok(())
        }))
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
