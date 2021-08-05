use actix_web::web::block;
use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, PooledConnection},
    sql_types::{Bigint, Integer, Nullable, Text},
    RunQueryDsl,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use futures::future::LocalBoxFuture;
use futures::TryFutureExt;

use std::{
    result,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{params, results};
use crate::db::error::{DbError, DbErrorKind};
use crate::error::ApiError;
use crate::sync_db_method;

pub type DbFuture<'a, T> = LocalBoxFuture<'a, Result<T, ApiError>>;
pub type DbResult<T> = result::Result<T, DbError>;
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
    // Note that this only works because an instance of `TokenserverDb` has *exclusive access* to
    // a connection from the r2d2 pool for its lifetime. `LAST_INSERT_ID()` returns the ID of the
    // most recently-inserted record *for a given connection*. If connections were shared across
    // requests, using this function would introduce a race condition, as we could potentially
    // get IDs from records created during other requests.
    const LAST_INSERT_ID_QUERY: &'static str = "SELECT LAST_INSERT_ID() AS id";

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

    /// Get the most current user record for the given email and service ID. This function also
    /// marks any old user records as replaced, in case of data races that may have occurred
    /// during row creation.
    fn get_user_sync(&self, params: params::GetUser) -> DbResult<results::GetUser> {
        const QUERY: &str = r#"
                     SELECT uid, nodes.node, generation, keys_changed_at, client_state, created_at,
                            replaced_at
                       FROM users
            LEFT OUTER JOIN nodes ON users.nodeid = nodes.id
                      WHERE email = ?
                        AND users.service = ?
                   ORDER BY created_at DESC, uid DESC
                      LIMIT 20
        "#;
        let mut raw_users = diesel::sql_query(QUERY)
            .bind::<Text, _>(&params.email)
            .bind::<Integer, _>(params.service_id)
            .load::<results::GetRawUser>(&self.inner.conn)?;

        if raw_users.is_empty() {
            return Err(DbErrorKind::TokenserverUserNotFound.into());
        }

        raw_users.sort_by_key(|raw_user| (raw_user.generation, raw_user.created_at));

        // The user with the greatest `generation` and `created_at` is the current user
        let raw_user = raw_users[0].clone();

        // Collect any old client states that differ from the current client state
        let old_client_states = raw_users[1..]
            .iter()
            .map(|user| user.client_state.clone())
            .filter(|client_state| client_state != &raw_user.client_state)
            .collect();

        // Make sure every old row is marked as replaced. They might not be, due to races in row
        // creation.
        for old_user in &raw_users[1..] {
            if old_user.replaced_at.is_none() {
                let params = params::ReplaceUser {
                    uid: old_user.uid,
                    service_id: params.service_id,
                    replaced_at: old_user.created_at,
                };

                self.replace_user_sync(params)?;
            }
        }

        let user = results::GetUser {
            uid: raw_user.uid,
            client_state: raw_user.client_state,
            generation: raw_user.generation,
            node: raw_user.node,
            keys_changed_at: raw_user.keys_changed_at,
            created_at: raw_user.created_at,
            old_client_states,
        };

        Ok(user)
    }

    /// Mark users matching the given email and service ID as replaced.
    fn replace_users_sync(&self, params: params::ReplaceUsers) -> DbResult<results::ReplaceUsers> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE service = ?
               AND email = ?
               AND replaced_at IS NULL
               AND created_at < ?
        "#;
        let timestamp = Self::get_timestamp_in_milliseconds();

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(timestamp)
            .bind::<Integer, _>(&params.service_id)
            .bind::<Text, _>(&params.email)
            .bind::<Bigint, _>(timestamp)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Mark the user with the given uid and service ID as being replaced.
    fn replace_user_sync(&self, params: params::ReplaceUser) -> DbResult<results::ReplaceUser> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE service = ?
               AND uid = ?
        "#;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Integer, _>(params.service_id)
            .bind::<Bigint, _>(params.uid)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Update the user with the given email and service ID with the given `generation` and
    /// `keys_changed_at`.
    fn put_user_sync(&self, params: params::PutUser) -> DbResult<results::PutUser> {
        // The `where` clause on this statement is designed as an extra layer of
        // protection, to ensure that concurrent updates don't accidentally move
        // timestamp fields backwards in time. The handling of `keys_changed_at`
        // is additionally weird because we want to treat the default `NULL` value
        // as zero.
        const QUERY: &str = r#"
            UPDATE users
               SET generation = ?,
                   keys_changed_at = ?
             WHERE service = ?
               AND email = ?
               AND generation <= ?
               AND COALESCE(keys_changed_at, 0) <= COALESCE(?, 0)
               AND replaced_at IS NULL
        "#;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.generation)
            .bind::<Nullable<Bigint>, _>(params.keys_changed_at)
            .bind::<Integer, _>(&params.service_id)
            .bind::<Text, _>(&params.email)
            .bind::<Bigint, _>(params.generation)
            .bind::<Nullable<Bigint>, _>(params.keys_changed_at)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Create a new user.
    fn post_user_sync(&self, user: params::PostUser) -> DbResult<results::PostUser> {
        const INSERT_USER_QUERY: &str = r#"
            INSERT INTO users (service, email, generation, client_state, created_at, nodeid, keys_changed_at, replaced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NULL);
        "#;
        diesel::sql_query(INSERT_USER_QUERY)
            .bind::<Integer, _>(user.service_id)
            .bind::<Text, _>(&user.email)
            .bind::<Bigint, _>(user.generation)
            .bind::<Text, _>(&user.client_state)
            .bind::<Bigint, _>(Self::get_timestamp_in_milliseconds())
            .bind::<Bigint, _>(user.node_id)
            .bind::<Nullable<Bigint>, _>(user.keys_changed_at)
            .execute(&self.inner.conn)?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .bind::<Text, _>(&user.email)
            .get_result::<results::PostUser>(&self.inner.conn)
            .map_err(Into::into)
    }

    fn get_timestamp_in_milliseconds() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    #[cfg(test)]
    fn set_user_created_at_sync(
        &self,
        params: params::SetUserCreatedAt,
    ) -> DbResult<results::SetUserCreatedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET created_at = ?
             WHERE uid = ?
        "#;
        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.created_at)
            .bind::<Bigint, _>(&params.uid)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn get_users_sync(&self, email: String) -> DbResult<results::GetRawUsers> {
        const QUERY: &str = r#"
            SELECT users.uid, users.email, users.client_state, users.generation,
                   users.keys_changed_at, users.created_at, users.replaced_at, nodes.node
              FROM users
              JOIN nodes
                ON nodes.id = users.nodeid
             WHERE users.email = ?
        "#;
        diesel::sql_query(QUERY)
            .bind::<Text, _>(email)
            .load::<results::GetRawUser>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn post_node_sync(&self, params: params::PostNode) -> DbResult<results::PostNode> {
        const INSERT_NODE_QUERY: &str = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;
        diesel::sql_query(INSERT_NODE_QUERY)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(&params.node)
            .bind::<Integer, _>(params.available)
            .bind::<Integer, _>(params.current_load)
            .bind::<Integer, _>(params.capacity)
            .bind::<Integer, _>(params.downed)
            .bind::<Integer, _>(params.backoff)
            .execute(&self.inner.conn)?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::PostNode>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn post_service_sync(&self, params: params::PostService) -> DbResult<results::PostService> {
        const INSERT_SERVICE_QUERY: &str = r#"
            INSERT INTO services (service, pattern)
            VALUES (?, ?)
        "#;
        diesel::sql_query(INSERT_SERVICE_QUERY)
            .bind::<Text, _>(&params.service)
            .bind::<Text, _>(&params.pattern)
            .execute(&self.inner.conn)?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .get_result::<results::PostService>(&self.inner.conn)
            .map_err(Into::into)
    }
}

impl Db for TokenserverDb {
    sync_db_method!(get_user, get_user_sync, GetUser);
    sync_db_method!(replace_users, replace_users_sync, ReplaceUsers);
    sync_db_method!(post_user, post_user_sync, PostUser);
    sync_db_method!(put_user, put_user_sync, PutUser);

    #[cfg(test)]
    sync_db_method!(
        set_user_created_at,
        set_user_created_at_sync,
        SetUserCreatedAt
    );

    #[cfg(test)]
    sync_db_method!(get_users, get_users_sync, GetRawUsers);

    #[cfg(test)]
    sync_db_method!(post_node, post_node_sync, PostNode);

    #[cfg(test)]
    sync_db_method!(post_service, post_service_sync, PostService);
}

pub trait Db {
    fn get_user(&self, params: params::GetUser) -> DbFuture<'_, results::GetUser>;

    fn replace_users(&self, params: params::ReplaceUsers) -> DbFuture<'_, results::ReplaceUsers>;

    fn post_user(&self, params: params::PostUser) -> DbFuture<'_, results::PostUser>;

    fn put_user(&self, params: params::PutUser) -> DbFuture<'_, results::PutUser>;

    #[cfg(test)]
    fn set_user_created_at(
        &self,
        params: params::SetUserCreatedAt,
    ) -> DbFuture<'_, results::SetUserCreatedAt>;

    #[cfg(test)]
    fn get_users(&self, params: params::GetRawUsers) -> DbFuture<'_, results::GetRawUsers>;

    #[cfg(test)]
    fn post_node(&self, params: params::PostNode) -> DbFuture<'_, results::PostNode>;

    #[cfg(test)]
    fn post_service(&self, params: params::PostService) -> DbFuture<'_, results::PostService>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settings::test_settings;
    use crate::tokenserver::db;
    use crate::tokenserver::db::pool::{DbPool, TokenserverPool};

    type Result<T> = std::result::Result<T, ApiError>;

    #[tokio::test]
    async fn get_user() -> Result<()> {
        let pool = db_pool().await?;
        let db = pool.get()?;

        // Add a node
        let node_id = {
            let node = params::PostNode {
                service_id: db::SYNC_1_5_SERVICE_ID,
                ..Default::default()
            };
            db.post_node(node).await?
        };

        // Add a user
        let email1 = "test_user_1";
        let user_id = {
            let user = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
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
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id: node_id.id,
                email: email2.to_owned(),
                ..Default::default()
            };

            db.post_user(user).await?;
        }

        let user = {
            let params = params::GetUser {
                email: email1.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            };

            db.get_user(params).await?
        };

        // Ensure that the correct user has been returned
        assert_eq!(user.uid, user_id.id);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_generation() -> Result<()> {
        let pool = db_pool().await?;
        let db = pool.get()?;

        // Add a node
        let node = "node";
        let node_id = db
            .post_node(params::PostNode {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node: node.to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        // Add a user
        let email = "test_user";
        let uid = {
            let user = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id,
                email: email.to_owned(),
                ..Default::default()
            };

            db.post_user(user).await?.id
        };

        let user = db
            .get_user(params::GetUser {
                email: email.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            })
            .await?;

        assert_eq!(user.generation, 0);
        assert_eq!(user.client_state, "");

        // Changing generation should leave other properties unchanged.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            generation: 42,
            keys_changed_at: user.keys_changed_at,
        })
        .await?;

        let user = db
            .get_user(params::GetUser {
                email: email.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            })
            .await?;

        assert_eq!(user.uid, uid);
        assert_eq!(user.node, node);
        assert_eq!(user.generation, 42);
        assert_eq!(user.client_state, "");

        // It's not possible to move the generation number backwards.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            generation: 17,
            keys_changed_at: user.keys_changed_at,
        })
        .await?;

        assert_eq!(user.uid, uid);
        assert_eq!(user.node, node);
        assert_eq!(user.generation, 42);
        assert_eq!(user.client_state, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_keys_changed_at() -> Result<()> {
        let pool = db_pool().await?;
        let db = pool.get()?;

        // Add a node
        let node = "node";
        let node_id = db
            .post_node(params::PostNode {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node: node.to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        // Add a user
        let email = "test_user";
        let uid = {
            let user = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id,
                email: email.to_owned(),
                ..Default::default()
            };

            db.post_user(user).await?.id
        };

        let user = db
            .get_user(params::GetUser {
                email: email.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            })
            .await?;

        assert_eq!(user.keys_changed_at, None);
        assert_eq!(user.client_state, "");

        // Changing keys_changed_at should leave other properties unchanged.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            generation: user.generation,
            keys_changed_at: Some(42),
        })
        .await?;

        let user = db
            .get_user(params::GetUser {
                email: email.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            })
            .await?;

        assert_eq!(user.uid, uid);
        assert_eq!(user.node, node);
        assert_eq!(user.keys_changed_at, Some(42));
        assert_eq!(user.client_state, "");

        // It's not possible to move keys_changed_at backwards.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id: db::SYNC_1_5_SERVICE_ID,
            generation: user.generation,
            keys_changed_at: Some(17),
        })
        .await?;

        assert_eq!(user.uid, uid);
        assert_eq!(user.node, node);
        assert_eq!(user.keys_changed_at, Some(42));
        assert_eq!(user.client_state, "");

        Ok(())
    }

    #[tokio::test]
    async fn replace_users() -> Result<()> {
        const MILLISECONDS_IN_A_MINUTE: i64 = 60 * 1000;
        const MILLISECONDS_IN_AN_HOUR: i64 = MILLISECONDS_IN_A_MINUTE * 60;

        let pool = db_pool().await?;
        let db = pool.get()?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let an_hour_ago = now - MILLISECONDS_IN_AN_HOUR;

        // Add a node
        let node_id = {
            let params = params::PostNode {
                service_id: db::SYNC_1_5_SERVICE_ID,
                ..Default::default()
            };
            db.post_node(params).await?
        };

        // Add a user to be updated
        let email1 = "test_user_1";
        let uid1 = {
            let params = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            };

            // Set created_at to be an hour ago
            let uid = db.post_user(params).await?.id;
            let params = params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            };

            db.set_user_created_at(params).await?;

            uid
        };

        // Add a user that has already been replaced
        let uid2 = {
            let mut params = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            };

            params.replaced_at = Some(an_hour_ago + MILLISECONDS_IN_A_MINUTE);

            // Set created_at to be an hour ago
            let uid = db.post_user(params).await?.id;
            let params = params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            };

            db.set_user_created_at(params).await?;

            uid
        };

        // Add a user created too recently
        {
            let params = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            };

            let uid = db.post_user(params).await?.id;
            let created_at = now + MILLISECONDS_IN_AN_HOUR;
            let params = params::SetUserCreatedAt { created_at, uid };

            db.set_user_created_at(params).await?;
        }

        // Add a user with the wrong email address
        let email2 = "test_user_2";
        {
            let params = params::PostUser {
                service_id: db::SYNC_1_5_SERVICE_ID,
                node_id: node_id.id,
                email: email2.to_owned(),
                ..Default::default()
            };

            // Set created_at to be an hour ago
            let uid = db.post_user(params).await?.id;
            let params = params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            };

            db.set_user_created_at(params).await?;
        }

        // Add a user with the wrong service
        {
            let params = params::PostUser {
                service_id: db::SYNC_1_1_SERVICE_ID,
                node_id: node_id.id,
                email: email1.to_owned(),
                ..Default::default()
            };

            // Set created_at to be an hour ago
            let uid = db.post_user(params).await?.id;
            let params = params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            };

            db.set_user_created_at(params).await?;
        }

        // Perform the bulk update
        let bulk_update_params = params::ReplaceUsers {
            service_id: db::SYNC_1_5_SERVICE_ID,
            email: email1.to_owned(),
        };
        db.replace_users(bulk_update_params).await?;

        // Get all of the users
        let users = {
            let mut users1 = db.get_users(email1.to_owned()).await?;
            let mut users2 = db.get_users(email2.to_owned()).await?;
            users1.append(&mut users2);

            users1
        };

        let mut users_with_replaced_at_uids: Vec<i64> = users
            .iter()
            .filter(|user| user.replaced_at.is_some())
            .map(|user| user.uid)
            .collect();

        users_with_replaced_at_uids.sort_unstable();

        // The users with replaced_at timestamps should have the expected uids
        let mut expected_user_uids = vec![uid1, uid2];
        expected_user_uids.sort_unstable();
        assert_eq!(users_with_replaced_at_uids, expected_user_uids);

        Ok(())
    }

    #[tokio::test]
    async fn post_user() -> Result<()> {
        let pool = db_pool().await?;
        let db = pool.get()?;

        // Add a node
        let post_node_params = params::PostNode {
            service_id: db::SYNC_1_5_SERVICE_ID,
            ..Default::default()
        };
        let node_id = db.post_node(post_node_params.clone()).await?;

        // Add a user
        let email1 = "test_user_1";
        let post_user_params1 = params::PostUser {
            service_id: db::SYNC_1_5_SERVICE_ID,
            node_id: node_id.id,
            email: email1.to_owned(),
            ..Default::default()
        };
        let post_user_result1 = db.post_user(post_user_params1.clone()).await?;

        // Add another user
        let email2 = "test_user_2";
        let post_user_params2 = params::PostUser {
            service_id: db::SYNC_1_5_SERVICE_ID,
            node_id: node_id.id,
            email: email2.to_owned(),
            ..Default::default()
        };
        let post_user_result2 = db.post_user(post_user_params2).await?;

        // Ensure that two separate users were created
        assert_ne!(post_user_result1.id, post_user_result2.id);

        // Get a user
        let user = {
            let params = params::GetUser {
                email: email1.to_owned(),
                service_id: db::SYNC_1_5_SERVICE_ID,
            };

            db.get_user(params).await?
        };

        // Ensure the user has the expected values
        let mut expected_get_user = results::GetUser {
            uid: post_user_result1.id,
            client_state: post_user_params1.client_state.clone(),
            generation: post_user_params1.generation,
            keys_changed_at: post_user_params1.keys_changed_at,
            node: post_node_params.node,
            created_at: 0,
            old_client_states: vec![],
        };

        // Set created_at manually, since there's no way for us to know that timestamp without
        // querying for the user
        expected_get_user.created_at = user.created_at;

        assert_eq!(user, expected_get_user);

        Ok(())
    }

    pub async fn db_pool() -> DbResult<TokenserverPool> {
        let _ = env_logger::try_init();

        let tokenserver_settings = test_settings().tokenserver;
        let use_test_transactions = true;

        TokenserverPool::new(&tokenserver_settings, use_test_transactions)
    }
}
