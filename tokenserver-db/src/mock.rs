#![allow(clippy::new_without_default)]

use std::sync::LazyLock;

use async_trait::async_trait;
use syncserver_common::Metrics;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_db_common::{params, results, Db, DbError, DbPool};

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

#[async_trait(?Send)]
impl Db for MockDb {
    async fn replace_user(
        &mut self,
        _params: params::ReplaceUser,
    ) -> Result<results::ReplaceUser, DbError> {
        Ok(())
    }

    async fn replace_users(
        &mut self,
        _params: params::ReplaceUsers,
    ) -> Result<results::ReplaceUsers, DbError> {
        Ok(())
    }

    async fn post_user(&mut self, _params: params::PostUser) -> Result<results::PostUser, DbError> {
        Ok(results::PostUser::default())
    }

    async fn put_user(&mut self, _params: params::PutUser) -> Result<results::PutUser, DbError> {
        Ok(())
    }

    async fn check(&mut self) -> Result<results::Check, DbError> {
        Ok(true)
    }

    async fn get_node_id(
        &mut self,
        _params: params::GetNodeId,
    ) -> Result<results::GetNodeId, DbError> {
        Ok(results::GetNodeId::default())
    }

    async fn get_best_node(
        &mut self,
        _params: params::GetBestNode,
    ) -> Result<results::GetBestNode, DbError> {
        Ok(results::GetBestNode::default())
    }

    async fn add_user_to_node(
        &mut self,
        _params: params::AddUserToNode,
    ) -> Result<results::AddUserToNode, DbError> {
        Ok(())
    }

    async fn get_users(&mut self, _params: params::GetUsers) -> Result<results::GetUsers, DbError> {
        Ok(results::GetUsers::default())
    }

    async fn get_or_create_user(
        &mut self,
        _params: params::GetOrCreateUser,
    ) -> Result<results::GetOrCreateUser, DbError> {
        Ok(results::GetOrCreateUser::default())
    }

    async fn get_service_id(
        &mut self,
        _params: params::GetServiceId,
    ) -> Result<results::GetServiceId, DbError> {
        Ok(results::GetServiceId::default())
    }

    fn metrics(&self) -> &Metrics {
        static METRICS: LazyLock<Metrics> = LazyLock::new(Metrics::noop);
        &METRICS
    }

    #[cfg(debug_assertions)]
    async fn set_user_created_at(
        &mut self,
        _params: params::SetUserCreatedAt,
    ) -> Result<results::SetUserCreatedAt, DbError> {
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn set_user_replaced_at(
        &mut self,
        _params: params::SetUserReplacedAt,
    ) -> Result<results::SetUserReplacedAt, DbError> {
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn get_user(&mut self, _params: params::GetUser) -> Result<results::GetUser, DbError> {
        Ok(results::GetUser::default())
    }

    #[cfg(debug_assertions)]
    async fn post_node(&mut self, _params: params::PostNode) -> Result<results::PostNode, DbError> {
        Ok(results::PostNode::default())
    }

    #[cfg(debug_assertions)]
    async fn get_node(&mut self, _params: params::GetNode) -> Result<results::GetNode, DbError> {
        Ok(results::GetNode::default())
    }

    #[cfg(debug_assertions)]
    async fn unassign_node(
        &mut self,
        _params: params::UnassignNode,
    ) -> Result<results::UnassignNode, DbError> {
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn remove_node(
        &mut self,
        _params: params::RemoveNode,
    ) -> Result<results::RemoveNode, DbError> {
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn post_service(
        &mut self,
        _params: params::PostService,
    ) -> Result<results::PostService, DbError> {
        Ok(results::PostService::default())
    }

    #[cfg(debug_assertions)]
    fn set_spanner_node_id(&mut self, _params: params::SpannerNodeId) {}
}
