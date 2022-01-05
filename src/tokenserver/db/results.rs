use diesel::{
    sql_types::{Bigint, Integer, Nullable, Text},
    QueryableByName,
};
use serde::{Deserialize, Serialize};

/// Represents a user record as it is stored in the database.
#[derive(Clone, Debug, Default, Deserialize, QueryableByName, Serialize)]
pub struct GetRawUser {
    #[sql_type = "Bigint"]
    pub uid: i64,
    #[sql_type = "Text"]
    pub client_state: String,
    #[sql_type = "Bigint"]
    pub generation: i64,
    #[sql_type = "Nullable<Text>"]
    pub node: Option<String>,
    #[sql_type = "Nullable<Bigint>"]
    pub keys_changed_at: Option<i64>,
    #[sql_type = "Bigint"]
    pub created_at: i64,
    #[sql_type = "Nullable<Bigint>"]
    pub replaced_at: Option<i64>,
}

pub type GetUsers = Vec<GetRawUser>;

#[derive(Debug, Default, PartialEq)]
pub struct AllocateUser {
    pub uid: i64,
    pub node: String,
    pub created_at: i64,
}

/// Represents the relevant information from the most recently-created user record in the database
/// for a given email and service ID, along with any previously-seen client states seen for the
/// user.
#[derive(Debug, Default, PartialEq)]
pub struct GetOrCreateUser {
    pub uid: i64,
    pub email: String,
    pub client_state: String,
    pub generation: i64,
    pub node: String,
    pub keys_changed_at: Option<i64>,
    pub created_at: i64,
    pub replaced_at: Option<i64>,
    pub first_seen_at: i64,
    pub old_client_states: Vec<String>,
}

#[derive(Default, QueryableByName)]
pub struct LastInsertId {
    #[sql_type = "Bigint"]
    pub id: i64,
}

pub type PostUser = LastInsertId;
pub type ReplaceUsers = ();
pub type ReplaceUser = ();
pub type PutUser = ();

#[derive(Default, QueryableByName)]
pub struct GetNodeId {
    #[sql_type = "Bigint"]
    pub id: i64,
}

#[derive(Default, QueryableByName)]
pub struct GetBestNode {
    #[sql_type = "Bigint"]
    pub id: i64,
    #[sql_type = "Text"]
    pub node: String,
}

pub type AddUserToNode = ();

#[derive(Default, QueryableByName)]
pub struct GetServiceId {
    #[sql_type = "Integer"]
    pub id: i32,
}

#[cfg(test)]
#[derive(Debug, Default, PartialEq, QueryableByName)]
pub struct GetUser {
    #[sql_type = "Integer"]
    #[column_name = "service"]
    pub service_id: i32,
    #[sql_type = "Text"]
    pub email: String,
    #[sql_type = "Bigint"]
    pub generation: i64,
    #[sql_type = "Text"]
    pub client_state: String,
    #[sql_type = "Nullable<Bigint>"]
    pub replaced_at: Option<i64>,
    #[sql_type = "Bigint"]
    #[column_name = "nodeid"]
    pub node_id: i64,
    #[sql_type = "Nullable<Bigint>"]
    pub keys_changed_at: Option<i64>,
}

#[cfg(test)]
pub type PostNode = LastInsertId;

#[cfg(test)]
#[derive(Default, QueryableByName)]
pub struct GetNode {
    #[sql_type = "Bigint"]
    pub id: i64,
    #[sql_type = "Integer"]
    #[column_name = "service"]
    pub service_id: i32,
    #[sql_type = "Text"]
    pub node: String,
    #[sql_type = "Integer"]
    pub available: i32,
    #[sql_type = "Integer"]
    pub current_load: i32,
    #[sql_type = "Integer"]
    pub capacity: i32,
    #[sql_type = "Integer"]
    pub downed: i32,
    #[sql_type = "Integer"]
    pub backoff: i32,
}

#[cfg(test)]
#[derive(Default, QueryableByName)]
pub struct PostService {
    #[sql_type = "Integer"]
    pub id: i32,
}

#[cfg(test)]
pub type SetUserCreatedAt = ();

#[cfg(test)]
pub type SetUserReplacedAt = ();

pub type Check = bool;

#[cfg(test)]
pub type UnassignNode = ();

#[cfg(test)]
pub type RemoveNode = ();
