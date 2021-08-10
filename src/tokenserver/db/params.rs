//! Parameter types for database methods.

#[derive(Default)]
pub struct GetUser {
    pub email: String,
    pub service_id: i32,
}

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

#[derive(Default)]
pub struct PostService {
    pub service: String,
    pub pattern: String,
}

#[derive(Clone, Default)]
pub struct PostUser {
    pub service_id: i32,
    pub email: String,
    pub generation: i64,
    pub client_state: String,
    pub replaced_at: Option<i64>,
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
}

#[derive(Default)]
pub struct ReplaceUser {
    pub uid: i64,
    pub service_id: i32,
    pub replaced_at: i64,
}

#[cfg(test)]
pub type GetRawUsers = String;

#[cfg(test)]
pub struct SetUserCreatedAt {
    pub uid: i64,
    pub created_at: i64,
}
