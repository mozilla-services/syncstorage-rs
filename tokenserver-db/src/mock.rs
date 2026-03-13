#![allow(clippy::new_without_default)]

use std::sync::{Arc, LazyLock, Mutex};

use async_trait::async_trait;
use syncserver_common::Metrics;
use syncserver_db_common::{GetPoolState, PoolState};
use tokenserver_db_common::{Db, DbError, DbPool, params, results};

#[derive(Clone, Default)]
pub struct CallLog {
    pub put_user: Arc<Mutex<Vec<params::PutUser>>>,
    pub retire_user: Arc<Mutex<Vec<params::RetireUser>>>,
}

#[derive(Clone, Default)]
pub struct MockDbPool {
    call_log: CallLog,
}

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool::default()
    }

    pub fn with_capture() -> (Self, CallLog) {
        let pool = MockDbPool::default();
        let call_log = pool.call_log.clone();
        (pool, call_log)
    }
}

#[async_trait(?Send)]
impl DbPool for MockDbPool {
    async fn init(&mut self) -> Result<(), DbError> {
        Ok(())
    }

    async fn get(&self) -> Result<Box<dyn Db>, DbError> {
        Ok(Box::new(MockDb {
            call_log: self.call_log.clone(),
        }))
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

#[derive(Clone, Default)]
pub struct MockDb {
    call_log: CallLog,
}

impl MockDb {
    pub fn new() -> Self {
        MockDb::default()
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

    async fn put_user(&mut self, params: params::PutUser) -> Result<results::PutUser, DbError> {
        self.call_log.put_user.lock().unwrap().push(params);
        Ok(())
    }

    async fn retire_user(
        &mut self,
        params: params::RetireUser,
    ) -> Result<results::RetireUser, DbError> {
        self.call_log.retire_user.lock().unwrap().push(params);
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

    async fn insert_sync15_node(&mut self, _params: params::Sync15Node) -> Result<bool, DbError> {
        Ok(false)
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
