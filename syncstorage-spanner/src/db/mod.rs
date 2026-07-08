use std::{collections::HashMap, convert::TryInto, fmt, sync::Arc};

use google_cloud_rust_raw::spanner::v1::{
    spanner::ExecuteSqlRequest,
    transaction::TransactionSelector,
    type_pb::{StructType, Type, TypeCode},
};
use protobuf::{
    RepeatedField,
    well_known_types::{ListValue, Value},
};
use syncserver_common::Metrics;
use syncstorage_db_common::{
    DEFAULT_BSO_TTL, Db, FIRST_CUSTOM_COLLECTION_ID, Sorting, UserIdentifier,
    error::DbErrorIntrospect,
    params,
    util::{SyncTimestamp, to_rfc3339},
};
use syncstorage_settings::Quota;

use crate::{
    DbResult,
    error::DbError,
    pool::{CollectionCache, Conn},
};
use support::{
    ExecuteSqlRequestBuilder, IntoSpannerValue, StreamedResultSetAsync, as_type, null_value,
    struct_type_field,
};

mod batch_impl;
mod db_impl;
mod stream;
pub(crate) mod support;

pub use batch_impl::validate_batch_id;

#[derive(Debug, Eq, PartialEq)]
enum CollectionLock {
    Read,
    Write,
}

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
        match self._get_collection_id(name).await {
            Err(e) if e.is_collection_not_found() => self._create_collection(name).await,
            result => result,
        }
    }

    async fn _create_collection(&mut self, name: &str) -> DbResult<i32> {
        // This should always run within a r/w transaction, so that: "If a
        // transaction successfully commits, then no other writer modified the
        // data that was read in the transaction after it was read."
        if !cfg!(debug_assertions) && !self.in_write_transaction() {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_owned(),
            ));
        }
        let result = self
            .sql(
                "SELECT COALESCE(MAX(collection_id), 1)
                   FROM collections",
            )
            .await?
            .execute(&self.conn)?
            .one()
            .await?;
        let max = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbError::integrity(e.to_string()))?;
        let id = FIRST_CUSTOM_COLLECTION_ID.max(max + 1);
        let (sqlparams, sqlparam_types) = params! {
            "name" => name.to_string(),
            "collection_id" => id,
        };

        self.sql(
            "INSERT INTO collections (collection_id, name)
             VALUES (@collection_id, @name)",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        Ok(id)
    }

    async fn _get_collection_id(&mut self, name: &str) -> DbResult<i32> {
        if let Some(id) = self.coll_cache.get_id(name).await {
            return Ok(id);
        }
        let (sqlparams, sqlparam_types) = params! { "name" => name.to_string() };
        let result = self
            .sql(
                "SELECT collection_id
                   FROM collections
                  WHERE name = @name",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?
            .ok_or_else(DbError::collection_not_found)?;
        let id = result[0]
            .get_string_value()
            .parse::<i32>()
            .map_err(|e| DbError::integrity(e.to_string()))?;
        if !self.in_write_transaction() {
            self.coll_cache.put(id, name.to_owned()).await;
        }
        Ok(id)
    }

    /// Return the current transaction metadata (TransactionSelector) if one is active.
    async fn get_transaction(&mut self) -> DbResult<Option<TransactionSelector>> {
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
        // Upsert the `user_collections` row so `modified` is always refreshed
        // after a write, including the just emptied collection case, where
        // tombstone retention depends on the timestamp sticking around.
        //
        // When quota is enabled, count + total_bytes are recomputed from
        // bsos. When disabled, those columns are omitted from
        // the write to keep Spanner's mutation count down.
        let timestamp = self.checked_timestamp()?;
        let (mut sqlparams, mut sqltypes) = params! {
            "fxa_uid" => user.fxa_uid.clone(),
            "fxa_kid" => user.fxa_kid.clone(),
            "collection_id" => collection_id,
            "modified" => timestamp.as_rfc3339()?,
        };
        sqltypes.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));

        let upsert_sql = if self.quota.enabled {
            self.metrics
                .clone()
                .start_timer("storage.quota.update_existing_totals", None);

            let (q_params, q_types) = params! {
                "fxa_uid" => user.fxa_uid.clone(),
                "fxa_kid" => user.fxa_kid.clone(),
                "collection_id" => collection_id,
            };
            let mut result = self
                .sql(
                    "SELECT COALESCE(SUM(BYTE_LENGTH(payload)), 0), COUNT(*)
                       FROM bsos
                      WHERE fxa_uid = @fxa_uid
                        AND fxa_kid = @fxa_kid
                        AND collection_id = @collection_id",
                )
                .await?
                .params(q_params)
                .param_types(q_types)
                .execute(&self.conn)?
                .one()
                .await?;
            sqlparams.insert(
                "total_bytes".to_owned(),
                result[0].take_string_value().into_spanner_value(),
            );
            sqlparams.insert(
                "count".to_owned(),
                result[1].take_string_value().into_spanner_value(),
            );
            sqltypes.insert("total_bytes".to_owned(), as_type(TypeCode::INT64));
            sqltypes.insert("count".to_owned(), as_type(TypeCode::INT64));

            "INSERT OR UPDATE INTO user_collections
                (fxa_uid, fxa_kid, collection_id, modified, total_bytes, count)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified, @total_bytes, @count)"
        } else {
            "INSERT OR UPDATE INTO user_collections
                (fxa_uid, fxa_kid, collection_id, modified)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)"
        };

        self.sql(upsert_sql)
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
            "collection_id" => self._get_collection_id(&params.collection).await?,
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
    ) -> DbResult<Option<usize>> {
        // duplicate quota trap in test func below.
        if !self.quota.enabled {
            return Ok(None);
        }
        let usage = self
            .get_quota_usage(params::GetQuotaUsage {
                user_id: user_id.clone(),
                collection: collection.to_owned(),
            })
            .await?;
        if usage.total_bytes >= self.quota.size {
            if self.quota.enforced {
                return Err(self.quota_error(collection));
            } else {
                warn!("Quota at limit for user ({} bytes)", usage.total_bytes; "collection"=>collection);
            }
        }
        Ok(Some(usage.total_bytes))
    }

    /// Write a bso using an `INSERT OR UPDATE`.
    async fn put_bso_dml(
        &mut self,
        user_id: &UserIdentifier,
        collection_id: i32,
        bso: params::PostCollectionBso,
        timestamp: SyncTimestamp,
    ) -> DbResult<()> {
        let has_payload_or_sortindex =
            bso.payload.is_some() || bso.payload_link.is_some() || bso.sortindex.is_some();

        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "bso_id" => bso.id,
            "timestamp" => timestamp.as_rfc3339()?,
            "default_bso_ttl" => DEFAULT_BSO_TTL,
        };
        sqlparam_types.insert("timestamp".to_owned(), as_type(TypeCode::TIMESTAMP));

        let modified_expr = if has_payload_or_sortindex {
            "@timestamp"
        } else {
            "COALESCE(existing.modified, @timestamp)"
        };

        // payload and payload_link are mutually exclusive: an inline write sets
        // payload and clears the link, an offload write sets the link and
        // clears payload, and a metadata-only update preserves whichever the
        // existing row holds.
        let (payload_expr, payload_link_expr) = match (bso.payload, bso.payload_link) {
            (Some(payload), _) => {
                sqlparam_types.insert("payload".to_owned(), payload.spanner_type());
                sqlparams.insert("payload".to_owned(), payload.into_spanner_value());
                ("@payload", "NULL")
            }
            (None, Some(payload_link)) => {
                sqlparam_types.insert("payload_link".to_owned(), payload_link.spanner_type());
                sqlparams.insert("payload_link".to_owned(), payload_link.into_spanner_value());
                ("NULL", "@payload_link")
            }
            (None, None) => ("existing.payload", "existing.payload_link"),
        };

        let expiry_expr = if let Some(ttl) = bso.ttl {
            let expiry = timestamp.as_i64() + (i64::from(ttl) * 1000);
            sqlparams.insert(
                "ttl_expiry".to_owned(),
                to_rfc3339(expiry)?.into_spanner_value(),
            );
            sqlparam_types.insert("ttl_expiry".to_owned(), as_type(TypeCode::TIMESTAMP));
            "@ttl_expiry"
        } else {
            "COALESCE(existing.expiry, TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND))"
        };

        let (sortindex_col, sortindex_expr) = if let Some(sortindex) = bso.sortindex {
            sqlparam_types.insert("sortindex".to_owned(), sortindex.spanner_type());
            sqlparams.insert("sortindex".to_owned(), sortindex.into_spanner_value());
            (", sortindex", ", @sortindex")
        } else {
            ("", "")
        };

        let sql = format!(
            "INSERT OR UPDATE INTO bsos
                 (fxa_uid, fxa_kid, collection_id, bso_id, modified, payload, expiry, payload_link{sortindex_col})
             SELECT
                 @fxa_uid, @fxa_kid, @collection_id, @bso_id,
                 {modified_expr},
                 {payload_expr},
                 {expiry_expr},
                 {payload_link_expr}{sortindex_expr}
               FROM UNNEST([1]) --  provides a row source for the LEFT JOIN
          LEFT JOIN (
                 SELECT modified, payload, expiry, sortindex, payload_link
                   FROM bsos
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND bso_id = @bso_id
             ) AS existing ON TRUE"
        );

        self.sql(&sql)
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml(&self.conn)
            .await?;
        Ok(())
    }

    /// Write N bsos to the same `(fxa_uid, fxa_kid, collection_id)` in an `INSERT OR UPDATE`
    async fn post_bsos_dml(
        &mut self,
        user_id: &UserIdentifier,
        collection_id: i32,
        bsos: Vec<params::PostCollectionBso>,
        timestamp: SyncTimestamp,
    ) -> DbResult<()> {
        if bsos.is_empty() {
            return Ok(());
        }

        let mut rows: Vec<Value> = Vec::with_capacity(bsos.len());
        for bso in bsos {
            // Optional columns are encoded as NULL when the request omitted them
            let sortindex = bso
                .sortindex
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);
            let payload = bso
                .payload
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);
            let ttl = bso
                .ttl
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);
            let payload_link = bso
                .payload_link
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);

            let mut row = ListValue::new();
            row.set_values(
                vec![
                    bso.id.into_spanner_value(),
                    sortindex,
                    payload,
                    ttl,
                    payload_link,
                ]
                .into(),
            );
            let mut value = Value::new();
            value.set_list_value(row);
            rows.push(value);
        }

        let fields = vec![
            ("bso_id", TypeCode::STRING),
            ("sortindex", TypeCode::INT64),
            ("payload", TypeCode::STRING),
            ("ttl", TypeCode::INT64),
            ("payload_link", TypeCode::STRING),
        ]
        .into_iter()
        .map(|(name, field_type)| struct_type_field(name, field_type))
        .collect();

        let mut list_values = ListValue::new();
        list_values.set_values(RepeatedField::from_vec(rows));
        let mut values = Value::new();
        values.set_list_value(list_values);

        let mut param_type = Type::new();
        param_type.set_code(TypeCode::ARRAY);
        let mut array_type = Type::new();
        array_type.set_code(TypeCode::STRUCT);
        let mut struct_type = StructType::new();
        struct_type.set_fields(RepeatedField::from_vec(fields));
        array_type.set_struct_type(struct_type);
        param_type.set_array_element_type(array_type);

        let (mut sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "timestamp" => timestamp.as_rfc3339()?,
            "default_bso_ttl" => DEFAULT_BSO_TTL,
        };
        sqlparam_types.insert("timestamp".to_owned(), as_type(TypeCode::TIMESTAMP));
        sqlparams.insert("bsos".to_owned(), values);
        sqlparam_types.insert("bsos".to_owned(), param_type);

        self.sql(
            "INSERT OR UPDATE INTO bsos
                 (fxa_uid, fxa_kid, collection_id, bso_id,
                  sortindex, payload, modified, expiry, payload_link)
             SELECT
                 @fxa_uid,
                 @fxa_kid,
                 @collection_id,
                 incoming.bso_id,
                 COALESCE(incoming.sortindex, existing.sortindex),
                 COALESCE(incoming.payload, existing.payload, ''),
                 IF(incoming.payload IS NOT NULL OR incoming.sortindex IS NOT NULL,
                    @timestamp,
                    COALESCE(existing.modified, @timestamp)),
                 COALESCE(
                     TIMESTAMP_ADD(@timestamp, INTERVAL incoming.ttl SECOND),
                     existing.expiry,
                     TIMESTAMP_ADD(@timestamp, INTERVAL @default_bso_ttl SECOND)
                 ),
                 COALESCE(incoming.payload_link, existing.payload_link)
               FROM UNNEST(@bsos) AS incoming
               LEFT JOIN bsos AS existing
                 ON existing.fxa_uid = @fxa_uid
                AND existing.fxa_kid = @fxa_kid
                AND existing.collection_id = @collection_id
                AND existing.bso_id = incoming.bso_id",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        Ok(())
    }
}
