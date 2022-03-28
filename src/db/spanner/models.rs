use std::{cell::RefCell, collections::HashMap, convert::TryInto, fmt, ops::Deref, sync::Arc};

use async_trait::async_trait;
use futures::executor;
use google_cloud_rust_raw::spanner::v1::{
    mutation::{Mutation, Mutation_Write},
    spanner::{BeginTransactionRequest, CommitRequest, ExecuteSqlRequest, RollbackRequest},
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

use crate::{
    db::{
        error::{DbError, DbErrorKind},
        params, results,
        spanner::now,
        util::SyncTimestamp,
        Db, Sorting, FIRST_CUSTOM_COLLECTION_ID,
    },
    error::ApiResult,
    server::metrics::Metrics,
    settings::Quota,
    web::{
        extractors::{BsoQueryParams, HawkIdentifier, Offset},
        tags::Tags,
    },
};

use super::{
    batch,
    pool::{CollectionCache, Conn},
    support::{
        as_type, bso_from_row, ExecuteSqlRequestBuilder, StreamedResultSetAsync, ToSpannerValue,
    },
};

#[cfg(not(test))]
use super::support::{bso_to_insert_row, bso_to_update_row};
#[cfg(not(test))]
use std::collections::HashSet;

#[derive(Debug, Eq, PartialEq)]
pub enum CollectionLock {
    Read,
    Write,
}

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
    /// Whether update_collection has already been called
    updated_collection: bool,
}

