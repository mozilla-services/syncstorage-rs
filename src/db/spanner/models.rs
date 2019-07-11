use futures::future;
use futures::lazy;

use diesel::r2d2::PooledConnection;

use std::cell::RefCell;
use std::cmp;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::ops::Deref;

use super::pool::CollectionCache;
use super::spanner::SpannerConnectionManager;

use crate::db::{
    error::{DbError, DbErrorKind},
    params, results,
    util::SyncTimestamp,
    Db, DbFuture,
};

use google_spanner1::BeginTransactionRequest;
use google_spanner1::ExecuteSqlRequest;
use google_spanner1::TransactionOptions;

#[derive(Debug)]
pub enum CollectionLock {
    Read,
    Write,
}

type Conn = PooledConnection<SpannerConnectionManager>;
pub type Result<T> = std::result::Result<T, DbError>;

/// Per session Db metadata
#[derive(Debug, Default)]
struct SpannerDbSession {
    /// The "current time" on the server used for this session's operations
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (user_id, collection_id)
    coll_modified_cache: HashMap<(u32, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(u32, i32), CollectionLock>,
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
        let spanner = &self.inner.conn;

        let mut sql = ExecuteSqlRequest::default();
        sql.sql = Some("SELECT id FROM collections WHERE name = @name;".to_string());
        let mut params = HashMap::new();
        params.insert("name".to_string(), name.to_string());
        sql.params = Some(params);

        let session = spanner.session.name.as_ref().unwrap();
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        let rv = match result {
            Err(e) => {
                println!("err {}", e);
                // TODO Return error
                Ok(0)
            }
            Ok(res) => {
                println!("ok {:?}", res.1);
                match res.1.rows {
                    Some(row) => Ok(row[0][0].parse().unwrap()),
                    None => Ok(0),
                }
            }
        };
        if let Ok(val) = rv {
            self.coll_cache.put(val, name.to_owned())?;
        };
        rv
        //let id = sql_query("SELECT id FROM collections WHERE name = ?")
        //    .bind::<Text, _>(name)
        //    .get_result::<IdResult>(&self.conn)
        //    .optional()?
        //    .ok_or(DbErrorKind::CollectionNotFound)?
        //    .id;
        //self.coll_cache.put(id, name.to_owned())?;
    }

    pub(super) fn create_collection(&self, name: &str) -> Result<i32> {
        // XXX: handle concurrent attempts at inserts
        let spanner = &self.inner.conn;
        let session = spanner.session.name.as_ref().unwrap();

        let mut sql = ExecuteSqlRequest::default();
        sql.sql = Some("SELECT MAX(collectionid) from collections".to_string());
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        let id = match result {
            Err(e) => {
                println!("err {}", e);
                // TODO Return error
                Ok(0)
            }
            Ok(res) => {
                println!("ok {:?}", res.1);
                match res.1.rows {
                    Some(row) => Ok(row[0][0].parse::<i32>().unwrap() + 1),
                    None => Ok(1),
                }
            }
        };
        if let Ok(id) = id {
            let mut sql = ExecuteSqlRequest::default();
            sql.sql = Some(
                "INSERT INTO collections (collectionid, name) VALUES (@id, @name)".to_string(),
            );
            let mut params = HashMap::new();
            params.insert("name".to_string(), name.to_string());
            params.insert("id".to_string(), cmp::max(id, 100).to_string());
            sql.params = Some(params);

            let result = spanner
                .hub
                .projects()
                .instances_databases_sessions_execute_sql(sql, session)
                .doit();
            let rv: Result<i32> = match result {
                Err(e) => {
                    println!("err {}", e);
                    // TODO Return error
                    Ok(0)
                }
                Ok(res) => {
                    println!("ok {:?}", res.1);
                    match res.1.rows {
                        Some(row) => Ok(row[0][0].parse().unwrap()),
                        None => Ok(0),
                    }
                }
            };
            if let Ok(val) = rv {
                self.coll_cache.put(val, name.to_owned())?;
            };
        };
        // let id = self.conn.transaction(|| {
        //     sql_query("INSERT INTO collections (name) VALUES (?)")
        //         .bind::<Text, _>(name)
        //         .execute(&self.conn)?;
        //     collections::table.select(last_insert_id).first(&self.conn)
        // })?;
        // self.coll_cache.put(id, name.to_owned())?;
        id
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
        self.begin()?;
        let spanner = &self.inner.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = ExecuteSqlRequest::default();
        sql.sql = Some("SELECT CURRENT_TIMESTAMP() as now, last_modified FROM user_collections WHERE userid=@userid AND collection=@collectionid LOCK IN SHARE MODE".to_string());
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        params.insert("collection".to_string(), collection_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        if let Ok(results) = results {
            if let Some(rows) = results.1.rows {
                let modified = SyncTimestamp::from_i64(rows[0][0].parse().unwrap())?;
                self.inner
                    .session
                    .borrow_mut()
                    .coll_modified_cache
                    .insert((user_id, collection_id), modified);
            }
        }
        self.inner
            .session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Read);

        // let modified = user_collections::table
        //     .select(user_collections::modified)
        //     .filter(user_collections::user_id.eq(user_id as i32))
        //     .filter(user_collections::collection_id.eq(collection_id))
        //     .lock_in_share_mode()
        //     .first(&self.conn)
        //     .optional()?;
        // if let Some(modified) = modified {
        //     let modified = SyncTimestamp::from_i64(modified)?;
        //     self.session
        //         .borrow_mut()
        //         .coll_modified_cache
        //         .insert((user_id, collection_id), modified);
        // }
        // // XXX: who's responsible for unlocking (removing the entry)
        // self.session
        //     .borrow_mut()
        //     .coll_locks
        //     .insert((user_id, collection_id), CollectionLock::Read);
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
        self.begin()?;
        let spanner = &self.inner.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let mut sql = ExecuteSqlRequest::default();
        sql.sql = Some("SELECT CURRENT_TIMESTAMP() as now, last_modified FROM user_collections WHERE userid=@userid AND collection=@collectionid FOR UPDATE".to_string());
        let mut params = HashMap::new();
        params.insert("userid".to_string(), user_id.to_string());
        params.insert("collection".to_string(), collection_id.to_string());
        sql.params = Some(params);

        let results = spanner
            .hub
            .projects()
            .instances_databases_sessions_execute_sql(sql, session)
            .doit();
        if let Ok(results) = results {
            if let Some(rows) = results.1.rows {
                let modified = SyncTimestamp::from_i64(rows[0][0].parse().unwrap())?;
                self.inner
                    .session
                    .borrow_mut()
                    .coll_modified_cache
                    .insert((user_id, collection_id), modified);
            }
        }
        self.inner
            .session
            .borrow_mut()
            .coll_locks
            .insert((user_id, collection_id), CollectionLock::Write);

        // let modified = user_collections::table
        //     .select(user_collections::modified)
        //     .filter(user_collections::user_id.eq(user_id as i32))
        //     .filter(user_collections::collection_id.eq(collection_id))
        //     .for_update()
        //     .first(&self.conn)
        //     .optional()?;
        // if let Some(modified) = modified {
        //     let modified = SyncTimestamp::from_i64(modified)?;
        //     // Forbid the write if it would not properly incr the timestamp
        //     if modified >= self.timestamp() {
        //         Err(DbErrorKind::Conflict)?
        //     }
        //     self.session
        //         .borrow_mut()
        //         .coll_modified_cache
        //         .insert((user_id, collection_id), modified);
        // }
        // self.session
        //     .borrow_mut()
        //     .coll_locks
        //     .insert((user_id, collection_id), CollectionLock::Write);
        Ok(())
    }

