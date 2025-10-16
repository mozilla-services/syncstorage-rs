#[macro_use]
extern crate slog_scope;

mod error;
pub mod params;
pub mod results;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use syncserver_common::Metrics;
use syncserver_db_common::{GetPoolState, PoolState};

pub use crate::error::DbError;

pub type DbResult<T> = Result<T, DbError>;

/// The maximum possible generation number. Used as a tombstone to mark users that have been
/// "retired" from the db.
pub const MAX_GENERATION: i64 = i64::MAX;

#[async_trait(?Send)]
pub trait DbPool: Sync + Send + GetPoolState {
    async fn init(&mut self) -> DbResult<()>;

    async fn get(&self) -> DbResult<Box<dyn Db>>;

    fn box_clone(&self) -> Box<dyn DbPool>;
}

impl GetPoolState for Box<dyn DbPool> {
    fn state(&self) -> PoolState {
        (**self).state()
    }
}

impl Clone for Box<dyn DbPool> {
    fn clone(&self) -> Box<dyn DbPool> {
        self.box_clone()
    }
}

#[async_trait(?Send)]
pub trait Db {
    /// Return the Db instance timeout duration.
    fn timeout(&self) -> Option<Duration> {
        None
    }

    /// Mark the user with the given uid and service ID as being replaced.
    async fn replace_user(&mut self, params: params::ReplaceUser)
        -> DbResult<results::ReplaceUser>;

    /// Mark users matching the given email and service ID as replaced.
    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> DbResult<results::ReplaceUsers>;

    /// Post complete user object and get last insert ID.
    async fn post_user(&mut self, params: params::PostUser) -> DbResult<results::PostUser>;

    /// Based on service_id, email, generation, and changed keys timestamp, update user.
    async fn put_user(&mut self, params: params::PutUser) -> DbResult<results::PutUser>;

    /// Show database uptime status and health as boolean.
    async fn check(&mut self) -> DbResult<results::Check>;

    /// Get Node ID based on service_id and node string.
    async fn get_node_id(&mut self, params: params::GetNodeId) -> DbResult<results::GetNodeId>;

    /// Get Node ID and string identifier based on node
    /// with lowest capacity or high release rate.
    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> DbResult<results::GetBestNode>;

    /// Add a user to a specific node, based on service and node string.
    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> DbResult<results::AddUserToNode>;

    /// Get vector of users based on passed in service and FxA email.
    async fn get_users(&mut self, params: params::GetUsers) -> DbResult<results::GetUsers>;

    /// Get the service id by passing in service string identifier.
    async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> DbResult<results::GetServiceId>;

    /// Return the Db instance Metrics.
    fn metrics(&self) -> &Metrics;

