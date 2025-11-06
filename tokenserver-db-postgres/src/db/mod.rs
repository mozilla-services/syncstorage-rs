use std::time::Duration;

use syncserver_common::Metrics;

use crate::pool::Conn;

mod db_impl;
pub mod orm_models;
mod schema;

/// Struct containing connection and related metadata to a Tokenserver
/// Postgres Database.
pub struct TokenserverPgDb {
    /// Async PgConnection handle.
    conn: Conn,
    /// Syncserver_common Metrics object.
    metrics: Metrics,
    /// Optional Service Identifier.
    service_id: Option<i32>,
    /// Optional Spanner Node ID.
    spanner_node_id: Option<i32>,
    /// Settings specified timeout for Db Connection.
    pub timeout: Option<Duration>,
}

impl TokenserverPgDb {
    /// Create new instance of `TokenserverPgDb`
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
