use futures::future;
use futures::lazy;

use diesel::r2d2::PooledConnection;

use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use super::manager::SpannerConnectionManager;
use super::pool::CollectionCache;
use super::support::SpannerType;

use crate::db::{
    error::{DbError, DbErrorKind},
    params, results,
    util::{to_rfc3339, SyncTimestamp},
    Db, DbFuture, Sorting, FIRST_CUSTOM_COLLECTION_ID,
};

use crate::web::extractors::BsoQueryParams;

use super::{
    batch,
    support::{as_list_value, as_value, bso_from_row, ExecuteSqlRequestBuilder},
};

#[cfg(not(feature = "google_grpc"))]
use google_spanner1::{
    BeginTransactionRequest, CommitRequest, ExecuteSqlRequest, ReadOnly, ReadWrite,
    RollbackRequest, TransactionOptions,
};

#[cfg(feature = "google_grpc")]
pub type TransactionSelector = googleapis_raw::spanner::v1::transaction::TransactionSelector;
#[cfg(not(feature = "google_grpc"))]
pub type TransactionSelector = google_spanner1::TransactionSelector;

#[derive(Debug, Eq, PartialEq)]
pub enum CollectionLock {
    Read,
    Write,
}

pub(super) type Conn = PooledConnection<SpannerConnectionManager>;
pub type Result<T> = std::result::Result<T, DbError>;

/// The ttl to use for rows that are never supposed to expire (in seconds)
pub const DEFAULT_BSO_TTL: i64 = 2_100_000_000;

pub const TOMBSTONE: i32 = 0;

/// Per session Db metadata
#[derive(Debug, Default)]
struct SpannerDbSession {
    /// CURRENT_TIMESTAMP() from Spanner, used for timestamping this session's
    /// operations
    timestamp: Option<SyncTimestamp>,
    /// Cache of collection modified timestamps per (user_id, collection_id)
    coll_modified_cache: HashMap<(u32, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(u32, i32), CollectionLock>,
    #[cfg(feature = "google_grpc")]
    transaction: Option<googleapis_raw::spanner::v1::transaction::TransactionSelector>,
    #[cfg(not(feature = "google_grpc"))]
    transaction: Option<TransactionSelector>,
    in_write_transaction: bool,
    execute_sql_count: u64,
}

#[derive(Clone, Debug)]
pub struct SpannerDb {
    pub(super) inner: Arc<SpannerDbInner>,

    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,
}

pub struct SpannerDbInner {
    pub(super) conn: Conn,

    thread_pool: Arc<::tokio_threadpool::ThreadPool>,
    session: RefCell<SpannerDbSession>,
}

impl fmt::Debug for SpannerDbInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpannerDbInner")
    }
}

impl Deref for SpannerDb {
    type Target = SpannerDbInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

macro_rules! batch_db_method {
    ($name:ident, $batch_name:ident, $type:ident) => {
        pub fn $name(&self, params: params::$type) -> Result<results::$type> {
            batch::$batch_name(self, params)
        }
    }
}

impl SpannerDb {
    pub fn new(
        conn: Conn,
        thread_pool: Arc<::tokio_threadpool::ThreadPool>,
        coll_cache: Arc<CollectionCache>,
    ) -> Self {
        let inner = SpannerDbInner {
            conn,
            thread_pool,
            session: RefCell::new(Default::default()),
        };
        SpannerDb {
            inner: Arc::new(inner),
            coll_cache,
        }
    }

    pub(super) fn get_collection_id(&self, name: &str) -> Result<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        let result = self
            .sql(
                "SELECT collectionid
                   FROM collections
                  WHERE name = @name",
            )?
            .params(params! {"name" => name.to_string()})
            .execute(&self.conn)?
            .one_or_none()?
            .ok_or(DbErrorKind::CollectionNotFound)?;
        let id = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        self.coll_cache.put(id, name.to_owned())?;
        Ok(id)
    }

    pub(super) fn create_collection(&self, name: &str) -> Result<i32> {
        // This should always run within a r/w transaction, so that: "If a
        // transaction successfully commits, then no other writer modified the
        // data that was read in the transaction after it was read."
        if cfg!(not(any(test, feature = "db_test"))) && !self.in_write_transaction() {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }
        let result = self
            .sql(
                "SELECT COALESCE(MAX(collectionid), 1)
                   FROM collections",
            )?
            .execute(&self.conn)?
            .one()?;
        let max = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        let id = FIRST_CUSTOM_COLLECTION_ID.max(max + 1);

        self.sql(
            "INSERT INTO collections (collectionid, name)
             VALUES (@collectionid, @name)",
        )?
        .params(params! {
            "name" => name.to_string(),
            "collectionid" => cmp::max(id, 100).to_string(),
        })
        .execute(&self.conn)?;
        self.coll_cache.put(id, name.to_owned())?;
        Ok(id)
    }

