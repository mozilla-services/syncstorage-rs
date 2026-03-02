use std::time::Duration;

use syncserver_common::Metrics;

use super::pool::Conn;

mod db_impl;

/// MySQL database connection for Tokenserver operations.
///
/// This struct wraps a MySQL connection and provides implementations
/// of all Tokenserver database operations.
pub struct TokenserverDb {
    /// The underlying MySQL database connection
    conn: Conn,
    /// Metrics collector for monitoring
    metrics: Metrics,
    /// Cached service ID for "sync-1.5"
    service_id: Option<i32>,
    /// Spanner node ID (used during migration)
    spanner_node_id: Option<i32>,
    /// Optional timeout for database operations
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

    /// Creates a new `TokenserverDb` instance.
    ///
    /// # Arguments
    ///
    /// * `conn` - The MySQL database connection
    /// * `metrics` - Metrics collector
    /// * `service_id` - Optional cached service ID for "sync-1.5"
    /// * `spanner_node_id` - Optional Spanner node ID
    /// * `timeout` - Optional timeout for database operations
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