#[derive(Clone, Debug)]
pub struct SpannerDb {
    pub(super) inner: Arc<SpannerDbInner>,

    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    pub metrics: Metrics,
    pub quota: Quota,
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
    pub fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: Quota,
    ) -> Self {
        let inner = SpannerDbInner {
            conn,
            session: RefCell::new(Default::default()),
        };
        SpannerDb {
            inner: Arc::new(inner),
            coll_cache,
            metrics: metrics.clone(),
            quota,
        }
    }

    pub(super) async fn get_collection_name(&self, id: i32) -> Option<String> {
        self.coll_cache.get_name(id).await
    }

    async fn create_collection(&self, name: String) -> ApiResult<i32> {
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
        let (sqlparams, sqlparam_types) = params! {
            "name" => name,
            "collection_id" => id,
        };

        self.sql(
            "INSERT INTO collections (collection_id, name)
             VALUES (@collection_id, @name)",
        )?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml_async(&self.conn)
        .await?;
        Ok(id)
    }

    async fn get_or_create_collection_id(&self, name: String) -> ApiResult<i32> {
        match self.get_collection_id(name.clone()).await {
            Err(e) if e.is_collection_not_found() => self.create_collection(name).await,
            result => result,
        }
    }

    fn set_timestamp(&self, timestamp: SyncTimestamp) {
        self.session.borrow_mut().timestamp = Some(timestamp);
    }

    // pub(super) fn begin(&self, for_write: bool) -> ApiResult<()> {
    //     let spanner = &self.conn;
    //     let mut options = TransactionOptions::new();
    //     if for_write {
    //         options.set_read_write(TransactionOptions_ReadWrite::new());
    //         self.session.borrow_mut().in_write_transaction = true;
    //     } else {
    //         options.set_read_only(TransactionOptions_ReadOnly::new());
    //     }
    //     let mut req = BeginTransactionRequest::new();
    //     req.set_session(spanner.session.get_name().to_owned());
    //     req.set_options(options);
    //     let mut transaction = spanner.client.begin_transaction(&req)?;

    //     let mut ts = TransactionSelector::new();
    //     ts.set_id(transaction.take_id());
    //     self.session.borrow_mut().transaction = Some(ts);
    //     Ok(())
    // }

    // /// Return the current transaction metadata (TransactionSelector) if one is active.
    // fn get_transaction(&self) -> ApiResult<Option<TransactionSelector>> {
    //     if self.session.borrow().transaction.is_none() {
    //         self.begin(true)?;
    //     }

    //     Ok(self.session.borrow().transaction.clone())
    // }

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    async fn get_transaction(&self) -> ApiResult<Option<TransactionSelector>> {
        if self.session.borrow().transaction.is_none() {
            self.begin(true).await?;
        }

        Ok(self.session.borrow().transaction.clone())
    }

    fn sql_request(&self, sql: &str) -> ApiResult<ExecuteSqlRequest> {
        let mut sqlr = ExecuteSqlRequest::new();
        sqlr.set_sql(sql.to_owned());
        if let Some(transaction) = executor::block_on(self.get_transaction())? {
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

    pub(super) fn sql(&self, sql: &str) -> ApiResult<ExecuteSqlRequestBuilder> {
        Ok(ExecuteSqlRequestBuilder::new(self.sql_request(sql)?))
    }

    #[cfg(not(test))]
    pub(super) fn insert(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_insert(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(Vec::new)
            .push(mutation);
    }

    #[cfg(not(test))]
    pub(super) fn update(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_update(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(Vec::new)
            .push(mutation);
    }

    #[allow(unused)]
    pub(super) fn insert_or_update(&self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_insert_or_update(self.mutation_write(table, columns, values));
        self.session
            .borrow_mut()
            .mutations
            .get_or_insert_with(Vec::new)
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

    async fn map_collection_names<T>(
        &self,
        by_id: HashMap<i32, T>,
    ) -> ApiResult<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys()).await?;
        by_id
            .into_iter()
            .map(|(id, value)| {
                names
                    .remove(&id)
                    .map(|name| (name, value))
                    .ok_or_else(|| DbError::internal("load_collection_names get"))
            })
            .collect::<Result<HashMap<String, T>, DbError>>()
            .map_err(Into::into)
    }

    async fn load_collection_names(
        &self,
        collection_ids: impl Iterator<Item = &i32>,
    ) -> ApiResult<HashMap<i32, String>> {
        let (mut names, uncached) = self
            .coll_cache
            .get_names(&collection_ids.cloned().collect::<Vec<_>>())
            .await;

        if !uncached.is_empty() {
            let mut params = HashMap::new();
            params.insert(
                "ids".to_owned(),
                uncached
                    .into_iter()
                    .map(|id| id.to_string())
                    .collect::<Vec<String>>()
                    .to_spanner_value(),
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
                    self.coll_cache.put(id, name).await;
                }
            }
        }

        Ok(names)
    }

    pub async fn get_collection_counts_async(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> ApiResult<results::GetCollectionCounts> {
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
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

    pub async fn update_user_collection_quotas(
        &self,
        user: &HawkIdentifier,
        collection_id: i32,
    ) -> ApiResult<SyncTimestamp> {
        // This will also update the counts in user_collections, since `update_collection_sync`
        // is called very early to ensure the record exists, and return the timestamp.
        // This will also write the tombstone if there are no records and we're explicitly
        // specifying a TOMBSTONE collection_id.
        // This function should be called after any write operation.
        let timestamp = self.timestamp()?;
        let (mut sqlparams, mut sqltypes) = params! {
            "fxa_uid" => user.fxa_uid.clone(),
            "fxa_kid" => user.fxa_kid.clone(),
            "collection_id" => collection_id,
            "modified" => timestamp.as_rfc3339()?,
        };
        sqltypes.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));

        self.metrics
            .clone()
            .start_timer("storage.quota.update_existing_totals", None);
        let calc_sql = if self.quota.enabled {
            "SELECT SUM(BYTE_LENGTH(payload)), COUNT(*)
            FROM bsos
           WHERE fxa_uid = @fxa_uid
             AND fxa_kid = @fxa_kid
             AND collection_id = @collection_id
           GROUP BY fxa_uid"
        } else {
            "SELECT COUNT(*)
            FROM bsos
           WHERE fxa_uid = @fxa_uid
             AND fxa_kid = @fxa_kid
             AND collection_id = @collection_id
           GROUP BY fxa_uid"
        };

        let result = {
            let (sqlparams, sqlparam_types) = params! {
                "fxa_uid" => user.fxa_uid.clone(),
                "fxa_kid" => user.fxa_kid.clone(),
                "collection_id" => collection_id,
            };

            self.sql(calc_sql)?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_async(&self.conn)?
                .one_or_none()
                .await?
        };
        let set_sql = if let Some(mut result) = result {
            // Update the user_collections table to reflect current numbers.
            // If there are BSOs, there are user_collections (or else something
            // really bad already happened.)
            if self.quota.enabled {
                sqlparams.insert(
                    "total_bytes".to_owned(),
                    result[0].take_string_value().to_spanner_value(),
                );
                sqlparams.insert(
                    "count".to_owned(),
                    result[1].take_string_value().to_spanner_value(),
                );
                sqltypes.insert(
                    "total_bytes".to_owned(),
                    crate::db::spanner::support::as_type(TypeCode::INT64),
                );
                sqltypes.insert(
                    "count".to_owned(),
                    crate::db::spanner::support::as_type(TypeCode::INT64),
                );
                "UPDATE user_collections
                SET modified = @modified,
                    count = @count,
                    total_bytes = @total_bytes
                WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id"
            } else {
                "UPDATE user_collections
                SET modified = @modified
                WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id"
            }
        } else {
            // Otherwise, there are no BSOs that match, check to see if there are
            // any collections.
            let result = self
                .sql(
                    "SELECT 1 FROM user_collections
                WHERE fxa_uid=@fxa_uid AND fxa_kid=@fxa_kid AND collection_id=@collection_id",
                )?
                .params(sqlparams.clone())
                .param_types(sqltypes.clone())
                .execute_async(&self.conn)?
                .one_or_none()
                .await?;
            if result.is_none() {
                // No collections, so insert what we've got.
                if self.quota.enabled {
                    "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified, total_bytes, count)
                    VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified, 0, 0)"
                } else {
                    "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
                    VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)"
                }
            } else {
                // there are collections, best modify what's there.
                // NOTE, tombstone is a single collection id, it would have been created above.
                if self.quota.enabled {
                    "UPDATE user_collections SET modified=@modified, total_bytes=0, count=0
                    WHERE fxa_uid=@fxa_uid AND fxa_kid=@fxa_kid AND collection_id=@collection_id"
                } else {
                    "UPDATE user_collections SET modified=@modified
                    WHERE fxa_uid=@fxa_uid AND fxa_kid=@fxa_kid AND collection_id=@collection_id"
                }
            }
        };
        self.sql(set_sql)?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute_dml_async(&self.conn)
            .await?;
        Ok(timestamp)
    }

    async fn erect_tombstone(&self, user_id: &HawkIdentifier) -> ApiResult<SyncTimestamp> {
        // Delete the old tombstone (if it exists)
        let (params, mut param_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => TOMBSTONE,
            "modified" => self.timestamp()?.as_rfc3339()?
        };
        param_types.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id",
        )?
        .params(params.clone())
        .param_types(param_types.clone())
        .execute_dml_async(&self.conn)
        .await?;
        self.update_user_collection_quotas(user_id, TOMBSTONE)
            .await?;
        // Return timestamp, because sometimes there's a delay between writing and
        // reading the database.
        Ok(self.timestamp()?)
    }

    pub fn timestamp(&self) -> ApiResult<SyncTimestamp> {
        self.session
            .borrow()
            .timestamp
            .ok_or_else(|| DbError::internal("CURRENT_TIMESTAMP() not read yet").into())
    }

    pub(super) async fn update_collection(
        &self,
        user_id: &HawkIdentifier,
        collection_id: i32,
        collection: &str,
    ) -> ApiResult<SyncTimestamp> {
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
        if !cfg!(test) && self.session.borrow().updated_collection {
            // No need to touch it again (except during tests where we
            // currently reuse Dbs for multiple requests)
            return Ok(timestamp);
        }

        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
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
            )?
            .params(sqlparams.clone())
            .param_types(sqlparam_types.clone())
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        if result.is_some() {
            let sql = "UPDATE user_collections
                    SET modified = @modified
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id";
            self.sql(sql)?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_dml_async(&self.conn)
                .await?;
        } else {
            let mut tags = Tags::default();
            tags.tags
                .insert("collection".to_owned(), collection.to_owned());
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
            self.sql(update_sql)?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_dml_async(&self.conn)
                .await?;
        }
        self.session.borrow_mut().updated_collection = true;
        Ok(timestamp)
    }

    async fn bsos_query(
        &self,
        query_str: &str,
        params: params::GetBsos,
    ) -> ApiResult<StreamedResultSetAsync> {
        let mut query = query_str.to_owned();
        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => self.get_collection_id(params.collection.clone()).await?,
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

        if !ids.is_empty() {
            query = format!("{} AND bso_id IN UNNEST(@ids)", query);
            sqlparams.insert("ids".to_owned(), ids.to_spanner_value());
            sqlparam_types.insert("ids".to_owned(), ids.spanner_type());
        }

        // issue559: Dead code (timestamp always None)
        /*
        if let Some(timestamp) = offset.clone().unwrap_or_default().timestamp {
            query = match sort {
                Sorting::Newest => {
                    sqlparams.insert(
                        "older_eq".to_string(),
                        timestamp.as_rfc3339()?.to_spanner_value(),
                    );
                    sqlparam_types.insert("older_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified <= @older_eq", query)
                }
                Sorting::Oldest => {
                    sqlparams.insert(
                        "newer_eq".to_string(),
                        timestamp.as_rfc3339()?.to_spanner_value(),
                    );
                    sqlparam_types.insert("newer_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified >= @newer_eq", query)
                }
                _ => query,
            };
        }
        */
        if let Some(older) = older {
            query = format!("{} AND modified < @older", query);
            sqlparams.insert("older".to_string(), older.as_rfc3339()?.to_spanner_value());
            sqlparam_types.insert("older".to_string(), as_type(TypeCode::TIMESTAMP));
        }
        if let Some(newer) = newer {
            query = format!("{} AND modified > @newer", query);
            sqlparams.insert("newer".to_string(), newer.as_rfc3339()?.to_spanner_value());
            sqlparam_types.insert("newer".to_string(), as_type(TypeCode::TIMESTAMP));
        }

        if self.stabilize_bsos_sort_order() {
            query = match sort {
                Sorting::Index => format!("{} ORDER BY sortindex DESC, bso_id DESC", query),
                Sorting::Newest | Sorting::None => {
                    format!("{} ORDER BY modified DESC, bso_id DESC", query)
                }
                Sorting::Oldest => format!("{} ORDER BY modified ASC, bso_id ASC", query),
            };
        } else {
            query = match sort {
                Sorting::Index => format!("{} ORDER BY sortindex DESC", query),
                Sorting::Newest => format!("{} ORDER BY modified DESC", query),
                Sorting::Oldest => format!("{} ORDER BY modified ASC", query),
                _ => query,
            };
        }

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
            query = format!(
                "{} LIMIT {}",
                query,
                i64::max_value() - offset.offset as i64
            );
        };

        if let Some(offset) = offset {
            query = format!("{} OFFSET {}", query, offset.offset);
        }
        self.sql(&query)?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)
            .map_err(Into::into)
    }

    /// Whether to stabilize the sort order for get_bsos_async
    fn stabilize_bsos_sort_order(&self) -> bool {
        self.inner.conn.using_spanner_emulator
    }

    pub fn encode_next_offset(
        &self,
        _sort: Sorting,
        offset: u64,
        _timestamp: Option<i64>,
        modifieds: Vec<i64>,
    ) -> Option<String> {
        // issue559: Use a simple numeric offset everwhere as previously for
        // now: was previously a value of "limit + offset", modifieds.len()
        // always equals limit
        Some(
            Offset {
                offset: offset + modifieds.len() as u64,
                timestamp: None,
            }
            .to_string(),
        )
        /*
        let mut calc_offset = 1;
        let mut i = (modifieds.len() as i64) - 2;

        let prev_bound = match sort {
            Sorting::Index => {
                // Use a simple numeric offset for sortindex ordering.
                return Some(
                    Offset {
                        offset: offset + modifieds.len() as u64,
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
        */
    }

    pub fn quota_error(&self, collection: &str) -> DbError {
        // return the over quota error.
        let mut tags = Tags::default();
        tags.tags
            .insert("collection".to_owned(), collection.to_owned());
        self.metrics
            .incr_with_tags("storage.quota.at_limit", Some(tags));
        DbErrorKind::Quota.into()
    }

    pub async fn check_quota(
        &self,
        user_id: &HawkIdentifier,
        collection: &str,
        collection_id: i32,
    ) -> ApiResult<Option<usize>> {
        // duplicate quota trap in test func below.
        if !self.quota.enabled {
            return Ok(None);
        }
        let usage = self
            .get_quota_usage(params::GetQuotaUsage {
                user_id: user_id.clone(),
                collection: collection.to_owned(),
                collection_id,
            })
            .await?;
        if usage.total_bytes >= self.quota.size {
            if self.quota.enforced {
                return Err(self.quota_error(collection).into());
            } else {
                warn!("Quota at limit for user's collection: ({} bytes)", usage.total_bytes; "collection"=>collection);
            }
        }
        Ok(Some(usage.total_bytes as usize))
    }
}

