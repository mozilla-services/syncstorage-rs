use std::time::Duration;
#[cfg(debug_assertions)]
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use diesel::{
    OptionalExtension,
    sql_types::{Bigint, Float, Integer, Nullable, Text},
};
use diesel_async::RunQueryDsl;
use http::StatusCode;
use syncserver_common::Metrics;
use tokenserver_db_common::{Db, DbError, DbResult, params, results};

use super::TokenserverDb;

#[async_trait(?Send)]
impl Db for TokenserverDb {
    async fn get_node_id(&mut self, params: params::GetNodeId) -> DbResult<results::GetNodeId> {
        const QUERY: &str = r#"
            SELECT id
              FROM nodes
             WHERE service = ?
               AND node = ?
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

    /// Mark users matching the given email and service ID as replaced.
    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> DbResult<results::ReplaceUsers> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE service = ?
               AND email = ?
               AND replaced_at IS NULL
               AND created_at < ?
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.replace_users", None);

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Integer, _>(&params.service_id)
            .bind::<Text, _>(&params.email)
            .bind::<Bigint, _>(params.replaced_at)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Mark the user with the given uid and service ID as being replaced.
    async fn replace_user(
        &mut self,
        params: params::ReplaceUser,
    ) -> DbResult<results::ReplaceUser> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE service = ?
               AND uid = ?
        "#;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Integer, _>(params.service_id)
            .bind::<Bigint, _>(params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Update the user with the given email and service ID with the given `generation` and
    /// `keys_changed_at`.
    async fn put_user(&mut self, params: params::PutUser) -> DbResult<results::PutUser> {
        // The `where` clause on this statement is designed as an extra layer of
        // protection, to ensure that concurrent updates don't accidentally move
        // timestamp fields backwards in time. The handling of `keys_changed_at`
        // is additionally weird because we want to treat the default `NULL` value
        // as zero.
        const QUERY: &str = r#"
            UPDATE users
               SET generation = ?,
                   keys_changed_at = ?
             WHERE service = ?
               AND email = ?
               AND generation <= ?
               AND COALESCE(keys_changed_at, 0) <= COALESCE(?, keys_changed_at, 0)
               AND replaced_at IS NULL
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.put_user", None);

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.generation)
            .bind::<Nullable<Bigint>, _>(params.keys_changed_at)
            .bind::<Integer, _>(&params.service_id)
            .bind::<Text, _>(&params.email)
            .bind::<Bigint, _>(params.generation)
            .bind::<Nullable<Bigint>, _>(params.keys_changed_at)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    /// Create a new user.
    async fn post_user(&mut self, user: params::PostUser) -> DbResult<results::PostUser> {
        const QUERY: &str = r#"
            INSERT INTO users (service, email, generation, client_state, created_at, nodeid, keys_changed_at, replaced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NULL);
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.post_user", None);

        diesel::sql_query(QUERY)
            .bind::<Integer, _>(user.service_id)
            .bind::<Text, _>(&user.email)
            .bind::<Bigint, _>(user.generation)
            .bind::<Text, _>(&user.client_state)
            .bind::<Bigint, _>(user.created_at)
            .bind::<Bigint, _>(user.node_id)
            .bind::<Nullable<Bigint>, _>(user.keys_changed_at)
            .execute(&mut self.conn)
            .await?;

        let result = diesel::sql_query(Self::LAST_INSERT_UID_QUERY)
            .get_result::<results::PostUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    async fn check(&mut self) -> DbResult<results::Check> {
        diesel::sql_query("SELECT 1")
            .execute(&mut self.conn)
            .await?;
        Ok(true)
    }

    /// Gets the least-loaded node that has available slots.
    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> DbResult<results::GetBestNode> {
        const DEFAULT_CAPACITY_RELEASE_RATE: f32 = 0.1;
        const GET_BEST_NODE_QUERY: &str = r#"
              SELECT id, node
                FROM nodes
               WHERE service = ?
                 AND available > 0
                 AND capacity > current_load
                 AND downed = 0
                 AND backoff = 0
            ORDER BY LOG(current_load) / LOG(capacity)
               LIMIT 1
        "#;
        const RELEASE_CAPACITY_QUERY: &str = r#"
            UPDATE nodes
               SET available = LEAST(capacity * ?, capacity - current_load)
             WHERE service = ?
               AND available <= 0
               AND capacity > current_load
               AND downed = 0
        "#;
        const SPANNER_QUERY: &str = r#"
              SELECT id, node
                FROM nodes
               WHERE id = ?
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
                    let mut db_error =
                        DbError::internal(format!("unable to get Spanner node: {}", e));
                    db_error.status = StatusCode::SERVICE_UNAVAILABLE;
                    db_error
                })
        } else {
            // We may have to retry the query if we need to release more capacity. This loop allows
            // a maximum of five retries before bailing out.
            for _ in 0..5 {
                let maybe_result = diesel::sql_query(GET_BEST_NODE_QUERY)
                    .bind::<Integer, _>(params.service_id)
                    .get_result::<results::GetBestNode>(&mut self.conn)
                    .await
                    .optional()?;

                if let Some(result) = maybe_result {
                    return Ok(result);
                }

                // There were no available nodes. Try to release additional capacity from any nodes
                // that are not fully occupied.
                let affected_rows = diesel::sql_query(RELEASE_CAPACITY_QUERY)
                    .bind::<Float, _>(
                        params
                            .capacity_release_rate
                            .unwrap_or(DEFAULT_CAPACITY_RELEASE_RATE),
                    )
                    .bind::<Integer, _>(params.service_id)
                    .execute(&mut self.conn)
                    .await?;

                // If no nodes were affected by the last query, give up.
                if affected_rows == 0 {
                    break;
                }
            }

            let mut db_error = DbError::internal("unable to get a node".to_owned());
            db_error.status = StatusCode::SERVICE_UNAVAILABLE;
            Err(db_error)
        }
    }

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
             WHERE service = ?
               AND node = ?
        "#;
        const SPANNER_QUERY: &str = r#"
            UPDATE nodes
               SET current_load = current_load + 1
             WHERE service = ?
               AND node = ?
        "#;

