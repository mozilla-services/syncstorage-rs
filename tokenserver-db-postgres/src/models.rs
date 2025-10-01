use std::time::Duration;

use super::pool::Conn;
use async_trait::async_trait;
use diesel::{
    sql_types::{BigInt, Float, Integer, Text},
    OptionalExtension,
};
use diesel_async::RunQueryDsl;
use http::StatusCode;
use syncserver_common::Metrics;
use tokenserver_db_common::{params, results, Db, DbError, DbResult};

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

    // Nodes Table Methods

    /**
    Get Node with complete metadata, given a provided Node ID.
    Returns a complete Node, including id, service_id, node string identifier
    availability, and current load.

        SELECT *
        FROM nodes
        WHERE id = <id i64>
     */
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> DbResult<results::GetNode> {
        const QUERY: &str = r#"
            SELECT *
            FROM nodes
            WHERE id = $1
            "#;

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.id)
            .get_result::<results::GetNode>(&mut self.conn)
            .await
            .map_err(Into::into)
    }

    /**
    Get the specific Node ID, given a provided service string and node.
    Returns a node_id.

        SELECT id
        FROM nodes
        WHERE service = <String service>
        AND node = <String node>
     */
    async fn get_node_id(&mut self, params: params::GetNodeId) -> DbResult<results::GetNodeId> {
        const QUERY: &str = r#"
            SELECT id
              FROM nodes
             WHERE service = $1
               AND node = $2
        "#;

        if let Some(id) = self.spanner_node_id {
            Ok(results::GetNodeId { id: id as i64 })
        } else {
            let mut metrics = self.metrics.clone();
            metrics.start_timer("storage.get_node_id", None);

            diesel::sql_query(QUERY)
                .bind::<Integer, _>(params.service_id)
                .bind::<Text, _>(&params.node)
                .get_result(&mut self.conn)
                .await
                .map_err(Into::into)
        }
    }

    /**
    Get the best Node ID, which is the least loaded node with most available slots,
    given a provided service string and node.
    Returns a node_id and identifier string.

        SELECT id, node
        FROM nodes
        WHERE service = <service_id i32>
            AND available > 0
            AND capacity > current_load
            AND downed = 0
            AND backoff = 0
        ORDER BY LOG(current_load) / LOG(capacity)
        LIMIT 1
     */
    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> DbResult<results::GetBestNode> {
        const DEFAULT_CAPACITY_RELEASE_RATE: f32 = 0.1;
        const GET_BEST_NODE_QUERY: &str = r#"
                SELECT id, node
                FROM nodes
                WHERE service = $1
                AND available > 0
                AND capacity > current_load
                AND downed = 0
                AND backoff = 0
                ORDER BY LOG(current_load) / LOG(capacity)
                LIMIT 1
            "#;
        const RELEASE_CAPACITY_QUERY: &str = r#"
            UPDATE nodes
            SET available = LEAST(capacity * $1, capacity - current_load)
            WHERE service = $2
            AND available <= 0
            AND capacity > current_load
            AND downed = 0
            "#;
        const SPANNER_QUERY: &str = r#"
            SELECT id, node
            FROM nodes
            WHERE id = $1
            LIMIT 1
            "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_best_node", None);

        if let Some(spanner_node_id) = self.spanner_node_id {
            diesel::sql_query(SPANNER_QUERY)
                .bind::<Integer, _>(spanner_node_id)
                .get_result::<results::GetBestNode>(&mut self.conn)
                .await
                .map_err(|e| {
                    let mut db_error = DbError::internal(format!(
                        "Tokenserver get_best_node query - Unable to get Spanner node: {}",
                        e
                    ));
                    db_error.status = StatusCode::SERVICE_UNAVAILABLE;
                    db_error
                })
        } else {
            // This loop allows for a maximum of 5 retries before stopping.
            // This allows for query retries if more capacity needs to be released.
            for _ in 0..5 {
                let possible_result: Option<results::GetBestNode> =
                    diesel::sql_query(GET_BEST_NODE_QUERY)
                        .bind::<Integer, _>(params.service_id)
                        .get_result::<results::GetBestNode>(&mut self.conn)
                        .await
                        .optional()?;

                if let Some(result) = possible_result {
                    return Ok(result);
                }

                // No available Nodes.
                // Attempt to release additional capacity from any nodes that are not full.
                let affected_rows = diesel::sql_query(RELEASE_CAPACITY_QUERY)
                    .bind::<Float, _>(
                        params
                            .capacity_release_rate
                            .unwrap_or(DEFAULT_CAPACITY_RELEASE_RATE),
                    )
                    .bind::<Integer, _>(params.service_id)
                    .execute(&mut self.conn)
                    .await?;

                // If no nodes were affected by the last query, terminate.
                if affected_rows == 0 {
                    break;
                }
            }

            let mut db_error: DbError = DbError::internal(String::from("unable to get a node"));
            db_error.status = StatusCode::SERVICE_UNAVAILABLE;
            Err(db_error)
        }
    }

    /**
    Create and Insert a new node.
    Returns the last_insert_id of the newly created node.

        INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
        VALUES (<service_id i32>, <node String>, <available i32>, <current_load i32>,
        <capacity i32>, <downed i32>, <backoff i32>)
     */
    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> DbResult<results::PostNode> {
        const QUERY: &str = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;
        diesel::sql_query(QUERY)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.node)
            .bind::<Integer, _>(params.available)
            .bind::<Integer, _>(params.current_load)
            .bind::<Integer, _>(params.capacity)
            .bind::<Integer, _>(params.downed)
            .bind::<Integer, _>(params.backoff)
            .execute(&mut self.conn)
            .await?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::PostNode>(&mut self.conn)
            .await
            .map_err(Into::into)
    }

    /**
    Update the current load count of a node, passing in the service string and node string.
    This represents the addition of a user to a node, while not defining which user specifically.
    Does not return anything.

        UPDATE nodes
        SET current_load = current_load + 1,
            available = GREATEST(available - 1, 0)
        WHERE service = <service String>
        AND node = <node String>
     */
    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> DbResult<results::AddUserToNode> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.add_user_to_node", None);

        const QUERY: &str = r#"
            UPDATE nodes
            SET current_load = current_load + 1,
                available = GREATEST(available - 1, 0)
            WHERE service = $1
            AND node = $2
        "#;
        const SPANNER_QUERY: &str = r#"
            UPDATE nodes
            SET current_load = current_load + 1
            WHERE service = $1
            AND node = $2
        "#;

        // Use the spanner query if the instance has spanner_node_id set.
        // Otherwise use the other query that calculates the available value.
        let query = if self.spanner_node_id.is_some() {
            SPANNER_QUERY
        } else {
            QUERY
        };

        diesel::sql_query(query)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.node)
            .execute(&mut self.conn)
            .await
            .map(|_| ())
            .map_err(Into::into)
    }

    /**
    Remove a node given the node ID.
    Does not return anything.

        DELETE FROM nodes WHERE id = <node_id i64>
     */
    #[cfg(debug_assertions)]
    async fn remove_node(&mut self, params: params::RemoveNode) -> DbResult<results::RemoveNode> {
        const QUERY: &str = "DELETE FROM nodes WHERE id = $1";

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.node_id)
            .execute(&mut self.conn)
            .await
            .map(|_| ())
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
