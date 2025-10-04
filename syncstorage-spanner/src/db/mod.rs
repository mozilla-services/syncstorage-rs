use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    fmt,
    sync::Arc,
};

use google_cloud_rust_raw::spanner::v1::{
    mutation::{Mutation, Mutation_Write},
    spanner::ExecuteSqlRequest,
    transaction::TransactionSelector,
    type_pb::TypeCode,
};
#[allow(unused_imports)]
use protobuf::{
    well_known_types::{ListValue, Value},
    Message, RepeatedField,
};
use syncserver_common::{Metrics, MAX_SPANNER_LOAD_SIZE};
use syncstorage_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, Db, Sorting, UserIdentifier,
    DEFAULT_BSO_TTL,
};
use syncstorage_settings::Quota;

use crate::{
    error::DbError,
    pool::{CollectionCache, Conn},
    DbResult,
};
use support::{
    as_type, bso_to_insert_row, bso_to_update_row, ExecuteSqlRequestBuilder, IntoSpannerValue,
    StreamedResultSetAsync,
};

#[derive(Debug, Eq, PartialEq)]
enum CollectionLock {
    Read,
    Write,
}

mod batch_impl;
mod db_impl;
mod stream;
pub(crate) mod support;

pub use batch_impl::validate_batch_id;

const TOMBSTONE: i32 = 0;
pub const PRETOUCH_TS: &str = "0001-01-01T00:00:00.00Z";

pub struct SpannerDb {
    pub(super) conn: Conn,
    session: SpannerDbSession,

    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,

    pub metrics: Metrics,
    pub quota: Quota,
}

impl fmt::Debug for SpannerDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpannerDb")
            .field("session", &self.session)
            .field("coll_cache", &self.coll_cache)
            .field("metrics", &self.metrics)
            .field("quota", &self.quota)
            .finish()
    }
}

/// Per session Db metadata
#[derive(Debug, Default)]
struct SpannerDbSession {
    /// CURRENT_TIMESTAMP() from Spanner, used for timestamping this session's
    /// operations
    timestamp: Option<SyncTimestamp>,
    /// Cache of collection modified timestamps per (HawkIdentifier, collection_id)
    coll_modified_cache: HashMap<(UserIdentifier, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(UserIdentifier, i32), CollectionLock>,
    transaction: Option<TransactionSelector>,
    /// Behind Vec so commit can take() it (maybe commit() should consume self
    /// instead?)
    mutations: Option<Vec<Mutation>>,
    in_write_transaction: bool,
    execute_sql_count: u64,
    /// Whether update_collection has already been called
    updated_collection: bool,
}

impl SpannerDb {
    pub(crate) fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: Quota,
    ) -> Self {
        SpannerDb {
            conn,
            session: Default::default(),
            coll_cache,
            metrics: metrics.clone(),
            quota,
        }
    }

    pub(super) async fn get_collection_name(&self, id: i32) -> Option<String> {
        self.coll_cache.get_name(id).await
    }

    async fn get_or_create_collection_id(&mut self, name: &str) -> DbResult<i32> {
        match self.get_collection_id(name).await {
            Err(err) if err.is_collection_not_found() => self.create_collection(name).await,
            result => result,
        }
    }

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    async fn get_transaction(&mut self) -> DbResult<Option<TransactionSelector>> {
        if self.session.transaction.is_none() {
            self.begin(true).await?;
        }

        Ok(self.session.transaction.clone())
    }

    async fn sql_request(&mut self, sql: &str) -> DbResult<ExecuteSqlRequest> {
        let mut sqlr = ExecuteSqlRequest::new();
        sqlr.set_sql(sql.to_owned());
        if let Some(transaction) = self.get_transaction().await? {
            sqlr.set_transaction(transaction);
            let session = &mut self.session;
            sqlr.seqno = session
                .execute_sql_count
                .try_into()
                .map_err(|_| DbError::internal("seqno overflow".to_owned()))?;
            session.execute_sql_count += 1;
        }
        Ok(sqlr)
    }

