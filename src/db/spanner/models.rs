use futures::compat::Future01CompatExt;
use futures::future::TryFutureExt;

use diesel::r2d2::PooledConnection;

use std::cell::RefCell;
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
    spanner::support::{as_type, StreamedResultSetAsync},
    util::SyncTimestamp,
    Db, DbFuture, Sorting, FIRST_CUSTOM_COLLECTION_ID,
};
use crate::server::metrics::Metrics;

use crate::web::extractors::{BsoQueryParams, HawkIdentifier, Offset};

use super::support::{bso_to_insert_row, bso_to_update_row};
use super::{
    batch,
    support::{as_list_value, as_value, bso_from_row, ExecuteSqlRequestBuilder},
};

use googleapis_raw::spanner::v1::transaction;
use googleapis_raw::spanner::v1::transaction::{
    TransactionOptions, TransactionOptions_ReadOnly, TransactionOptions_ReadWrite,
};
use googleapis_raw::spanner::v1::{
    mutation::{Mutation, Mutation_Write},
    spanner::{BeginTransactionRequest, CommitRequest, ExecuteSqlRequest, RollbackRequest},
    type_pb::TypeCode,
};

#[allow(unused_imports)]
use itertools::Itertools;

#[allow(unused_imports)]
use protobuf::{well_known_types::ListValue, Message, RepeatedField};

pub type TransactionSelector = transaction::TransactionSelector;

#[derive(Debug, Eq, PartialEq)]
pub enum CollectionLock {
    Read,
    Write,
}

pub(super) type Conn = PooledConnection<SpannerConnectionManager>;
pub type Result<T> = std::result::Result<T, DbError>;

/// The ttl to use for rows that are never supposed to expire (in seconds)
pub const DEFAULT_BSO_TTL: u32 = 2_100_000_000;

pub const TOMBSTONE: i32 = 0;

pub const PRETOUCH_TS: &str = "0001-01-01T00:00:00.00Z";

// max load size in bytes
pub const MAX_SPANNER_LOAD_SIZE: usize = 100_000_000;

/// Per session Db metadata
#[derive(Debug, Default)]
struct SpannerDbSession {
    /// CURRENT_TIMESTAMP() from Spanner, used for timestamping this session's
    /// operations
    timestamp: Option<SyncTimestamp>,
    /// Cache of collection modified timestamps per (HawkIdentifier, collection_id)
    coll_modified_cache: HashMap<(HawkIdentifier, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(HawkIdentifier, i32), CollectionLock>,
    transaction: Option<TransactionSelector>,
    /// Behind Vec so commit can take() it (maybe commit() should consume self
    /// instead?)
    mutations: Option<Vec<Mutation>>,
    in_write_transaction: bool,
    execute_sql_count: u64,
    /// Whether touch_collection has already been called
    touched_collection: bool,
}

#[derive(Clone, Debug)]
pub struct SpannerDb {
    pub(super) inner: Arc<SpannerDbInner>,

    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    pub metrics: Metrics,
}

pub struct SpannerDbInner {
    pub(super) conn: Conn,

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

impl SpannerDb {
    pub fn new(conn: Conn, coll_cache: Arc<CollectionCache>, metrics: &Metrics) -> Self {
        let inner = SpannerDbInner {
            conn,
            session: RefCell::new(Default::default()),
        };
        SpannerDb {
            inner: Arc::new(inner),
            coll_cache,
            metrics: metrics.clone(),
        }
    }

    pub(super) async fn get_collection_id_async(&self, name: &str) -> Result<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }
        let result = self
            .sql(
                "SELECT collection_id
                   FROM collections
                  WHERE name = @name",
            )?
            .params(params! {"name" => name.to_string()})
            .execute_async(&self.conn)?
            .one_or_none()
            .await?
            .ok_or(DbErrorKind::CollectionNotFound)?;
        let id = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        if !self.in_write_transaction() {
            self.coll_cache.put(id, name.to_owned())?;
        }
        Ok(id)
    }

    pub(super) async fn create_collection_async(&self, name: &str) -> Result<i32> {
        // This should always run within a r/w transaction, so that: "If a
        // transaction successfully commits, then no other writer modified the
        // data that was read in the transaction after it was read."
        if !cfg!(test) && !self.in_write_transaction() {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }
        let result = self
            .sql(
                "SELECT COALESCE(MAX(collection_id), 1)
                   FROM collections",
            )?
            .execute_async(&self.conn)?
            .one()
            .await?;
        let max = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        let id = FIRST_CUSTOM_COLLECTION_ID.max(max + 1);

        self.sql(
            "INSERT INTO collections (collection_id, name)
             VALUES (@collection_id, @name)",
        )?
        .params(params! {
            "name" => name.to_string(),
            "collection_id" => id.to_string(),
        })
        .execute_dml_async(&self.conn)
        .await?;
        Ok(id)
    }

    async fn get_or_create_collection_id_async(&self, name: &str) -> Result<i32> {
        let result = self.get_collection_id_async(name).await;
        if let Err(err) = result {
            match err.kind() {
                DbErrorKind::CollectionNotFound => self.create_collection_async(name).await,
                _ => Err(err),
            }
        } else {
            result
        }
    }

    pub async fn lock_for_read_async(&self, params: params::LockCollection) -> Result<()> {
        // Begin a transaction
        self.begin_async(false).await?;

        let collection_id = self
            .get_collection_id_async(&params.collection)
            .await
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
            .get(&(params.user_id.clone(), collection_id))
            .is_some()
        {
            return Ok(());
        }

        self.session
            .borrow_mut()
            .coll_locks
            .insert((params.user_id, collection_id), CollectionLock::Read);

        Ok(())
    }

