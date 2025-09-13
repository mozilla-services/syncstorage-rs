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
    fn timeout(&self) -> Option<Duration> {
        None
    }

    async fn replace_user(
        &mut self,
        params: params::ReplaceUser,
    ) -> Result<results::ReplaceUser, DbError>;

    async fn replace_users(
        &mut self,
        params: params::ReplaceUsers,
    ) -> Result<results::ReplaceUsers, DbError>;

    async fn post_user(&mut self, params: params::PostUser) -> Result<results::PostUser, DbError>;

    async fn put_user(&mut self, params: params::PutUser) -> Result<results::PutUser, DbError>;

    async fn check(&mut self) -> Result<results::Check, DbError>;

    async fn get_node_id(
        &mut self,
        params: params::GetNodeId,
    ) -> Result<results::GetNodeId, DbError>;

    async fn get_best_node(
        &mut self,
        params: params::GetBestNode,
    ) -> Result<results::GetBestNode, DbError>;

    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> Result<results::AddUserToNode, DbError>;

    async fn get_users(&mut self, params: params::GetUsers) -> Result<results::GetUsers, DbError>;

    async fn get_or_create_user(
        &mut self,
        params: params::GetOrCreateUser,
    ) -> Result<results::GetOrCreateUser, DbError>;

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

    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> Result<results::SetUserReplacedAt, DbError>;

    #[cfg(debug_assertions)]
    async fn get_user(&mut self, params: params::GetUser) -> Result<results::GetUser, DbError>;

    #[cfg(debug_assertions)]
    async fn post_node(&mut self, params: params::PostNode) -> Result<results::PostNode, DbError>;

    #[cfg(debug_assertions)]
    async fn get_node(&mut self, params: params::GetNode) -> Result<results::GetNode, DbError>;

    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> Result<results::UnassignNode, DbError>;

    #[cfg(debug_assertions)]
    async fn remove_node(
        &mut self,
        params: params::RemoveNode,
    ) -> Result<results::RemoveNode, DbError>;

    #[cfg(debug_assertions)]
    async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> Result<results::PostService, DbError>;
}
