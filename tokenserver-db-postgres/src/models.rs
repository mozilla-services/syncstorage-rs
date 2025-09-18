use std::time::Duration;

use super::pool::Conn;
use async_trait::async_trait;
use syncserver_common::Metrics;
use tokenserver_db_common::Db;

#[allow(dead_code)]
pub struct TokenserverPgDb {
    conn: Conn,
    metrics: Metrics,
    service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
}

impl TokenserverPgDb {
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

#[async_trait(?Send)]
impl Db for TokenserverPgDb {}