    pub async fn lock_for_write_async(&self, params: params::LockCollection) -> Result<()> {
        // Begin a transaction
        self.begin_async(true).await?;
        let collection_id = self
            .get_or_create_collection_id_async(&params.collection)
            .await?;
        if let Some(CollectionLock::Read) = self
            .inner
            .session
            .borrow()
            .coll_locks
            .get(&(params.user_id.clone(), collection_id))
        {
            Err(DbError::internal("Can't escalate read-lock to write-lock"))?
        }

        let result = self
            .sql(
                "SELECT CURRENT_TIMESTAMP(), modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )?
            .params(params! {
                "fxa_uid" => params.user_id.fxa_uid.clone(),
                "fxa_kid" => params.user_id.fxa_kid.clone(),
                "collection_id" => collection_id.to_string(),
                "pretouch_ts" => PRETOUCH_TS.to_owned(),
            })
            .param_types(param_types! {
                "pretouch_ts" => TypeCode::TIMESTAMP,
            })
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;

        let timestamp = if let Some(result) = result {
            let modified = SyncTimestamp::from_rfc3339(result[1].get_string_value())?;
            let now = SyncTimestamp::from_rfc3339(result[0].get_string_value())?;
            // Forbid the write if it would not properly incr the modified
            // timestamp
            if modified >= now {
                Err(DbErrorKind::Conflict)?
            }
            self.session
                .borrow_mut()
                .coll_modified_cache
                .insert((params.user_id.clone(), collection_id), modified);
            now
        } else {
            let result = self
                .sql("SELECT CURRENT_TIMESTAMP()")?
                .execute_async(&self.conn)?
                .one()
                .await?;
            SyncTimestamp::from_rfc3339(result[0].get_string_value())?
        };
        self.set_timestamp(timestamp);

        self.session
            .borrow_mut()
            .coll_locks
            .insert((params.user_id, collection_id), CollectionLock::Write);

        Ok(())
    }

    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = Some(timestamp);
    }

    pub(super) fn begin(&self, for_write: bool) -> Result<()> {
        let spanner = &self.conn;
        let mut options = TransactionOptions::new();
        if for_write {
            options.set_read_write(TransactionOptions_ReadWrite::new());
            self.session.borrow_mut().in_write_transaction = true;
        } else {
            options.set_read_only(TransactionOptions_ReadOnly::new());
        }
        let mut req = BeginTransactionRequest::new();
        req.set_session(spanner.session.get_name().to_owned());
        req.set_options(options);
        let mut transaction = spanner.client.begin_transaction(&req)?;

        let mut ts = TransactionSelector::new();
        ts.set_id(transaction.take_id());
        self.session.borrow_mut().transaction = Some(ts);
        Ok(())
    }

