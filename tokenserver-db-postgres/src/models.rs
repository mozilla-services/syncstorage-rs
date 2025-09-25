use std::time::Duration;

use super::pool::Conn;
use async_trait::async_trait;
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;
use syncserver_common::Metrics;
use tokenserver_db_common::{params, results, Db, DbError, DbResult};

/// Struct containing connection and related metadata to a Tokenserver
/// Postgres Database.
#[allow(dead_code)]
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
    /// Utility constant to get the most recent id value after an insert.
    const LAST_INSERT_ID_QUERY: &'static str = "SELECT LAST_INSERT_ID() AS id";

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

    // Services Table Methods

    /**
    Acquire service_id through passed in service string.

        SELECT id
        FROM services
        WHERE service = <String service>
     */
    pub async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> DbResult<results::GetServiceId> {
        const QUERY: &str = r#"
            SELECT id
              FROM services
             WHERE service = $1
        "#;

        if let Some(id) = self.service_id {
            Ok(results::GetServiceId { id })
        } else {
            diesel::sql_query(QUERY)
                .bind::<Text, _>(params.service)
                .get_result::<results::GetServiceId>(&mut self.conn)
                .await
                .map_err(Into::into)
        }
    }

    /**
    Create a new service, given a provided service string and pattern.
    Returns a service_id.

        INSERT INTO services (service, pattern)
        VALUES (<String service>, <String pattern>)

     */
    #[cfg(debug_assertions)]
    pub async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> DbResult<results::PostService> {
        const INSERT_SERVICE_QUERY: &str = r#"
            INSERT INTO services (service, pattern)
            VALUES ($1, $2)
        "#;
        diesel::sql_query(INSERT_SERVICE_QUERY)
            .bind::<Text, _>(&params.service)
            .bind::<Text, _>(&params.pattern)
            .execute(&mut self.conn)
            .await?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::LastInsertId>(&mut self.conn)
            .await
            .map(|result| results::PostService {
                id: result.id as i32,
            })
            .map_err(Into::into)
    }
}

#[async_trait(?Send)]
impl Db for TokenserverPgDb {
    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    async fn check(&mut self) -> Result<results::Check, DbError> {
        TokenserverPgDb::check(self).await
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
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> Result<results::GetNode, DbError> {
        TokenserverPgDb::get_node(self, params).await
    }

    async fn get_node_id(
        &mut self,
        params: params::GetNodeId,
    ) -> Result<results::GetNodeId, DbError> {
        TokenserverPgDb::get_node_id(self, params).await
    }

    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> Result<results::GetBestNode, DbError> {
        TokenserverPgDb::get_best_node(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> Result<results::PostNode, DbError> {
        TokenserverPgDb::post_node(self, params).await
    }

    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> Result<results::AddUserToNode, DbError> {
        TokenserverPgDb::add_user_to_node(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn remove_node(
        &mut self,
        params: params::RemoveNode,
    ) -> Result<results::RemoveNode, DbError> {
        TokenserverPgDb::remove_node(self, params).await
    }

    // Users Methods
    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> Result<results::GetUser, DbError> {
        TokenserverPgDb::get_user(self, params).await
    }

    async fn get_or_create_user(
        &mut self,
        params: params::GetOrCreateUser,
    ) -> Result<results::GetOrCreateUser, DbError> {
        TokenserverPgDb::get_or_create_user(self, params).await
    }

    async fn get_users(&mut self, params: params::GetUsers) -> Result<results::GetUsers, DbError> {
        TokenserverPgDb::get_users(self, params).await
    }

    async fn post_user(&mut self, params: params::PostUser) -> Result<results::PostUser, DbError> {
        TokenserverPgDb::post_user(self, params).await
    }

    async fn put_user(&mut self, params: params::PutUser) -> Result<results::PutUser, DbError> {
        TokenserverPgDb::put_user(self, params).await
    }

    async fn replace_user(
        &mut self,
        params: params::ReplaceUser,
    ) -> Result<results::ReplaceUser, DbError> {
        TokenserverPgDb::replace_user(self, params).await
    }

    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> Result<results::ReplaceUsers, DbError> {
        TokenserverPgDb::replace_users(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> Result<results::UnassignNode, DbError> {
        TokenserverPgDb::unassign_node(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> Result<results::SetUserCreatedAt, DbError> {
        TokenserverPgDb::set_user_created_at(self, params).await
    }

    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> Result<results::SetUserReplacedAt, DbError> {
        TokenserverPgDb::set_user_replaced_at(self, params).await
    }

    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, params: params::SpannerNodeId) {
        self.spanner_node_id = params;
    }
}
