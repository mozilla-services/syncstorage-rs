#![allow(clippy::new_without_default)]

use futures::future;

use super::models::{Db, DbFuture};
#[cfg(test)]
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
    fn get_user(&self, _email: String) -> DbFuture<'_, results::GetUser> {
        Box::pin(future::ok(results::GetUser::default()))
    }

    #[cfg(test)]
    fn post_node(&self, _node: params::PostNode) -> DbFuture<'_, results::PostNode> {
        Box::pin(future::ok(results::PostNode::default()))
    }

    #[cfg(test)]
    fn post_service(&self, _service: params::PostService) -> DbFuture<'_, results::PostService> {
        Box::pin(future::ok(results::PostService::default()))
    }
    #[cfg(test)]
    fn post_user(&self, _user: params::PostUser) -> DbFuture<'_, results::PostUser> {
        Box::pin(future::ok(results::PostUser::default()))
    }
}
