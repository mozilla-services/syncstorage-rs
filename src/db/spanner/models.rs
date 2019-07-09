use futures::future;

use diesel::r2d2::PooledConnection;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use super::pool::CollectionCache;
use super::spanner::SpannerConnectionManager;

use crate::db::{
    error::{DbError, DbErrorKind},
    params, results,
    util::SyncTimestamp,
    Db, DbFuture,
};

use google_spanner1::ExecuteSqlRequest;

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
        params.insert("name".to_string(), name);

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

        // // Lock the db
        // self.begin()?;
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
}
