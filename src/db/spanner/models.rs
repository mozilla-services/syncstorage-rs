use futures::future;
use futures::lazy;

use chrono::{DateTime, SecondsFormat, TimeZone, Utc};

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

use crate::db::{
    error::{DbError, DbErrorKind},
    params, results,
    util::{to_rfc3339, SyncTimestamp},
    Db, DbFuture, Sorting,
};

use crate::web::extractors::BsoQueryParams;

use super::batch;

use google_spanner1::{
    BeginTransactionRequest, CommitRequest, ExecuteSqlRequest, ReadOnly, ReadWrite,
    RollbackRequest, TransactionOptions, TransactionSelector, Type,
};

#[derive(Debug)]
pub enum CollectionLock {
    Read,
    Write,
}

type Conn = PooledConnection<SpannerConnectionManager>;
pub type Result<T> = std::result::Result<T, DbError>;

/// The ttl to use for rows that are never supposed to expire (in seconds)
pub const DEFAULT_BSO_TTL: i64 = 2_100_000_000;

/// Per session Db metadata
#[derive(Debug, Default)]
struct SpannerDbSession {
    /// The "current time" on the server used for this session's operations
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (user_id, collection_id)
    coll_modified_cache: HashMap<(u32, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(u32, i32), CollectionLock>,
    transaction: Option<TransactionSelector>,
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
        let spanner = &self.conn;

        let mut sql = self.sql_request("SELECT collectionid FROM collections WHERE name = @name");
        let mut params = HashMap::new();
        params.insert("name".to_string(), name.to_string());
        sql.params = Some(params);

        let session = spanner.session.name.as_ref().unwrap();
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit()?;
        println!("ok {:?}", result.1);
        let rows = result.1.rows.ok_or(DbErrorKind::CollectionNotFound)?;
        let id = rows[0][0]
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        self.coll_cache.put(id, name.to_owned())?;
        Ok(id)
    }

    pub(super) fn create_collection(&self, name: &str) -> Result<i32> {
        // XXX: handle concurrent attempts at inserts
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();

        let sql = self.sql_request("SELECT MAX(collectionid) from collections");
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit()?;
        let id = if let Some(rows) = result.1.rows {
            let max = rows[0][0]
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            max + 1
        } else {
            // XXX: should never happen but defaulting to id of 1 is bad
            1
        };

        let mut sql = self.sql_request(
            "INSERT INTO collections (collectionid, name) VALUES (@collectionid, @name)",
        );
        let mut params = HashMap::new();
        params.insert("name".to_string(), name.to_string());
        params.insert("collectionid".to_string(), cmp::max(id, 100).to_string());
        sql.params = Some(params);

        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit()?;
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

        // Lock the db
        self.begin(false)?;
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT CURRENT_TIMESTAMP() as now, last_modified FROM user_collections WHERE userid=@userid AND collection=@collectionid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        params.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        if let Ok(results) = results {
            if let Some(rows) = results.1.rows {
                let modified = SyncTimestamp::from_rfc3339(&rows[0][0])?;
                self.session
                    .borrow_mut()
                    .coll_modified_cache
                    .insert((user_id, collection_id), modified);
            }
        }
        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Read);

