use diesel::{
    sql_types::{Bigint, Integer, Nullable, Text},
    QueryableByName,
};
use serde::{Deserialize, Serialize};

/// Represents a user record as it is stored in the database.
#[derive(Clone, Debug, Default, Deserialize, QueryableByName, Serialize)]
pub struct GetRawUser {
    #[diesel(sql_type = Bigint)]
    pub uid: i64,
    #[diesel(sql_type = Text)]
    pub client_state: String,
    #[diesel(sql_type = Bigint)]
    pub generation: i64,
    #[diesel(sql_type = Nullable<Text>)]
    pub node: Option<String>,
    #[diesel(sql_type = Nullable<Bigint>)]
    pub keys_changed_at: Option<i64>,
    #[diesel(sql_type = Bigint)]
    pub created_at: i64,
    #[diesel(sql_type = Nullable<Bigint>)]
    pub replaced_at: Option<i64>,
}

pub type GetUsers = Vec<GetRawUser>;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct AllocateUser {
    pub uid: i64,
    pub node: String,
    pub created_at: i64,
}

/// Represents the relevant information from the most recently-created user record in the database
/// for a given email and service ID, along with any previously-seen client states seen for the
/// user.
#[derive(Debug, Default, Eq, PartialEq)]
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
pub struct PostUser {
    #[diesel(sql_type = Bigint)]
    pub uid: i64,
}

pub type ReplaceUsers = ();
pub type ReplaceUser = ();
pub type PutUser = ();

#[derive(Default, QueryableByName)]
pub struct GetNodeId {
    #[diesel(sql_type = Bigint)]
    pub id: i64,
}

#[derive(Default, QueryableByName)]
pub struct GetBestNode {
    #[diesel(sql_type = Bigint)]
    pub id: i64,
    #[diesel(sql_type = Text)]
    pub node: String,
}

pub type AddUserToNode = ();

#[derive(Default, QueryableByName)]
pub struct GetServiceId {
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

#[cfg(debug_assertions)]
#[derive(Debug, Default, Eq, PartialEq, QueryableByName)]
pub struct GetUser {
    #[diesel(sql_type = Integer)]
    #[diesel(column_name = service)]
    pub service_id: i32,
    #[diesel(sql_type = Text)]
    pub email: String,
    #[diesel(sql_type = Bigint)]
    pub generation: i64,
    #[diesel(sql_type = Text)]
    pub client_state: String,
    #[diesel(sql_type = Nullable<Bigint>)]
    pub replaced_at: Option<i64>,
    #[diesel(sql_type = Bigint)]
    #[diesel(column_name = nodeid)]
    pub node_id: i64,
    #[diesel(sql_type = Nullable<Bigint>)]
    pub keys_changed_at: Option<i64>,
}

#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct PostNode {
    #[diesel(sql_type = Bigint)]
    pub id: i64,
}

#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct GetNode {
    #[diesel(sql_type = Bigint)]
    pub id: i64,
    #[diesel(sql_type = Integer)]
    #[diesel(column_name = service)]
    pub service_id: i32,
    #[diesel(sql_type = Text)]
    pub node: String,
    #[diesel(sql_type = Integer)]
    pub available: i32,
    #[diesel(sql_type = Integer)]
    pub current_load: i32,
    #[diesel(sql_type = Integer)]
    pub capacity: i32,
    #[diesel(sql_type = Integer)]
    pub downed: i32,
    #[diesel(sql_type = Integer)]
    pub backoff: i32,
}

#[cfg(debug_assertions)]
#[derive(Default, QueryableByName)]
pub struct PostService {
    #[diesel(sql_type = Integer)]
    pub id: i32,
}

#[cfg(debug_assertions)]
pub type SetUserCreatedAt = ();

#[cfg(debug_assertions)]
pub type SetUserReplacedAt = ();

pub type Check = bool;

#[cfg(debug_assertions)]
pub type UnassignNode = ();

#[cfg(debug_assertions)]
pub type RemoveNode = ();
