use std::time::Duration;

use syncserver_common::Metrics;

use super::pool::Conn;

mod db_impl;

pub struct TokenserverDb {
    conn: Conn,
    metrics: Metrics,
    service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
}

impl TokenserverDb {
    // Note that this only works because an instance of `TokenserverDb` has *exclusive access* to
    // a connection from the r2d2 pool for its lifetime. `LAST_INSERT_ID()` returns the ID of the
    // most recently-inserted record *for a given connection*. If connections were shared across
    // requests, using this function would introduce a race condition, as we could potentially
    // get IDs from records created during other requests.
    const LAST_INSERT_ID_QUERY: &'static str = "SELECT LAST_INSERT_ID() AS id";
    const LAST_INSERT_UID_QUERY: &'static str = "SELECT LAST_INSERT_ID() AS uid";

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