        Ok(())
    }

    pub fn lock_for_write_sync(&self, params: params::LockCollection) -> Result<()> {
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

        // Lock the db
        self.begin(true)?;
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT CURRENT_TIMESTAMP() as now, last_modified FROM user_collections WHERE userid=@userid AND collection=@collectionid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        params.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        if let Ok(results) = results {
            if let Some(rows) = results.1.rows {
                let modified = SyncTimestamp::from_rfc3339(&rows[0][0])?;
                self.session
                    .borrow_mut()
                    .coll_modified_cache
                    .insert((user_id, collection_id), modified);
            }
        }
        self.session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Write);

        Ok(())
    }

    pub(super) fn begin(&self, for_write: bool) -> Result<()> {
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut options = TransactionOptions::default();
        if for_write {
            options.read_write = Some(ReadWrite::default());
        } else {
            options.read_only = Some(ReadOnly::default());
        }
        let req = BeginTransactionRequest {
            options: Some(options),
        };
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_begin_transaction(req, session)
            .doit();
        match result {
            Ok((response, transaction)) => {
                self.session.borrow_mut().transaction =
                    Some(google_spanner1::TransactionSelector {
                        id: transaction.id,
                        ..Default::default()
                    });
                Ok(())
            }
            Err(_e) => {
                // TODO Handle error
                Ok(())
            }
        }
    }

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    ///
    /// May return a test transaction (that's never committed) during tests.
    fn get_transaction(&self) -> Option<TransactionSelector> {
        let transaction = self.session.borrow().transaction.clone();
        #[cfg(any(test, feature = "db_test"))]
        let transaction = if self.conn.test_transaction.is_some() {
            self.conn.test_transaction.clone()
        } else {
            transaction
        };
        transaction
    }

    fn sql_request(&self, sql: &str) -> ExecuteSqlRequest {
        let mut sqlr = ExecuteSqlRequest::default();
        sqlr.sql = Some(sql.to_owned());
        sqlr.transaction = self.get_transaction();
        let mut session = self.session.borrow_mut();
        // XXX: include seqno if no transaction?
        sqlr.seqno = Some(session.execute_sql_count.to_string());
        session.execute_sql_count += 1;
        sqlr
    }

    pub fn commit_sync(&self) -> Result<()> {
        // XXX: maybe guard against test_transaction?
        if let Some(transaction) = self.get_transaction() {
            let spanner = &self.conn;
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
            Err(DbErrorKind::Internal("No transaction to commit".to_owned()))?
        }
    }

    pub fn rollback_sync(&self) -> Result<()> {
        if let Some(transaction) = self.get_transaction() {
            let spanner = &self.conn;
            let session = spanner.session.name.as_ref().unwrap();
            spanner
                .hub
                .projects()
                .instances_databases_sessions_rollback(
                    RollbackRequest {
                        transaction_id: transaction.id,
                        ..Default::default()
                    },
                    session,
                )
                .doit()?;
            Ok(())
        } else {
            Err(DbErrorKind::Internal(
                "No transaction to rollback".to_owned(),
            ))?
        }
    }

    pub fn get_collection_timestamp_sync(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        println!(
            "!!QQQ get_collection_timestamp_sync {:}",
            &params.collection
        );

        let collection_id = self.get_collection_id(&params.collection)?;
        if let Some(modified) = self
            .session
            .borrow()
            .coll_modified_cache
            .get(&(user_id, collection_id))
        {
            return Ok(*modified);
        }

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT last_modified FROM user_collections WHERE userid=@userid AND collection=@collectionid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        params.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let modified = SyncTimestamp::from_rfc3339(&rows[0][0])?;
                    Ok(modified)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn get_collection_timestamps_sync(
        &self,
        user_id: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request(
            "SELECT collection, last_modified FROM user_collections WHERE userid=@userid",
        );
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let mut timestamps = results::GetCollectionTimestamps::new();
                    rows.iter().for_each(|row| {
                        if let Ok(timestamp) = SyncTimestamp::from_rfc3339(&row[0]) {
                            timestamps.insert(row[0].clone(), timestamp);
                        }
                    });
                    Ok(timestamps)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    fn map_collection_names<T>(&self, by_id: HashMap<i32, T>) -> Result<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys())?;
        by_id
            .into_iter()
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
            // TODO only select names that are in uncached.
            let mut sql = self.sql_request("SELECT collectionid, name FROM collections");
            let spanner = &self.conn;
            let session = spanner.session.name.as_ref().unwrap();
            let results = spanner
                .hub
                .projects()
                .instances_databases_sessions_execute_sql(sql, session)
                .doit();
            match results {
                Ok(results) => match results.1.rows {
                    Some(rows) => {
                        rows.iter().for_each(|row| {
                            let id = row[0].parse::<i32>().unwrap();
                            let name = row[1].clone();
                            if uncached.contains(&id) {
                                names.insert(id, name.clone());
                                self.coll_cache.put(id, name.clone()).unwrap();
                            }
                        });
                    }
                    None => return Err(DbErrorKind::CollectionNotFound.into()),
                },
                // TODO Return the correct error
                Err(_e) => return Err(DbErrorKind::CollectionNotFound.into()),
            }
        }

        Ok(names)
    }

    pub fn get_collection_counts_sync(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT collection, COUNT(collection) FROM bso WHERE userid=@userid AND ttl > CURRENT_TIMESTAMP() GROUP BY collection");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let mut counts = HashMap::new();
                    rows.iter().for_each(|row| {
                        counts.insert(row[0].parse::<i32>().unwrap(), row[1].parse().unwrap());
                    });
                    self.map_collection_names(counts)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn get_collection_usage_sync(
        &self,
        user_id: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT collection, SUM(LENGTH(payload)) FROM bso WHERE userid=@userid AND ttl > CURRENT_TIMESTAMP() GROUP BY collection");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let mut usage = results::GetCollectionUsage::new();
                    rows.iter().for_each(|row| {
                        usage.insert(row[0].clone(), row[1].parse().unwrap());
                    });
                    Ok(usage)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn get_storage_timestamp_sync(
        &self,
        user_id: params::GetStorageTimestamp,
    ) -> Result<SyncTimestamp> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self
            .sql_request("SELECT MAX(last_modified) FROM user_collections WHERE userid=@userid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let modified = SyncTimestamp::from_rfc3339(&rows[0][0])?;
                    Ok(modified)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn get_storage_usage_sync(
        &self,
        user_id: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT SUM(LENGTH(payload)) FROM bso WHERE userid=@userid AND ttl > CURRENT_TIMESTAMP() GROUP BY userid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let usage = rows[0][0].parse().unwrap();
                    Ok(usage)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn delete_storage_sync(&self, user_id: params::DeleteStorage) -> Result<()> {
        let user_id = user_id.legacy_id as u32;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("DELETE FROM user_collections WHERE userid=@userid");
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(_results) => Ok(()),
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn timestamp(&self) -> SyncTimestamp {
        self.session.borrow().timestamp
    }

    pub fn delete_collection_sync(
        &self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection> {
        let user_id = params.user_id.legacy_id as u32;
        println!("!!QQQ delete_collection_sync {:}", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql =
            self.sql_request("DELETE FROM bso WHERE userid=@userid AND collection=@collectionid");
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(sqlparams);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(_results) => {}
            // TODO Return the correct error
            Err(_e) => return Err(DbErrorKind::CollectionNotFound.into()),
        }
        let mut sql = self.sql_request(
            "DELETE FROM user_collections WHERE userid=@userid AND collection=@collectionid",
        );
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(sqlparams);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(_results) => {}
            // TODO Return the correct error
            Err(_e) => return Err(DbErrorKind::CollectionNotFound.into()),
        }
        self.get_storage_timestamp_sync(params.user_id)
    }

    pub(super) fn touch_collection(
        &self,
        user_id: u32,
        collection_id: i32,
    ) -> Result<SyncTimestamp> {
        // XXX: We should be able to use Spanner's insert_or_update here (unlike w/ put_bsos)
        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT 1 as count FROM user_collections WHERE userid = @userid AND collection = @collectionid;");
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sql.params = Some(sqlparams);
        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        let exists = match results {
            Ok(results) => results.1.rows.is_some(),
            _ => return Err(DbErrorKind::CollectionNotFound.into()),
        };

        if exists {
            let mut sql = self.sql_request("UPDATE user_collections SET last_modified=@last_modified WHERE userid=@userid AND collection=@collectionid");
            let mut sqlparams = HashMap::new();
            let mut sqltypes = HashMap::new();
            sqlparams.insert("userid".to_string(), user_id.to_string());
            sqlparams.insert("collectionid".to_string(), collection_id.to_string());
            let timestamp = self.timestamp().as_i64();
            let modifiedstring = to_rfc3339(timestamp)?;
            sqlparams.insert("last_modified".to_string(), modifiedstring);
            sqltypes.insert(
                "last_modified".to_string(),
                Type {
                    array_element_type: None,
                    code: Some("TIMESTAMP".to_string()),
                    struct_type: None,
                },
            );
            sql.params = Some(sqlparams);
            sql.param_types = Some(sqltypes);

            let results = spanner
                .hub
                .projects()
                .instances_databases_sessions_execute_sql(sql, session)
                .doit();
            match results {
                Ok(_results) => Ok(self.timestamp()),
                // TODO Return the correct error
                Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
            }
        } else {
            let mut sql = self.sql_request("INSERT INTO user_collections (userid, collection, last_modified) VALUES (@userid, @collectionid, @modified);");
            let mut sqlparams = HashMap::new();
            let mut sqltypes = HashMap::new();
            sqlparams.insert("userid".to_string(), user_id.to_string());
            sqlparams.insert("collectionid".to_string(), collection_id.to_string());
            let timestamp = self.timestamp().as_i64();
            let modifiedstring = to_rfc3339(timestamp)?;
            sqlparams.insert("modified".to_string(), modifiedstring);
            sqltypes.insert(
                "modified".to_string(),
                Type {
                    array_element_type: None,
                    code: Some("TIMESTAMP".to_string()),
                    struct_type: None,
                },
            );
            sql.params = Some(sqlparams);
            sql.param_types = Some(sqltypes);

            spanner
                .hub
                .projects()
                .instances_databases_sessions_execute_sql(sql, session)
                .doit()?;
            Ok(self.timestamp())
        }
    }

    pub fn delete_bso_sync(&self, params: params::DeleteBso) -> Result<results::DeleteBso> {
        let user_id = params.user_id.legacy_id as u32;
        println!("!!QQQ delete_bso_sync {:}", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request(
            "DELETE FROM bso WHERE userid=@userid AND collection=@collectionid AND id=@bsoid",
        );
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sqlparams.insert("bsoid".to_string(), params.id.to_string());
        sql.params = Some(sqlparams);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(_results) => self.touch_collection(user_id, collection_id),
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn delete_bsos_sync(&self, params: params::DeleteBsos) -> Result<results::DeleteBsos> {
        let user_id = params.user_id.legacy_id as u32;
        println!("!!QQQ delete_bsos_sync {:}", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let mut deleted = 0;
        // TODO figure out how spanner specifies an "IN" query
        params.ids.iter().for_each(|id| {
            let spanner = &self.conn;
            let session = spanner.session.name.as_ref().unwrap();
            let mut sql = self.sql_request(
                "SELECT 1 FROM bso WHERE userid=@userid AND collection=@collectionid AND id=@bsoid",
            );
            let mut sqlparams = HashMap::new();
            sqlparams.insert("userid".to_string(), user_id.to_string());
            sqlparams.insert("collectionid".to_string(), collection_id.to_string());
            sqlparams.insert("bsoid".to_string(), id.to_string());
            sql.params = Some(sqlparams);

            let results = spanner
                .hub
                .projects()
                .instances_databases_sessions_execute_sql(sql, session)
                .doit();
            if !results.is_err() {
                deleted += 1;
                self.delete_bso_sync(params::DeleteBso {
                    user_id: params.user_id.clone(),
                    collection: params.collection.clone(),
                    id: id.to_string(),
                })
                .unwrap();
            }
        });
        if deleted > 0 {
            self.touch_collection(user_id, collection_id)
        } else {
            Err(DbErrorKind::CollectionNotFound.into())
        }
    }

    pub fn get_bsos_sync(&self, params: params::GetBsos) -> Result<results::GetBsos> {
        let user_id = params.user_id.legacy_id as i32;
        println!("!!QQQ get_bsos_sync {:}", &params.collection);
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

        let mut query = "SELECT (id, modified, payload, sortindex, expiry) FROM bso WHERE user_id = @userid AND collection_id = @collectionid AND expiry > @timestamp".to_string();
        let mut sqlparams = HashMap::new();
        let mut sqltypes = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        let timestamp = self.timestamp().as_i64();
        let modifiedstring = to_rfc3339(timestamp)?;
        sqlparams.insert("timestamp".to_string(), modifiedstring);
        sqltypes.insert(
            "timestamp".to_string(),
            Type {
                array_element_type: None,
                code: Some("TIMESTAMP".to_string()),
                struct_type: None,
            },
        );

        if let Some(older) = older {
            query = format!("{} AND modified < @older", query).to_string();
            sqlparams.insert("older".to_string(), older.as_i64().to_string());
        }
        if let Some(newer) = newer {
            query = format!("{} AND modified > @newer", query).to_string();
            sqlparams.insert("newer".to_string(), newer.as_i64().to_string());
        }

        let idlen = ids.len();
        if !ids.is_empty() {
            // TODO use UNNEST and pass a vec later
            let mut i = 0;
            query = format!("{} AND id IN (", query).to_string();
            while i < idlen {
                if i == 0 {
                    query = format!("{}arg{}", query, i.to_string()).to_string();
                } else {
                    query = format!("{}, arg{}", query, i.to_string()).to_string();
                }
                sqlparams.insert(
                    format!("arg{}", i.to_string()).to_string(),
                    ids[i].to_string(),
                );
                i += 1;
            }
            query = format!("{})", query).to_string();
        }

        query = match sort {
            Sorting::Index => format!("{} SORT BY sortindex DESCENDING", query).to_string(),
            Sorting::Newest => format!("{} SORT BY modified DESCENDING", query).to_string(),
            Sorting::Oldest => format!("{} SORT BY modified ASCENDING", query).to_string(),
            _ => query,
        };

        let limit = limit.map(i64::from).unwrap_or(-1);
        // fetch an extra row to detect if there are more rows that
        // match the query conditions

        //query = query.limit(if limit >= 0 { limit + 1 } else { limit });
        if limit >= 0 {
            query = format!("{} LIMIT {}", query, limit + 1).to_string();
        } else {
            query = format!("{} LIMIT {}", query, limit).to_string();
        }

        let offset = offset.unwrap_or(0) as i64;
        if offset != 0 {
            // XXX: copy over this optimization:
            // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
            query = format!("{} OFFSET {}", query, offset).to_string();
        }

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request(&query);
        sql.params = Some(sqlparams);
        sql.param_types = Some(sqltypes);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        let mut bsos = match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let mut vec = Vec::new();
                    rows.iter().for_each(|row| {
                        vec.push(results::GetBso {
                            id: row[0].parse().unwrap(),
                            modified: SyncTimestamp::from_rfc3339(&row[1]).unwrap(),
                            payload: row[2].parse().unwrap(),
                            sortindex: Some(row[3].parse().unwrap()),
                            expiry: row[4].parse().unwrap(),
                        });
                    });
                    vec
                }
                None => Vec::new(),
            },
            // TODO Return the correct error
            Err(_e) => Vec::new(),
        };

        // XXX: an additional get_collection_timestamp is done here in
        // python to trigger potential CollectionNotFoundErrors
        //if bsos.len() == 0 {
        //}

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
        println!("!!QQQ get_bso_sync {:}", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT id, modified, payload, coalesce(sortindex, 0), ttl FROM bso WHERE userid=@userid AND collection=@collectionid AND id=@bsoid AND ttl > @timestamp");
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sqlparams.insert("bsoid".to_string(), params.id.to_string());
        let timestamp = self.timestamp().as_i64();
        let modifiedstring = to_rfc3339(timestamp)?;
        sqlparams.insert("timestamp".to_string(), modifiedstring);
        let mut sqltypes = HashMap::new();
        sqltypes.insert(
            "timestamp".to_string(),
            Type {
                array_element_type: None,
                code: Some("TIMESTAMP".to_string()),
                struct_type: None,
            },
        );
        sql.params = Some(sqlparams);
        sql.param_types = Some(sqltypes);

        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit()?;
        eprintln!("RRRRRRRRRR {:#?}", result);
        Ok(if let Some(rows) = result.1.rows {
            let modified = SyncTimestamp::from_rfc3339(&rows[0][1])?;
            let expiry_dt = DateTime::parse_from_rfc3339(&rows[0][4]).map_err(|e| {
                DbErrorKind::Integrity(format!("Invalid TIMESTAMP {}", e.to_string()))
            })?;
            // XXX: expiry is i64?
            let expiry = expiry_dt.timestamp_millis();
            eprintln!("!!!! GET expiry {} ({}) {}", &rows[0][4], expiry_dt, expiry);
            Some(results::GetBso {
                id: rows[0][0].clone(),
                modified,
                payload: rows[0][2].clone(),
                sortindex: Some(rows[0][3].parse().unwrap()),
                expiry,
            })
        } else {
            None
        })
        /*
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let modified = SyncTimestamp::from_rfc3339(&rows[0][1])?;
                    let expiry_dt = DateTime::parse_from_rfc3339(&rows[0][4]).map_err(|e| DbErrorKind::Integrity(format!("Invalid TIMESTAMP {}", e.to_string())))?;
                    // XXX: expiry is i64?
                    let expiry = expiry_dt.timestamp_millis();
                    eprintln!("!!!! GET expiry {} ({}) {}", &rows[0][4], expiry_dt, expiry);
                    Ok(Some(results::GetBso {
                        id: rows[0][0].clone(),
                        modified,
                        payload: rows[0][2].clone(),
                        sortindex: Some(rows[0][3].parse().unwrap()),
                        expiry,
                    }))
                }
                ,
                None => Ok(None),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
        */
    }

    pub fn get_bso_timestamp_sync(&self, params: params::GetBsoTimestamp) -> Result<SyncTimestamp> {
        let user_id = params.user_id.legacy_id as u32;
        println!("!!QQQ get_bso_timestamp_sync {:}", &params.collection);
        let collection_id = self.get_collection_id(&params.collection)?;

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT modified FROM bso WHERE collection=@collectionid AND userid=@userid AND id=@bsoid AND ttl>@ttl");
        let mut sqlparams = HashMap::new();
        let mut sqltypes = HashMap::new();
        let timestamp = self.timestamp().as_i64();
        let expirystring = to_rfc3339(timestamp)?;

        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sqlparams.insert("bsoid".to_string(), params.id.to_string());
        sqlparams.insert("ttl".to_string(), expirystring);
        sqltypes.insert(
            "ttl".to_string(),
            Type {
                array_element_type: None,
                code: Some("TIMESTAMP".to_string()),
                struct_type: None,
            },
        );

        sql.params = Some(sqlparams);
        sql.param_types = Some(sqltypes);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match results {
            Ok(results) => match results.1.rows {
                Some(rows) => {
                    let modified = SyncTimestamp::from_rfc3339(&rows[0][0])?;
                    Ok(modified)
                }
                None => Err(DbErrorKind::CollectionNotFound.into()),
            },
            // TODO Return the correct error
            Err(_e) => Err(DbErrorKind::CollectionNotFound.into()),
        }
    }

    pub fn put_bso_sync(&self, bso: params::PutBso) -> Result<results::PutBso> {
        let collection_id = self.get_or_create_collection_id(&bso.collection)?;
        let user_id: u64 = bso.user_id.legacy_id;
        let touch = self.touch_collection(user_id as u32, collection_id)?;
        let timestamp = self.timestamp().as_i64();

        let spanner = &self.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = self.sql_request("SELECT 1 as count FROM bso WHERE userid = @userid AND collection = @collectionid AND id = @bsoid");
        let mut sqlparams = HashMap::new();
        sqlparams.insert("userid".to_string(), user_id.to_string());
        sqlparams.insert("collectionid".to_string(), collection_id.to_string());
        sqlparams.insert("bsoid".to_string(), bso.id.to_string());
        sql.params = Some(sqlparams);
        #[derive(Default)]
        pub struct Dlg;

        impl google_spanner1::Delegate for Dlg {
            fn http_failure(
                &mut self,
                r: &hyper::client::Response,
                a: Option<google_spanner1::JsonServerError>,
                b: Option<google_spanner1::ServerError>,
            ) -> yup_oauth2::Retry {
                if let Some(a) = a {
                    eprintln!(
                        "DDDDDDDDDDDDDDDDDDDD1 |{}| |{:#?}|",
                        a.error, a.error_description
                    );
                }
                eprintln!(
                    "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD {:#?}",
                    b
                );
                yup_oauth2::Retry::Abort
            }
        }
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .delegate(&mut Dlg {})
            .doit()?;
        // XXX: should we rows.len() == 1?
        let exists = result.1.rows.is_some();

        let sql = if exists {
            // XXX: the "ttl" column is more aptly named "expiry": our mysql
            // schema names it this. the current spanner schema prefers "ttl"
            // to more closely match the python code

            let mut sqlparams = HashMap::new();
            let mut sqltypes = HashMap::new();

            let mut q = "".to_string();
            let comma = |q: &String| if q.is_empty() { "" } else { ", " };

            q = format!(
                "{}{}",
                q,
                if let Some(sortindex) = bso.sortindex {
                    sqlparams.insert("sortindex".to_string(), sortindex.to_string());
                    sqltypes.insert(
                        "sortindex".to_string(),
                        Type {
                            code: Some("INT64".to_string()),
                            ..Default::default()
                        },
                    );

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
                    let expiry = timestamp + (i64::from(ttl) * 1000);
                    sqlparams.insert("expiry".to_string(), to_rfc3339(expiry)?);
                    sqltypes.insert(
                        "expiry".to_string(),
                        Type {
                            code: Some("TIMESTAMP".to_string()),
                            ..Default::default()
                        },
                    );
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
                    sqlparams.insert("modified".to_string(), self.timestamp().as_rfc3339()?);
                    sqltypes.insert(
                        "modified".to_string(),
                        Type {
                            code: Some("TIMESTAMP".to_string()),
                            ..Default::default()
                        },
                    );
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
                    sqlparams.insert("payload".to_string(), payload);
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
                q, " WHERE userid = @userid AND collection = @collectionid AND id = @bsoid"
            );
            sqlparams.insert("userid".to_string(), user_id.to_string());
            sqlparams.insert("collectionid".to_string(), collection_id.to_string());
            sqlparams.insert("bsoid".to_string(), bso.id.to_string());

            let mut sql = self.sql_request(&q);
            sql.params = Some(sqlparams);
            sql.param_types = Some(sqltypes);
            sql
        } else {
            let use_sortindex = bso
                .sortindex
                .map(|sortindex| sortindex.to_string())
                .unwrap_or_else(|| "NULL".to_owned())
                != "NULL";
            let mut sql = if use_sortindex {
                self.sql_request("INSERT INTO bso (userid, collection, id, sortindex, payload, modified, ttl) VALUES (@userid, @collectionid, @bsoid, @sortindex, @payload, @modified, @expiry)")
            } else {
                self.sql_request("INSERT INTO bso (userid, collection, id, payload, modified, ttl) VALUES (@userid, @collectionid, @bsoid,  @payload, @modified, @expiry)")
            };
            let mut sqlparams = HashMap::new();
            let mut sqltypes = HashMap::new();
            sqlparams.insert("userid".to_string(), user_id.to_string());
            sqlparams.insert("collectionid".to_string(), collection_id.to_string());
            sqlparams.insert("bsoid".to_string(), bso.id.to_string());

            if use_sortindex {
                sqlparams.insert(
                    "sortindex".to_string(),
                    bso.sortindex
                        .map(|sortindex| sortindex.to_string())
                        .unwrap_or_else(|| "NULL".to_owned()),
                );
                sqltypes.insert(
                    "sortindex".to_string(),
                    Type {
                        code: Some("INT64".to_string()),
                        ..Default::default()
                    },
                );
            }
            sqlparams.insert(
                "payload".to_string(),
                bso.payload.unwrap_or("DEFAULT".to_owned()),
            );
            let now_millis = self.timestamp().as_i64();
            let ttl = bso
                .ttl
                .map_or(DEFAULT_BSO_TTL, |ttl| ttl.try_into().unwrap())
                * 1000;
            let expirystring = to_rfc3339(now_millis + ttl)?;
            println!(
                "!!!!! INSERT {:} ({}) ttl: {}",
                expirystring, timestamp, ttl
            );
            sqlparams.insert("expiry".to_string(), expirystring);
            sqltypes.insert(
                "expiry".to_string(),
                Type {
                    code: Some("TIMESTAMP".to_string()),
                    ..Default::default()
                },
            );

            sqlparams.insert("modified".to_string(), self.timestamp().as_rfc3339()?);
            sqltypes.insert(
                "modified".to_string(),
                Type {
                    code: Some("TIMESTAMP".to_string()),
                    ..Default::default()
                },
            );
            sql.params = Some(sqlparams);
            sql.param_types = Some(sqltypes);
            sql
        };

        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        match result {
            Ok(_) => {
                println!("OK!!!");
            }
            Err(e) => {
                println!("ERR!!! {:}", e);
                Err(e)?;
            }
        }
        Ok(touch)
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
            // XXX: python version doesn't report failures from db layer..
            // XXX: sanitize to.to_string()?
            match put_result {
                Ok(_) => result.success.push(id),
                Err(e) => {
                    result.failed.insert(id, e.to_string());
                }
            }
        }
        self.touch_collection(input.user_id.legacy_id as u32, collection_id)?;
        Ok(result)
    }

    batch_db_method!(create_batch_sync, create, CreateBatch);
    batch_db_method!(validate_batch_sync, validate, ValidateBatch);
    batch_db_method!(append_to_batch_sync, append, AppendToBatch);
    batch_db_method!(commit_batch_sync, commit, CommitBatch);

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
    }

    #[cfg(any(test, feature = "db_test"))]
    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = timestamp;
    }
}
