use std::time::Duration;

use super::pool::Conn;
use async_trait::async_trait;
use syncserver_common::Metrics;
use tokenserver_db_common::{error::DbError, params, results, Db};

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
impl Db for TokenserverPgDb {
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    // Services Methods
    async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> Result<results::GetServiceId, DbError> {
        TokenserverPgDb::get_service_id(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> Result<results::PostService, DbError> {
        TokenserverPgDb::post_service(self, params).await
    }

    // Nodes Methods

    // Users Methods
}