        let query = if self.spanner_node_id.is_some() {
            SPANNER_QUERY
        } else {
            QUERY
        };

        diesel::sql_query(query)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(&params.node)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    async fn get_users(&mut self, params: params::GetUsers) -> DbResult<results::GetUsers> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_users", None);

        const QUERY: &str = r#"
                     SELECT uid, nodes.node, generation, keys_changed_at, client_state, created_at,
                            replaced_at
                       FROM users
            LEFT OUTER JOIN nodes ON users.nodeid = nodes.id
                      WHERE email = ?
                        AND users.service = ?
                   ORDER BY created_at DESC, uid DESC
                      LIMIT 20
        "#;

        let result = diesel::sql_query(QUERY)
            .bind::<Text, _>(&params.email)
            .bind::<Integer, _>(params.service_id)
            .load::<results::GetRawUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> DbResult<results::GetServiceId> {
        const QUERY: &str = r#"
            SELECT id
              FROM services
             WHERE service = ?
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

    fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> DbResult<results::SetUserCreatedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET created_at = ?
             WHERE uid = ?
        "#;
        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.created_at)
            .bind::<Bigint, _>(&params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> DbResult<results::SetUserReplacedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE uid = ?
        "#;
        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Bigint, _>(&params.uid)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> DbResult<results::GetUser> {
        const QUERY: &str = r#"
            SELECT service, email, generation, client_state, replaced_at, nodeid, keys_changed_at
              FROM users
             WHERE uid = ?
        "#;

        let result = diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.id)
            .get_result::<results::GetUser>(&mut self.conn)
            .await?;
        Ok(result)
    }

    async fn insert_sync15_node(&mut self, params: params::Sync15Node) -> DbResult<()> {
        let query = format!(
            r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES (
                (SELECT id FROM services WHERE service = '{}'),
                ?, 1, 0, ?, 0, 0
            )
            "#,
            params::Sync15Node::SERVICE_NAME
        );

        diesel::sql_query(query)
            .bind::<Text, _>(&params.node)
            .bind::<Integer, _>(params.capacity)
            .execute(&mut self.conn)
            .await?;

        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> DbResult<results::PostNode> {
        const QUERY: &str = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;
        diesel::sql_query(QUERY)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(&params.node)
            .bind::<Integer, _>(params.available)
            .bind::<Integer, _>(params.current_load)
            .bind::<Integer, _>(params.capacity)
            .bind::<Integer, _>(params.downed)
            .bind::<Integer, _>(params.backoff)
            .execute(&mut self.conn)
            .await?;

        let result = diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::PostNode>(&mut self.conn)
            .await?;
        Ok(result)
    }

    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> DbResult<results::GetNode> {
        const QUERY: &str = r#"
            SELECT *
              FROM nodes
             WHERE id = ?
        "#;

        let result = diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.id)
            .get_result::<results::GetNode>(&mut self.conn)
            .await?;
        Ok(result)
    }

    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> DbResult<results::UnassignNode> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE nodeid = ?
        "#;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(current_time)
            .bind::<Bigint, _>(params.node_id)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn remove_node(&mut self, params: params::RemoveNode) -> DbResult<results::RemoveNode> {
        const QUERY: &str = "DELETE FROM nodes WHERE id = ?";

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.node_id)
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> DbResult<results::PostService> {
        const INSERT_SERVICE_QUERY: &str = r#"
            INSERT INTO services (service, pattern)
            VALUES (?, ?)
        "#;

        diesel::sql_query(INSERT_SERVICE_QUERY)
            .bind::<Text, _>(&params.service)
            .bind::<Text, _>(&params.pattern)
            .execute(&mut self.conn)
            .await?;

        let result = diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::PostService>(&mut self.conn)
            .await?;
        Ok(result)
    }

    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, params: params::SpannerNodeId) {
        self.spanner_node_id = params;
    }
}
