//! Postgres DB Models for the Tokenserver Users table

use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Identifiable)]
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
