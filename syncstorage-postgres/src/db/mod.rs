#![allow(dead_code)] // XXX:
use std::{collections::HashMap, fmt, sync::Arc};

use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;

use syncserver_common::Metrics;
use syncstorage_db_common::{results, util::SyncTimestamp, UserIdentifier};
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

    /// NOTE: Will be completed with other db method task.
    pub(super) async fn get_or_create_collection_id(
        &mut self,
        name: &str,
    ) -> DbResult<results::GetOrCreateCollectionId> {
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
}
