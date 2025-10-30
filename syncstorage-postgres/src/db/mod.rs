#![allow(dead_code)] // XXX:
use std::{collections::HashMap, fmt, sync::Arc};

use syncserver_common::Metrics;
use syncstorage_db_common::{util::SyncTimestamp, UserIdentifier};
use syncstorage_settings::Quota;

use super::pool::{CollectionCache, Conn};

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
