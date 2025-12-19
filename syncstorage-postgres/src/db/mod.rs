use chrono::{DateTime, NaiveDate, Utc};
use diesel::{
    dsl::{now, sql},
    sql_types::BigInt,
    upsert::excluded,
    ExpressionMethods, OptionalExtension, QueryDsl,
};
use diesel_async::RunQueryDsl;
use std::{collections::HashMap, fmt, sync::Arc};

use syncserver_common::Metrics;
use syncstorage_db_common::diesel::DbError;
use syncstorage_db_common::{results, util::SyncTimestamp, Db, UserIdentifier};
use syncstorage_settings::Quota;

use super::schema::{bsos, collections, user_collections};
use super::{
    pool::{CollectionCache, Conn},
    DbResult,
};

mod batch_impl;
mod db_impl;

pub use batch_impl::validate_batch_id;

const TOMBSTONE: i32 = 0;

const PRETOUCH_DT: DateTime<Utc> = NaiveDate::from_ymd_opt(1, 1, 1)
    .unwrap()
    .and_hms_opt(0, 0, 0)
    .unwrap()
    .and_utc();

#[derive(Debug, Eq, PartialEq)]
enum CollectionLock {
    Read,
    Write,
}
pub struct PgDb {
    // Reference to asynchronous database connection.
    pub(super) conn: Conn,
    /// Database session struct reference.
    session: PgDbSession,
    /// Pool level cache of collection_ids and their names.
    coll_cache: Arc<CollectionCache>,
    /// Configured quota, with defined size, enabled, and enforced attributes.
    metrics: Metrics,
    /// Configured quota, with defined size, enabled, and enforced attributes.
    quota: Quota,
}

impl fmt::Debug for PgDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgDb")
            .field("session", &self.session)
            .field("coll_cache", &self.coll_cache)
            .field("metrics", &self.metrics)
            .field("quota", &self.quota)
            .finish()
    }
}

/// Per-session Db metadata.
#[derive(Debug, Default)]
struct PgDbSession {
    /// The "current time" on the server used for this session's operations.
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (HawkIdentifier, collection_id).
    coll_modified_cache: HashMap<(UserIdentifier, i32), SyncTimestamp>,
    /// Currently locked collections.
    coll_locks: HashMap<(UserIdentifier, i32), CollectionLock>,
    /// Whether a transaction was started (begin() called)
    in_transaction: bool,
    /// Boolean to identify if query in active transaction.
    in_write_transaction: bool,
}

