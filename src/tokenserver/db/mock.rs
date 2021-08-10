#![allow(clippy::new_without_default)]

use futures::future;

use super::models::{Db, DbFuture};
use super::params;
use super::pool::DbPool;
use super::results;
use crate::db::error::DbError;

#[derive(Clone, Debug)]
pub struct MockDbPool;

impl MockDbPool {
    pub fn new() -> Self {
        MockDbPool
    }
}

impl DbPool for MockDbPool {
    fn get(&self) -> Result<Box<dyn Db>, DbError> {
        Ok(Box::new(MockDb::new()))
    }

    fn box_clone(&self) -> Box<dyn DbPool> {
        Box::new(self.clone())
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
    fn get_user(&self, _params: params::GetUser) -> DbFuture<'_, results::GetUser> {
        Box::pin(future::ok(results::GetUser::default()))
    }

    fn replace_users(&self, _params: params::ReplaceUsers) -> DbFuture<'_, results::ReplaceUsers> {
        Box::pin(future::ok(()))
    }

    fn post_user(&self, _params: params::PostUser) -> DbFuture<'_, results::PostUser> {
        Box::pin(future::ok(results::PostUser::default()))
    }

    fn put_user(&self, _params: params::PutUser) -> DbFuture<'_, results::PutUser> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn set_user_created_at(
        &self,
        _params: params::SetUserCreatedAt,
    ) -> DbFuture<'_, results::SetUserCreatedAt> {
        Box::pin(future::ok(()))
    }

    #[cfg(test)]
    fn get_users(&self, _params: params::GetRawUsers) -> DbFuture<'_, results::GetRawUsers> {
        Box::pin(future::ok(results::GetRawUsers::default()))
    }

    #[cfg(test)]
    fn post_node(&self, _params: params::PostNode) -> DbFuture<'_, results::PostNode> {
        Box::pin(future::ok(results::PostNode::default()))
    }

    #[cfg(test)]
    fn post_service(&self, _params: params::PostService) -> DbFuture<'_, results::PostService> {
        Box::pin(future::ok(results::PostService::default()))
    }
}