    fn get_or_create_collection_id(&self, name: &str) -> Result<i32> {
        self.get_collection_id(name).or_else(|e| match e.kind() {
            DbErrorKind::CollectionNotFound => self.create_collection(name),
            _ => Err(e),
        })
    }

    pub fn lock_for_read_sync(&self, params: params::LockCollection) -> Result<()> {
        // Begin a transaction
        self.begin(false)?;

        let user_id = params.user_id.legacy_id as u32;
        let collection_id =
            self.get_collection_id(&params.collection)
                .or_else(|e| match e.kind() {
                    // If the collection doesn't exist, we still want to start a
                    // transaction so it will continue to not exist.
                    DbErrorKind::CollectionNotFound => Ok(0),
                    _ => Err(e),
                })?;
        // If we already have a read or write lock then it's safe to
        // use it as-is.
        if self
            .inner
            .session
            .borrow()
            .coll_locks
            .get(&(user_id, collection_id))
            .is_some()
        {
            return Ok(());
        }

        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Read);

        Ok(())
    }

    pub fn lock_for_write_sync(&self, params: params::LockCollection) -> Result<()> {
        // Begin a transaction
        self.begin(true)?;

        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_or_create_collection_id(&params.collection)?;
        if let Some(CollectionLock::Read) = self
            .inner
            .session
            .borrow()
            .coll_locks
            .get(&(user_id, collection_id))
        {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }

        let result = self
            .sql(
                "SELECT CURRENT_TIMESTAMP(), last_modified
                   FROM user_collections
                  WHERE userid = @userid
                    AND collection = @collectionid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?;

        let timestamp = if let Some(result) = result {
            let modified = SyncTimestamp::from_rfc3339(result[1].get_string_value())?;
            self.session
                .borrow_mut()
                .coll_modified_cache
                .insert((user_id, collection_id), modified);
            SyncTimestamp::from_rfc3339(result[0].get_string_value())?
        } else {
            let result = self
                .sql("SELECT CURRENT_TIMESTAMP()")?
                .execute(&self.conn)?
                .one()?;
            SyncTimestamp::from_rfc3339(result[0].get_string_value())?
        };
        self.set_timestamp(timestamp);

        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Write);

        Ok(())
    }

    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = Some(timestamp);
    }

    #[cfg(feature = "google_grpc")]
    pub(super) fn begin(&self, for_write: bool) -> Result<()> {
        let spanner = &self.conn;
        let mut options = googleapis_raw::spanner::v1::transaction::TransactionOptions::new();
        if for_write {
            options.set_read_write(
                googleapis_raw::spanner::v1::transaction::TransactionOptions_ReadWrite::new(),
            );
            self.session.borrow_mut().in_write_transaction = true;
        } else {
            options.set_read_only(
                googleapis_raw::spanner::v1::transaction::TransactionOptions_ReadOnly::new(),
            );
        }
        let mut req = googleapis_raw::spanner::v1::spanner::BeginTransactionRequest::new();
        req.set_session(spanner.session.get_name().to_owned());
        req.set_options(options);
        let mut transaction = spanner.client.begin_transaction(&req)?;

        let mut ts = googleapis_raw::spanner::v1::transaction::TransactionSelector::new();
        ts.set_id(transaction.take_id());
        self.session.borrow_mut().transaction = Some(ts);
        Ok(())
    }

    #[cfg(not(feature = "google_grpc"))]
    pub(super) fn begin(&self, for_write: bool) -> Result<()> {
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut options = TransactionOptions::default();
        if for_write {
            options.read_write = Some(ReadWrite::default());
            self.session.borrow_mut().in_write_transaction = true;
        } else {
            options.read_only = Some(ReadOnly::default());
        }
        let req = BeginTransactionRequest {
            options: Some(options),
        };
        let (_, transaction) = spanner
            .hub
            .projects()
            .instances_databases_sessions_begin_transaction(req, session)
            .doit()?;
        self.session.borrow_mut().transaction = Some(google_spanner1::TransactionSelector {
            id: transaction.id,
            ..Default::default()
        });
        Ok(())
    }

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    fn get_transaction(&self) -> Result<Option<TransactionSelector>> {
        Ok(if self.session.borrow().transaction.is_some() {
            self.session.borrow().transaction.clone()
        } else {
            self.begin(true)?;
            self.session.borrow().transaction.clone()
        })
    }

    #[cfg(feature = "google_grpc")]
    fn sql_request(
        &self,
        sql: &str,
    ) -> Result<googleapis_raw::spanner::v1::spanner::ExecuteSqlRequest> {
        let mut sqlr = googleapis_raw::spanner::v1::spanner::ExecuteSqlRequest::new();
        sqlr.set_sql(sql.to_owned());
        if let Some(transaction) = self.get_transaction()? {
            sqlr.set_transaction(transaction);
            let mut session = self.session.borrow_mut();
            sqlr.seqno = session
                .execute_sql_count
                .try_into()
                .map_err(|_| DbError::internal("seqno overflow"))?;
            session.execute_sql_count += 1;
        }
        Ok(sqlr)
    }

    #[cfg(not(feature = "google_grpc"))]
    fn sql_request(&self, sql: &str) -> Result<ExecuteSqlRequest> {
        let mut sqlr = ExecuteSqlRequest::default();
        sqlr.sql = Some(sql.to_owned());
        let transaction = self.get_transaction()?;
        if transaction.is_some() {
            sqlr.transaction = transaction;
            let mut session = self.session.borrow_mut();
            sqlr.seqno = Some(session.execute_sql_count.to_string());
            session.execute_sql_count += 1;
        }
        Ok(sqlr)
    }

    pub fn sql(&self, sql: &str) -> Result<ExecuteSqlRequestBuilder> {
        Ok(ExecuteSqlRequestBuilder::new(self.sql_request(sql)?))
    }

    fn in_write_transaction(&self) -> bool {
        self.session.borrow().in_write_transaction
    }

    #[cfg(feature = "google_grpc")]
    pub fn commit_sync(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        let spanner = &self.conn;

        if cfg!(any(test, feature = "db_test")) && spanner.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let mut req = googleapis_raw::spanner::v1::spanner::CommitRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner.client.commit(&req)?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit"))?
        }
    }

    #[cfg(not(feature = "google_grpc"))]
    pub fn commit_sync(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        let spanner = &self.conn;

        if cfg!(any(test, feature = "db_test")) && spanner.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let session = spanner.session.name.as_ref().unwrap();
            spanner
                .hub
                .projects()
                .instances_databases_sessions_commit(
                    CommitRequest {
                        transaction_id: transaction.id,
                        ..Default::default()
                    },
                    session,
                )
                .doit()?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit"))?
        }
    }

    #[cfg(feature = "google_grpc")]
    pub fn rollback_sync(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let spanner = &self.conn;
            let mut req = googleapis_raw::spanner::v1::spanner::RollbackRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner.client.rollback(&req)?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback"))?
        }
    }

    #[cfg(not(feature = "google_grpc"))]
    pub fn rollback_sync(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let spanner = &self.conn;
            let session = spanner.session.name.as_ref().unwrap();
            spanner
                .hub
                .projects()
                .instances_databases_sessions_rollback(
                    RollbackRequest {
                        transaction_id: transaction.id,
                    },
                    session,
                )
                .doit()?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback"))?
        }
    }

    pub fn get_collection_timestamp_sync(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        dbg!("!!QQQ get_collection_timestamp_sync", &params.collection);

        let collection_id = self.get_collection_id(&params.collection)?;
        if let Some(modified) = self
            .session
            .borrow()
            .coll_modified_cache
            .get(&(user_id, collection_id))
        {
            return Ok(*modified);
        }

        let result = self
            .sql(
                "SELECT last_modified
                   FROM user_collections
                  WHERE userid = @userid
                    AND collection = @collectionid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?
            .ok_or_else(|| DbErrorKind::CollectionNotFound)?;
        let modified = SyncTimestamp::from_rfc3339(&result[0].get_string_value())?;
        Ok(modified)
    }

    pub fn get_collection_timestamps_sync(
        &self,
        user_id: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps> {
        let user_id = user_id.legacy_id as u32;
        let modifieds = self
            .sql(
                "SELECT collection, last_modified
                   FROM user_collections
                  WHERE userid = @userid",
            )?
            .params(params! {"userid" => user_id.to_string()})
            .execute(&self.conn)?
            .map(|row| {
                let collection_id = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                let ts = SyncTimestamp::from_rfc3339(&row[1].get_string_value())?;
                Ok((collection_id, ts))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        self.map_collection_names(modifieds)
    }

    fn map_collection_names<T>(&self, by_id: HashMap<i32, T>) -> Result<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys())?;
        by_id
            .into_iter()
            .filter(|id| id.0 > 0) // ignore any tombstones (they're alive again)
            .map(|(id, value)| {
                names
                    .remove(&id)
                    .map(|name| (name, value))
                    .ok_or_else(|| DbError::internal("load_collection_names get"))
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
            let mut params = HashMap::new();
            params.insert(
                "ids".to_owned(),
                as_list_value(uncached.into_iter().map(|id| id.to_string())),
            );
            let result = self
                .sql(
                    "SELECT collectionid, name
                       FROM collections
                      WHERE collectionid IN UNNEST(@ids)",
                )?
                .params(params)
                .execute(&self.conn)?;
            for row in result {
                let id = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                let name = row[1].get_string_value().to_owned();
                names.insert(id, name.clone());
                self.coll_cache.put(id, name)?;
            }
        }

        Ok(names)
    }

    pub fn get_collection_counts_sync(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts> {
        let user_id = user_id.legacy_id as u32;
        let counts = self
            .sql(
                "SELECT collection, COUNT(collection)
                   FROM bso
                  WHERE userid = @userid
                    AND ttl > CURRENT_TIMESTAMP()
                  GROUP BY collection",
            )?
            .params(params! {"userid" => user_id.to_string()})
            .execute(&self.conn)?
            .map(|row| {
                let collection = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                let count = row[1]
                    .get_string_value()
                    .parse::<i64>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                Ok((collection, count))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        self.map_collection_names(counts)
    }

    pub fn get_collection_usage_sync(
        &self,
        user_id: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage> {
        let user_id = user_id.legacy_id as u32;
        let usages = self
            .sql(
                "SELECT collection, SUM(LENGTH(payload))
                   FROM bso
                  WHERE userid = @userid
                    AND ttl > CURRENT_TIMESTAMP()
                  GROUP BY collection",
            )?
            .params(params! {"userid" => user_id.to_string()})
            .execute(&self.conn)?
            .map(|row| {
                let collection = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                let usage = row[1]
                    .get_string_value()
                    .parse::<i64>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                Ok((collection, usage))
            })
            .collect::<Result<HashMap<_, _>>>()?;
        self.map_collection_names(usages)
    }

    pub fn get_storage_timestamp_sync(
        &self,
        user_id: params::GetStorageTimestamp,
    ) -> Result<SyncTimestamp> {
        let user_id = user_id.legacy_id as u32;
        let ts0 = "0001-01-01T00:00:00Z";
        let result = self
            .sql(&format!(
                "SELECT COALESCE(MAX(last_modified), TIMESTAMP '{}')
                   FROM user_collections
                  WHERE userid = @userid",
                ts0
            ))?
            .params(params! {"userid" => user_id.to_string()})
            .execute(&self.conn)?
            .one_or_none()?;
        if let Some(result) = result {
            let val = result[0].get_string_value();
            // XXX: detect not max last_modified found via ts0 to workaround
            // google-apis-rs barfing on a null result
            if val == ts0 {
                SyncTimestamp::from_i64(0)
            } else {
                SyncTimestamp::from_rfc3339(val)
            }
        } else {
            SyncTimestamp::from_i64(0)
        }
    }

    pub fn get_storage_usage_sync(
        &self,
        user_id: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage> {
        let user_id = user_id.legacy_id as u32;
        let result = self
            .sql(
                "SELECT SUM(LENGTH(payload))
                   FROM bso
                  WHERE userid = @userid
                    AND ttl > CURRENT_TIMESTAMP()
                  GROUP BY userid",
            )?
            .params(params! {"userid" => user_id.to_string()})
            .execute(&self.conn)?
            .one_or_none()?;
        if let Some(result) = result {
            let usage = result[0]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            Ok(usage as u64)
        } else {
            Ok(0)
        }
    }

    fn erect_tombstone(&self, user_id: u32) -> Result<()> {
        // Delete the old tombstone (if it exists)
        self.sql(
            "DELETE FROM user_collections
              WHERE userid = @userid
                AND collection = @collection",
        )?
        .params(params! {
            "userid" => user_id.to_string(),
            "collection" => TOMBSTONE.to_string(),
        })
        .param_types(param_types! {
            "collection" => SpannerType::Int64,
        })
        .execute(&self.conn)?;

        self.sql(
            "INSERT INTO user_collections (userid, collection, last_modified)
             VALUES (@userid, @collection, @modified)",
        )?
        .params(params! {
            "userid" => user_id.to_string(),
            "collection" => TOMBSTONE.to_string(),
            "modified" => self.timestamp()?.as_rfc3339()?
        })
        .param_types(param_types! {
            "modified" => SpannerType::Timestamp,
            "collection" => SpannerType::Int64,
        })
        .execute(&self.conn)?;
        Ok(())
    }

    pub fn delete_storage_sync(&self, user_id: params::DeleteStorage) -> Result<()> {
        let user_id = user_id.legacy_id as u32;
        self.sql(
            "DELETE FROM user_collections
              WHERE userid = @userid",
        )?
        .params(params! {"userid" => user_id.to_string()})
        .execute(&self.conn)?;

        self.sql(
            "DELETE FROM bso
              WHERE userid = @userid",
        )?
        .params(params! {"userid" => user_id.to_string()})
        .execute(&self.conn)?;
        Ok(())
    }

    pub fn timestamp(&self) -> Result<SyncTimestamp> {
        self.session
            .borrow()
            .timestamp
            .ok_or_else(|| DbError::internal("CURRENT_TIMESTAMP() not read yet"))
    }

    pub fn delete_collection_sync(
        &self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection> {
        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_collection_id(&params.collection)?;

        self.sql(
            "DELETE FROM bso
              WHERE userid = @userid
                AND collection = @collectionid",
        )?
        .params(params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
        })
        .execute(&self.conn)?;

        self.sql(
            "DELETE FROM user_collections
              WHERE userid = @userid
                AND collection = @collectionid",
        )?
        .params(params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
        })
        .execute(&self.conn)?;

        self.erect_tombstone(user_id)?;
        self.get_storage_timestamp_sync(params.user_id)
    }

    pub(super) fn touch_collection(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> Result<SyncTimestamp> {
        // NOTE: Spanner supports upserts via its InsertOrUpdate mutation but
        // lacks a SQL equivalent. This call could be 1 InsertOrUpdate instead
        // of 2 queries but would require put/post_bsos to also use mutations.
        // Due to case of when no parent row exists (in user_collections)
        // before writing to bsos. Spanner requires a parent table row exist
        // before child table rows are written.
        // Mutations don't run in the same order as ExecuteSql calls, they are
        // buffered on the client side and only issued to Spanner in the final
        // transaction Commit.
        let timestamp = self.timestamp()?;
        let result = self
            .sql(
                "SELECT 1 as count
                   FROM user_collections
                  WHERE userid = @userid
                    AND collection = @collectionid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?;
        let exists = result.is_some();

        if exists {
            self.sql(
                "UPDATE user_collections
                    SET last_modified = @last_modified
                  WHERE userid = @userid
                    AND collection = @collectionid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "last_modified" => timestamp.as_rfc3339()?,
            })
            .param_types(param_types! {
                "last_modified" => SpannerType::Timestamp,
            })
            .execute(&self.conn)?;
            Ok(timestamp)
        } else {
            self.sql(
                "INSERT INTO user_collections (userid, collection, last_modified)
                 VALUES (@userid, @collectionid, @modified)",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "modified" => timestamp.as_rfc3339()?,
            })
            .param_types(param_types! {
                "modified" => SpannerType::Timestamp,
            })
            .execute(&self.conn)?;
            Ok(timestamp)
        }
    }

    pub fn delete_bso_sync(&self, params: params::DeleteBso) -> Result<results::DeleteBso> {
        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_collection_id(&params.collection)?;
        let touch = self.touch_collection(user_id as u32, collection_id)?;

        let result = self
            .sql(
                "DELETE FROM bso
                  WHERE userid = @userid
                    AND collection = @collectionid
                    AND id = @bsoid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => params.id.to_string(),
            })
            .execute(&self.conn)?;
        if result.affected_rows()? == 0 {
            Err(DbErrorKind::BsoNotFound)?
        } else {
            Ok(touch)
        }
    }

    pub fn delete_bsos_sync(&self, params: params::DeleteBsos) -> Result<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as u32;
        let collection_id = self.get_collection_id(&params.collection)?;

        let mut sqlparams = params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
        };
        sqlparams.insert("ids".to_owned(), as_list_value(params.ids.into_iter()));
        self.sql(
            "DELETE FROM bso
              WHERE userid = @userid
                AND collection = @collectionid
                AND id IN UNNEST(@ids)",
        )?
        .params(sqlparams)
        .execute(&self.conn)?;
        self.touch_collection(user_id, collection_id)
    }

    pub fn get_bsos_sync(&self, params: params::GetBsos) -> Result<results::GetBsos> {
        let user_id = params.user_id.legacy_id as i32;
        let collection_id = self.get_collection_id(&params.collection)?;
        let BsoQueryParams {
            newer,
            older,
            sort,
            limit,
            offset,
            ids,
            ..
        } = params.params;

        let mut query = "SELECT id, modified, payload, COALESCE(sortindex, NULL), ttl
                           FROM bso
                          WHERE userid = @userid
                            AND collection = @collectionid
                            AND ttl > CURRENT_TIMESTAMP()"
            .to_string();
        let mut sqlparams = params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
        };
        let mut sqltypes = HashMap::new();

        if let Some(older) = older {
            query = format!("{} AND modified < @older", query).to_string();
            sqlparams.insert("older".to_string(), as_value(older.as_rfc3339()?));
            sqltypes.insert("older".to_string(), SpannerType::Timestamp.into());
        }
        if let Some(newer) = newer {
            query = format!("{} AND modified > @newer", query).to_string();
            sqlparams.insert("newer".to_string(), as_value(newer.as_rfc3339()?));
            sqltypes.insert("newer".to_string(), SpannerType::Timestamp.into());
        }

        if !ids.is_empty() {
            query = format!("{} AND id IN UNNEST(@ids)", query).to_string();
            sqlparams.insert("ids".to_owned(), as_list_value(ids.into_iter()));
        }

        query = match sort {
            Sorting::Index => format!("{} ORDER BY sortindex DESC", query).to_string(),
            Sorting::Newest => format!("{} ORDER BY modified DESC", query).to_string(),
            Sorting::Oldest => format!("{} ORDER BY modified ASC", query).to_string(),
            _ => query,
        };

        let offset = offset.unwrap_or(0) as i64;
        if let Some(limit) = limit {
            // fetch an extra row to detect if there are more rows that match
            // the query conditions
            query = format!("{} LIMIT {}", query, i64::from(limit) + 1);
        } else if offset != 0 {
            // Special case no limit specified but still required for an
            // offset. Spanner doesn't accept a simpler limit of -1 (common in
            // most databases) so we specify a max value with offset subtracted
            // to avoid overflow errors (that only occur w/ a FORCE_INDEX=
            // directive) OutOfRange: 400 int64 overflow: <INT64_MAX> + offset
            query = format!("{} LIMIT {}", query, i64::max_value() - offset);
        };
        let limit = limit.map(i64::from).unwrap_or(-1);

        if offset != 0 {
            // XXX: copy over this optimization:
            // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
            query = format!("{} OFFSET {}", query, offset).to_string();
        }

        let result: Result<Vec<_>> = self
            .sql(&query)?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute(&self.conn)?
            .map(bso_from_row)
            .collect();
        let mut bsos = result?;

        // NOTE: when bsos.len() == 0, server-syncstorage (the Python impl)
        // makes an additional call to get_collection_timestamp to potentially
        // trigger CollectionNotFound errors.  However it ultimately eats the
        // CollectionNotFound and returns empty anyway, for the sake of
        // backwards compat.:
        // https://bugzilla.mozilla.org/show_bug.cgi?id=963332

        let next_offset = if limit >= 0 && bsos.len() > limit as usize {
            bsos.pop();
            Some(limit + offset)
        } else {
            None
        };

        Ok(results::GetBsos {
            items: bsos,
            offset: next_offset,
        })
    }

    pub fn get_bso_ids_sync(&self, params: params::GetBsos) -> Result<results::GetBsoIds> {
        // XXX: should be a more efficient select of only the id column
        let result = self.get_bsos_sync(params)?;
        Ok(results::GetBsoIds {
            items: result.items.into_iter().map(|bso| bso.id).collect(),
            offset: result.offset,
        })
    }

    pub fn get_bso_sync(&self, params: params::GetBso) -> Result<Option<results::GetBso>> {
        let user_id = params.user_id.legacy_id;
        let collection_id = self.get_collection_id(&params.collection)?;

        let result = self
            .sql(
                "SELECT id, modified, payload, COALESCE(sortindex, NULL), ttl
                   FROM bso
                  WHERE userid = @userid
                    AND collection = @collectionid
                    AND id = @bsoid
                    AND ttl > CURRENT_TIMESTAMP()",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => params.id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?;
        Ok(if let Some(row) = result {
            Some(bso_from_row(row)?)
        } else {
            None
        })
    }

    pub fn get_bso_timestamp_sync(&self, params: params::GetBsoTimestamp) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        dbg!("!!QQQ get_bso_timestamp_sync", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let result = self
            .sql(
                "SELECT modified
                   FROM bso
                  WHERE collection = @collectionid
                    AND userid = @userid
                    AND id = @bsoid
                    AND ttl > CURRENT_TIMESTAMP()",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => params.id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?;
        if let Some(result) = result {
            SyncTimestamp::from_rfc3339(&result[0].get_string_value())
        } else {
            SyncTimestamp::from_i64(0)
        }
    }

    pub fn put_bso_sync(&self, bso: params::PutBso) -> Result<results::PutBso> {
        let collection_id = self.get_or_create_collection_id(&bso.collection)?;
        let user_id: u64 = bso.user_id.legacy_id;
        let touch = self.touch_collection(user_id as u32, collection_id)?;
        let timestamp = self.timestamp()?;

        let result = self
            .sql(
                "SELECT 1 as count
                   FROM bso
                  WHERE userid = @userid
                    AND collection = @collectionid
                    AND id = @bsoid",
            )?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => bso.id.to_string(),
            })
            .execute(&self.conn)?
            .one_or_none()?;
        let exists = result.is_some();

        let mut sqlparams = params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
            "bsoid" => bso.id.to_string(),
        };
        let mut sqltypes = HashMap::new();

        let sql = if exists {
            // NOTE: the "ttl" column is more aptly named "expiry": our mysql
            // schema names it this. the current spanner schema prefers "ttl"
            // to more closely match the python code

            let mut q = "".to_string();
            let comma = |q: &String| if q.is_empty() { "" } else { ", " };

            q = format!(
                "{}{}",
                q,
                if let Some(sortindex) = bso.sortindex {
                    sqlparams.insert("sortindex".to_string(), as_value(sortindex.to_string()));
                    sqltypes.insert("sortindex".to_string(), SpannerType::Int64.into());

                    format!("{}{}", comma(&q), "sortindex = @sortindex")
                } else {
                    "".to_string()
                }
            )
            .to_string();
            q = format!(
                "{}{}",
                q,
                if let Some(ttl) = bso.ttl {
                    let expiry = timestamp.as_i64() + (i64::from(ttl) * 1000);
                    sqlparams.insert("expiry".to_string(), as_value(to_rfc3339(expiry)?));
                    sqltypes.insert("expiry".to_string(), SpannerType::Timestamp.into());
                    format!("{}{}", comma(&q), "ttl = @expiry")
                } else {
                    "".to_string()
                }
            )
            .to_string();
            q = format!(
                "{}{}",
                q,
                if bso.payload.is_some() || bso.sortindex.is_some() {
                    sqlparams.insert("modified".to_string(), as_value(timestamp.as_rfc3339()?));
                    sqltypes.insert("modified".to_string(), SpannerType::Timestamp.into());
                    format!("{}{}", comma(&q), "modified = @modified")
                } else {
                    "".to_string()
                }
            )
            .to_string();
            q = format!(
                "{}{}",
                q,
                if let Some(payload) = bso.payload {
                    sqlparams.insert("payload".to_string(), as_value(payload));
                    format!("{}{}", comma(&q), "payload = @payload")
                } else {
                    "".to_string()
                }
            )
            .to_string();

            if q.is_empty() {
                // Nothing to update
                return Ok(touch);
            }

            q = format!(
                "UPDATE bso SET {}{}",
                q,
                " WHERE userid = @userid
                    AND collection = @collectionid
                    AND id = @bsoid"
            );

            q
        } else {
            let use_sortindex = bso
                .sortindex
                .map(|sortindex| sortindex.to_string())
                .unwrap_or_else(|| "NULL".to_owned())
                != "NULL";
            let sql = if use_sortindex {
                "INSERT INTO bso (userid, collection, id, sortindex, payload, modified, ttl)
                 VALUES (@userid, @collectionid, @bsoid, @sortindex, @payload, @modified, @expiry)"
            } else {
                "INSERT INTO bso (userid, collection, id, payload, modified, ttl)
                 VALUES (@userid, @collectionid, @bsoid,  @payload, @modified, @expiry)"
            };

            if use_sortindex {
                // special handling for google_grpc (null)
                #[cfg(feature = "google_grpc")]
                let sortindex = bso
                    .sortindex
                    .map(|sortindex| as_value(sortindex.to_string()))
                    .unwrap_or_else(|| {
                        use protobuf::well_known_types::{NullValue, Value};
                        let mut value = Value::new();
                        value.set_null_value(NullValue::NULL_VALUE);
                        value
                    });

                #[cfg(not(feature = "google_grpc"))]
                let sortindex = bso
                    .sortindex
                    .map(|sortindex| sortindex.to_string())
                    .unwrap_or_else(|| "NULL".to_owned());

                sqlparams.insert("sortindex".to_string(), sortindex);
                sqltypes.insert("sortindex".to_string(), SpannerType::Int64.into());
            }
            sqlparams.insert(
                "payload".to_string(),
                as_value(bso.payload.unwrap_or_else(|| "".to_owned())),
            );
            let now_millis = timestamp.as_i64();
            let ttl = bso
                .ttl
                .map_or(DEFAULT_BSO_TTL, |ttl| ttl.try_into().unwrap())
                * 1000;
            let expirystring = to_rfc3339(now_millis + ttl)?;
            dbg!("!!!!! INSERT", &expirystring, timestamp, ttl);
            sqlparams.insert("expiry".to_string(), as_value(expirystring));
            sqltypes.insert("expiry".to_string(), SpannerType::Timestamp.into());

            sqlparams.insert("modified".to_string(), as_value(timestamp.as_rfc3339()?));
            sqltypes.insert("modified".to_string(), SpannerType::Timestamp.into());
            sql.to_owned()
        };

        self.sql(&sql)?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute(&self.conn)?;
        Ok(touch)
    }

    pub fn post_bsos_sync(&self, input: params::PostBsos) -> Result<results::PostBsos> {
        let collection_id = self.get_or_create_collection_id(&input.collection)?;
        let mut result = results::PostBsos {
            modified: self.timestamp()?,
            success: Default::default(),
            failed: input.failed,
        };

        for pbso in input.bsos {
            let id = pbso.id;
            self.put_bso_sync(params::PutBso {
                user_id: input.user_id.clone(),
                collection: input.collection.clone(),
                id: id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            })?;
            result.success.push(id);
        }
        self.touch_collection(input.user_id.legacy_id as u32, collection_id)?;
        Ok(result)
    }

    batch_db_method!(create_batch_sync, create, CreateBatch);
    batch_db_method!(validate_batch_sync, validate, ValidateBatch);
    batch_db_method!(append_to_batch_sync, append, AppendToBatch);
    batch_db_method!(commit_batch_sync, commit, CommitBatch);
    #[cfg(any(test, feature = "db_test"))]
    batch_db_method!(delete_batch_sync, delete, DeleteBatch);

    pub fn get_batch_sync(&self, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
        batch::get(&self, params)
    }
}

unsafe impl Send for SpannerDb {}

macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<$result> {
            let db = self.clone();
            Box::new(self.thread_pool.spawn_handle(lazy(move || {
                future::result(db.$sync_name(params).map_err(Into::into))
            })))
        }
    };
}

