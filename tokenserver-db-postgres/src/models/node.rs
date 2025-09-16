//! Postgres DB Models for the Tokenserver Nodes table

use diesel::{Insertable, Queryable};

#[derive(Queryable, Debug, Identifiable)]
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