    pub(super) fn begin(&self) -> Result<()> {
        let spanner = &self.inner.conn;
        let session = spanner.session.name.as_ref().unwrap();
        let options = Some(TransactionOptions::default());
        let req = BeginTransactionRequest { options };
        let result = spanner
            .hub
            .projects()
            .instances_databases_sessions_begin_transaction(req, session)
            .doit();
        match result {
            Ok(_) => Ok(()),
            Err(e) => {
                // TODO Handle error
                Ok(())
            }
        }
        // Ok(self
        //     .conn
        //     .transaction_manager()
        //     .begin_transaction(&self.conn)?)
    }
}

unsafe impl Send for SpannerDb {}

macro_rules! mock_db_method {
    ($name:ident, $type:ident) => {
        mock_db_method!($name, $type, results::$type);
    };
    ($name:ident, $type:ident, $result:ty) => {
        fn $name(&self, _params: params::$type) -> DbFuture<$result> {
            let result: $result = Default::default();
            Box::new(future::ok(result))
        }
    };
}

impl Db for SpannerDb {
    fn commit(&self) -> DbFuture<()> {
        Box::new(future::ok(()))
    }

    fn rollback(&self) -> DbFuture<()> {
        Box::new(future::ok(()))
    }

    fn box_clone(&self) -> Box<dyn Db> {
        Box::new(self.clone())
    }

    mock_db_method!(lock_for_read, LockCollection);
    mock_db_method!(lock_for_write, LockCollection);
    mock_db_method!(get_collection_timestamps, GetCollectionTimestamps);
    mock_db_method!(get_collection_timestamp, GetCollectionTimestamp);
    mock_db_method!(get_collection_counts, GetCollectionCounts);
    mock_db_method!(get_collection_usage, GetCollectionUsage);
    mock_db_method!(get_storage_timestamp, GetStorageTimestamp);
    mock_db_method!(get_storage_usage, GetStorageUsage);
    mock_db_method!(delete_storage, DeleteStorage);
    mock_db_method!(delete_collection, DeleteCollection);
    mock_db_method!(delete_bsos, DeleteBsos);
    mock_db_method!(get_bsos, GetBsos);
    mock_db_method!(get_bso_ids, GetBsoIds);
    mock_db_method!(post_bsos, PostBsos);
    mock_db_method!(delete_bso, DeleteBso);
    mock_db_method!(get_bso, GetBso, Option<results::GetBso>);
    mock_db_method!(get_bso_timestamp, GetBsoTimestamp);
    mock_db_method!(put_bso, PutBso);
    mock_db_method!(create_batch, CreateBatch);
    mock_db_method!(validate_batch, ValidateBatch);
    mock_db_method!(append_to_batch, AppendToBatch);
    mock_db_method!(get_batch, GetBatch, Option<results::GetBatch>);
    mock_db_method!(commit_batch, CommitBatch);

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
        /*
        let db = self.clone();
        Box::new(self.thread_pool.spawn_handle(lazy(move || {
            future::result(
                db.touch_collection(param.user_id.legacy_id as u32, param.collection_id)
                    .map_err(Into::into),
            )
        })))
         */
        Box::new(future::ok(Default::default()))
    }

    #[cfg(any(test, feature = "db_test"))]
    fn timestamp(&self) -> SyncTimestamp {
        //self.timestamp()
        Default::default()
    }

    #[cfg(any(test, feature = "db_test"))]
    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = timestamp;
    }
}
