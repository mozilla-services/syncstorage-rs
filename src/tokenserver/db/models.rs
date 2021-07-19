use actix_web::web::block;
#[cfg(test)]
use diesel::sql_types::{Bigint, Integer, Nullable};
use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, PooledConnection},
    sql_types::Text,
    RunQueryDsl,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use futures::future::LocalBoxFuture;
use futures::TryFutureExt;

use std::{self, sync::Arc};

use super::{params, results};
use crate::db::error::{DbError, DbErrorKind};
use crate::error::ApiError;
use crate::sync_db_method;

pub type DbFuture<'a, T> = LocalBoxFuture<'a, Result<T, ApiError>>;
pub type DbResult<T> = std::result::Result<T, DbError>;
type Conn = PooledConnection<ConnectionManager<MysqlConnection>>;

#[derive(Clone)]
pub struct TokenserverDb {
    /// Synchronous Diesel calls are executed in actix_web::web::block to satisfy
    /// the Db trait's asynchronous interface.
    ///
    /// Arc<MysqlDbInner> provides a Clone impl utilized for safely moving to
    /// the thread pool but does not provide Send as the underlying db
    /// conn. structs are !Sync (Arc requires both for Send). See the Send impl
    /// below.
    pub(super) inner: Arc<DbInner>,
}

/// Despite the db conn structs being !Sync (see Arc<MysqlDbInner> above) we
/// don't spawn multiple MysqlDb calls at a time in the thread pool. Calls are
/// queued to the thread pool via Futures, naturally serialized.
unsafe impl Send for TokenserverDb {}

pub struct DbInner {
    #[cfg(not(test))]
    pub(super) conn: Conn,
    #[cfg(test)]
    pub(super) conn: LoggingConnection<Conn>, // display SQL when RUST_LOG="diesel_logger=trace"
}

impl TokenserverDb {
    pub fn new(conn: Conn) -> Self {
        let inner = DbInner {
            #[cfg(not(test))]
            conn,
            #[cfg(test)]
            conn: LoggingConnection::new(conn),
        };

        Self {
            inner: Arc::new(inner),
        }
    }

    fn get_user_sync(&self, email: String) -> DbResult<results::GetUser> {
        let query = r#"
            SELECT users.uid, users.email, users.client_state, users.generation,
                users.keys_changed_at, users.created_at, nodes.node
            FROM users
            JOIN nodes ON nodes.id = users.nodeid
            WHERE users.email = ?
        "#;
        let mut user_records = diesel::sql_query(query)
            .bind::<Text, _>(email)
            .load::<results::GetUser>(&self.inner.conn)?;

        if user_records.is_empty() {
            return Err(DbErrorKind::TokenserverUserNotFound.into());
        }

        user_records.sort_by_key(|user_record| (user_record.generation, user_record.created_at));
        let user_record = user_records[0].clone();

        Ok(user_record)
    }