#[async_trait(?Send)]
impl Db for SpannerDb {
    async fn commit(&self) -> ApiResult<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        let spanner = &self.conn;

        if cfg!(test) && spanner.use_test_transactions {
            // don't commit test transactions
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction().await? {
            let mut req = CommitRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            if let Some(mutations) = self.session.borrow_mut().mutations.take() {
                req.set_mutations(RepeatedField::from_vec(mutations));
            }
            spanner.client.commit_async(&req)?.await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to commit"))?
        }
    }

    async fn rollback(&self) -> ApiResult<()> {
        if !self.in_write_transaction() {
            // read-only
            return Ok(());
        }

        if let Some(transaction) = self.get_transaction().await? {
            let spanner = &self.conn;
            let mut req = RollbackRequest::new();
            req.set_session(spanner.session.get_name().to_owned());
            req.set_transaction_id(transaction.get_id().to_vec());
            spanner.client.rollback_async(&req)?.await?;
            Ok(())
        } else {
            Err(DbError::internal("No transaction to rollback"))?
        }
    }

    async fn lock_for_read(&self, params: params::LockCollection) -> ApiResult<()> {
        // Begin a transaction
        self.begin(false).await?;

        let collection_id = self
            .get_collection_id(params.collection.clone())
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

    async fn lock_for_write(&self, params: params::LockCollection) -> ApiResult<()> {
        // Begin a transaction
        self.begin(true).await?;
        let collection_id = self
            .get_or_create_collection_id(params.collection.clone())
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
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

    async fn begin(&self, for_write: bool) -> ApiResult<()> {
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
        let mut transaction = spanner.client.begin_transaction_async(&req)?.await?;

        let mut ts = TransactionSelector::new();
        ts.set_id(transaction.take_id());
        self.session.borrow_mut().transaction = Some(ts);
        Ok(())
    }

    async fn get_collection_timestamp(
        &self,
        params: params::GetCollectionTimestamp,
    ) -> ApiResult<results::GetCollectionTimestamp> {
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        if let Some(modified) = self
            .session
            .borrow()
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?
            .one_or_none()
            .await?
            .ok_or(DbErrorKind::CollectionNotFound)?;
        let modified = SyncTimestamp::from_rfc3339(result[0].get_string_value())?;
        Ok(modified)
    }

    async fn get_storage_timestamp(
        &self,
        params: params::GetStorageTimestamp,
    ) -> ApiResult<results::GetStorageTimestamp> {
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.fxa_uid,
            "fxa_kid" => params.fxa_kid,
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?
            .one()
            .await?;
        if row[0].has_null_value() {
            SyncTimestamp::from_i64(0).map_err(Into::into)
        } else {
            SyncTimestamp::from_rfc3339(row[0].get_string_value()).map_err(Into::into)
        }
    }

    async fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> ApiResult<results::DeleteCollection> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id.clone(),
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml_async(&self.conn)
            .await?;
        if affected_rows > 0 {
            let mut tags = Tags::default();
            tags.tags
                .insert("collection".to_string(), params.collection.clone());
            self.metrics
                .incr_with_tags("storage.spanner.delete_collection", Some(tags));
            self.erect_tombstone(&params.user_id).await
        } else {
            self.get_storage_timestamp(params.user_id).await
        }
    }

    async fn check(&self) -> ApiResult<results::Check> {
        // TODO: is there a better check than just fetching UTC?
        self.sql("SELECT CURRENT_TIMESTAMP()")?
            .execute_async(&self.conn)?
            .one()
            .await?;
        Ok(true)
    }

    async fn get_collection_timestamps(
        &self,
        user_id: params::GetCollectionTimestamps,
    ) -> ApiResult<results::GetCollectionTimestamps> {
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?;
        let mut results = HashMap::new();
        while let Some(row) = streaming.next_async().await {
            let row = row?;
            let collection_id = row[0]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            let modified = SyncTimestamp::from_rfc3339(row[1].get_string_value())?;
            results.insert(collection_id, modified);
        }
        self.map_collection_names(results).await
    }

    async fn get_collection_counts(
        &self,
        user_id: params::GetCollectionCounts,
    ) -> ApiResult<results::GetCollectionCounts> {
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
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

    async fn get_collection_usage(
        &self,
        user_id: params::GetCollectionUsage,
    ) -> ApiResult<results::GetCollectionUsage> {
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
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

    async fn get_storage_usage(
        &self,
        params: params::GetStorageUsage,
    ) -> ApiResult<results::GetStorageUsage> {
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.fxa_uid,
            "fxa_kid" => params.fxa_kid
        };
        let result = self
            .sql(
                "SELECT SUM(BYTE_LENGTH(payload))
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND expiry > CURRENT_TIMESTAMP()
                  GROUP BY fxa_uid",
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
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

    async fn get_quota_usage(
        &self,
        params: params::GetQuotaUsage,
    ) -> ApiResult<results::GetQuotaUsage> {
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
            .sql(check_sql)?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            let total_bytes = if self.quota.enabled {
                result[0]
                    .get_string_value()
                    .parse::<usize>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?
            } else {
                0
            };
            let count = result[1]
                .get_string_value()
                .parse::<i32>()
                .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
            Ok(results::GetQuotaUsage { total_bytes, count })
        } else {
            Ok(results::GetQuotaUsage::default())
        }
    }

    async fn delete_storage(
        &self,
        params: params::DeleteStorage,
    ) -> ApiResult<results::DeleteStorage> {
        // Also deletes child bsos/batch rows (INTERLEAVE IN PARENT
        // user_collections ON DELETE CASCADE)
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.fxa_uid,
            "fxa_kid" => params.fxa_kid
        };
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid",
        )?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml_async(&self.conn)
        .await?;
        Ok(())
    }

    async fn delete_bso(&self, params: params::DeleteBso) -> ApiResult<results::DeleteBso> {
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml_async(&self.conn)
            .await?;
        if affected_rows == 0 {
            Err(DbErrorKind::BsoNotFound)?
        } else {
            self.metrics.incr("storage.spanner.delete_bso");
            Ok(self
                .update_user_collection_quotas(&user_id, collection_id)
                .await?)
        }
    }

    async fn delete_bsos(&self, params: params::DeleteBsos) -> ApiResult<results::DeleteBsos> {
        let user_id = params.user_id.clone();
        let collection_id = self.get_collection_id(params.collection.clone()).await?;

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
        )?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml_async(&self.conn)
        .await?;
        let mut tags = Tags::default();
        tags.tags
            .insert("collection".to_string(), params.collection.clone());
        self.metrics
            .incr_with_tags("self.storage.delete_bsos", Some(tags));
        self.update_user_collection_quotas(&params.user_id, collection_id)
            .await
    }

