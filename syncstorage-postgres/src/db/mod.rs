#![allow(dead_code)] // XXX:
use std::sync::Arc;

use syncserver_common::Metrics;
use syncstorage_settings::Quota;

use super::pool::{CollectionCache, Conn};

pub struct PgDb {
    pub(super) conn: Conn,
    //session: PgDbSession,
    /// Pool level cache of collection_ids and their names
    coll_cache: Arc<CollectionCache>,
    metrics: Metrics,
    quota: Quota,
}
