use crate::schema::{nodes, services, users};
use diesel::{Identifiable, Insertable, Queryable};

#[derive(Queryable, Debug, Identifiable, Insertable)]
pub struct Service {
    pub id: i32,
    pub service: Option<String>,
    pub pattern: Option<String>,
}

#[derive(Queryable, Debug, Identifiable, Insertable)]
#[diesel(primary_key(uid))]
pub struct User {
    pub uid: i64,
    pub service: i32,
    pub email: String,
    pub generation: i64,
    pub client_state: String,
    pub created_at: i64,
    pub replaced_at: Option<i64>,
    pub nodeid: i64,
    pub keys_changed_at: Option<i64>,
}

#[derive(Queryable, Debug, Identifiable, Insertable)]
pub struct Node {
    pub id: i64,
    pub service: i32,
    pub node: String,
    pub available: i32,
    pub current_load: i32,
    pub capacity: i32,
    pub downed: i32,
    pub backoff: i32,
}
