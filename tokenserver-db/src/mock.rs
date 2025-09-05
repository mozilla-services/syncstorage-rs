#![allow(clippy::new_without_default)]

use async_trait::async_trait;
use futures::future;
use syncserver_db_common::{GetPoolState, PoolState};

use super::error::{DbError, DbFuture};
use super::models::Db;
use super::params;
use super::pool::DbPool;
use super::results;

#[derive(Clone, Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

#[async_trait(?Send)]
impl DbPool for MockDbPool {
    async fn init(&mut self) -> Result<(), DbError> {
        Ok(())
    }

    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        Ok(Box::new(MockDb::new()))
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
    }
}

impl GetPoolState for MockDbPool {
    fn state(&self) -> PoolState {
        PoolState::default()
    }
}

#[derive(Clone, Debug)]
pub struct MockDb;

impl MockDb {
    pub fn new() -> Self {
        MockDb
    }
}

impl Db for MockDb {
    fn replace_user(&mut self, _params: params::ReplaceUser) -> DbFuture<'_, results::ReplaceUser> {
        Box::pin(future::ok(()))
    }

    fn replace_users(
        &mut self,
        _params: params::ReplaceUsers,
    ) -> DbFuture<'_, results::ReplaceUsers> {
        Box::pin(future::ok(()))
    }

    fn post_user(&mut self, _params: params::PostUser) -> DbFuture<'_, results::PostUser> {
        Box::pin(future::ok(results::PostUser::default()))
    }

    fn put_user(&mut self, _params: params::PutUser) -> DbFuture<'_, results::PutUser> {
        Box::pin(future::ok(()))
    }

    fn check(&mut self) -> DbFuture<'_, results::Check> {
        Box::pin(future::ok(true))
    }

    fn get_node_id(&mut self, _params: params::GetNodeId) -> DbFuture<'_, results::GetNodeId> {
        Box::pin(future::ok(results::GetNodeId::default()))
    }

    fn get_best_node(
        &mut self,
        _params: params::GetBestNode,
    ) -> DbFuture<'_, results::GetBestNode> {
        Box::pin(future::ok(results::GetBestNode::default()))
    }

    fn add_user_to_node(
        &mut self,
        _params: params::AddUserToNode,
    ) -> DbFuture<'_, results::AddUserToNode> {
        Box::pin(future::ok(()))
    }

    fn get_users(&mut self, _params: params::GetUsers) -> DbFuture<'_, results::GetUsers> {
        Box::pin(future::ok(results::GetUsers::default()))
    }

    fn get_or_create_user(
        &mut self,
        _params: params::GetOrCreateUser,
    ) -> DbFuture<'_, results::GetOrCreateUser> {
        Box::pin(future::ok(results::GetOrCreateUser::default()))
    }

    fn get_service_id(
        &mut self,
        _params: params::GetServiceId,
    ) -> DbFuture<'_, results::GetServiceId> {
        Box::pin(future::ok(results::GetServiceId::default()))
    }

    #[cfg(test)]
    fn set_user_created_at(
        &mut self,
        _params: params::SetUserCreatedAt,
    ) -> DbFuture<'_, results::SetUserCreatedAt> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn set_user_replaced_at(
        &mut self,
        _params: params::SetUserReplacedAt,
    ) -> DbFuture<'_, results::SetUserReplacedAt> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn get_user(&mut self, _params: params::GetUser) -> DbFuture<'_, results::GetUser> {
        Box::pin(future::ok(results::GetUser::default()))
    }

    #[cfg(test)]
    fn post_node(&mut self, _params: params::PostNode) -> DbFuture<'_, results::PostNode> {
        Box::pin(future::ok(results::PostNode::default()))
    }

    #[cfg(test)]
    fn get_node(&mut self, _params: params::GetNode) -> DbFuture<'_, results::GetNode> {
        Box::pin(future::ok(results::GetNode::default()))
    }

    #[cfg(test)]
    fn unassign_node(
        &mut self,
        _params: params::UnassignNode,
    ) -> DbFuture<'_, results::UnassignNode> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn remove_node(&mut self, _params: params::RemoveNode) -> DbFuture<'_, results::RemoveNode> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn post_service(&mut self, _params: params::PostService) -> DbFuture<'_, results::PostService> {
        Box::pin(future::ok(results::PostService::default()))
    }
}