impl Db for SpannerDb {
    fn commit(&self) -> DbFuture<()> {
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(db.commit_sync().map_err(Into::into))
        })))
    }

    fn rollback(&self) -> DbFuture<()> {
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(db.rollback_sync().map_err(Into::into))
        })))
    }

    fn box_clone(&self) -> Box<dyn Db> {
        Box::new(self.clone())
    }

    sync_db_method!(lock_for_read, lock_for_read_sync, LockCollection);
    sync_db_method!(lock_for_write, lock_for_write_sync, LockCollection);
    sync_db_method!(
        get_collection_timestamp,
        get_collection_timestamp_sync,
        GetCollectionTimestamp
    );
    sync_db_method!(
        get_collection_timestamps,
        get_collection_timestamps_sync,
        GetCollectionTimestamps
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
    sync_db_method!(delete_storage, delete_storage_sync, DeleteStorage);
    sync_db_method!(delete_collection, delete_collection_sync, DeleteCollection);
    sync_db_method!(delete_bso, delete_bso_sync, DeleteBso);
    sync_db_method!(delete_bsos, delete_bsos_sync, DeleteBsos);
    sync_db_method!(get_bsos, get_bsos_sync, GetBsos);
    sync_db_method!(get_bso_ids, get_bso_ids_sync, GetBsoIds);
    sync_db_method!(get_bso, get_bso_sync, GetBso, Option<results::GetBso>);
    sync_db_method!(
        get_bso_timestamp,
        get_bso_timestamp_sync,
        GetBsoTimestamp,
        results::GetBsoTimestamp
    );
    sync_db_method!(put_bso, put_bso_sync, PutBso);
    sync_db_method!(post_bsos, post_bsos_sync, PostBsos);
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

    #[cfg(any(test, feature = "db_test"))]
    fn get_collection_id(&self, name: String) -> DbFuture<i32> {
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(db.get_collection_id(&name).map_err(Into::into))
        })))
    }

    #[cfg(any(test, feature = "db_test"))]
    fn create_collection(&self, name: String) -> DbFuture<i32> {
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(db.create_collection(&name).map_err(Into::into))
        })))
    }

    #[cfg(any(test, feature = "db_test"))]
    fn touch_collection(&self, param: params::TouchCollection) -> DbFuture<SyncTimestamp> {
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(
                db.touch_collection(param.user_id.legacy_id as u32, param.collection_id)
                    .map_err(Into::into),
            )
        })))
    }

    #[cfg(any(test, feature = "db_test"))]
    fn timestamp(&self) -> SyncTimestamp {
        self.timestamp()
            .expect("set_timestamp() not called yet for SpannerDb")
    }

    #[cfg(any(test, feature = "db_test"))]
    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        SpannerDb::set_timestamp(self, timestamp)
    }

    #[cfg(any(test, feature = "db_test"))]
    sync_db_method!(delete_batch, delete_batch_sync, DeleteBatch);

    #[cfg(any(test, feature = "db_test"))]
    fn clear_coll_cache(&self) {
        self.coll_cache.clear();
    }
}