impl PgDb {
    /// Create a new instance of PgDb
    /// Fresh metrics clone and default impl of session.
    pub(super) fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: &Quota,
    ) -> Self {
        PgDb {
            conn,
            session: Default::default(),
            coll_cache,
            metrics: metrics.clone(),
            quota: *quota,
        }
    }

    /// Gets the provided collection by name and creates it if not present.
    /// Checks collection cache first to see if matching collection stored.
    /// Uses logic to not make change sif there is a conflict during insert.
    pub(super) async fn get_or_create_collection_id(&mut self, name: &str) -> DbResult<i32> {
        if let Some(id) = self.coll_cache.get_id(name)? {
            return Ok(id);
        }

        // Postgres specific ON CONFLICT DO NOTHING statement.
        // https://docs.diesel.rs/2.0.x/diesel/query_builder/struct.InsertStatement.html#method.on_conflict_do_nothing
        diesel::insert_into(collections::table)
            .values(collections::name.eq(name))
            .on_conflict_do_nothing()
            .execute(&mut self.conn)
            .await?;

        let id = collections::table
            .select(collections::collection_id)
            .filter(collections::name.eq(name))
            .first(&mut self.conn)
            .await?;

        if !self.session.in_write_transaction {
            self.coll_cache.put(id, name.to_owned())?;
        }

        Ok(id)
    }

    /// Given a set of collection_ids, return a HashMap of collection_id's
    /// and their matching collection names.
    /// First attempts to read values from cache if they are present, otherwise
    /// does a lookup in the `collections` table.
    async fn load_collection_names<'a>(
        &mut self,
        collection_ids: impl Iterator<Item = &'a i32>,
    ) -> DbResult<HashMap<i32, String>> {
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
                .select((collections::collection_id, collections::name))
                .filter(collections::collection_id.eq_any(uncached))
                .load::<(i32, String)>(&mut self.conn)
                .await?;

            for (id, name) in result {
                names.insert(id, name.clone());
                if !self.session.in_write_transaction {
                    self.coll_cache.put(id, name)?;
                }
            }
        }
        Ok(names)
    }

    async fn map_collection_names<T>(
        &mut self,
        by_id: HashMap<i32, T>,
    ) -> DbResult<HashMap<String, T>> {
        let mut names = self.load_collection_names(by_id.keys()).await?;
        by_id
            .into_iter()
            .map(|(id, value)| {
                names.remove(&id).map(|name| (name, value)).ok_or_else(|| {
                    DbError::internal("load_collection_names unknown collection id".to_owned())
                })
            })
            .collect()
    }

    async fn erect_tombstone(&mut self, user_id: i64) -> DbResult<()> {
        diesel::insert_into(user_collections::table)
            .values((
                user_collections::user_id.eq(user_id),
                user_collections::collection_id.eq(TOMBSTONE),
                user_collections::modified.eq(self.timestamp().as_datetime()?),
            ))
            .on_conflict((user_collections::user_id, user_collections::collection_id))
            .do_update()
            .set(user_collections::modified.eq(excluded(user_collections::modified)))
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Due to foreign key constraints we need to ensure an entry exists in user_collections prior
    /// to inserting bsos or batches.
    async fn ensure_user_collection(&mut self, user_id: i64, collection_id: i32) -> DbResult<()> {
        diesel::insert_into(user_collections::table)
            .values((
                user_collections::user_id.eq(user_id),
                user_collections::collection_id.eq(collection_id),
                user_collections::modified.eq(PRETOUCH_DT),
            ))
            .on_conflict((user_collections::user_id, user_collections::collection_id))
            .do_nothing()
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    // perform a heavier weight quota calculation
    async fn calc_quota_usage(
        &mut self,
        user_id: i64,
        collection_id: i32,
    ) -> DbResult<results::GetQuotaUsage> {
        let (total_bytes, count): (i64, i64) = bsos::table
            .select((
                sql::<BigInt>("COALESCE(SUM(LENGTH(COALESCE(payload, ''))),0)::BIGINT"),
                sql::<BigInt>("COALESCE(COUNT(*),0)"),
            ))
            .filter(bsos::user_id.eq(user_id))
            .filter(bsos::expiry.gt(now))
            .filter(bsos::collection_id.eq(&collection_id))
            .get_result(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count: count as i32,
        })
    }
}

#[macro_export]
macro_rules! bsos_query {
    ($self:expr, $params:expr, $selection:expr) => {
        {
            let user_id = $params.user_id.legacy_id as i64;
            let collection_id = $self.get_collection_id(&$params.collection).await?;
            let limit = $params.limit.map(i64::from);

            let mut query = bsos::table
                .select($selection)
                .filter(bsos::user_id.eq(user_id))
                .filter(bsos::collection_id.eq(collection_id))
                .filter(bsos::expiry.gt(now))
                .into_boxed();

            if let Some(older) = $params.older {
                query = query.filter(bsos::modified.lt(older.as_datetime()?));
            }
            if let Some(newer) = $params.newer {
                query = query.filter(bsos::modified.gt(newer.as_datetime()?));
            }

            if !$params.ids.is_empty() {
                query = query.filter(bsos::bso_id.eq_any($params.ids));
            }

            query = match $params.sort {
                Sorting::Index => query.order((bsos::sortindex.desc(), bsos::bso_id.desc())),
                Sorting::Newest => query.order((bsos::modified.desc(), bsos::bso_id.desc())),
                Sorting::Oldest => query.order((bsos::modified.asc(), bsos::bso_id.asc())),
                _ => query,
            };

            // fetch an extra row to detect if there are more rows that
            // match the query conditions. Negative limits will cause an error.
            if let Some(limit) = limit {
                query = query.limit(limit + 1);
            }
            let numeric_offset = $params.offset.map_or(0, |offset| offset.offset as i64);
            if numeric_offset != 0 {
                // XXX: copy over this optimization:
                // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
                query = query.offset(numeric_offset);
            }
            let mut items = query.load(&mut $self.conn).await?;

            // Note that "Non-existent collections do not trigger a 404 Not
            // Found for backwards-compatibility reasons.": an empty list is
            // returned in those cases

            let limit = limit.unwrap_or(-1);
            let next_offset = if limit >= 0 && items.len() > limit as usize {
                items.pop();
                Some((limit + numeric_offset).to_string())
            } else {
                None
            };
            (items, next_offset)
        }
    }
}
