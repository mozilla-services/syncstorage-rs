//! Common database interface for Tokenserver.
//!
//! This crate provides the core database abstraction layer used by Tokenserver implementations.
//! It defines traits, types, and common functionality for interacting with Tokenserver,
//! supporting multiple database backends (MySQL & PostgreSQL).
//!
//! The main components are:
//! - [`DbPool`] trait for managing database connection pools
//! - [`Db`] trait defining all database operations
//! - [`DbError`] for database error handling
//! - Parameter types in the [`params`] module
//! - Result types in the [`results`] module

#[macro_use]
extern crate slog_scope;

mod error;
/// Parameter types for database operations.
pub mod params;
/// Result types returned from database operations.
pub mod results;

use std::{
    cmp,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use syncserver_common::Metrics;
use syncserver_db_common::{GetPoolState, PoolState};

pub use crate::error::DbError;

/// Result type for database operations that may fail with a [`DbError`].
pub type DbResult<T> = Result<T, DbError>;

/// The maximum possible generation number. Used as a tombstone to mark users that have been
/// "retired" from the db.
pub const MAX_GENERATION: i64 = i64::MAX;

/// Trait for managing a pool of database connections.
///
/// Implementors of this trait provide connection pooling functionality for a specific
/// database backend. The pool can be cloned and shared across threads.
#[async_trait(?Send)]
pub trait DbPool: Sync + Send + GetPoolState {
    /// Initializes the database pool.
    ///
    /// This performs tasks such as running migrations or verifying connectivity.
    async fn init(&mut self) -> DbResult<()>;

    /// Gets a database connection from the pool.
    ///
    /// Returns a boxed [`Db`] trait object representing an active database connection.
    async fn get(&self) -> DbResult<Box<dyn Db>>;

    /// Creates a clone of this pool as a boxed trait object.
    ///
    /// This is used to implement [`Clone`] for `Box<dyn DbPool>`.
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

/// Trait defining all database operations for Tokenserver.
///
/// This trait provides the complete interface for interacting with a Tokenserver database,
/// including user management, node allocation, and service queries.
#[async_trait(?Send)]
pub trait Db {
    /// Returns the timeout duration for this database connection.
    ///
    /// If `None`, no timeout is configured.
    fn timeout(&self) -> Option<Duration> {
        None
    }

    /// Marks the user with the given uid and service ID as being replaced.
    ///
    /// This is used when a user is moved to a new record or node.
    async fn replace_user(&mut self, params: params::ReplaceUser)
    -> DbResult<results::ReplaceUser>;

    /// Marks all users matching the given email and service ID as replaced.
    ///
    /// This is used when all existing user records for an email should be retired.
    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> DbResult<results::ReplaceUsers>;

    /// Creates a new user record and returns the assigned user ID.
    async fn post_user(&mut self, params: params::PostUser) -> DbResult<results::PostUser>;

    /// Updates an existing user's `generation` and `keys_changed_at` timestamp.
    ///
    /// The user is identified by service_id and email.
    async fn put_user(&mut self, params: params::PutUser) -> DbResult<results::PutUser>;

    /// Checks the database health and availability.
    ///
    /// Returns `true` if the database is healthy and accessible.
    async fn check(&mut self) -> DbResult<results::Check>;

    /// Inserts an initial Sync 1.5 node record into the database.
    ///
    /// Does nothing if a node with the same identifier already exists.
    async fn insert_sync15_node(&mut self, params: params::Sync15Node) -> DbResult<()>;

    /// Retrieves the node ID for a given service and node identifier string.
    async fn get_node_id(&mut self, params: params::GetNodeId) -> DbResult<results::GetNodeId>;

    /// Retrieves the best available node for user allocation.
    ///
    /// Selects a node based on capacity and load.
    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> DbResult<results::GetBestNode>;

    /// Assigns a user to a specific node by updating the node's capacity and load.
    ///
    /// Decrements available capacity and increments current load for the specified node.
    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> DbResult<results::AddUserToNode>;

    /// Retrieves all user records matching the given email and service ID.
    async fn get_users(&mut self, params: params::GetUsers) -> DbResult<results::GetUsers>;

    /// Retrieves the service ID for a given service name.
    async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> DbResult<results::GetServiceId>;

    /// Returns the metrics collector for this database connection.
    fn metrics(&self) -> &Metrics;

    /// Gets the user with the given email and service ID, or creates a new one if needed.
    ///
    /// This is the primary method for user retrieval in Tokenserver. It handles several scenarios:
    /// - If no user exists, allocates a new user on an available node
    /// - If users exist, returns the most recent (highest generation/created_at)
    /// - Handles replaced users by creating new user records
    /// - Ensures old user records are marked as replaced
    /// - Collects old client states for validation
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
            raw_users
                .sort_by_key(|raw_user| cmp::Reverse((raw_user.generation, raw_user.created_at)));

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

    /// Creates a new user record and assigns them to an available node.
    ///
    /// This method:
    /// 1. Finds the best available node based on capacity
    /// 2. Updates the node's capacity and load
    /// 3. Creates a new user record with a generated UID
    /// 4. Returns the new user's UID, assigned node, and creation timestamp
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

    /// Updates the created_at timestamp for a user. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> DbResult<results::SetUserCreatedAt>;

    /// Updates the replaced_at timestamp for a user. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> DbResult<results::SetUserReplacedAt>;

    /// Retrieves a complete user record by user ID. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> DbResult<results::GetUser>;

    /// Creates a new node record and returns its ID. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> DbResult<results::PostNode>;

    /// Retrieves a complete node record by node ID. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> DbResult<results::GetNode>;

    /// Unassigns a node from all users by clearing their node assignments. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> DbResult<results::UnassignNode>;

    /// Removes a node record from the database. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn remove_node(&mut self, params: params::RemoveNode) -> DbResult<results::RemoveNode>;

    /// Creates a new service and returns its service ID. Only available in debug builds.
    #[cfg(debug_assertions)]
    async fn post_service(&mut self, params: params::PostService)
    -> DbResult<results::PostService>;

    /// Sets the Spanner node ID for testing. Only available in debug builds.
    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, params: params::SpannerNodeId);
}