    async fn get_bsos(&self, params: params::GetBsos) -> ApiResult<results::GetBsos> {
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

        let mut streaming = self.bsos_query(query, params).await?;
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

    async fn get_bso_ids(&self, params: params::GetBsoIds) -> ApiResult<results::GetBsoIds> {
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
        let mut stream = self.bsos_query(query, params).await?;

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

    async fn get_bso(&self, params: params::GetBso) -> ApiResult<Option<results::GetBso>> {
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
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
        )?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_async(&self.conn)?
        .one_or_none()
        .await?
        .map(bso_from_row)
        .transpose()
        .map_err(Into::into)
    }

    async fn get_bso_timestamp(
        &self,
        params: params::GetBsoTimestamp,
    ) -> ApiResult<results::GetBsoTimestamp> {
        let collection_id = self.get_collection_id(params.collection.clone()).await?;
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
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?
            .one_or_none()
            .await?;
        if let Some(result) = result {
            SyncTimestamp::from_rfc3339(result[0].get_string_value()).map_err(Into::into)
        } else {
            SyncTimestamp::from_i64(0).map_err(Into::into)
        }
    }

    #[cfg(not(test))]
    async fn put_bso(&self, params: params::PutBso) -> ApiResult<results::PutBso> {
        let bsos = vec![params::PostCollectionBso {
            id: params.id,
            sortindex: params.sortindex,
            payload: params.payload,
            ttl: params.ttl,
        }];
        let result = self
            .post_bsos(params::PostBsos {
                user_id: params.user_id,
                collection: params.collection,
                bsos,
                for_batch: false,
                failed: HashMap::new(),
            })
            .await?;

        Ok(result.modified)
    }

    // NOTE: Currently this put_bso_async_test impl. is only used during db tests,
    // see above for the non-tests version
    #[cfg(test)]
    async fn put_bso(&self, bso: params::PutBso) -> ApiResult<results::PutBso> {
        use crate::db::util::to_rfc3339;
        let collection_id = self
            .get_or_create_collection_id(bso.collection.clone())
            .await?;

        self.check_quota(&bso.user_id, &bso.collection, collection_id)
            .await?;

        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => bso.user_id.fxa_uid.clone(),
            "fxa_kid" => bso.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "bso_id" => bso.id,
        };
        // prewarm the collections table by ensuring that the row is added if not present.
        self.update_collection(&bso.user_id, collection_id, &bso.collection)
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
            .param_types(sqlparam_types.clone())
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
                    sqlparams.insert("sortindex".to_string(), sortindex.to_spanner_value());
                    sqlparam_types.insert("sortindex".to_string(), sortindex.spanner_type());

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
                    sqlparams.insert("expiry".to_string(), to_rfc3339(expiry)?.to_spanner_value());
                    sqlparam_types.insert("expiry".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{}{}", comma(&q), "expiry = @expiry")
                } else {
                    "".to_string()
                }
            );

