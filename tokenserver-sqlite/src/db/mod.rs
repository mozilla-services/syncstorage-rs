use std::time::Duration;

use syncserver_common::Metrics;

use super::pool::Conn;

mod db_impl;

pub struct TokenserverSqliteDb {
    conn: Conn,
    metrics: Metrics,
    service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
}

impl TokenserverSqliteDb {
    // Note that this only works because an instance of `TokenserverSqliteDb` has *exclusive
    // access* to a connection from the pool for its lifetime. `last_insert_rowid()` returns the
    // rowid of the most recently-inserted record *for a given connection*. If connections were
    // shared across requests, using this function would introduce a race condition, as we could
    // potentially get IDs from records created during other requests.
    #[allow(dead_code)]
    const LAST_INSERT_ID_QUERY: &'static str = "SELECT last_insert_rowid() AS id";
    const LAST_INSERT_UID_QUERY: &'static str = "SELECT last_insert_rowid() AS uid";

    pub fn new(
        conn: Conn,
        metrics: &Metrics,
        service_id: Option<i32>,
        spanner_node_id: Option<i32>,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            conn,
            metrics: metrics.clone(),
            service_id,
            spanner_node_id,
            timeout,
        }
    }
}
