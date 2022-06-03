use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, PooledConnection},
    sql_types::{Bigint, Float, Integer, Nullable, Text},
    OptionalExtension, RunQueryDsl,
};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use http::StatusCode;
use syncserver_common::Metrics;
use syncserver_db_common::{sync_db_method, util};
use syncstorage_mysql::error::DbError;

use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{params, results, DbFuture, DbResult};

/// The maximum possible generation number. Used as a tombstone to mark users that have been
/// "retired" from the db.
const MAX_GENERATION: i64 = i64::MAX;

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
    metrics: Metrics,
    service_id: Option<i32>,
    spanner_node_id: Option<i32>,
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

    pub fn new(
        conn: Conn,
        metrics: &Metrics,
        service_id: Option<i32>,
        spanner_node_id: Option<i32>,
    ) -> Self {
        let inner = DbInner {
            #[cfg(not(test))]
            conn,
            #[cfg(test)]
            conn: LoggingConnection::new(conn),
        };

        Self {
            inner: Arc::new(inner),
            metrics: metrics.clone(),
            service_id,
            spanner_node_id,
        }
    }

    fn get_node_id_sync(&self, params: params::GetNodeId) -> DbResult<results::GetNodeId> {
        const QUERY: &str = r#"
            SELECT id
              FROM nodes
             WHERE service = ?
               AND node = ?
        "#;

        if let Some(id) = self.spanner_node_id {
            Ok(results::GetNodeId { id: id as i64 })
        } else {
            let mut metrics = self.metrics.clone();
            metrics.start_timer("storage.get_node_id", None);

            diesel::sql_query(QUERY)
                .bind::<Integer, _>(params.service_id)
                .bind::<Text, _>(&params.node)
                .get_result(&self.inner.conn)
                .map_err(Into::into)
        }
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

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.replace_users", None);

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Integer, _>(&params.service_id)
            .bind::<Text, _>(&params.email)
            .bind::<Bigint, _>(params.replaced_at)
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
               AND COALESCE(keys_changed_at, 0) <= COALESCE(?, keys_changed_at, 0)
               AND replaced_at IS NULL
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.put_user", None);

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
        const QUERY: &str = r#"
            INSERT INTO users (service, email, generation, client_state, created_at, nodeid, keys_changed_at, replaced_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, NULL);
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.post_user", None);

        diesel::sql_query(QUERY)
            .bind::<Integer, _>(user.service_id)
            .bind::<Text, _>(&user.email)
            .bind::<Bigint, _>(user.generation)
            .bind::<Text, _>(&user.client_state)
            .bind::<Bigint, _>(user.created_at)
            .bind::<Bigint, _>(user.node_id)
            .bind::<Nullable<Bigint>, _>(user.keys_changed_at)
            .execute(&self.inner.conn)?;

        diesel::sql_query(Self::LAST_INSERT_ID_QUERY)
            .bind::<Text, _>(&user.email)
            .get_result::<results::PostUser>(&self.inner.conn)
            .map_err(Into::into)
    }

    fn check_sync(&self) -> DbResult<results::Check> {
        // has the database been up for more than 0 seconds?
        let result = diesel::sql_query("SHOW STATUS LIKE \"Uptime\"").execute(&self.inner.conn)?;
        Ok(result as u64 > 0)
    }

    /// Gets the least-loaded node that has available slots.
    fn get_best_node_sync(&self, params: params::GetBestNode) -> DbResult<results::GetBestNode> {
        const DEFAULT_CAPACITY_RELEASE_RATE: f32 = 0.1;
        const GET_BEST_NODE_QUERY: &str = r#"
              SELECT id, node
                FROM nodes
               WHERE service = ?
                 AND available > 0
                 AND capacity > current_load
                 AND downed = 0
                 AND backoff = 0
            ORDER BY LOG(current_load) / LOG(capacity) 
               LIMIT 1
        "#;
        const RELEASE_CAPACITY_QUERY: &str = r#"
            UPDATE nodes
               SET available = LEAST(capacity * ?, capacity - current_load)
             WHERE service = ?
               AND available <= 0
               AND capacity > current_load
               AND downed = 0
        "#;
        const SPANNER_QUERY: &str = r#"
              SELECT id, node
                FROM nodes
               WHERE id = ?
               LIMIT 1
        "#;

        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_best_node", None);

        if let Some(spanner_node_id) = self.spanner_node_id {
            diesel::sql_query(SPANNER_QUERY)
                .bind::<Integer, _>(spanner_node_id)
                .get_result::<results::GetBestNode>(&self.inner.conn)
                .map_err(|e| {
                    let mut db_error =
                        DbError::internal(&format!("unable to get Spanner node: {}", e));
                    db_error.status = StatusCode::SERVICE_UNAVAILABLE;
                    db_error
                })
        } else {
            // We may have to retry the query if we need to release more capacity. This loop allows
            // a maximum of five retries before bailing out.
            for _ in 0..5 {
                let maybe_result = diesel::sql_query(GET_BEST_NODE_QUERY)
                    .bind::<Integer, _>(params.service_id)
                    .get_result::<results::GetBestNode>(&self.inner.conn)
                    .optional()?;

                if let Some(result) = maybe_result {
                    return Ok(result);
                }

                // There were no available nodes. Try to release additional capacity from any nodes
                // that are not fully occupied.
                let affected_rows = diesel::sql_query(RELEASE_CAPACITY_QUERY)
                    .bind::<Float, _>(
                        params
                            .capacity_release_rate
                            .unwrap_or(DEFAULT_CAPACITY_RELEASE_RATE),
                    )
                    .bind::<Integer, _>(params.service_id)
                    .execute(&self.inner.conn)?;

                // If no nodes were affected by the last query, give up.
                if affected_rows == 0 {
                    break;
                }
            }

            let mut db_error = DbError::internal("unable to get a node");
            db_error.status = StatusCode::SERVICE_UNAVAILABLE;
            Err(db_error)
        }
    }

    fn add_user_to_node_sync(
        &self,
        params: params::AddUserToNode,
    ) -> DbResult<results::AddUserToNode> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.add_user_to_node", None);

        const QUERY: &str = r#"
            UPDATE nodes
               SET current_load = current_load + 1,
                   available = GREATEST(available - 1, 0)
             WHERE service = ?
               AND node = ?
        "#;
        const SPANNER_QUERY: &str = r#"
            UPDATE nodes
               SET current_load = current_load + 1
             WHERE service = ?
               AND node = ?
        "#;

        let query = if self.spanner_node_id.is_some() {
            SPANNER_QUERY
        } else {
            QUERY
        };

        diesel::sql_query(query)
            .bind::<Integer, _>(params.service_id)
            .bind::<Text, _>(&params.node)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    fn get_users_sync(&self, params: params::GetUsers) -> DbResult<results::GetUsers> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.get_users", None);

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

        diesel::sql_query(QUERY)
            .bind::<Text, _>(&params.email)
            .bind::<Integer, _>(params.service_id)
            .load::<results::GetRawUser>(&self.inner.conn)
            .map_err(Into::into)
    }

    /// Gets the user with the given email and service ID, or if one doesn't exist, allocates a new
    /// user.
    fn get_or_create_user_sync(
        &self,
        params: params::GetOrCreateUser,
    ) -> DbResult<results::GetOrCreateUser> {
        let mut raw_users = self.get_users_sync(params::GetUsers {
            service_id: params.service_id,
            email: params.email.clone(),
        })?;

        if raw_users.is_empty() {
            // There are no users in the database with the given email and service ID, so
            // allocate a new one.
            let allocate_user_result =
                self.allocate_user_sync(params.clone() as params::AllocateUser)?;

            Ok(results::GetOrCreateUser {
                uid: allocate_user_result.uid,
                email: params.email,
                client_state: params.client_state,
                generation: params.generation,
                node: allocate_user_result.node,
                keys_changed_at: params.keys_changed_at,
                created_at: allocate_user_result.created_at,
                replaced_at: None,
                first_seen_at: allocate_user_result.created_at,
                old_client_states: vec![],
            })
        } else {
            raw_users.sort_by_key(|raw_user| (raw_user.generation, raw_user.created_at));
            raw_users.reverse();

            // The user with the greatest `generation` and `created_at` is the current user
            let raw_user = raw_users[0].clone();

            // Collect any old client states that differ from the current client state
            let old_client_states = {
                raw_users[1..]
                    .iter()
                    .map(|user| user.client_state.clone())
                    .filter(|client_state| client_state != &raw_user.client_state)
                    .collect()
            };

            // Make sure every old row is marked as replaced. They might not be, due to races in row
            // creation.
            for old_user in &raw_users[1..] {
                if old_user.replaced_at.is_none() {
                    let params = params::ReplaceUser {
                        uid: old_user.uid,
                        service_id: params.service_id,
                        replaced_at: raw_user.created_at,
                    };

                    self.replace_user_sync(params)?;
                }
            }

            let first_seen_at = raw_users[raw_users.len() - 1].created_at;

            match (raw_user.replaced_at, raw_user.node) {
                // If the most up-to-date user is marked as replaced or does not have a node
                // assignment, allocate a new user. Note that, if the current user is marked
                // as replaced, we do not want to create a new user with the account metadata
                // in the parameters to this method. Rather, we want to create a duplicate of
                // the replaced user assigned to a new node. This distinction is important
                // because the account metadata in the parameters to this method may not match
                // that currently stored on the most up-to-date user and may be invalid.
                (Some(_), _) | (_, None) if raw_user.generation < MAX_GENERATION => {
                    let allocate_user_result = {
                        self.allocate_user_sync(params::AllocateUser {
                            service_id: params.service_id,
                            email: params.email.clone(),
                            generation: raw_user.generation,
                            client_state: raw_user.client_state.clone(),
                            keys_changed_at: raw_user.keys_changed_at,
                            capacity_release_rate: params.capacity_release_rate,
                        })?
                    };

                    Ok(results::GetOrCreateUser {
                        uid: allocate_user_result.uid,
                        email: params.email,
                        client_state: raw_user.client_state,
                        generation: raw_user.generation,
                        node: allocate_user_result.node,
                        keys_changed_at: raw_user.keys_changed_at,
                        created_at: allocate_user_result.created_at,
                        replaced_at: None,
                        first_seen_at,
                        old_client_states,
                    })
                }
                // The most up-to-date user has a node. Note that this user may be retired or
                // replaced.
                (_, Some(node)) => Ok(results::GetOrCreateUser {
                    uid: raw_user.uid,
                    email: params.email,
                    client_state: raw_user.client_state,
                    generation: raw_user.generation,
                    node,
                    keys_changed_at: raw_user.keys_changed_at,
                    created_at: raw_user.created_at,
                    replaced_at: None,
                    first_seen_at,
                    old_client_states,
                }),
                // The most up-to-date user doesn't have a node and is retired. This is an internal
                // service error for compatibility reasons (the legacy Tokenserver returned an
                // internal service error in this situation).
                (_, None) => Err(DbError::internal("Tokenserver user retired")),
            }
        }
    }

    /// Creates a new user and assigns them to a node.
    fn allocate_user_sync(&self, params: params::AllocateUser) -> DbResult<results::AllocateUser> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.allocate_user", None);

        // Get the least-loaded node
        let node = self.get_best_node_sync(params::GetBestNode {
            service_id: params.service_id,
            capacity_release_rate: params.capacity_release_rate,
        })?;

        // Decrement `available` and increment `current_load` on the node assigned to the user.
        self.add_user_to_node_sync(params::AddUserToNode {
            service_id: params.service_id,
            node: node.node.clone(),
        })?;

        let created_at = {
            let start = SystemTime::now();
            start.duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
        };
        let uid = self
            .post_user_sync(params::PostUser {
                service_id: params.service_id,
                email: params.email.clone(),
                generation: params.generation,
                client_state: params.client_state.clone(),
                created_at,
                node_id: node.id,
                keys_changed_at: params.keys_changed_at,
            })?
            .id;

        Ok(results::AllocateUser {
            uid,
            node: node.node,
            created_at,
        })
    }

    pub fn get_service_id_sync(
        &self,
        params: params::GetServiceId,
    ) -> DbResult<results::GetServiceId> {
        const QUERY: &str = r#"
            SELECT id
              FROM services
             WHERE service = ?
        "#;

        if let Some(id) = self.service_id {
            Ok(results::GetServiceId { id })
        } else {
            diesel::sql_query(QUERY)
                .bind::<Text, _>(params.service)
                .get_result::<results::GetServiceId>(&self.inner.conn)
                .map_err(Into::into)
        }
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
    fn set_user_replaced_at_sync(
        &self,
        params: params::SetUserReplacedAt,
    ) -> DbResult<results::SetUserReplacedAt> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE uid = ?
        "#;
        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.replaced_at)
            .bind::<Bigint, _>(&params.uid)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn get_user_sync(&self, params: params::GetUser) -> DbResult<results::GetUser> {
        const QUERY: &str = r#"
            SELECT service, email, generation, client_state, replaced_at, nodeid, keys_changed_at
              FROM users
             WHERE uid = ?
        "#;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.id)
            .get_result::<results::GetUser>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn post_node_sync(&self, params: params::PostNode) -> DbResult<results::PostNode> {
        const QUERY: &str = r#"
            INSERT INTO nodes (service, node, available, current_load, capacity, downed, backoff)
            VALUES (?, ?, ?, ?, ?, ?, ?)
        "#;
        diesel::sql_query(QUERY)
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
    fn get_node_sync(&self, params: params::GetNode) -> DbResult<results::GetNode> {
        const QUERY: &str = r#"
            SELECT *
              FROM nodes
             WHERE id = ?
        "#;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.id)
            .get_result::<results::GetNode>(&self.inner.conn)
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn unassign_node_sync(&self, params: params::UnassignNode) -> DbResult<results::UnassignNode> {
        const QUERY: &str = r#"
            UPDATE users
               SET replaced_at = ?
             WHERE nodeid = ?
        "#;

        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(current_time)
            .bind::<Bigint, _>(params.node_id)
            .execute(&self.inner.conn)
            .map(|_| ())
            .map_err(Into::into)
    }

    #[cfg(test)]
    fn remove_node_sync(&self, params: params::RemoveNode) -> DbResult<results::RemoveNode> {
        const QUERY: &str = "DELETE FROM nodes WHERE id = ?";

        diesel::sql_query(QUERY)
            .bind::<Bigint, _>(params.node_id)
            .execute(&self.inner.conn)
            .map(|_| ())
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
            .get_result::<results::LastInsertId>(&self.inner.conn)
            .map(|result| results::PostService {
                id: result.id as i32,
            })
            .map_err(Into::into)
    }
}

macro_rules! sync_db_method {
    ($name:ident, $sync_name:ident, $type:ident) => {
        sync_db_method!($name, $sync_name, $type, results::$type);
    };
    ($name:ident, $sync_name:ident, $type:ident, $result:ty) => {
        fn $name(&self, params: params::$type) -> DbFuture<'_, $result> {
            let db = self.clone();
            Box::pin(util::run_on_blocking_threadpool(
                move || db.$sync_name(params),
                DbError::internal,
            ))
        }
    };
}

impl Db for TokenserverDb {
    sync_db_method!(replace_user, replace_user_sync, ReplaceUser);
    sync_db_method!(replace_users, replace_users_sync, ReplaceUsers);
    sync_db_method!(post_user, post_user_sync, PostUser);

    sync_db_method!(put_user, put_user_sync, PutUser);
    sync_db_method!(get_node_id, get_node_id_sync, GetNodeId);
    sync_db_method!(get_best_node, get_best_node_sync, GetBestNode);
    sync_db_method!(add_user_to_node, add_user_to_node_sync, AddUserToNode);
    sync_db_method!(get_users, get_users_sync, GetUsers);
    sync_db_method!(get_or_create_user, get_or_create_user_sync, GetOrCreateUser);
    sync_db_method!(get_service_id, get_service_id_sync, GetServiceId);

    #[cfg(test)]
    sync_db_method!(get_user, get_user_sync, GetUser);

    fn check(&self) -> DbFuture<'_, results::Check> {
        let db = self.clone();
        Box::pin(util::run_on_blocking_threadpool(
            move || db.check_sync(),
            DbError::internal,
        ))
    }

    #[cfg(test)]
    sync_db_method!(
        set_user_created_at,
        set_user_created_at_sync,
        SetUserCreatedAt
    );

    #[cfg(test)]
    sync_db_method!(
        set_user_replaced_at,
        set_user_replaced_at_sync,
        SetUserReplacedAt
    );

    #[cfg(test)]
    sync_db_method!(post_node, post_node_sync, PostNode);

    #[cfg(test)]
    sync_db_method!(get_node, get_node_sync, GetNode);

    #[cfg(test)]
    sync_db_method!(unassign_node, unassign_node_sync, UnassignNode);

    #[cfg(test)]
    sync_db_method!(remove_node, remove_node_sync, RemoveNode);

    #[cfg(test)]
    sync_db_method!(post_service, post_service_sync, PostService);
}

