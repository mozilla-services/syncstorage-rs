#![allow(dead_code)] // XXX:
use std::{collections::HashMap, fmt, sync::Arc};

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use syncserver_common::Metrics;
use syncstorage_db_common::diesel::DbError;
use syncstorage_db_common::{util::SyncTimestamp, UserIdentifier};
use syncstorage_settings::Quota;

use super::schema::collections;
use super::{
    pool::{CollectionCache, Conn},
    DbResult,
};

mod batch_impl;
mod db_impl;

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
    /// Whether update_collection has already been called.
    updated_collection: bool,
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
}
