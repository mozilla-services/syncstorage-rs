use diesel::{
    QueryableByName,
    sql_types::{Bigint, Integer, Nullable, Text},
};
use serde::{Deserialize, Serialize};

/// Represents a user record as it is stored in the database.
#[derive(Clone, Debug, Default, Deserialize, QueryableByName, Serialize)]
pub struct GetRawUser {
    /// The user's unique identifier.
    #[diesel(sql_type = Bigint)]
    pub uid: i64,
    /// The user's client state hash.
    #[diesel(sql_type = Text)]
    pub client_state: String,
    /// Versioning or generation for user updates based on login credential change.
    #[diesel(sql_type = Bigint)]
    pub generation: i64,
    /// The node identifier this user is assigned to, if any.
    #[diesel(sql_type = Nullable<Text>)]
    pub node: Option<String>,
    /// Timestamp of last key change, based on FxA server timestamp.
    #[diesel(sql_type = Nullable<Bigint>)]
    pub keys_changed_at: Option<i64>,
    /// Timestamp when the user record was created.
    #[diesel(sql_type = Bigint)]
    pub created_at: i64,
    /// Timestamp when the user was replaced, if applicable.
    #[diesel(sql_type = Nullable<Bigint>)]
    pub replaced_at: Option<i64>,
}

/// Result type for [`crate::Db::get_users`], containing a list of user records.
pub type GetUsers = Vec<GetRawUser>;

/// Result from allocating a new user.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct AllocateUser {
    /// The newly created user's ID.
    pub uid: i64,
    /// The node identifier the user was assigned to.
    pub node: String,
    /// Timestamp when the user was created.
    pub created_at: i64,
}

/// Represents the relevant information from the most recently-created user record in the database
/// for a given email and service ID, along with any previously-seen client states seen for the
/// user.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct GetOrCreateUser {
    /// The user's unique identifier.
    pub uid: i64,
    /// User's email address; `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// State of the client; hash of sync key.
    pub client_state: String,
    ///  Versioning or generation for user updates based on login credential change.
    pub generation: i64,
    /// The node identifier the user is assigned to.
    pub node: String,
    /// Timestamp of last key change, based on FxA server timestamp.
    pub keys_changed_at: Option<i64>,
    /// Timestamp when this user record was created.
    pub created_at: i64,
    /// Timestamp when the user was replaced, if applicable.
    pub replaced_at: Option<i64>,
    /// Timestamp when the user was first seen (created_at of oldest record).
    pub first_seen_at: i64,
    /// List of previously-seen client state hashes for this user.
    pub old_client_states: Vec<String>,
}

/// Result from creating a new user.
#[derive(Default, QueryableByName)]
pub struct PostUser {
    /// The newly created user's ID.
    #[diesel(sql_type = Bigint)]
    pub uid: i64,
}

/// Type alais. Result from marking users as replaced. Returns unit type.
pub type ReplaceUsers = ();

/// Type alais. Result from marking a user as replaced. Returns unit type.
pub type ReplaceUser = ();

/// Type alais. Result from updating a user. Returns unit type.
pub type PutUser = ();

/// Result from retrieving a node ID.
#[derive(Default, QueryableByName)]
pub struct GetNodeId {
    /// The node's unique identifier.
    #[diesel(sql_type = Bigint)]
    pub id: i64,
}

/// Result from finding the best available node.
#[derive(Default, QueryableByName)]
pub struct GetBestNode {
    /// The node's unique identifier.
    #[diesel(sql_type = Bigint)]
    pub id: i64,
    /// The node identifier string.
    #[diesel(sql_type = Text)]
    pub node: String,
}

/// Type alais. Result from assigning a user to a node. Returns unit type.
pub type AddUserToNode = ();

/// Result from retrieving a service ID.
#[derive(Default, QueryableByName)]
pub struct GetServiceId {
    /// The service's unique identifier.
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

/// Result from retrieving a complete user record. Only available in debug builds.
#[cfg(debug_assertions)]
#[derive(Debug, Default, Eq, PartialEq, QueryableByName)]
pub struct GetUser {
    /// The service ID.
    #[diesel(sql_type = Integer)]
    #[diesel(column_name = service)]
    pub service_id: i32,
    /// User's email address; `<fxa_uid>@api.accounts.firefox.com`
    #[diesel(sql_type = Text)]
    pub email: String,
    ///  Versioning or generation for user updates based on login credential change.
    #[diesel(sql_type = Bigint)]
    pub generation: i64,
    /// The user's client state hash.
    #[diesel(sql_type = Text)]
    pub client_state: String,
    /// Timestamp when the user was replaced, if applicable.
    #[diesel(sql_type = Nullable<Bigint>)]
    pub replaced_at: Option<i64>,
    /// The node ID the user is assigned to.
    #[diesel(sql_type = Bigint)]
    #[diesel(column_name = nodeid)]
    pub node_id: i64,
    /// Timestamp of last key change, based on FxA server timestamp.
    #[diesel(sql_type = Nullable<Bigint>)]
    pub keys_changed_at: Option<i64>,
}

/// Result from creating a new node. Only available in debug builds.
#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct PostNode {
    /// The newly created node's ID.
    #[diesel(sql_type = Bigint)]
    pub id: i64,
}

/// Result from retrieving a complete node record. Only available in debug builds.
#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct GetNode {
    /// The node's unique identifier.
    #[diesel(sql_type = Bigint)]
    pub id: i64,
    /// The service ID this node belongs to.
    #[diesel(sql_type = Integer)]
    #[diesel(column_name = service)]
    pub service_id: i32,
    /// The node identifier string.
    #[diesel(sql_type = Text)]
    pub node: String,
    /// Number of available user slots.
    #[diesel(sql_type = Integer)]
    pub available: i32,
    /// Number of active users/sessions assigned to node.
    #[diesel(sql_type = Integer)]
    pub current_load: i32,
    /// Max allowed capacity, measured by number of users allowed to be assigned to node.
    #[diesel(sql_type = Integer)]
    pub capacity: i32,
    /// Flag indicating if the node is down.
    #[diesel(sql_type = Integer)]
    pub downed: i32,
    /// Backoff period in seconds.
    #[diesel(sql_type = Integer)]
    pub backoff: i32,
}

/// Result from creating a new service. Only available in debug builds.
#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct PostService {
    /// The newly created service's ID.
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

/// Result from updating a user's created_at timestamp. Returns unit type. Only available in debug builds.
#[cfg(debug_assertions)]
pub type SetUserCreatedAt = ();

/// Result from updating a user's replaced_at timestamp. Returns unit type. Only available in debug builds.
#[cfg(debug_assertions)]
pub type SetUserReplacedAt = ();

/// Result from checking database health. Returns `true` if healthy.
pub type Check = bool;

/// Result from unassigning a node. Returns unit type. Only available in debug builds.
#[cfg(debug_assertions)]
pub type UnassignNode = ();

/// Result from removing a node. Returns unit type. Only available in debug builds.
#[cfg(debug_assertions)]
pub type RemoveNode = ();
