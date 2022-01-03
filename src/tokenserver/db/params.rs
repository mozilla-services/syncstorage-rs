//! Parameter types for database methods.

#[derive(Clone, Default)]
pub struct PostNode {
    pub service_id: i32,
    pub node: String,
    pub available: i32,
    pub current_load: i32,
    pub capacity: i32,
    pub downed: i32,
    pub backoff: i32,
}

#[derive(Clone, Default)]
pub struct GetNode {
    pub id: i64,
}

#[derive(Default)]
pub struct PostService {
    pub service: String,
    pub pattern: String,
}

pub struct GetUsers {
    pub service_id: i32,
    pub email: String,
}

#[derive(Clone, Default)]
pub struct GetOrCreateUser {
    pub service_id: i32,
    pub email: String,
    pub generation: i64,
    pub client_state: String,
    pub keys_changed_at: Option<i64>,
    pub capacity_release_rate: Option<f32>,
}

pub type AllocateUser = GetOrCreateUser;

#[derive(Clone, Default)]
pub struct PostUser {
    pub service_id: i32,
    pub email: String,
    pub generation: i64,
    pub client_state: String,
    pub created_at: i64,
    pub node_id: i64,
    pub keys_changed_at: Option<i64>,
}

/// The parameters used to update a user record. `generation` and `keys_changed_at` are applied to
/// the user record matching the given `service_id` and `email`.
#[derive(Default)]
pub struct PutUser {
    pub service_id: i32,
    pub email: String,
    pub generation: i64,
    pub keys_changed_at: Option<i64>,
}

#[derive(Default)]
pub struct ReplaceUsers {
    pub email: String,
    pub service_id: i32,
    pub replaced_at: i64,
}

#[derive(Default)]
pub struct ReplaceUser {
    pub uid: i64,
    pub service_id: i32,
    pub replaced_at: i64,
}

#[derive(Debug, Default)]
pub struct GetNodeId {
    pub service_id: i32,
    pub node: String,
}

#[derive(Default)]
pub struct GetBestNode {
    pub service_id: i32,
    pub capacity_release_rate: Option<f32>,
}

#[derive(Default)]
pub struct AddUserToNode {
    pub service_id: i32,
    pub node: String,
}

pub struct GetServiceId {
    pub service: String,
}

#[cfg(test)]
pub struct SetUserCreatedAt {
    pub uid: i64,
    pub created_at: i64,
}

#[cfg(test)]
pub struct SetUserReplacedAt {
    pub uid: i64,
    pub replaced_at: i64,
}

#[cfg(test)]
#[derive(Default)]
pub struct GetUser {
    pub id: i64,
}

#[cfg(test)]
pub struct UnassignNode {
    pub node_id: i64,
}

#[cfg(test)]
pub struct RemoveNode {
    pub node_id: i64,
}