pub trait Db {
    fn replace_user(&self, params: params::ReplaceUser) -> DbFuture<'_, results::ReplaceUser>;

    fn replace_users(&self, params: params::ReplaceUsers) -> DbFuture<'_, results::ReplaceUsers>;

    fn post_user(&self, params: params::PostUser) -> DbFuture<'_, results::PostUser>;

    fn put_user(&self, params: params::PutUser) -> DbFuture<'_, results::PutUser>;

    fn check(&self) -> DbFuture<'_, results::Check>;

    fn get_node_id(&self, params: params::GetNodeId) -> DbFuture<'_, results::GetNodeId>;

    fn get_best_node(&self, params: params::GetBestNode) -> DbFuture<'_, results::GetBestNode>;

    fn add_user_to_node(
        &self,
        params: params::AddUserToNode,
    ) -> DbFuture<'_, results::AddUserToNode>;

    fn get_users(&self, params: params::GetUsers) -> DbFuture<'_, results::GetUsers>;

    fn get_or_create_user(
        &self,
        params: params::GetOrCreateUser,
    ) -> DbFuture<'_, results::GetOrCreateUser>;

    fn get_service_id(&self, params: params::GetServiceId) -> DbFuture<'_, results::GetServiceId>;

    #[cfg(test)]
    fn set_user_created_at(
        &self,
        params: params::SetUserCreatedAt,
    ) -> DbFuture<'_, results::SetUserCreatedAt>;

