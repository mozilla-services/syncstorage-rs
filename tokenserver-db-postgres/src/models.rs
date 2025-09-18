use std::time::Duration;

use super::pool::Conn;
use async_trait::async_trait;
use syncserver_common::Metrics;
use tokenserver_db_common::{error::DbError, params, results, Db};

#[allow(dead_code)]
pub struct TokenserverPgDb {
    conn: Conn,
    metrics: Metrics,
    service_id: Option<i32>,
    spanner_node_id: Option<i32>,
    pub timeout: Option<Duration>,
}

impl TokenserverPgDb {
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
}

#[async_trait(?Send)]
impl Db for TokenserverPgDb {
    fn timeout(&self) -> Option<Duration> {
        self.timeout
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

    async fn post_service(
        &mut self,
        params: params::PostService,
    ) -> Result<results::PostService, DbError> {
        TokenserverPgDb::post_service(self, params).await
    }

    // Nodes Methods
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

    async fn post_node(&mut self, params: params::PostNode) -> Result<results::PostNode, DbError> {
        TokenserverPgDb::post_node(self, params).await
    }

    async fn add_user_to_node(
        &mut self,
        params: params::AddUserToNode,
    ) -> Result<results::AddUserToNode, DbError> {
        TokenserverPgDb::add_user_to_node(self, params).await
    }

    async fn remove_node(
        &mut self,
        params: params::RemoveNode,
    ) -> Result<results::RemoveNode, DbError> {
        TokenserverPgDb::remove_node(self, params).await
    }

    // Users Methods
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

    async fn unassign_node(
        &mut self,
        params: params::UnassignNode,
    ) -> Result<results::UnassignNode, DbError> {
        TokenserverPgDb::unassign_node(self, params).await
    }

    async fn set_user_created_at(
        &mut self,
        params: params::SetUserCreatedAt,
    ) -> Result<results::SetUserCreatedAt, DbError> {
        TokenserverPgDb::set_user_created_at(self, params).await
    }

    async fn set_user_replaced_at(
        &mut self,
        params: params::SetUserReplacedAt,
    ) -> Result<results::SetUserReplacedAt, DbError> {
        TokenserverPgDb::set_user_replaced_at(self, params).await
    }
}
