/// Note the addition of `#[cfg(debug_assertions)]` flags methods and
/// imports only to be added during debug builds.
/// cargo build --release will not include this code in the binary.
use std::time::Duration;
#[cfg(debug_assertions)]
use std::time::{SystemTime, UNIX_EPOCH};

use super::pool::Conn;
use async_trait::async_trait;
use diesel::{
    sql_types::{BigInt, Float, Integer, Nullable, Text},
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

    // Services Table Methods

    /// Acquire service_id through passed in service string.
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
            let result = diesel::sql_query(QUERY)
                .bind::<Text, _>(params.service)
                .get_result::<results::GetServiceId>(&mut self.conn)
                .await?;
            Ok(result)
        }
    }

    // Create a new service, given a provided service string and pattern.
    // Returns a service_id.
    #[cfg(debug_assertions)]
    pub async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> DbResult<results::PostService> {
        const INSERT_SERVICE_QUERY: &str = r#"
            INSERT INTO services (service, pattern)
            VALUES ($1, $2)
            RETURNING id
        "#;

        let result = diesel::sql_query(INSERT_SERVICE_QUERY)
            .bind::<Text, _>(&params.service)
            .bind::<Text, _>(&params.pattern)
            .get_result::<results::PostService>(&mut self.conn)
            .await?;
        Ok(result)
    }

    // Nodes Table Methods

    /// Get Node with complete metadata, given a provided Node ID.
    /// Returns a complete Node, including id, service_id, node string identifier
    /// availability, and current load.
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> DbResult<results::GetNode> {
        const QUERY: &str = r#"
            SELECT *
              FROM nodes
             WHERE id = $1
            "#;

        let result = diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.id)
            .get_result::<results::GetNode>(&mut self.conn)
            .await?;
        Ok(result)
    }

    /// Get the specific Node ID, given a provided service string and node.
    /// Returns a node_id.
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

            let result = diesel::sql_query(QUERY)
                .bind::<Integer, _>(params.service_id)
                .bind::<Text, _>(&params.node)
                .get_result(&mut self.conn)
                .await?;
            Ok(result)
        }
    }

    /// Get the best Node ID, which is the least loaded node with most available slots,
    /// given a provided service string and node.
    /// Returns a node_id and identifier string.
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

    /// Create and Insert a new node.
    /// Returns the last inserted `id` of the newly created node.
    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> DbResult<results::PostNode> {
        const QUERY: &str = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
        "#;
        let result = diesel::sql_query(QUERY)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.node)
            .bind::<Integer, _>(params.available)
            .bind::<Integer, _>(params.current_load)
            .bind::<Integer, _>(params.capacity)
            .bind::<Integer, _>(params.downed)
            .bind::<Integer, _>(params.backoff)
            .get_result::<results::PostNode>(&mut self.conn)
            .await?;
        Ok(result)
    }

    /// Update the current load count of a node, passing in the service string and node string.
    /// This represents the addition of a user to a node, while not defining which user specifically.
    /// Does not return anything.
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
            .await?;
        Ok(())
    }

    /// Remove a node given the node ID.
    #[cfg(debug_assertions)]
    async fn remove_node(&mut self, params: params::RemoveNode) -> DbResult<results::RemoveNode> {
        const QUERY: &str = "DELETE FROM nodes WHERE id = $1";

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.node_id)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    // Users Table Methods

    /// Given a user id, return a single user (GetUser) struct.
    /// Contains all data relevant to particular user.
    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> DbResult<results::GetUser> {
        const QUERY: &str = r#"
            SELECT service, email, generation, client_state, replaced_at, nodeid, keys_changed_at
              FROM users
             WHERE uid = $1
        "#;

        let result = diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.id)
            .get_result::<results::GetUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    /// Given a service_id and email, return all matching users (up to 20).
    /// Returns vector of matching `GetUser` structs, a type alias for `GetRawUsers`
    async fn get_users(&mut self, params: params::GetUsers) -> DbResult<results::GetUsers> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_users", None);

        const QUERY: &str = r#"
                     SELECT uid, nodes.node, generation, keys_changed_at, client_state, created_at,
                            replaced_at
                       FROM users
            LEFT OUTER JOIN nodes ON users.nodeid = nodes.id
                      WHERE email = $1
                        AND users.service = $2
                   ORDER BY created_at DESC, uid DESC
                      LIMIT 20
        "#;

        let result = diesel::sql_query(QUERY)
            .bind::<Text, _>(params.email)
            .bind::<Integer, _>(params.service_id)
            .load::<results::GetRawUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    /// Method to create a new user, given a `PostUser` struct containing data regarding the user.
    #[cfg(debug_assertions)]
    async fn post_user(&mut self, params: params::PostUser) -> DbResult<results::PostUser> {
        const QUERY: &str = r#"
            INSERT INTO users (service, email, generation, client_state, created_at,
                               nodeid, keys_changed_at, replaced_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NULL)
            RETURNING uid;
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.post_user", None);

        let result = diesel::sql_query(QUERY)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.email)
            .bind::<BigInt, _>(params.generation)
            .bind::<Text, _>(params.client_state)
            .bind::<BigInt, _>(params.created_at)
            .bind::<BigInt, _>(params.node_id)
            .bind::<Nullable<BigInt>, _>(params.keys_changed_at)
            .get_result::<results::PostUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    /// Update the user with the given email and service ID with the given `generation` and
    /// `keys_changed_at`. Additionally, the other parameters ensure greater certainty to prevent
    /// timestamp fields from regressing. More information below.
    async fn put_user(&mut self, params: params::PutUser) -> DbResult<results::PutUser> {
        // As an added layer of safety, the `WHERE` clause ensures that concurrent updates
        // don't accidentally move timestamp fields backwards in time. The handling of
        // `keys_changed_at`can be problematic as we want to treat the default `NULL` as zero (0).
        const QUERY: &str = r#"
            UPDATE users
               SET generation = $1,
                   keys_changed_at = $2
             WHERE service = $3
               AND email = $4
               AND  generation <= $5
               AND COALESCE(keys_changed_at, 0) <= COALESCE(<keys_changed_at i64>, keys_changed_at, 0)
               AND replaced_at IS NULL
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.put_user", None);

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.generation)
            .bind::<Nullable<BigInt>, _>(params.keys_changed_at)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.email)
            .bind::<BigInt, _>(params.generation)
            .bind::<Nullable<BigInt>, _>(params.keys_changed_at)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Update the user record with the given uid and service id
    /// marking it as 'replaced'. This is through updating the `replaced_at` field.
    async fn replace_user(
        &mut self,
        params: params::ReplaceUser,
    ) -> DbResult<results::ReplaceUser> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = $1
             WHERE service = $2
               AND uid = $3
        "#;

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.replaced_at)
            .bind::<Integer, _>(params.service_id)
            .bind::<BigInt, _>(params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Update several user records with the given email and service id
    /// marking them as 'replaced'. This is through updating the `replaced_at` field.
    /// The `replaced_at` field should be null AND the `created_at` field should be earlier
    /// than the `replaced_at`.
    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> DbResult<results::ReplaceUsers> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = $1
             WHERE service = $2
               AND email = $3
               AND replaced_at IS NULL
               AND created_at < $4
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.replace_users", None);
        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.replaced_at)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(params.email)
            .bind::<BigInt, _>(params.replaced_at)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Given ONLY a particular `node_id`, update the users table to indicate an unassigned
    /// node by updating the `replaced_at` field with the current time since Unix Epoch.
    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> DbResult<results::UnassignNode> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = $1
             WHERE nodeid = $2
        "#;

        let current_time: i64 = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(current_time)
            .bind::<BigInt, _>(params.node_id)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Given ONLY a particular `uid`, update the users table `created_at` value
    /// with the passed parameter.
    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> DbResult<results::SetUserCreatedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET created_at = $1
             WHERE uid = $2
        "#;

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.created_at)
            .bind::<BigInt, _>(params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Given ONLY a particular `uid`, update the users table `replaced_at` value
    /// with the passed parameter.
    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> DbResult<results::SetUserReplacedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = $1
             WHERE uid = $2
        "#;

        diesel::sql_query(QUERY)
            .bind::<BigInt, _>(params.replaced_at)
            .bind::<BigInt, _>(params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Simple check function to ensure database liveliness.
    async fn check(&mut self) -> DbResult<results::Check> {
        diesel::sql_query("SELECT 1")
            .execute(&mut self.conn)
            .await?;
        Ok(true)
    }
    #[allow(dead_code)]
    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, params: params::SpannerNodeId) {
        self.spanner_node_id = params;
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

    async fn get_users(&mut self, params: params::GetUsers) -> Result<results::GetUsers, DbError> {
        TokenserverPgDb::get_users(self, params).await
    }

    async fn get_or_create_user(
        &mut self,
        params: params::GetOrCreateUser,
    ) -> Result<results::GetOrCreateUser, DbError> {
        TokenserverPgDb::get_or_create_user(self, params).await
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
