pub mod error;
pub mod params;
pub mod results;

use std::time::Duration;

use async_trait::async_trait;
use syncserver_db_common::{GetPoolState, PoolState};

use crate::error::DbError;

#[async_trait(?Send)]
pub trait DbPool: Sync + Send + GetPoolState {
    async fn init(&mut self) -> Result<(), DbError>;

    async fn get(&self) -> Result<Box<dyn Db>, DbError>;

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
    async fn replace_user(
        &mut self,
        params: params::ReplaceUser,
    ) -> Result<results::ReplaceUser, DbError>;

    /// Mark users matching the given email and service ID as replaced.
    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> Result<results::ReplaceUsers, DbError>;

    /// Post complete user object and get last insert ID.
    async fn post_user(&mut self, params: params::PostUser) -> Result<results::PostUser, DbError>;

    /// Based on service_id, email, generation, and changed keys timestamp, update user.
    async fn put_user(&mut self, params: params::PutUser) -> Result<results::PutUser, DbError>;

    /// Show database uptime status and health as boolean.
    async fn check(&mut self) -> Result<results::Check, DbError>;

    /// Get Node ID based on service_id and node string.
    async fn get_node_id(
        &mut self,
        params: params::GetNodeId,
    ) -> Result<results::GetNodeId, DbError>;

    /// Get Node ID and string identifier based on node
    /// with lowest capacity or high release rate.
    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> Result<results::GetBestNode, DbError>;

    /// Add a user to a specific node, based on service and node string.
    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> Result<results::AddUserToNode, DbError>;

    /// Get vector of users based on passed in service and FxA email.
    async fn get_users(&mut self, params: params::GetUsers) -> Result<results::GetUsers, DbError>;

    /// Gets the user with the given email and service ID.
    /// If one doesn't exist, allocates a new user.
    async fn get_or_create_user(
        &mut self,
        params: params::GetOrCreateUser,
    ) -> Result<results::GetOrCreateUser, DbError>;

    /// Get the service id by passing in service string identifier.
    async fn get_service_id(
        &mut self,
        params: params::GetServiceId,
    ) -> Result<results::GetServiceId, DbError>;

    // Internal methods used by the db tests

    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> Result<results::SetUserCreatedAt, DbError>;

    /// Update users replaced_at attribute based on user uid.
    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> Result<results::SetUserReplacedAt, DbError>;

    /// Get full user object based on passed user ID.
    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> Result<results::GetUser, DbError>;

    /// Create a complete node and return insert id from node.
    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> Result<results::PostNode, DbError>;

    /// Get complete node entry based on passed id.
    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> Result<results::GetNode, DbError>;

    /// Based on Node ID, unassign node from `users`.
    #[cfg(debug_assertions)]
    async fn unassign_node_from_users(
        &mut self,
        params: params::UnassignNode,
    ) -> Result<results::UnassignNode, DbError>;

    /// Remove Node based on Node ID
    #[cfg(debug_assertions)]
    async fn remove_node(
        &mut self,
        params: params::RemoveNode,
    ) -> Result<results::RemoveNode, DbError>;

    #[cfg(debug_assertions)]
    /// Creates new service and returns new service_id.
    async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> Result<results::PostService, DbError>;
}