    pub(super) async fn begin_async(&self, for_write: bool) -> Result<()> {
        let spanner = &self.conn;
        let mut options = TransactionOptions::new();
        if for_write {
            options.set_read_write(TransactionOptions_ReadWrite::new());
            self.session.borrow_mut().in_write_transaction = true;
        } else {
            options.set_read_only(TransactionOptions_ReadOnly::new());
        }
        let mut req = BeginTransactionRequest::new();
        req.set_session(spanner.session.get_name().to_owned());
        req.set_options(options);
        let mut transaction = spanner
            .client
            .begin_transaction_async(&req)?
            .compat()
            .await?;

        let mut ts = TransactionSelector::new();
        ts.set_id(transaction.take_id());
        self.session.borrow_mut().transaction = Some(ts);
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

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    async fn get_transaction_async(&self) -> Result<Option<TransactionSelector>> {
        Ok(if self.session.borrow().transaction.is_some() {
            self.session.borrow().transaction.clone()
        } else {
            self.begin_async(true).await?;
            self.session.borrow().transaction.clone()
        })
    }

    fn sql_request(&self, sql: &str) -> Result<ExecuteSqlRequest> {
        let mut sqlr = ExecuteSqlRequest::new();
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

    pub(super) fn sql(&self, sql: &str) -> Result<ExecuteSqlRequestBuilder> {
        Ok(ExecuteSqlRequestBuilder::new(self.sql_request(sql)?))
    }

    pub(super) fn insert(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_insert(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(|| vec![])
            .push(mutation);
    }

    pub(super) fn update(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_update(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(|| vec![])
            .push(mutation);
    }

    #[allow(unused)]
    pub(super) fn insert_or_update(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_insert_or_update(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(|| vec![])
            .push(mutation);
    }

    fn mutation_write(
        &self,
        table: &str,
        columns: &[&str],
        values: Vec<ListValue>,
    ) -> Mutation_Write {
        let mut write = Mutation_Write::new();
        write.set_table(table.to_owned());
        write.set_columns(RepeatedField::from_vec(
            columns.iter().map(|&column| column.to_owned()).collect(),
        ));
        write.set_values(RepeatedField::from_vec(values));
        write
    }

    fn in_write_transaction(&self) -> bool {
        self.session.borrow().in_write_transaction
    }

    pub fn commit(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        let spanner = &self.conn;

        if cfg!(test) && spanner.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let mut req = CommitRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            if let Some(mutations) = self.session.borrow_mut().mutations.take() {
                req.set_mutations(RepeatedField::from_vec(mutations));
            }
            spanner.client.commit(&req)?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit"))?
        }
    }

    pub async fn commit_async(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        let spanner = &self.conn;

        if cfg!(test) && spanner.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction_async().await? {
            let mut req = CommitRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            if let Some(mutations) = self.session.borrow_mut().mutations.take() {
                req.set_mutations(RepeatedField::from_vec(mutations));
            }
            spanner.client.commit_async(&req)?.compat().await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit"))?
        }
    }

    pub fn rollback(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction()? {
            let spanner = &self.conn;
            let mut req = RollbackRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner.client.rollback(&req)?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback"))?
        }
    }

    pub async fn rollback_async(&self) -> Result<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction_async().await? {
            let spanner = &self.conn;
            let mut req = RollbackRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner.client.rollback_async(&req)?.compat().await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback"))?
        }
    }

    pub async fn get_collection_timestamp_async(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> Result<SyncTimestamp> {
        debug!("!!QQQ get_collection_timestamp {:?}", &params.collection);

        let collection_id = self.get_collection_id_async(&params.collection).await?;
        if let Some(modified) = self
            .session
            .borrow()
            .coll_modified_cache
            .get(&(params.user_id.clone(), collection_id))
        {
            return Ok(*modified);
        }

        let result = self
            .sql(
                "SELECT modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )?
            .params(params! {
                "fxa_uid" => params.user_id.fxa_uid,
                "fxa_kid" => params.user_id.fxa_kid,
                "collection_id" => collection_id.to_string(),
                "pretouch_ts" => PRETOUCH_TS.to_owned(),
            })
            .param_types(param_types! {
                "pretouch_ts" => TypeCode::TIMESTAMP,
            })
            .execute_async(&self.conn)?
            .one_or_none()
            .await?
            .ok_or_else(|| DbErrorKind::CollectionNotFound)?;
        let modified = SyncTimestamp::from_rfc3339(&result[0].get_string_value())?;
        Ok(modified)
    }

    pub async fn get_collection_timestamps_async(
        &self,
        user_id: params::GetCollectionTimestamps,
    ) -> Result<results::GetCollectionTimestamps> {
        let mut streaming = self
            .sql(
                "SELECT collection_id, modified
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id != @collection_id
                    AND modified > @pretouch_ts",
            )?
            .params(params! {
                "fxa_uid" => user_id.fxa_uid,
                "fxa_kid" => user_id.fxa_kid,
                "collection_id" => TOMBSTONE.to_string(),
                "pretouch_ts" => PRETOUCH_TS.to_owned(),
            })
            .param_types(param_types! {
                "pretouch_ts" => TypeCode::TIMESTAMP,
            })
            .execute_async(&self.conn)?;
        let mut results = HashMap::new();
        while let Some(row) = streaming.next_async().await {
            let row = row?;
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            let modified = SyncTimestamp::from_rfc3339(&row[1].get_string_value())?;
            results.insert(collection_id, modified);
        }
        self.map_collection_names(results).await
    }

    async fn map_collection_names<T>(&self, by_id: HashMap<i32, T>) -> Result<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys()).await?;
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

    async fn load_collection_names<'a>(
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
            let mut rs = self
                .sql(
                    "SELECT collection_id, name
                       FROM collections
                      WHERE collection_id IN UNNEST(@ids)",
                )?
                .params(params)
                .execute_async(&self.conn)?;
            while let Some(row) = rs.next_async().await {
                let mut row = row?;
                let id = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
                let name = row[1].take_string_value();
                names.insert(id, name.clone());
                if !self.in_write_transaction() {
                    self.coll_cache.put(id, name)?;
                }
            }
        }

        Ok(names)
    }

    pub async fn get_collection_counts_async(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> Result<results::GetCollectionCounts> {
        let mut streaming = self
            .sql(
                "SELECT collection_id, COUNT(collection_id)
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY collection_id",
            )?
            .params(params! {
                "fxa_uid" => user_id.fxa_uid,
                "fxa_kid" => user_id.fxa_kid,
            })
            .execute_async(&self.conn)?;
        let mut counts = HashMap::new();
        while let Some(row) = streaming.next_async().await {
            let row = row?;
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            let count = row[1]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            counts.insert(collection_id, count);
        }
        self.map_collection_names(counts).await
    }

    pub async fn get_collection_usage_async(
        &self,
        user_id: params::GetCollectionUsage,
    ) -> Result<results::GetCollectionUsage> {
        let mut streaming = self
            .sql(
                "SELECT collection_id, SUM(LENGTH(payload))
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY collection_id",
            )?
            .params(params! {
                "fxa_uid" => user_id.fxa_uid,
                "fxa_kid" => user_id.fxa_kid
            })
            .execute_async(&self.conn)?;
        let mut usages = HashMap::new();
        while let Some(row) = streaming.next_async().await {
            let row = row?;
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            let usage = row[1]
                .get_string_value()
                .parse::<i64>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            usages.insert(collection_id, usage);
        }
        self.map_collection_names(usages).await
    }

    pub async fn get_storage_timestamp(
        &self,
        user_id: params::GetStorageTimestamp,
    ) -> Result<SyncTimestamp> {
        let row = self
            .sql(
                "SELECT MAX(modified)
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND modified > @pretouch_ts",
            )?
            .params(params! {
                "fxa_uid" => user_id.fxa_uid,
                "fxa_kid" => user_id.fxa_kid,
                "pretouch_ts" => PRETOUCH_TS.to_owned(),
            })
            .param_types(param_types! {
                "pretouch_ts" => TypeCode::TIMESTAMP,
            })
            .execute_async(&self.conn)?
            .one()
            .await?;
        if row[0].has_null_value() {
            SyncTimestamp::from_i64(0)
        } else {
            SyncTimestamp::from_rfc3339(row[0].get_string_value())
        }
    }

    pub async fn get_storage_usage_async(
        &self,
        user_id: params::GetStorageUsage,
    ) -> Result<results::GetStorageUsage> {
        let result = self
            .sql(
                "SELECT SUM(LENGTH(payload))
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY fxa_uid",
            )?
            .params(params! {
                "fxa_uid" => user_id.fxa_uid,
                "fxa_kid" => user_id.fxa_kid
            })
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
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

    async fn erect_tombstone(&self, user_id: &HawkIdentifier) -> Result<SyncTimestamp> {
        // Delete the old tombstone (if it exists)
        let params = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => TOMBSTONE.to_string(),
            "modified" => self.timestamp()?.as_rfc3339()?
        };
        let types = param_types! {
            "collection_id" => TypeCode::INT64,
            "modified" => TypeCode::TIMESTAMP,
        };
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id",
        )?
        .params(params.clone())
        .param_types(types.clone())
        .execute_dml_async(&self.conn)
        .await?;

        self.sql(
            "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)",
        )?
        .params(params)
        .param_types(types)
        .execute_dml_async(&self.conn)
        .await?;
        // Return timestamp, because sometimes there's a delay between writing and
        // reading the database.
        Ok(self.timestamp()?)
    }

    pub async fn delete_storage_async(&self, user_id: params::DeleteStorage) -> Result<()> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid",
        )?
        .params(params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
        })
        .execute_dml_async(&self.conn)
        .await?;
        Ok(())
    }

    pub fn timestamp(&self) -> Result<SyncTimestamp> {
        self.session
            .borrow()
            .timestamp
            .ok_or_else(|| DbError::internal("CURRENT_TIMESTAMP() not read yet"))
    }

    pub async fn delete_collection_async(
        &self,
        params: params::DeleteCollection,
    ) -> Result<results::DeleteCollection> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        let affected_rows = self
            .sql(
                "DELETE FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND modified > @pretouch_ts",
            )?
            .params(params! {
                "fxa_uid" => params.user_id.fxa_uid.clone(),
                "fxa_kid" => params.user_id.fxa_kid.clone(),
                "collection_id" => self.get_collection_id_async(&params.collection).await?.to_string(),
                "pretouch_ts" => PRETOUCH_TS.to_owned(),
            })
            .param_types(param_types! {
                "pretouch_ts" => TypeCode::TIMESTAMP,
            })
            .execute_dml_async(&self.conn)
            .await?;
        if affected_rows > 0 {
            self.erect_tombstone(&params.user_id).await
        } else {
            self.get_storage_timestamp(params.user_id).await
        }
    }

    pub(super) async fn touch_collection_async(
        &self,
        user_id: &HawkIdentifier,
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
        if !cfg!(test) && self.session.borrow().touched_collection {
            // No need to touch it again (except during tests where we
            // currently reuse Dbs for multiple requests)
            return Ok(timestamp);
        }

        let sqlparams = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "modified" => timestamp.as_rfc3339()?,
        };
        let sql_types = param_types! {
            "modified" => TypeCode::TIMESTAMP,
        };
        let result = self
            .sql(
                "SELECT 1 AS count
                   FROM user_collections
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id",
            )?
            .params(sqlparams.clone())
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        let exists = result.is_some();

        if exists {
            self.sql(
                "UPDATE user_collections
                    SET modified = @modified
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id",
            )?
            .params(sqlparams)
            .param_types(sql_types)
            .execute_dml_async(&self.conn)
            .await?;
        } else {
            self.sql(
                "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
                 VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)",
            )?
            .params(sqlparams)
            .param_types(sql_types)
            .execute_dml_async(&self.conn)
            .await?;
        }
        self.session.borrow_mut().touched_collection = true;
        Ok(timestamp)
    }

    pub async fn delete_bso_async(&self, params: params::DeleteBso) -> Result<results::DeleteBso> {
        let collection_id = self.get_collection_id_async(&params.collection).await?;
        let touch = self
            .touch_collection_async(&params.user_id, collection_id)
            .await?;
        let affected_rows = self
            .sql(
                "DELETE FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id",
            )?
            .params(params! {
                "fxa_uid" => params.user_id.fxa_uid,
                "fxa_kid" => params.user_id.fxa_kid,
                "collection_id" => collection_id.to_string(),
                "bso_id" => params.id,
            })
            .execute_dml_async(&self.conn)
            .await?;
        if affected_rows == 0 {
            Err(DbErrorKind::BsoNotFound)?
        } else {
            Ok(touch)
        }
    }

    pub async fn delete_bsos_async(
        &self,
        params: params::DeleteBsos,
    ) -> Result<results::DeleteBsos> {
        let user_id = params.user_id.clone();
        let collection_id = self.get_collection_id_async(&params.collection).await?;

        let mut sqlparams = params! {
            "fxa_uid" => user_id.fxa_uid,
            "fxa_kid" => user_id.fxa_kid,
            "collection_id" => collection_id.to_string(),
        };
        sqlparams.insert("ids".to_owned(), as_list_value(params.ids.into_iter()));
        self.sql(
            "DELETE FROM bsos
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND bso_id IN UNNEST(@ids)",
        )?
        .params(sqlparams)
        .execute_dml_async(&self.conn)
        .await?;
        self.touch_collection_async(&params.user_id, collection_id)
            .await
    }

    async fn bsos_query_async(
        &self,
        query_str: &str,
        params: params::GetBsos,
    ) -> Result<StreamedResultSetAsync> {
        let mut query = query_str.to_owned();
        let mut sqlparams = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => self.get_collection_id_async(&params.collection).await?.to_string(),
        };
        let BsoQueryParams {
            newer,
            older,
            sort,
            limit,
            offset,
            ids,
            ..
        } = params.params;

        let mut sqltypes = HashMap::new();

        if !ids.is_empty() {
            query = format!("{} AND bso_id IN UNNEST(@ids)", query);
            sqlparams.insert("ids".to_owned(), as_list_value(ids.into_iter()));
        }

        if let Some(timestamp) = offset.clone().unwrap_or_default().timestamp {
            query = match sort {
                Sorting::Newest => {
                    sqlparams.insert("older_eq".to_string(), as_value(timestamp.as_rfc3339()?));
                    sqltypes.insert("older_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified <= @older_eq", query)
                }
                Sorting::Oldest => {
                    sqlparams.insert("newer_eq".to_string(), as_value(timestamp.as_rfc3339()?));
                    sqltypes.insert("newer_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified >= @newer_eq", query)
                }
                _ => query,
            };
        }
        if let Some(older) = older {
            query = format!("{} AND modified < @older", query);
            sqlparams.insert("older".to_string(), as_value(older.as_rfc3339()?));
            sqltypes.insert("older".to_string(), as_type(TypeCode::TIMESTAMP));
        }
        if let Some(newer) = newer {
            query = format!("{} AND modified > @newer", query);
            sqlparams.insert("newer".to_string(), as_value(newer.as_rfc3339()?));
            sqltypes.insert("newer".to_string(), as_type(TypeCode::TIMESTAMP));
        }
        query = match sort {
            Sorting::Index => format!("{} ORDER BY sortindex DESC, bso_id DESC", query),
            Sorting::Newest | Sorting::None => {
                format!("{} ORDER BY modified DESC, bso_id DESC", query)
            }
            Sorting::Oldest => format!("{} ORDER BY modified ASC, bso_id ASC", query),
        };

        if let Some(limit) = limit {
            // fetch an extra row to detect if there are more rows that match
            // the query conditions
            query = format!("{} LIMIT {}", query, i64::from(limit) + 1);
        } else if let Some(ref offset) = offset {
            // Special case no limit specified but still required for an
            // offset. Spanner doesn't accept a simpler limit of -1 (common in
            // most databases) so we specify a max value with offset subtracted
            // to avoid overflow errors (that only occur w/ a FORCE_INDEX=
            // directive) OutOfRange: 400 int64 overflow: <INT64_MAX> + offset
            query = format!("{} LIMIT {}", query, i64::max_value() - offset.offset);
        };

        if let Some(offset) = offset {
            query = format!("{} OFFSET {}", query, offset.offset);
        }
        self.sql(&query)?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute_async(&self.conn)
    }

    pub fn encode_next_offset(
        &self,
        sort: Sorting,
        offset: i64,
        timestamp: Option<i64>,
        modifieds: Vec<i64>,
    ) -> Option<String> {
        let mut calc_offset = 1;
        let mut i = (modifieds.len() as i64) - 2;

        let prev_bound = match sort {
            Sorting::Index => {
                // Use a simple numeric offset for sortindex ordering.
                return Some(
                    Offset {
                        offset: offset + modifieds.len() as i64,
                        timestamp: None,
                    }
                    .to_string(),
                );
            }
            Sorting::None => timestamp,
            Sorting::Newest => timestamp,
            Sorting::Oldest => timestamp,
        };
        // Find an appropriate upper bound for faster timestamp ordering.
        let bound = *modifieds.last().unwrap_or(&0);
        // Count how many previous items have that same timestamp, and hence
        // will need to be skipped over.  The number of matches here is limited
        // by upload batch size.
        while i >= 0 && modifieds[i as usize] == bound {
            calc_offset += 1;
            i -= 1;
        }
        if i < 0 && prev_bound.is_some() && prev_bound.unwrap() == bound {
            calc_offset += offset;
        }

        Some(format!("{}:{}", bound, calc_offset))
    }

    pub async fn get_bsos_async(&self, params: params::GetBsos) -> Result<results::GetBsos> {
        let query = "\
            SELECT bso_id, sortindex, payload, modified, expiry
              FROM bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND expiry > CURRENT_TIMESTAMP()";
        let limit = params.params.limit.map(i64::from).unwrap_or(-1);
        let Offset { offset, timestamp } = params.params.offset.clone().unwrap_or_default();
        let sort = params.params.sort;

        let mut streaming = self.bsos_query_async(query, params).await?;
        let mut bsos = vec![];
        while let Some(row) = streaming.next_async().await {
            let row = row?;
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

    pub async fn get_bso_ids_async(&self, params: params::GetBsos) -> Result<results::GetBsoIds> {
        let limit = params.params.limit.map(i64::from).unwrap_or(-1);
        let Offset { offset, timestamp } = params.params.offset.clone().unwrap_or_default();
        let sort = params.params.sort;

        let query = "\
            SELECT bso_id, modified
              FROM bsos
             WHERE fxa_uid = @fxa_uid
               AND fxa_kid = @fxa_kid
               AND collection_id = @collection_id
               AND expiry > CURRENT_TIMESTAMP()";
        let mut stream = self.bsos_query_async(query, params).await?;

        let mut ids = vec![];
        let mut modifieds = vec![];
        while let Some(row) = stream.next_async().await {
            let mut row = row?;
            ids.push(row[0].take_string_value());
            modifieds.push(SyncTimestamp::from_rfc3339(row[1].get_string_value())?.as_i64());
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

    pub async fn get_bso_async(&self, params: params::GetBso) -> Result<Option<results::GetBso>> {
        let collection_id = self.get_collection_id_async(&params.collection).await?;
        self.sql(
            "SELECT bso_id, sortindex, payload, modified, expiry
               FROM bsos
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND bso_id = @bso_id
                AND expiry > CURRENT_TIMESTAMP()",
        )?
        .params(params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id.to_string(),
            "bso_id" => params.id,
        })
        .execute_async(&self.conn)?
        .one_or_none()
        .await?
        .map(bso_from_row)
        .transpose()
    }

    pub async fn get_bso_timestamp_async(
        &self,
        params: params::GetBsoTimestamp,
    ) -> Result<SyncTimestamp> {
        debug!("!!QQQ get_bso_timestamp_async: {:?}", &params.collection);
        let collection_id = self.get_collection_id_async(&params.collection).await?;

        let result = self
            .sql(
                "SELECT modified
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id
                    AND expiry > CURRENT_TIMESTAMP()",
            )?
            .params(params! {
                "fxa_uid" => params.user_id.fxa_uid,
                "fxa_kid" => params.user_id.fxa_kid,
                "collection_id" => collection_id.to_string(),
                "bso_id" => params.id.to_string(),
            })
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            SyncTimestamp::from_rfc3339(&result[0].get_string_value())
        } else {
            SyncTimestamp::from_i64(0)
        }
    }

    pub async fn put_bso_async(&self, params: params::PutBso) -> Result<results::PutBso> {
        let bsos = vec![params::PostCollectionBso {
            id: params.id,
            sortindex: params.sortindex,
            payload: params.payload,
            ttl: params.ttl,
        }];
        let result = self
            .post_bsos_async(params::PostBsos {
                user_id: params.user_id,
                collection: params.collection,
                bsos,
                failed: HashMap::new(),
            })
            .await?;
        Ok(result.modified)
    }

    pub async fn post_bsos_async(&self, params: params::PostBsos) -> Result<results::PostBsos> {
        let user_id = params.user_id;
        let collection_id = self
            .get_or_create_collection_id_async(&params.collection)
            .await?;
        // Ensure a parent record exists in user_collections before writing to
        // bsos (INTERLEAVE IN PARENT user_collections)
        let timestamp = self.touch_collection_async(&user_id, collection_id).await?;

        let mut sqlparams = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
        };
        sqlparams.insert(
            "ids".to_owned(),
            as_list_value(params.bsos.iter().map(|pbso| pbso.id.clone())),
        );
        let mut streaming = self
            .sql(
                "SELECT bso_id
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id IN UNNEST(@ids)",
            )?
            .params(sqlparams)
            .execute_async(&self.conn)?;
        let mut existing = vec![];
        while let Some(row) = streaming.next_async().await {
            let mut row = row?;
            existing.push(row[0].take_string_value());
        }

        let mut inserts = vec![];
        let mut updates = HashMap::new();
        let mut success = vec![];
        let mut load_size: usize = 0;
        for bso in params.bsos {
            success.push(bso.id.clone());
            if existing.contains(&bso.id) {
                let (columns, values) = bso_to_update_row(&user_id, collection_id, bso, timestamp)?;
                load_size += values.compute_size() as usize;
                updates
                    .entry(columns)
                    .or_insert_with(|| vec![])
                    .push(values);
            } else {
                let values = bso_to_insert_row(&user_id, collection_id, bso, timestamp)?;
                load_size += values.compute_size() as usize;
                inserts.push(values);
            }
        }
        if load_size > MAX_SPANNER_LOAD_SIZE {
            self.metrics.clone().incr("error.tooMuchData");
            debug!(
                "⚠️Attempted to load too much data into Spanner: {:?} bytes",
                load_size
            );
            return Err(DbErrorKind::SpannerTooLarge(format!(
                "Committed data too large: {}",
                load_size
            ))
            .into());
        }

        if !inserts.is_empty() {
            debug!("inserts: {:?}", &inserts);
            self.insert(
                "bsos",
                &[
                    "fxa_uid",
                    "fxa_kid",
                    "collection_id",
                    "bso_id",
                    "sortindex",
                    "payload",
                    "modified",
                    "expiry",
                ],
                inserts,
            );
        }
        for (columns, values) in updates {
            debug!("columns: {:?}, values:{:?}", &columns, &values);
            self.update("bsos", &columns, values);
        }

        let result = results::PostBsos {
            modified: timestamp,
            success,
            failed: params.failed,
        };
        Ok(result)
    }

    // NOTE: Currently this put_bso_async_test impl. is only used during db tests,
    // see above for the non-tests version
    #[cfg(test)]
    pub async fn put_bso_async_test(&self, bso: params::PutBso) -> Result<results::PutBso> {
        use crate::db::util::to_rfc3339;
        let collection_id = self
            .get_or_create_collection_id_async(&bso.collection)
            .await?;
        let mut sqlparams = params! {
            "fxa_uid" => bso.user_id.fxa_uid.clone(),
            "fxa_kid" => bso.user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "bso_id" => bso.id.to_string(),
        };
        let mut sqltypes = HashMap::new();
        let touch = self
            .touch_collection_async(&bso.user_id, collection_id)
            .await?;
        let timestamp = self.timestamp()?;

        let result = self
            .sql(
                "SELECT 1 AS count
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id",
            )?
            .params(sqlparams.clone())
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        let exists = result.is_some();

        let sql = if exists {
            let mut q = "".to_string();
            let comma = |q: &String| if q.is_empty() { "" } else { ", " };

            q = format!(
                "{}{}",
                q,
                if let Some(sortindex) = bso.sortindex {
                    sqlparams.insert("sortindex".to_string(), as_value(sortindex.to_string()));
                    sqltypes.insert("sortindex".to_string(), as_type(TypeCode::INT64));

                    format!("{}{}", comma(&q), "sortindex = @sortindex")
                } else {
                    "".to_string()
                }
            );

            q = format!(
                "{}{}",
                q,
                if let Some(ttl) = bso.ttl {
                    let expiry = timestamp.as_i64() + (i64::from(ttl) * 1000);
                    sqlparams.insert("expiry".to_string(), as_value(to_rfc3339(expiry)?));
                    sqltypes.insert("expiry".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{}{}", comma(&q), "expiry = @expiry")
                } else {
                    "".to_string()
                }
            );

            q = format!(
                "{}{}",
                q,
                if bso.payload.is_some() || bso.sortindex.is_some() {
                    sqlparams.insert("modified".to_string(), as_value(timestamp.as_rfc3339()?));
                    sqltypes.insert("modified".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{}{}", comma(&q), "modified = @modified")
                } else {
                    "".to_string()
                }
            );

            q = format!(
                "{}{}",
                q,
                if let Some(payload) = bso.payload {
                    sqlparams.insert("payload".to_string(), as_value(payload));
                    format!("{}{}", comma(&q), "payload = @payload")
                } else {
                    "".to_string()
                }
            );

            if q.is_empty() {
                // Nothing to update
                return Ok(touch);
            }

            format!(
                "UPDATE bsos SET {}{}",
                q,
                " WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id"
            )
        } else {
            let use_sortindex = bso
                .sortindex
                .map(|sortindex| sortindex.to_string())
                .unwrap_or_else(|| "NULL".to_owned())
                != "NULL";
            let sql = if use_sortindex {
                "INSERT INTO bsos
                        (fxa_uid, fxa_kid, collection_id, bso_id, sortindex, payload, modified,
                         expiry)
                 VALUES
                        (@fxa_uid, @fxa_kid, @collection_id, @bso_id, @sortindex, @payload,
                         @modified, @expiry)"
            } else {
                "INSERT INTO bsos (fxa_uid, fxa_kid, collection_id, bso_id, payload, modified,
                                   expiry)
                 VALUES (@fxa_uid, @fxa_kid, @collection_id, @bso_id, @payload, @modified,
                         @expiry)"
            };

            if use_sortindex {
                use super::support::null_value;
                let sortindex = bso
                    .sortindex
                    .map(|sortindex| as_value(sortindex.to_string()))
                    .unwrap_or_else(null_value);
                sqlparams.insert("sortindex".to_string(), sortindex);
                sqltypes.insert("sortindex".to_string(), as_type(TypeCode::INT64));
            }
            sqlparams.insert(
                "payload".to_string(),
                as_value(bso.payload.unwrap_or_else(|| "".to_owned())),
            );
            let now_millis = timestamp.as_i64();
            let ttl = bso.ttl.map_or(i64::from(DEFAULT_BSO_TTL), |ttl| {
                ttl.try_into()
                    .expect("Could not get ttl in put_bso_async_test")
            }) * 1000;
            let expirystring = to_rfc3339(now_millis + ttl)?;
            debug!(
                "!!!!! INSERT expirystring:{:?}, timestamp:{:?}, ttl:{:?}",
                &expirystring, timestamp, ttl
            );
            sqlparams.insert("expiry".to_string(), as_value(expirystring));
            sqltypes.insert("expiry".to_string(), as_type(TypeCode::TIMESTAMP));

            sqlparams.insert("modified".to_string(), as_value(timestamp.as_rfc3339()?));
            sqltypes.insert("modified".to_string(), as_type(TypeCode::TIMESTAMP));
            sql.to_owned()
        };

        self.sql(&sql)?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute_dml_async(&self.conn)
            .await?;

        Ok(touch)
    }

    // NOTE: Currently this post_bso_async_test impl. is only used during db tests,
    // see above for the non-tests version
    #[cfg(test)]
    pub async fn post_bsos_async_test(&self, input: params::PostBsos) -> Result<results::PostBsos> {
        let collection_id = self
            .get_or_create_collection_id_async(&input.collection)
            .await?;
        let mut result = results::PostBsos {
            modified: self.timestamp()?,
            success: Default::default(),
            failed: input.failed,
        };

        for pbso in input.bsos {
            let id = pbso.id;
            self.put_bso_async_test(params::PutBso {
                user_id: input.user_id.clone(),
                collection: input.collection.clone(),
                id: id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            })
            .await?;
            result.success.push(id);
        }
        self.touch_collection_async(&input.user_id, collection_id)
            .await?;
        Ok(result)
    }

    async fn check_async(&self) -> Result<results::Check> {
        // TODO: is there a better check than just fetching UTC?
        self.sql("SELECT CURRENT_TIMESTAMP()")?
            .execute_async(&self.conn)?
            .one()
            .await?;
        Ok(true)
    }
}

unsafe impl Send for SpannerDb {}

impl Db for SpannerDb {
    fn commit(&self) -> DbFuture<()> {
        let db = self.clone();
        Box::pin(async move { db.commit_async().map_err(Into::into).await })
    }

    fn rollback(&self) -> DbFuture<()> {
        let db = self.clone();
        Box::pin(async move { db.rollback_async().map_err(Into::into).await })
    }

    fn lock_for_read(&self, param: params::LockCollection) -> DbFuture<()> {
        let db = self.clone();
        Box::pin(async move { db.lock_for_read_async(param).map_err(Into::into).await })
    }

    fn lock_for_write(&self, param: params::LockCollection) -> DbFuture<()> {
        let db = self.clone();
        Box::pin(async move { db.lock_for_write_async(param).map_err(Into::into).await })
    }

    fn get_collection_timestamp(
        &self,
        param: params::GetCollectionTimestamp,
    ) -> DbFuture<results::GetCollectionTimestamp> {
        let db = self.clone();
        Box::pin(async move {
            db.get_collection_timestamp_async(param)
                .map_err(Into::into)
                .await
        })
    }

    fn get_storage_timestamp(
        &self,
        param: params::GetStorageTimestamp,
    ) -> DbFuture<results::GetStorageTimestamp> {
        let db = self.clone();
        Box::pin(async move { db.get_storage_timestamp(param).map_err(Into::into).await })
    }

    fn delete_collection(
        &self,
        param: params::DeleteCollection,
    ) -> DbFuture<results::DeleteCollection> {
        let db = self.clone();
        Box::pin(async move { db.delete_collection_async(param).map_err(Into::into).await })
    }

    fn box_clone(&self) -> Box<dyn Db> {
        Box::new(self.clone())
    }

    fn check(&self) -> DbFuture<results::Check> {
        let db = self.clone();
        Box::pin(async move { db.check_async().map_err(Into::into).await })
    }

    fn get_collection_timestamps(
        &self,
        user_id: params::GetCollectionTimestamps,
    ) -> DbFuture<results::GetCollectionTimestamps> {
        let db = self.clone();
        Box::pin(async move {
            db.get_collection_timestamps_async(user_id)
                .map_err(Into::into)
                .await
        })
    }

    fn get_collection_counts(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> DbFuture<results::GetCollectionCounts> {
        let db = self.clone();
        Box::pin(async move {
            db.get_collection_counts_async(user_id)
                .map_err(Into::into)
                .await
        })
    }

    fn get_collection_usage(
        &self,
        user_id: params::GetCollectionUsage,
    ) -> DbFuture<results::GetCollectionUsage> {
        let db = self.clone();
        Box::pin(async move {
            db.get_collection_usage_async(user_id)
                .map_err(Into::into)
                .await
        })
    }

    fn get_storage_usage(
        &self,
        param: params::GetStorageUsage,
    ) -> DbFuture<results::GetStorageUsage> {
        let db = self.clone();
        Box::pin(async move { db.get_storage_usage_async(param).map_err(Into::into).await })
    }

    fn delete_storage(&self, param: params::DeleteStorage) -> DbFuture<results::DeleteStorage> {
        let db = self.clone();
        Box::pin(async move { db.delete_storage_async(param).map_err(Into::into).await })
    }

    fn delete_bso(&self, param: params::DeleteBso) -> DbFuture<results::DeleteBso> {
        let db = self.clone();
        Box::pin(async move { db.delete_bso_async(param).map_err(Into::into).await })
    }

    fn delete_bsos(&self, param: params::DeleteBsos) -> DbFuture<results::DeleteBsos> {
        let db = self.clone();
        Box::pin(async move { db.delete_bsos_async(param).map_err(Into::into).await })
    }

    fn get_bsos(&self, param: params::GetBsos) -> DbFuture<results::GetBsos> {
        let db = self.clone();
        Box::pin(async move { db.get_bsos_async(param).map_err(Into::into).await })
    }

    fn get_bso_ids(&self, param: params::GetBsoIds) -> DbFuture<results::GetBsoIds> {
        let db = self.clone();
        Box::pin(async move { db.get_bso_ids_async(param).map_err(Into::into).await })
    }

    fn get_bso(&self, param: params::GetBso) -> DbFuture<Option<results::GetBso>> {
        let db = self.clone();
        Box::pin(async move { db.get_bso_async(param).map_err(Into::into).await })
    }

    fn get_bso_timestamp(
        &self,
        param: params::GetBsoTimestamp,
    ) -> DbFuture<results::GetBsoTimestamp> {
        let db = self.clone();
        Box::pin(async move { db.get_bso_timestamp_async(param).map_err(Into::into).await })
    }

    #[cfg(not(test))]
    fn put_bso(&self, param: params::PutBso) -> DbFuture<results::PutBso> {
        let db = self.clone();
        Box::pin(async move { db.put_bso_async(param).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn put_bso(&self, param: params::PutBso) -> DbFuture<results::PutBso> {
        let db = self.clone();
        Box::pin(async move { db.put_bso_async_test(param).map_err(Into::into).await })
    }

    #[cfg(not(test))]
    fn post_bsos(&self, param: params::PostBsos) -> DbFuture<results::PostBsos> {
        let db = self.clone();
        Box::pin(async move { db.post_bsos_async(param).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn post_bsos(&self, param: params::PostBsos) -> DbFuture<results::PostBsos> {
        let db = self.clone();
        Box::pin(async move { db.post_bsos_async_test(param).map_err(Into::into).await })
    }

    fn validate_batch_id(&self, id: String) -> Result<()> {
        batch::validate_batch_id(&id)
    }

    fn create_batch(&self, param: params::CreateBatch) -> DbFuture<results::CreateBatch> {
        let db = self.clone();
        Box::pin(async move { batch::create_async(&db, param).map_err(Into::into).await })
    }

    fn validate_batch(&self, param: params::ValidateBatch) -> DbFuture<results::ValidateBatch> {
        let db = self.clone();
        Box::pin(async move { batch::validate_async(&db, param).map_err(Into::into).await })
    }

    fn append_to_batch(&self, param: params::AppendToBatch) -> DbFuture<results::AppendToBatch> {
        let db = self.clone();
        Box::pin(async move { batch::append_async(&db, param).map_err(Into::into).await })
    }

    fn get_batch(&self, param: params::GetBatch) -> DbFuture<Option<results::GetBatch>> {
        let db = self.clone();
        Box::pin(async move { batch::get_async(&db, param).map_err(Into::into).await })
    }

    fn commit_batch(&self, param: params::CommitBatch) -> DbFuture<results::CommitBatch> {
        let db = self.clone();
        Box::pin(async move { batch::commit_async(&db, param).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn get_collection_id(&self, name: String) -> DbFuture<i32> {
        let db = self.clone();
        Box::pin(async move { db.get_collection_id_async(&name).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn create_collection(&self, name: String) -> DbFuture<i32> {
        let db = self.clone();
        Box::pin(async move { db.create_collection_async(&name).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn touch_collection(&self, param: params::TouchCollection) -> DbFuture<SyncTimestamp> {
        let db = self.clone();
        Box::pin(async move {
            db.touch_collection_async(&param.user_id, param.collection_id)
                .map_err(Into::into)
                .await
        })
    }

    #[cfg(test)]
    fn timestamp(&self) -> SyncTimestamp {
        self.timestamp()
            .expect("set_timestamp() not called yet for SpannerDb")
    }

    #[cfg(test)]
    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        SpannerDb::set_timestamp(self, timestamp)
    }

    #[cfg(test)]
    fn delete_batch(&self, param: params::DeleteBatch) -> DbFuture<results::DeleteBatch> {
        let db = self.clone();
        Box::pin(async move { batch::delete_async(&db, param).map_err(Into::into).await })
    }

    #[cfg(test)]
    fn clear_coll_cache(&self) {
        self.coll_cache.clear();
    }
}
