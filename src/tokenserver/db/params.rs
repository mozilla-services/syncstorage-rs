//! Parameter types for database methods.

pub type GetUser = String;

#[derive(Default)]
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

#[derive(Default)]
pub struct PostUser {
    pub service_id: i32,
    pub email: String,
    pub generation: i64,
    pub client_state: String,
    pub created_at: i64,
    pub replaced_at: Option<i64>,
    pub node_id: i64,
    pub keys_changed_at: Option<i64>,
}
