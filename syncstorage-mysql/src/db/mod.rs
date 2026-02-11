use std::{collections::HashMap, fmt, sync::Arc};

use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl,
    dsl::sql,
    sql_query,
    sql_types::{BigInt, Integer, Text},
};
use diesel_async::RunQueryDsl;
use syncserver_common::Metrics;
use syncstorage_db_common::{
    Db, FIRST_CUSTOM_COLLECTION_ID, UserIdentifier, error::DbErrorIntrospect, results,
    util::SyncTimestamp,
};
use syncstorage_settings::Quota;

use crate::{
    DbError, DbResult,
    pool::{CollectionCache, Conn},
};
use schema::{bso, collections, last_insert_id};

mod batch_impl;
mod db_impl;
mod diesel_ext;
pub(crate) mod schema;

pub use batch_impl::validate_batch_id;

const TOMBSTONE: i32 = 0;
/// SQL Variable remapping
/// These names are the legacy values mapped to the new names.
const COLLECTION_ID: &str = "collection";
const USER_ID: &str = "userid";
const MODIFIED: &str = "modified";
const EXPIRY: &str = "ttl";
const LAST_MODIFIED: &str = "last_modified";
const COUNT: &str = "count";
const TOTAL_BYTES: &str = "total_bytes";

#[derive(Debug)]
enum CollectionLock {
    Read,
    Write,
}

/// Per session Db metadata
#[derive(Debug, Default)]
struct MysqlDbSession {
    /// The "current time" on the server used for this session's operations
    timestamp: SyncTimestamp,
    /// Cache of collection modified timestamps per (user_id, collection_id)
    coll_modified_cache: HashMap<(UserIdentifier, i32), SyncTimestamp>,
    /// Currently locked collections
    coll_locks: HashMap<(UserIdentifier, i32), CollectionLock>,
    /// Whether a transaction was started (begin() called)
    in_transaction: bool,
    in_write_transaction: bool,
}

pub struct MysqlDb {
    pub(super) conn: Conn,
    session: MysqlDbSession,
    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,
    metrics: Metrics,
    quota: Quota,
}

impl fmt::Debug for MysqlDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MysqlDb")
            .field("session", &self.session)
            .field("coll_cache", &self.coll_cache)
            .field("metrics", &self.metrics)
            .field("quota", &self.quota)
            .finish()
    }
}

impl MysqlDb {
    pub(super) fn new(
        conn: Conn,
        coll_cache: Arc<CollectionCache>,
        metrics: &Metrics,
        quota: &Quota,
    ) -> Self {
        MysqlDb {
            conn,
            session: Default::default(),
            coll_cache,
            metrics: metrics.clone(),
            quota: *quota,
        }
    }

    async fn erect_tombstone(&mut self, user_id: i32) -> DbResult<()> {
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
        .bind::<BigInt, _>(self.session.timestamp.as_i64())
        .execute(&mut self.conn)
        .await?;
        Ok(())
    }

    pub(super) async fn get_or_create_collection_id(&mut self, name: &str) -> DbResult<i32> {
        match self.get_collection_id(name).await {
            Err(e) if e.is_collection_not_found() => self._create_collection(name).await,
            result => result,
        }
    }

    async fn _create_collection(&mut self, name: &str) -> DbResult<i32> {
        if !cfg!(debug_assertions) && !self.session.in_write_transaction {
            return Err(DbError::internal(
                "Can't escalate read-lock to write-lock".to_owned(),
            ));
        }
        diesel::insert_into(collections::table)
            .values(collections::name.eq(name))
            .execute(&mut self.conn)
            .await?;
        let collection_id = diesel::select(last_insert_id())
            .first::<u64>(&mut self.conn)
            .await?
            .try_into()
            .map_err(|e| {
                DbError::internal(format!("Couldn't convert last_insert_id() to i32: {e}"))
            })?;
        if collection_id < FIRST_CUSTOM_COLLECTION_ID {
            // DDL ensures this should never occur
            return Err(DbError::internal(
                "create_collection < {FIRST_CUSTOM_COLLECTION_ID}".to_owned(),
            ));
        }
        Ok(collection_id)
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
                .select((collections::id, collections::name))
                .filter(collections::id.eq_any(uncached))
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

    // perform a heavier weight quota calculation
    async fn calc_quota_usage(
        &mut self,
        user_id: i64,
        collection_id: i32,
    ) -> DbResult<results::GetQuotaUsage> {
        let (total_bytes, count): (i64, i32) = bso::table
            .select((
                sql::<BigInt>(r#"COALESCE(SUM(LENGTH(COALESCE(payload, ""))),0)"#),
                sql::<Integer>("COALESCE(COUNT(*),0)"),
            ))
            .filter(bso::user_id.eq(user_id))
            .filter(bso::expiry.gt(self.session.timestamp.as_i64()))
            .filter(bso::collection_id.eq(collection_id))
            .get_result(&mut self.conn)
            .await
            .optional()?
            .unwrap_or_default();
        Ok(results::GetQuotaUsage {
            total_bytes: total_bytes as usize,
            count,
        })
    }
}

#[allow(dead_code)] // Not really dead, Rust can't see the use above
#[derive(Debug, QueryableByName)]
struct NameResult {
    #[diesel(sql_type = Text)]
    name: String,
}