    #[cfg(test)]
    fn set_user_replaced_at(
        &self,
        params: params::SetUserReplacedAt,
    ) -> DbFuture<'_, results::SetUserReplacedAt>;

    #[cfg(test)]
    fn get_user(&self, params: params::GetUser) -> DbFuture<'_, results::GetUser>;

    #[cfg(test)]
    fn post_node(&self, params: params::PostNode) -> DbFuture<'_, results::PostNode>;

    #[cfg(test)]
    fn get_node(&self, params: params::GetNode) -> DbFuture<'_, results::GetNode>;

    #[cfg(test)]
    fn unassign_node(&self, params: params::UnassignNode) -> DbFuture<'_, results::UnassignNode>;

    #[cfg(test)]
    fn remove_node(&self, params: params::RemoveNode) -> DbFuture<'_, results::RemoveNode>;

    #[cfg(test)]
    fn post_service(&self, params: params::PostService) -> DbFuture<'_, results::PostService>;
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::thread;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use syncserver_settings::Settings;

    use crate::pool::{DbPool, TokenserverPool};

    #[tokio::test]
    async fn test_update_generation() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        // Add a user
        let email = "test_user";
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id,
                email: email.to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.generation, 0);
        assert_eq!(user.client_state, "");

        // Changing generation should leave other properties unchanged.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id,
            generation: 42,
            keys_changed_at: user.keys_changed_at,
        })
        .await?;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.node_id, node_id);
        assert_eq!(user.generation, 42);
        assert_eq!(user.client_state, "");

        // It's not possible to move the generation number backwards.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id,
            generation: 17,
            keys_changed_at: user.keys_changed_at,
        })
        .await?;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.node_id, node_id);
        assert_eq!(user.generation, 42);
        assert_eq!(user.client_state, "");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_keys_changed_at() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node".to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        // Add a user
        let email = "test_user";
        let uid = db
            .post_user(params::PostUser {
                service_id,
                node_id,
                email: email.to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.keys_changed_at, None);
        assert_eq!(user.client_state, "");

        // Changing keys_changed_at should leave other properties unchanged.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id,
            generation: user.generation,
            keys_changed_at: Some(42),
        })
        .await?;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.node_id, node_id);
        assert_eq!(user.keys_changed_at, Some(42));
        assert_eq!(user.client_state, "");

        // It's not possible to move keys_changed_at backwards.
        db.put_user(params::PutUser {
            email: email.to_owned(),
            service_id,
            generation: user.generation,
            keys_changed_at: Some(17),
        })
        .await?;

        let user = db.get_user(params::GetUser { id: uid }).await?;

        assert_eq!(user.node_id, node_id);
        assert_eq!(user.keys_changed_at, Some(42));
        assert_eq!(user.client_state, "");

        Ok(())
    }

    #[tokio::test]
    async fn replace_users() -> DbResult<()> {
        const MILLISECONDS_IN_A_MINUTE: i64 = 60 * 1000;
        const MILLISECONDS_IN_AN_HOUR: i64 = MILLISECONDS_IN_A_MINUTE * 60;

        let pool = db_pool().await?;
        let db = pool.get().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let an_hour_ago = now - MILLISECONDS_IN_AN_HOUR;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id = db
            .post_node(params::PostNode {
                service_id,
                ..Default::default()
            })
            .await?;

        // Add a user to be updated
        let email1 = "test_user_1";
        let uid1 = {
            // Set created_at to be an hour ago
            let uid = db
                .post_user(params::PostUser {
                    service_id,
                    node_id: node_id.id,
                    email: email1.to_owned(),
                    ..Default::default()
                })
                .await?
                .id;

            db.set_user_created_at(params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            })
            .await?;

            uid
        };

        // Add a user that has already been replaced
        let uid2 = {
            // Set created_at to be an hour ago
            let uid = db
                .post_user(params::PostUser {
                    service_id,
                    node_id: node_id.id,
                    email: email1.to_owned(),
                    ..Default::default()
                })
                .await?
                .id;

            db.set_user_replaced_at(params::SetUserReplacedAt {
                replaced_at: an_hour_ago + MILLISECONDS_IN_A_MINUTE,
                uid,
            })
            .await?;

            db.set_user_created_at(params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            })
            .await?;

            uid
        };

        // Add a user created too recently
        {
            let uid = db
                .post_user(params::PostUser {
                    service_id,
                    node_id: node_id.id,
                    email: email1.to_owned(),
                    ..Default::default()
                })
                .await?
                .id;

            db.set_user_created_at(params::SetUserCreatedAt {
                created_at: now + MILLISECONDS_IN_AN_HOUR,
                uid,
            })
            .await?;
        }

        // Add a user with the wrong email address
        let email2 = "test_user_2";
        {
            // Set created_at to be an hour ago
            let uid = db
                .post_user(params::PostUser {
                    service_id,
                    node_id: node_id.id,
                    email: email2.to_owned(),
                    ..Default::default()
                })
                .await?
                .id;

            db.set_user_created_at(params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            })
            .await?;
        }

        // Add a user with the wrong service
        {
            let uid = db
                .post_user(params::PostUser {
                    service_id: service_id + 1,
                    node_id: node_id.id,
                    email: email1.to_owned(),
                    ..Default::default()
                })
                .await?
                .id;

            // Set created_at to be an hour ago
            db.set_user_created_at(params::SetUserCreatedAt {
                created_at: an_hour_ago,
                uid,
            })
            .await?;
        }

        // Perform the bulk update
        db.replace_users(params::ReplaceUsers {
            service_id,
            email: email1.to_owned(),
            replaced_at: now,
        })
        .await?;

        // Get all of the users
        let users = {
            let mut users1 = db
                .get_users(params::GetUsers {
                    email: email1.to_owned(),
                    service_id,
                })
                .await?;
            let mut users2 = db
                .get_users(params::GetUsers {
                    email: email2.to_owned(),
                    service_id,
                })
                .await?;
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
    async fn post_user() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let post_node_params = params::PostNode {
            service_id,
            ..Default::default()
        };
        let node_id = db.post_node(post_node_params.clone()).await?.id;

        // Add a user
        let email1 = "test_user_1";
        let post_user_params1 = params::PostUser {
            service_id,
            email: email1.to_owned(),
            generation: 1,
            client_state: "aaaa".to_owned(),
            created_at: 2,
            node_id,
            keys_changed_at: Some(3),
        };
        let uid1 = db.post_user(post_user_params1.clone()).await?.id;

        // Add another user
        let email2 = "test_user_2";
        let post_user_params2 = params::PostUser {
            service_id,
            node_id,
            email: email2.to_owned(),
            ..Default::default()
        };
        let uid2 = db.post_user(post_user_params2).await?.id;

        // Ensure that two separate users were created
        assert_ne!(uid1, uid2);

        // Get a user
        let user = db.get_user(params::GetUser { id: uid1 }).await?;

        // Ensure the user has the expected values
        let expected_get_user = results::GetUser {
            service_id,
            email: email1.to_owned(),
            generation: 1,
            client_state: "aaaa".to_owned(),
            replaced_at: None,
            node_id,
            keys_changed_at: Some(3),
        };

        assert_eq!(user, expected_get_user);

        Ok(())
    }

    #[tokio::test]
    async fn get_node_id() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id1 = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                ..Default::default()
            })
            .await?
            .id;

        // Add another node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node2".to_owned(),
            ..Default::default()
        })
        .await?;

        // Get the ID of the first node
        let id = db
            .get_node_id(params::GetNodeId {
                service_id,
                node: "https://node1".to_owned(),
            })
            .await?
            .id;

        // The ID should match that of the first node
        assert_eq!(node_id1, id);

        Ok(())
    }

    #[tokio::test]
    async fn test_node_allocation() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                current_load: 0,
                capacity: 100,
                available: 100,
                ..Default::default()
            })
            .await?
            .id;

        // Allocating a user assigns it to the node
        let user = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })?;
        assert_eq!(user.node, "https://node1");

        // Getting the user from the database does not affect node assignment
        let user = db.get_user(params::GetUser { id: user.uid }).await?;
        assert_eq!(user.node_id, node_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_allocation_to_least_loaded_node() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add two nodes
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?;

        db.post_node(params::PostNode {
            service_id,
            node: "https://node2".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?;

        // Allocate two users
        let user1 = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test1@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })?;

        let user2 = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test2@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })?;

        // Because users are always assigned to the least-loaded node, the users should have been
        // assigned to different nodes
        assert_ne!(user1.node, user2.node);

        Ok(())
    }

    #[tokio::test]
    async fn test_allocation_is_not_allowed_to_downed_nodes() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a downed node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            downed: 1,
            ..Default::default()
        })
        .await?;

        // User allocation fails because allocation is not allowed to downed nodes
        let result = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        });
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "Unexpected error: unable to get a node");

        Ok(())
    }

    #[tokio::test]
    async fn test_allocation_is_not_allowed_to_backoff_nodes() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a backoff node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            backoff: 1,
            ..Default::default()
        })
        .await?;

        // User allocation fails because allocation is not allowed to backoff nodes
        let result = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        });
        let error = result.unwrap_err();
        assert_eq!(error.to_string(), "Unexpected error: unable to get a node");

        Ok(())
    }

    #[tokio::test]
    async fn test_node_reassignment_when_records_are_replaced() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?;

        // Allocate a user
        let allocate_user_result = db.allocate_user_sync(params::AllocateUser {
            service_id,
            generation: 1234,
            email: "test@test.com".to_owned(),
            client_state: "aaaa".to_owned(),
            keys_changed_at: Some(1234),
            capacity_release_rate: None,
        })?;
        let user1 = db
            .get_user(params::GetUser {
                id: allocate_user_result.uid,
            })
            .await?;

        // Mark the user as replaced
        db.replace_user(params::ReplaceUser {
            uid: allocate_user_result.uid,
            service_id,
            replaced_at: 1234,
        })
        .await?;

        let user2 = db
            .get_or_create_user(params::GetOrCreateUser {
                email: "test@test.com".to_owned(),
                service_id,
                generation: 1235,
                client_state: "bbbb".to_owned(),
                keys_changed_at: Some(1235),
                capacity_release_rate: None,
            })
            .await?;

        // Calling get_or_create_user() results in the creation of a new user record, since the
        // previous record was marked as replaced
        assert_ne!(allocate_user_result.uid, user2.uid);

        // The account metadata should match that of the original user and *not* that in the
        // method parameters
        assert_eq!(user1.generation, user2.generation);
        assert_eq!(user1.keys_changed_at, user2.keys_changed_at);
        assert_eq!(user1.client_state, user2.client_state);

        Ok(())
    }

    #[tokio::test]
    async fn test_node_reassignment_not_done_for_retired_users() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 0,
            capacity: 100,
            available: 100,
            ..Default::default()
        })
        .await?;

        // Add a retired user
        let user1 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: MAX_GENERATION,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        let user2 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        // Calling get_or_create_user() does not update the user's node
        assert_eq!(user1.uid, user2.uid);
        assert_eq!(user2.generation, MAX_GENERATION);
        assert_eq!(user1.client_state, user2.client_state);

        Ok(())
    }

    #[tokio::test]
    async fn test_node_reassignment_and_removal() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add two nodes
        let node1_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                current_load: 0,
                capacity: 100,
                available: 100,
                ..Default::default()
            })
            .await?
            .id;

        let node2_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node2".to_owned(),
                current_load: 0,
                capacity: 100,
                available: 100,
                ..Default::default()
            })
            .await?
            .id;

        // Create four users. We should get two on each node.
        let user1 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test1@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        let user2 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test2@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        let user3 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test3@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        let user4 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        let node1_count = [&user1, &user2, &user3, &user4]
            .iter()
            .filter(|user| user.node == "https://node1")
            .count();
        assert_eq!(node1_count, 2);
        let node2_count = [&user1, &user2, &user3, &user4]
            .iter()
            .filter(|user| user.node == "https://node2")
            .count();
        assert_eq!(node2_count, 2);

        // Clear the assignments on the first node.
        db.unassign_node(params::UnassignNode { node_id: node1_id })
            .await?;

        // The users previously on the first node should balance across both nodes,
        // giving 1 on the first node and 3 on the second node.
        let mut node1_count = 0;
        let mut node2_count = 0;

        for user in [&user1, &user2, &user3, &user4] {
            let new_user = db
                .get_or_create_user(params::GetOrCreateUser {
                    service_id,
                    email: user.email.clone(),
                    generation: user.generation,
                    client_state: user.client_state.clone(),
                    keys_changed_at: user.keys_changed_at,
                    capacity_release_rate: None,
                })
                .await?;

            if new_user.node == "https://node1" {
                node1_count += 1;
            } else {
                assert_eq!(new_user.node, "https://node2");

                node2_count += 1;
            }
        }

        assert_eq!(node1_count, 1);
        assert_eq!(node2_count, 3);

        // Remove the second node. Everyone should end up on the first node.
        db.remove_node(params::RemoveNode { node_id: node2_id })
            .await?;

        // Every user should be on the first node now.
        for user in [&user1, &user2, &user3, &user4] {
            let new_user = db
                .get_or_create_user(params::GetOrCreateUser {
                    service_id,
                    email: user.email.clone(),
                    generation: user.generation,
                    client_state: user.client_state.clone(),
                    keys_changed_at: user.keys_changed_at,
                    capacity_release_rate: None,
                })
                .await?;

            assert_eq!(new_user.node, "https://node1");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_gradual_release_of_node_capacity() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add two nodes
        let node1_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                current_load: 4,
                capacity: 8,
                available: 1,
                ..Default::default()
            })
            .await?
            .id;

        let node2_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node2".to_owned(),
                current_load: 4,
                capacity: 6,
                available: 1,
                ..Default::default()
            })
            .await?
            .id;

        // Two user creations should succeed without releasing capacity on either of the nodes.
        // The users should be assigned to different nodes.
        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test1@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node1");
        let node = db.get_node(params::GetNode { id: node1_id }).await?;
        assert_eq!(node.current_load, 5);
        assert_eq!(node.capacity, 8);
        assert_eq!(node.available, 0);

        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test2@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node2");
        let node = db.get_node(params::GetNode { id: node2_id }).await?;
        assert_eq!(node.current_load, 5);
        assert_eq!(node.capacity, 6);
        assert_eq!(node.available, 0);

        // The next allocation attempt will release 10% more capacity, which is one more slot for
        // each node.
        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test3@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node1");
        let node = db.get_node(params::GetNode { id: node1_id }).await?;
        assert_eq!(node.current_load, 6);
        assert_eq!(node.capacity, 8);
        assert_eq!(node.available, 0);

        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node2");
        let node = db.get_node(params::GetNode { id: node2_id }).await?;
        assert_eq!(node.current_load, 6);
        assert_eq!(node.capacity, 6);
        assert_eq!(node.available, 0);

        // Now that node2 is full, further allocations will go to node1.
        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test5@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node1");
        let node = db.get_node(params::GetNode { id: node1_id }).await?;
        assert_eq!(node.current_load, 7);
        assert_eq!(node.capacity, 8);
        assert_eq!(node.available, 0);

        let user = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test6@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        assert_eq!(user.node, "https://node1");
        let node = db.get_node(params::GetNode { id: node1_id }).await?;
        assert_eq!(node.current_load, 8);
        assert_eq!(node.capacity, 8);
        assert_eq!(node.available, 0);

        // Once the capacity is reached, further user allocations will result in an error.
        let result = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test7@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await;

        assert_eq!(
            result.unwrap_err().to_string(),
            "Unexpected error: unable to get a node"
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_correct_created_at_used_during_node_reassignment() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        let node_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://node1".to_owned(),
                current_load: 4,
                capacity: 8,
                available: 1,
                ..Default::default()
            })
            .await?
            .id;

        // Create a user
        let user1 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        // Clear the user's node
        db.unassign_node(params::UnassignNode { node_id }).await?;

        // Sleep very briefly to ensure the timestamp created during node reassignment is greater
        // than the timestamp created during user creation
        thread::sleep(Duration::from_millis(5));

        // Get the user, prompting the user's reassignment to the same node
        let user2 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        // The user's timestamp should be updated since a new user record was created.
        assert!(user2.created_at > user1.created_at);

        Ok(())
    }

    #[tokio::test]
    async fn test_correct_created_at_used_during_user_retrieval() -> DbResult<()> {
        let pool = db_pool().await?;
        let db = pool.get().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node
        db.post_node(params::PostNode {
            service_id,
            node: "https://node1".to_owned(),
            current_load: 4,
            capacity: 8,
            available: 1,
            ..Default::default()
        })
        .await?;

        // Create a user
        let user1 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        // Sleep very briefly to ensure that any timestamp that might be created below is greater
        // than the timestamp created during user creation
        thread::sleep(Duration::from_millis(5));

        // Get the user
        let user2 = db
            .get_or_create_user(params::GetOrCreateUser {
                service_id,
                generation: 1234,
                email: "test4@test.com".to_owned(),
                client_state: "aaaa".to_owned(),
                keys_changed_at: Some(1234),
                capacity_release_rate: None,
            })
            .await?;

        // The user's timestamp should be equal to the one generated when the user was created
        assert_eq!(user1.created_at, user2.created_at);

        Ok(())
    }

    #[tokio::test]
    async fn test_get_spanner_node() -> DbResult<()> {
        let pool = db_pool().await?;
        let mut db = pool.get_tokenserver_db().await?;

        // Add a service
        let service_id = db
            .post_service(params::PostService {
                service: "sync-1.5".to_owned(),
                pattern: "{node}/1.5/{uid}".to_owned(),
            })
            .await?
            .id;

        // Add a node with capacity and available set to 0
        let spanner_node_id = db
            .post_node(params::PostNode {
                service_id,
                node: "https://spanner_node".to_owned(),
                current_load: 1000,
                capacity: 0,
                available: 0,
                ..Default::default()
            })
            .await?
            .id;

        // Add another node with available capacity
        db.post_node(params::PostNode {
            service_id,
            node: "https://another_node".to_owned(),
            current_load: 0,
            capacity: 1000,
            available: 1000,
            ..Default::default()
        })
        .await?;

        // Ensure the node with available capacity is selected if the Spanner node ID is not
        // cached
        assert_ne!(
            db.get_best_node(params::GetBestNode {
                service_id,
                capacity_release_rate: None,
            })
            .await?
            .id,
            spanner_node_id
        );

        // Ensure the Spanner node is selected if the Spanner node ID is cached
        db.spanner_node_id = Some(spanner_node_id as i32);

        assert_eq!(
            db.get_best_node(params::GetBestNode {
                service_id,
                capacity_release_rate: None,
            })
            .await?
            .id,
            spanner_node_id
        );

        Ok(())
    }

    async fn db_pool() -> DbResult<TokenserverPool> {
        let _ = env_logger::try_init();

        let mut settings = Settings::test_settings().tokenserver;
        settings.run_migrations = true;
        let use_test_transactions = true;

        TokenserverPool::new(&settings, &Metrics::noop(), use_test_transactions)
    }
}