    pub(super) async fn sql(&mut self, sql: &str) -> DbResult<ExecuteSqlRequestBuilder> {
        Ok(ExecuteSqlRequestBuilder::new(self.sql_request(sql).await?))
    }

    #[allow(unused)]
    pub(super) fn insert(&mut self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_insert(self.mutation_write(table, columns, values));
        self.session
            .mutations
            .get_or_insert_with(Vec::new)
            .push(mutation);
    }

    #[allow(unused)]
    pub(super) fn update(&mut self, table: &str, columns: &[&str], values: Vec<ListValue>) {
        let mut mutation = Mutation::new();
        mutation.set_update(self.mutation_write(table, columns, values));
        self.session
            .mutations
            .get_or_insert_with(Vec::new)
            .push(mutation);
    }

    #[allow(unused)]
    pub(super) fn insert_or_update(
        &mut self,
        table: &str,
        columns: &[&str],
        values: Vec<ListValue>,
    ) {
        let mut mutation = Mutation::new();
        mutation.set_insert_or_update(self.mutation_write(table, columns, values));
        self.session
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
        self.session.in_write_transaction
    }

    async fn map_collection_names<T>(
        &mut self,
        by_id: HashMap<i32, T>,
    ) -> DbResult<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys()).await?;
        by_id
            .into_iter()
            .map(|(id, value)| {
                names
                    .remove(&id)
                    .map(|name| (name, value))
                    .ok_or_else(|| DbError::internal("load_collection_names get".to_owned()))
            })
            .collect()
    }

    async fn load_collection_names(
        &mut self,
        collection_ids: impl Iterator<Item = &i32>,
    ) -> DbResult<HashMap<i32, String>> {
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
                    .into_spanner_value(),
            );
            let mut rs = self
                .sql(
                    "SELECT collection_id, name
                       FROM collections
                      WHERE collection_id IN UNNEST(@ids)",
                )
                .await?
                .params(params)
                .execute(&self.conn)?;
            while let Some(mut row) = rs.try_next().await? {
                let id = row[0]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbError::integrity(e.to_string()))?;
                let name = row[1].take_string_value();
                names.insert(id, name.clone());
                if !self.in_write_transaction() {
                    self.coll_cache.put(id, name).await;
                }
            }
        }

        Ok(names)
    }

    pub(super) async fn update_user_collection_quotas(
        &mut self,
        user: &UserIdentifier,
        collection_id: i32,
    ) -> DbResult<SyncTimestamp> {
        // This will also update the counts in user_collections, since `update_collection_sync`
        // is called very early to ensure the record exists, and return the timestamp.
        // This will also write the tombstone if there are no records and we're explicitly
        // specifying a TOMBSTONE collection_id.
        // This function should be called after any write operation.
        let timestamp = self.checked_timestamp()?;
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

            self.sql(calc_sql)
                .await?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute(&self.conn)?
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
                    result[0].take_string_value().into_spanner_value(),
                );
                sqlparams.insert(
                    "count".to_owned(),
                    result[1].take_string_value().into_spanner_value(),
                );
                sqltypes.insert("total_bytes".to_owned(), support::as_type(TypeCode::INT64));
                sqltypes.insert("count".to_owned(), support::as_type(TypeCode::INT64));
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
                )
                .await?
                .params(sqlparams.clone())
                .param_types(sqltypes.clone())
                .execute(&self.conn)?
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
        self.sql(set_sql)
            .await?
            .params(sqlparams)
            .param_types(sqltypes)
            .execute_dml(&self.conn)
            .await?;
        Ok(timestamp)
    }

    async fn erect_tombstone(&mut self, user_id: &UserIdentifier) -> DbResult<SyncTimestamp> {
        // Delete the old tombstone (if it exists)
        let (params, mut param_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => TOMBSTONE,
            "modified" => self.checked_timestamp()?.as_rfc3339()?
        };
        param_types.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));
        self.sql(
            "DELETE FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id",
        )
        .await?
        .params(params.clone())
        .param_types(param_types.clone())
        .execute_dml(&self.conn)
        .await?;
        self.update_user_collection_quotas(user_id, TOMBSTONE)
            .await?;
        // Return timestamp, because sometimes there's a delay between writing and
        // reading the database.
        self.checked_timestamp()
    }

    pub fn checked_timestamp(&self) -> DbResult<SyncTimestamp> {
        self.session
            .timestamp
            .ok_or_else(|| DbError::internal("CURRENT_TIMESTAMP() not read yet".to_owned()))
    }

    async fn bsos_query(
        &mut self,
        query_str: &str,
        params: params::GetBsos,
    ) -> DbResult<StreamedResultSetAsync> {
        let mut query = query_str.to_owned();
        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => self.get_collection_id(&params.collection).await?,
        };

        if !params.ids.is_empty() {
            query = format!("{} AND bso_id IN UNNEST(@ids)", query);
            sqlparam_types.insert("ids".to_owned(), params.ids.spanner_type());
            sqlparams.insert("ids".to_owned(), params.ids.into_spanner_value());
        }

        // issue559: Dead code (timestamp always None)
        /*
        if let Some(timestamp) = offset.clone().unwrap_or_default().timestamp {
            query = match sort {
                Sorting::Newest => {
                    sqlparams.insert(
                        "older_eq".to_string(),
                        timestamp.as_rfc3339()?.into_spanner_value(),
                    );
                    sqlparam_types.insert("older_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified <= @older_eq", query)
                }
                Sorting::Oldest => {
                    sqlparams.insert(
                        "newer_eq".to_string(),
                        timestamp.as_rfc3339()?.into_spanner_value(),
                    );
                    sqlparam_types.insert("newer_eq".to_string(), as_type(TypeCode::TIMESTAMP));
                    format!("{} AND modified >= @newer_eq", query)
                }
                _ => query,
            };
        }
        */
        if let Some(older) = params.older {
            query = format!("{} AND modified < @older", query);
            sqlparams.insert(
                "older".to_string(),
                older.as_rfc3339()?.into_spanner_value(),
            );
            sqlparam_types.insert("older".to_string(), as_type(TypeCode::TIMESTAMP));
        }
        if let Some(newer) = params.newer {
            query = format!("{} AND modified > @newer", query);
            sqlparams.insert(
                "newer".to_string(),
                newer.as_rfc3339()?.into_spanner_value(),
            );
            sqlparam_types.insert("newer".to_string(), as_type(TypeCode::TIMESTAMP));
        }

        if self.stabilize_bsos_sort_order() {
            query = match params.sort {
                Sorting::Index => format!("{} ORDER BY sortindex DESC, bso_id DESC", query),
                Sorting::Newest | Sorting::None => {
                    format!("{} ORDER BY modified DESC, bso_id DESC", query)
                }
                Sorting::Oldest => format!("{} ORDER BY modified ASC, bso_id ASC", query),
            };
        } else {
            query = match params.sort {
                Sorting::Index => format!("{} ORDER BY sortindex DESC", query),
                Sorting::Newest => format!("{} ORDER BY modified DESC", query),
                Sorting::Oldest => format!("{} ORDER BY modified ASC", query),
                _ => query,
            };
        }

        if let Some(limit) = params.limit {
            // fetch an extra row to detect if there are more rows that match
            // the query conditions
            query = format!("{} LIMIT {}", query, i64::from(limit) + 1);
        } else if let Some(ref offset) = params.offset {
            // Special case no limit specified but still required for an
            // offset. Spanner doesn't accept a simpler limit of -1 (common in
            // most databases) so we specify a max value with offset subtracted
            // to avoid overflow errors (that only occur w/ a FORCE_INDEX=
            // directive) OutOfRange: 400 int64 overflow: <INT64_MAX> + offset
            query = format!("{} LIMIT {}", query, i64::MAX - offset.offset as i64);
        };

        if let Some(offset) = params.offset {
            query = format!("{} OFFSET {}", query, offset.offset);
        }
        self.sql(&query)
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)
    }

    /// Whether to stabilize the sort order for get_bsos_async
    fn stabilize_bsos_sort_order(&self) -> bool {
        self.conn.settings.using_spanner_emulator()
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
            params::Offset {
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
                    params::Offset {
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

    async fn put_bso_with_mutations(
        &mut self,
        params: params::PutBso,
    ) -> DbResult<results::PutBso> {
        let bsos = vec![params::PostCollectionBso {
            id: params.id,
            sortindex: params.sortindex,
            payload: params.payload,
            ttl: params.ttl,
        }];
        let result = self
            .post_bsos_with_mutations(params::PostBsos {
                user_id: params.user_id,
                collection: params.collection,
                bsos,
                for_batch: false,
            })
            .await?;

        Ok(result)
    }

    async fn post_bsos_with_mutations(
        &mut self,
        params: params::PostBsos,
    ) -> DbResult<SyncTimestamp> {
        let user_id = params.user_id;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;

        if !params.for_batch {
            self.check_quota(&user_id, &params.collection, collection_id)
                .await?;
        }

        // Ensure a parent record exists in user_collections before writing to
        // bsos (INTERLEAVE IN PARENT user_collections)
        let timestamp = self
            .update_collection(params::UpdateCollection {
                user_id: user_id.clone(),
                collection_id,
                collection: params.collection,
            })
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
        // Determine what bsos already exist (need to be inserted vs updated)
        // NOTE: Here and in batch commit we match the original Python
        // syncstorage's behavior:
        //  - not specifying "AND expiry > CURRENT_TIMESTAMP()"
        //  - thus treating existing but expired bsos as existing (and not
        //  expired)
        // This simplifies the writes, avoiding the need to delete those
        // expired bsos before inserting new ones with the same id.
        // Unfortunately, this means updates may resurrect expired bsos (or at
        // least a subset of their fields), or possibly even write new data
        // without an associated ttl to an expired record that will be
        // deleted. This in practice should be a very rare occurrence
        let mut streaming = self
            .sql(
                "SELECT bso_id
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id IN UNNEST(@ids)",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?;
        let mut existing = HashSet::new();
        while let Some(mut row) = streaming.try_next().await? {
            existing.insert(row[0].take_string_value());
        }
        let mut inserts = vec![];
        let mut updates = HashMap::new();
        let mut load_size: usize = 0;
        for bso in params.bsos {
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
            return Err(DbError::too_large(format!(
                "Committed data too large: {}",
                load_size
            )));
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
        };
        Ok(timestamp)
    }

    pub fn quota_error(&self, collection: &str) -> DbError {
        // return the over quota error.
        let mut tags = HashMap::default();
        tags.insert("collection".to_owned(), collection.to_owned());
        self.metrics.incr_with_tags("storage.quota.at_limit", tags);
        DbError::quota()
    }

    pub(super) async fn check_quota(
        &mut self,
        user_id: &UserIdentifier,
        collection: &str,
        collection_id: i32,
    ) -> DbResult<Option<usize>> {
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
                return Err(self.quota_error(collection));
            } else {
                warn!("Quota at limit for user's collection: ({} bytes)", usage.total_bytes; "collection"=>collection);
            }
        }
        Ok(Some(usage.total_bytes))
    }

    // NOTE: Currently this put_bso_async_without_mutations impl is only used
    // during db tests, see the with_mutations impl for the non-tests version
    async fn put_bso_without_mutations(
        &mut self,
        bso: params::PutBso,
    ) -> DbResult<results::PutBso> {
        use syncstorage_db_common::util::to_rfc3339;
        let collection_id = self.get_or_create_collection_id(&bso.collection).await?;

        self.check_quota(&bso.user_id, &bso.collection, collection_id)
            .await?;

        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => bso.user_id.fxa_uid.clone(),
            "fxa_kid" => bso.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "bso_id" => bso.id,
        };
        // prewarm the collections table by ensuring that the row is added if not present.
        self.update_collection(params::UpdateCollection {
            user_id: bso.user_id.clone(),
            collection_id,
            collection: bso.collection,
        })
        .await?;
        let timestamp = self.checked_timestamp()?;

        let result = self
            .sql(
                "SELECT 1 AS count
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id",
            )
            .await?
            .params(sqlparams.clone())
            .param_types(sqlparam_types.clone())
            .execute(&self.conn)?
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
                    sqlparam_types.insert("sortindex".to_string(), sortindex.spanner_type());
                    sqlparams.insert("sortindex".to_string(), sortindex.into_spanner_value());

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
                    sqlparams.insert(
                        "expiry".to_string(),
                        to_rfc3339(expiry)?.into_spanner_value(),
                    );
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
                        timestamp.as_rfc3339()?.into_spanner_value(),
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
                    sqlparam_types.insert("payload".to_string(), payload.spanner_type());
                    sqlparams.insert("payload".to_string(), payload.into_spanner_value());
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
                use support::null_value;
                let sortindex = bso
                    .sortindex
                    .map(|sortindex| sortindex.into_spanner_value())
                    .unwrap_or_else(null_value);
                sqlparams.insert("sortindex".to_string(), sortindex);
                sqlparam_types.insert("sortindex".to_string(), as_type(TypeCode::INT64));
            }
            let payload = bso.payload.unwrap_or_else(|| "".to_owned());
            sqlparam_types.insert("payload".to_owned(), payload.spanner_type());
            sqlparams.insert("payload".to_string(), payload.into_spanner_value());
            let now_millis = timestamp.as_i64();
            let ttl = bso.ttl.map_or(i64::from(DEFAULT_BSO_TTL), |ttl| ttl.into()) * 1000;
            let expirystring = to_rfc3339(now_millis + ttl)?;
            debug!(
                "!!!!! [test] INSERT expirystring:{:?}, timestamp:{:?}, ttl:{:?}",
                &expirystring, timestamp, ttl
            );
            sqlparams.insert("expiry".to_string(), expirystring.into_spanner_value());
            sqlparam_types.insert("expiry".to_string(), as_type(TypeCode::TIMESTAMP));

            sqlparams.insert(
                "modified".to_string(),
                timestamp.as_rfc3339()?.into_spanner_value(),
            );
            sqlparam_types.insert("modified".to_string(), as_type(TypeCode::TIMESTAMP));
            sql.to_owned()
        };

        self.sql(&sql)
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml(&self.conn)
            .await?;
        // update the counts for the user_collections table.
        self.update_user_collection_quotas(&bso.user_id, collection_id)
            .await
    }

    // NOTE: Currently this post_bsos_without_mutations impl is only used
    // during db tests, see the with_mutations impl for the non-tests version
    async fn post_bsos_without_mutations(
        &mut self,
        input: params::PostBsos,
    ) -> DbResult<SyncTimestamp> {
        let collection_id = self.get_or_create_collection_id(&input.collection).await?;
        let modified = self.checked_timestamp()?;

        for pbso in input.bsos {
            let id = pbso.id;
            self.put_bso_without_mutations(params::PutBso {
                user_id: input.user_id.clone(),
                collection: input.collection.clone(),
                id: id.clone(),
                payload: pbso.payload,
                sortindex: pbso.sortindex,
                ttl: pbso.ttl,
            })
            .await?;
        }
        self.update_user_collection_quotas(&input.user_id, collection_id)
            .await?;
        Ok(modified)
    }
}