            q = format!(
                "{}{}",
                q,
                if bso.payload.is_some() || bso.sortindex.is_some() {
                    sqlparams.insert(
                        "modified".to_string(),
                        timestamp.as_rfc3339()?.to_spanner_value(),
                    );
                    sqlparam_types.insert("modified".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{}{}", comma(&q), "modified = @modified")
                } else {
                    "".to_string()
                }
            );

            q = format!(
                "{}{}",
                q,
                if let Some(payload) = bso.payload {
                    sqlparams.insert("payload".to_string(), payload.to_spanner_value());
                    sqlparam_types.insert("payload".to_string(), payload.spanner_type());
                    format!("{}{}", comma(&q), "payload = @payload")
                } else {
                    "".to_string()
                }
            );

            if q.is_empty() {
                // Nothing to update
                return Ok(timestamp);
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
                    .map(|sortindex| sortindex.to_spanner_value())
                    .unwrap_or_else(null_value);
                sqlparams.insert("sortindex".to_string(), sortindex);
                sqlparam_types.insert("sortindex".to_string(), as_type(TypeCode::INT64));
            }
            let payload = bso.payload.unwrap_or_else(|| "".to_owned());
            sqlparams.insert("payload".to_string(), payload.to_spanner_value());
            sqlparam_types.insert("payload".to_owned(), payload.spanner_type());
            let now_millis = timestamp.as_i64();
            let ttl = bso.ttl.map_or(i64::from(DEFAULT_BSO_TTL), |ttl| {
                ttl.try_into()
                    .expect("Could not get ttl in put_bso_async_test")
            }) * 1000;
            let expirystring = to_rfc3339(now_millis + ttl)?;
            debug!(
                "!!!!! [test] INSERT expirystring:{:?}, timestamp:{:?}, ttl:{:?}",
                &expirystring, timestamp, ttl
            );
            sqlparams.insert("expiry".to_string(), expirystring.to_spanner_value());
            sqlparam_types.insert("expiry".to_string(), as_type(TypeCode::TIMESTAMP));

            sqlparams.insert(
                "modified".to_string(),
                timestamp.as_rfc3339()?.to_spanner_value(),
            );
            sqlparam_types.insert("modified".to_string(), as_type(TypeCode::TIMESTAMP));
            sql.to_owned()
        };