    /// Gets the user with the given email and service ID.
    /// If one doesn't exist, allocates a new user.
    async fn get_or_create_user(
        &mut self,
        params: params::GetOrCreateUser,
    ) -> DbResult<results::GetOrCreateUser> {
        let mut raw_users = self
            .get_users(params::GetUsers {
                service_id: params.service_id,
                email: params.email.clone(),
            })
            .await?;

        if raw_users.is_empty() {
            // There are no users in the database with the given email and service ID, so
            // allocate a new one.
            let allocate_user_result = self
                .allocate_user(params.clone() as params::AllocateUser)
                .await?;

            Ok(results::GetOrCreateUser {
                uid: allocate_user_result.uid,
                email: params.email,
                client_state: params.client_state,
                generation: params.generation,
                node: allocate_user_result.node,
                keys_changed_at: params.keys_changed_at,
                created_at: allocate_user_result.created_at,
                replaced_at: None,
                first_seen_at: allocate_user_result.created_at,
                old_client_states: vec![],
            })
        } else {
            raw_users.sort_by_key(|raw_user| (raw_user.generation, raw_user.created_at));
            raw_users.reverse();

            // The user with the greatest `generation` and `created_at` is the current user
            let raw_user = raw_users[0].clone();

            // Collect any old client states that differ from the current client state
            let old_client_states: Vec<String> = {
                raw_users[1..]
                    .iter()
                    .map(|user| user.client_state.clone())
                    .filter(|client_state| client_state != &raw_user.client_state)
                    .collect()
            };

            // Make sure every old row is marked as replaced. They might not be, due to races in row
            // creation.
            for old_user in &raw_users[1..] {
                if old_user.replaced_at.is_none() {
                    let params = params::ReplaceUser {
                        uid: old_user.uid,
                        service_id: params.service_id,
                        replaced_at: raw_user.created_at,
                    };

                    self.replace_user(params).await?;
                }
            }

            let first_seen_at = raw_users[raw_users.len() - 1].created_at;

            match (raw_user.replaced_at, raw_user.node) {
                // If the most up-to-date user is marked as replaced or does not have a node
                // assignment, allocate a new user. Note that, if the current user is marked
                // as replaced, we do not want to create a new user with the account metadata
                // in the parameters to this method. Rather, we want to create a duplicate of
                // the replaced user assigned to a new node. This distinction is important
                // because the account metadata in the parameters to this method may not match
                // that currently stored on the most up-to-date user and may be invalid.
                (Some(_), _) | (_, None) if raw_user.generation < MAX_GENERATION => {
                    let allocate_user_result = {
                        self.allocate_user(params::AllocateUser {
                            service_id: params.service_id,
                            email: params.email.clone(),
                            generation: raw_user.generation,
                            client_state: raw_user.client_state.clone(),
                            keys_changed_at: raw_user.keys_changed_at,
                            capacity_release_rate: params.capacity_release_rate,
                        })
                        .await?
                    };

                    Ok(results::GetOrCreateUser {
                        uid: allocate_user_result.uid,
                        email: params.email,
                        client_state: raw_user.client_state,
                        generation: raw_user.generation,
                        node: allocate_user_result.node,
                        keys_changed_at: raw_user.keys_changed_at,
                        created_at: allocate_user_result.created_at,
                        replaced_at: None,
                        first_seen_at,
                        old_client_states,
                    })
                }
                // The most up-to-date user has a node. Note that this user may be retired or
                // replaced.
                (_, Some(node)) => Ok(results::GetOrCreateUser {
                    uid: raw_user.uid,
                    email: params.email,
                    client_state: raw_user.client_state,
                    generation: raw_user.generation,
                    node,
                    keys_changed_at: raw_user.keys_changed_at,
                    created_at: raw_user.created_at,
                    replaced_at: None,
                    first_seen_at,
                    old_client_states,
                }),
                // The most up-to-date user doesn't have a node and is retired. This is an internal
                // service error for compatibility reasons (the legacy Tokenserver returned an
                // internal service error in this situation).
                (_, None) => {
                    let uid = raw_user.uid;
                    warn!("Tokenserver user retired"; "uid" => &uid);
                    Err(DbError::internal("Tokenserver user retired".to_owned()))
                }
            }
        }
    }

    /// Creates a new user and assigns them to a node.
    async fn allocate_user(
        &mut self,
        params: params::AllocateUser,
    ) -> DbResult<results::AllocateUser> {
        let mut metrics = self.metrics().clone();
        metrics.start_timer("storage.allocate_user", None);

        // Get the least-loaded node
        let node = self
            .get_best_node(params::GetBestNode {
                service_id: params.service_id,
                capacity_release_rate: params.capacity_release_rate,
            })
            .await?;

        // Decrement `available` and increment `current_load` on the node assigned to the user.
        self.add_user_to_node(params::AddUserToNode {
            service_id: params.service_id,
            node: node.node.clone(),
        })
        .await?;

        let created_at = {
            let start = SystemTime::now();
            start.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
        };
        let uid = self
            .post_user(params::PostUser {
                service_id: params.service_id,
                email: params.email.clone(),
                generation: params.generation,
                client_state: params.client_state.clone(),
                created_at,
                node_id: node.id,
                keys_changed_at: params.keys_changed_at,
            })
            .await?
            .uid;

        Ok(results::AllocateUser {
            uid,
            node: node.node,
            created_at,
        })
    }

    // Internal methods used by the db tests

    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> DbResult<results::SetUserCreatedAt>;

    /// Update users replaced_at attribute based on user uid.
    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> DbResult<results::SetUserReplacedAt>;

    /// Get full user object based on passed user ID.
    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> DbResult<results::GetUser>;

    /// Create a complete node and return insert id from node.
    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> DbResult<results::PostNode>;

    /// Get complete node entry based on passed id.
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> DbResult<results::GetNode>;

    /// Based on Node ID, unassign node from `users`.
    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> DbResult<results::UnassignNode>;

    /// Remove Node based on Node ID
    #[cfg(debug_assertions)]
    async fn remove_node(&mut self, params: params::RemoveNode) -> DbResult<results::RemoveNode>;

    #[cfg(debug_assertions)]
    /// Creates new service and returns new service_id.
    async fn post_service(&mut self, params: params::PostService)
        -> DbResult<results::PostService>;

    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, params: params::SpannerNodeId);
}
