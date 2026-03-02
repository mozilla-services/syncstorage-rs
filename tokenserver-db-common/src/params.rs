//! Parameter types for database methods.

/// Parameters for creating a new node record.
#[derive(Clone, Default)]
pub struct PostNode {
    /// The service ID this node belongs to.
    pub service_id: i32,
    /// The node identifier string (e.g., URL).
    pub node: String,
    /// Number of free slots currently available on node.
    pub available: i32,
    /// Number of active users/sessions assigned to node.
    pub current_load: i32,
    /// Max allowed capacity, measured by number of users allowed to be assigned to node.
    pub capacity: i32,
    /// Flag indicating if the node is in service (0 = up, 1 = down).
    pub downed: i32,
    /// Backoff period in seconds before retrying this node.
    pub backoff: i32,
}

/// Parameters for retrieving a node by ID.
#[derive(Clone, Default)]
pub struct GetNode {
    /// The node ID to retrieve.
    pub id: i64,
}

/// Parameters for creating a new service.
#[derive(Default)]
pub struct PostService {
    /// A short name or identifier for the service (e.g., `sync-1.5`).
    pub service: String,
    /// An optional pattern string for URI templating (e.g., `"{node}/1.5/{uid}"`)
    pub pattern: String,
}

/// Parameters for retrieving users by email and service.
pub struct GetUsers {
    /// User's email address; `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// The service ID.
    pub service_id: i32,
}

/// Parameters for getting an existing user or creating a new one.
#[derive(Clone, Default)]
pub struct GetOrCreateUser {
    /// The service ID.
    pub service_id: i32,
    /// User's email address; `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// Versioning or generation for user updates based on login credential change.
    pub generation: i64,
    ///  State of the client; hash of sync key.
    pub client_state: String,
    /// Timestamp of last key change, based on FxA server timestamp.
    pub keys_changed_at: Option<i64>,
    /// Optional capacity release rate for node selection.
    pub capacity_release_rate: Option<f32>,
}

/// Parameters for allocating a new user. Same as [`GetOrCreateUser`].
pub type AllocateUser = GetOrCreateUser;

/// Parameters for creating a new user record.
#[derive(Clone, Default)]
pub struct PostUser {
    /// The service ID.
    pub service_id: i32,
    /// The user's email address.
    pub email: String,
    /// The user's generation number.
    pub generation: i64,
    /// The user's client state hash.
    pub client_state: String,
    /// Timestamp when the user was created.
    pub created_at: i64,
    /// The node ID to assign the user to.
    pub node_id: i64,
    /// Timestamp when keys were last changed.
    pub keys_changed_at: Option<i64>,
}

/// Parameters for updating a user record.
///
/// The `generation` and `keys_changed_at` are applied to the user record matching
/// the given `service_id` and `email`.
#[derive(Default)]
pub struct PutUser {
    /// The service ID.
    pub service_id: i32,
    ///  User's email address; `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// Versioning or generation for user updates based on login credential change.
    pub generation: i64,
    /// Timestamp of last key change, based on FxA server timestamp.
    pub keys_changed_at: Option<i64>,
}

/// Parameters for marking all users with a given email as replaced.
#[derive(Default)]
pub struct ReplaceUsers {
    /// User's email address; `<fxa_uid>@api.accounts.firefox.com`
    pub email: String,
    /// The service ID.
    pub service_id: i32,
    /// Timestamp when the users were replaced.
    pub replaced_at: i64,
}

/// Parameters for marking a specific user as replaced.
#[derive(Default)]
pub struct ReplaceUser {
    /// The user ID to replace.
    pub uid: i64,
    /// The service ID.
    pub service_id: i32,
    /// Timestamp when the user was replaced.
    pub replaced_at: i64,
}

/// Parameters for retrieving a node ID by service and node identifier.
#[derive(Debug, Default)]
pub struct GetNodeId {
    /// The service ID.
    pub service_id: i32,
    /// The node identifier string.
    pub node: String,
}

/// Parameters for finding the best available node for user allocation.
#[derive(Default)]
pub struct GetBestNode {
    /// The service ID.
    pub service_id: i32,
    /// Optional capacity release rate for node selection.
    pub capacity_release_rate: Option<f32>,
}

/// Parameters for assigning a user to a node.
#[derive(Default)]
pub struct AddUserToNode {
    /// The service ID.
    pub service_id: i32,
    /// The node identifier string.
    pub node: String,
}

/// Parameters for retrieving a service ID by service name.
pub struct GetServiceId {
    /// The service name.
    pub service: String,
}

/// Parameters for updating a user's created_at timestamp. Only available in debug builds.
#[cfg(debug_assertions)]
pub struct SetUserCreatedAt {
    /// The new created_at timestamp.
    pub created_at: i64,
    /// The user ID to update.
    pub uid: i64,
}

/// Parameters for updating a user's replaced_at timestamp. Only available in debug builds.
#[cfg(debug_assertions)]
pub struct SetUserReplacedAt {
    /// The new replaced_at timestamp.
    pub replaced_at: i64,
    /// The user ID to update.
    pub uid: i64,
}

/// Parameters for retrieving a user by ID. Only available in debug builds.
#[cfg(debug_assertions)]
#[derive(Default)]
pub struct GetUser {
    /// The user ID to retrieve.
    pub id: i64,
}

/// Parameters for unassigning a node from all users. Only available in debug builds.
#[cfg(debug_assertions)]
pub struct UnassignNode {
    /// The node ID to unassign.
    pub node_id: i64,
}

/// Parameters for removing a node. Only available in debug builds.
#[cfg(debug_assertions)]
pub struct RemoveNode {
    /// The node ID to remove.
    pub node_id: i64,
}

/// Spanner node ID parameter for testing. Only available in debug builds.
#[cfg(debug_assertions)]
pub type SpannerNodeId = Option<i32>;

/// Parameters for inserting an initial Sync 1.5 node.
pub struct Sync15Node {
    /// The node identifier string.
    pub node: String,
    /// The node's capacity.
    pub capacity: i32,
}

impl Sync15Node {
    /// The service name for Sync 1.5.
    pub const SERVICE_NAME: &str = "sync-1.5";
}