        self.sql(&sql)?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml_async(&self.conn)
            .await?;
        // update the counts for the user_collections table.
        self.update_user_collection_quotas(&bso.user_id, collection_id)
            .await
    }

    #[cfg(not(test))]
    async fn post_bsos(&self, params: params::PostBsos) -> ApiResult<results::PostBsos> {
        let user_id = params.user_id;
        let collection_id = self
            .get_or_create_collection_id(params.collection.clone())
            .await?;

        if !params.for_batch {
            self.check_quota(&user_id, &params.collection, collection_id)
                .await?;
        }

        // Ensure a parent record exists in user_collections before writing to
        // bsos (INTERLEAVE IN PARENT user_collections)
        let timestamp = self
            .update_collection(&user_id, collection_id, &params.collection)
            .await?;

        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "ids" => params
                .bsos
                .iter()
                .map(|pbso| pbso.id.clone())
                .collect::<Vec<String>>(),
        };
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
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?;
        let mut existing = HashSet::new();
        while let Some(row) = streaming.next_async().await {
            let mut row = row?;
            existing.insert(row[0].take_string_value());
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
                updates.entry(columns).or_insert_with(Vec::new).push(values);
            } else {
                let values = bso_to_insert_row(&user_id, collection_id, bso, timestamp)?;
                load_size += values.compute_size() as usize;
                inserts.push(values);
            }
        }
        if load_size > MAX_SPANNER_LOAD_SIZE {
            self.metrics.clone().incr("error.tooMuchData");
            trace!(
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
            self.update("bsos", &columns, values);
        }
        if !params.for_batch {
            // update the quotas
            self.update_user_collection_quotas(&user_id, collection_id)
                .await?;
        }

        let result = results::PostBsos {
            modified: timestamp,
            success,
            failed: params.failed,
        };
        Ok(result)
    }

    // NOTE: Currently this post_bso_async_test impl. is only used during db tests,
    // see above for the non-tests version
    #[cfg(test)]
    async fn post_bsos(&self, params: params::PostBsos) -> ApiResult<results::PostBsos> {
        let collection_id = self
            .get_or_create_collection_id(params.collection.clone())
            .await?;
        let mut result = results::PostBsos {
            modified: self.timestamp()?,
            success: Default::default(),
            failed: params.failed,
        };

        for pbso in params.bsos {
            let id = pbso.id;
            self.put_bso(params::PutBso {
                user_id: params.user_id.clone(),
                collection: params.collection.clone(),
                id: id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            })
            .await?;
            result.success.push(id);
        }
        self.update_user_collection_quotas(&params.user_id, collection_id)
            .await?;
        Ok(result)
    }

    async fn create_batch(&self, param: params::CreateBatch) -> ApiResult<results::CreateBatch> {
        batch::create(self, param).await.map_err(Into::into)
    }

    async fn validate_batch(
        &self,
        param: params::ValidateBatch,
    ) -> ApiResult<results::ValidateBatch> {
        batch::validate(self, param).await.map_err(Into::into)
    }

    async fn append_to_batch(
        &self,
        param: params::AppendToBatch,
    ) -> ApiResult<results::AppendToBatch> {
        batch::append(self, param).await.map_err(Into::into)
    }

    async fn get_batch(&self, param: params::GetBatch) -> ApiResult<Option<results::GetBatch>> {
        batch::get(self, param).await.map_err(Into::into)
    }

    async fn commit_batch(&self, param: params::CommitBatch) -> ApiResult<results::CommitBatch> {
        batch::commit(self, param).await.map_err(Into::into)
    }

    async fn get_collection_id(&self, name: String) -> ApiResult<i32> {
        if let Some(id) = self.coll_cache.get_id(&name).await {
            return Ok(id);
        }
        let (sqlparams, sqlparam_types) = params! { "name" => name.to_string() };
        let result = self
            .sql(
                "SELECT collection_id
                   FROM collections
                  WHERE name = @name",
            )?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_async(&self.conn)?
            .one_or_none()
            .await?
            .ok_or(DbErrorKind::CollectionNotFound)?;
        let id = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbErrorKind::Integrity(e.to_string()))?;
        if !self.in_write_transaction() {
            self.coll_cache.put(id, name.to_owned()).await;
        }
        Ok(id)
    }

    fn get_connection_info(&self) -> results::ConnectionInfo {
        let session = self.conn.session.clone();
        let now = now();
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

    #[cfg(test)]
    async fn create_collection(&self, name: String) -> ApiResult<i32> {
        self.create_collection(name).await
    }

    #[cfg(test)]
    async fn update_collection(&self, param: params::UpdateCollection) -> ApiResult<SyncTimestamp> {
        self.update_collection(&param.user_id, param.collection_id, &param.collection)
            .await
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
    async fn delete_batch(&self, param: params::DeleteBatch) -> ApiResult<results::DeleteBatch> {
        batch::delete_async(self, param).await.map_err(Into::into)
    }

    #[cfg(test)]
    async fn clear_coll_cache(&self) -> ApiResult<()> {
        let db = self.clone();
        db.coll_cache.clear().await;
        Ok(())
    }

    #[cfg(test)]
    fn set_quota(&mut self, enabled: bool, limit: usize, enforced: bool) {
        self.quota = Quota {
            size: limit,
            enabled,
            enforced,
        };
    }
}
