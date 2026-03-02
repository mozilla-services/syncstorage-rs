use diesel::{Identifiable, Insertable, Queryable};

use super::schema::{nodes, services, users};

/// Represents a service record in the database.
///
/// Services define the different types of Mozilla services (by default, "sync-1.5")
/// that can be managed by the Tokenserver.
#[derive(Queryable, Debug, Identifiable, Insertable)]
pub struct Service {
    /// Primary key for the service. Auto-increments with each new entry.
    pub id: i32,
    /// A short name or identifier for the service (e.g., `sync-1.5`). Must be unique.
    pub service: Option<String>,
    /// An optional pattern string for URI templating (e.g., `"{node}/1.5/{uid}"
    pub pattern: Option<String>,
}

/// Represents a user record in the database.
///
/// Users are associated with a service and a storage node, and contain
/// metadata about their sync state and authentication.
#[derive(Queryable, Debug, Identifiable, Insertable)]
#[diesel(primary_key(uid))]
pub struct User {
    /// Auto-incrementing numeric user id.
    pub uid: i64,
    /// The service the user is accessing; in practice this is always `sync-1.5`
    pub service: i32,
    /// Stable user identifier email address: `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// A monotonically increasing number provided by the FxA server, indicating the last time at which the user's login credentials were changed.
    pub generation: i64,
    /// The hash of the user's sync encryption key.
    pub client_state: String,
    ///  Timestamp at which this node-assignment record was created.
    pub created_at: i64,
    /// Timestamp at which this node-assignment record was replaced by a newer assignment, if any.
    pub replaced_at: Option<i64>,
    /// The storage node to which the user has been assigned.
    pub nodeid: i64,
    /// A monotonically increasing timestamp provided by the FxA server, indicating the last time at which the user's encryption keys were changed.
    pub keys_changed_at: Option<i64>,
}

/// Represents a storage node record in the database.
///
/// Nodes are the backend storage servers that users are assigned to.
/// The Tokenserver uses this information to load-balance users across nodes.
#[derive(Queryable, Debug, Identifiable, Insertable)]
pub struct Node {
    /// Primary key, auto-incrementing unique node identifier
    pub id: i64,
    /// Foreign key to the service this node provides
    pub service: i32,
    /// Unique node name under a given service
    pub node: String,
    /// Number of available slots on this node
    pub available: i32,
    /// Current number of active users/sessions assigned to node
    pub current_load: i32,
    /// Max allowed capacity, measured by number of users allowed to be assigned to node.
    pub capacity: i32,
    /// Flag indicating whether node is in service (0 = up, 1 = down)
    pub downed: i32,
    /// Backoff flag to temporarily avoid assigning new users
    pub backoff: i32,
}