    #[cfg(test)]
    fn post_node_sync(&self, node: params::PostNode) -> DbResult<results::PostNode> {
        let query = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
               VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;

        diesel::sql_query(query)
            .bind::<Integer, _>(node.service_id)
            .bind::<Text, _>(&node.node)
            .bind::<Integer, _>(node.available)
            .bind::<Integer, _>(node.current_load)
            .bind::<Integer, _>(node.capacity)
            .bind::<Integer, _>(node.downed)
            .bind::<Integer, _>(node.backoff)
            .execute(&self.inner.conn)?;

        let query = r#"
            SELECT id FROM nodes
            WHERE service = ? AND
                  node = ? AND
                  available = ? AND
                  current_load = ? AND
                  capacity = ? AND
                  downed = ? AND
                  backoff = ?
        "#;

        diesel::sql_query(query)
            .bind::<Integer, _>(node.service_id)
            .bind::<Text, _>(&node.node)
            .bind::<Integer, _>(node.available)
            .bind::<Integer, _>(node.current_load)
            .bind::<Integer, _>(node.capacity)
            .bind::<Integer, _>(node.downed)
            .bind::<Integer, _>(node.backoff)
            .get_result::<results::PostNode>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn post_service_sync(&self, service: params::PostService) -> DbResult<results::PostService> {
        let query = "INSERT INTO services (service, pattern) VALUES (?, ?)";
        diesel::sql_query(query)
            .bind::<Text, _>(&service.service)
            .bind::<Text, _>(service.pattern)
            .execute(&self.inner.conn)?;

        let query = "SELECT id FROM services WHERE service = ? AND pattern = ?";
        diesel::sql_query(query)
            .bind::<Text, _>(&service.service)
            .bind::<Text, _>(&service.service)
            .get_result::<results::PostService>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn post_user_sync(&self, user: params::PostUser) -> DbResult<results::PostUser> {
        let query = r#"
            INSERT INTO users (service, email, generation, client_state, created_at, replaced_at, nodeid, keys_changed_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        diesel::sql_query(query)
            .bind::<Integer, _>(user.service_id)
            .bind::<Text, _>(&user.email)
            .bind::<Bigint, _>(user.generation)
            .bind::<Text, _>(&user.client_state)
            .bind::<Bigint, _>(user.created_at)
            .bind::<Nullable<Bigint>, _>(user.replaced_at)
            .bind::<Bigint, _>(user.node_id)
            .bind::<Nullable<Bigint>, _>(user.keys_changed_at)
            .execute(&self.inner.conn)?;

        let query = "SELECT uid FROM users WHERE email = ?";
        diesel::sql_query(query)
            .bind::<Text, _>(&user.email)
            .get_result::<results::PostUser>(&self.inner.conn)
            .map_err(Into::into)
    }
}

impl Db for TokenserverDb {
    sync_db_method!(get_user, get_user_sync, GetUser);

    #[cfg(test)]
    sync_db_method!(post_node, post_node_sync, PostNode);

    #[cfg(test)]
    sync_db_method!(post_service, post_service_sync, PostService);

    #[cfg(test)]
    sync_db_method!(post_user, post_user_sync, PostUser);
}

pub trait Db {
    fn get_user(&self, email: String) -> DbFuture<'_, results::GetUser>;

    #[cfg(test)]
    fn post_node(&self, node: params::PostNode) -> DbFuture<'_, results::PostNode>;

    #[cfg(test)]
    fn post_service(&self, service: params::PostService) -> DbFuture<'_, results::PostService>;

    #[cfg(test)]
    fn post_user(&self, user: params::PostUser) -> DbFuture<'_, results::PostUser>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settings::test_settings;
    use crate::tokenserver::db::pool::{DbPool, TokenserverPool};

    type Result<T> = std::result::Result<T, ApiError>;

    #[tokio::test]
    async fn get_user() -> Result<()> {
        let pool = db_pool().await?;
        let db = pool.get()?;

        // Add a service
        let service_id = db.post_service(params::PostService::default()).await?;

        // Add a node
        let node_id = {
            let node = params::PostNode {
                service_id: service_id.id,
                ..Default::default()
            };
            db.post_node(node).await?
        };

        // Add a user
        let email1 = "test_user_1";
        let user_id = {
            let user = params::PostUser {
                service_id: service_id.id,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            };

            db.post_user(user).await?
        };

        // Add another user
        {
            let email2 = "test_user_2";
            let user = params::PostUser {
                service_id: service_id.id,
                node_id: node_id.id,
                email: email2.to_owned(),
                ..Default::default()
            };

            db.post_user(user).await?;
        }

        let user = db.get_user(email1.to_owned()).await?;

        // Ensure that the correct user has been returned
        assert_eq!(user.uid, user_id.uid);

        Ok(())
    }

    pub async fn db_pool() -> DbResult<TokenserverPool> {
        let _ = env_logger::try_init();

        let tokenserver_settings = test_settings().tokenserver;
        let use_test_transactions = true;

        TokenserverPool::new(&tokenserver_settings, use_test_transactions)
    }
}
