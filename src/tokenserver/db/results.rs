use diesel::{
    sql_types::{Bigint, Nullable, Text},
    QueryableByName,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use diesel::sql_types::Integer;

/// Represents a user record as it is stored in the database.
#[derive(Clone, Debug, Default, Deserialize, QueryableByName, Serialize)]
pub struct GetRawUser {
    #[sql_type = "Bigint"]
    pub uid: i64,
    #[sql_type = "Text"]
    pub client_state: String,
    #[sql_type = "Bigint"]
    pub generation: i64,
    #[sql_type = "Text"]
    pub node: String,
    #[sql_type = "Nullable<Bigint>"]
    pub keys_changed_at: Option<i64>,
    #[sql_type = "Bigint"]
    pub created_at: i64,
    #[sql_type = "Nullable<Bigint>"]
    pub replaced_at: Option<i64>,
}

#[cfg(test)]
pub type GetRawUsers = Vec<GetRawUser>;

/// Represents the relevant information from the most recently-created user record in the database
/// for a given email and service ID, along with any previously-seen client states seen for the
/// user.
#[derive(Debug, PartialEq)]
pub struct GetUser {
    pub uid: i64,
    pub client_state: String,
    pub generation: i64,
    pub node: String,
    pub keys_changed_at: Option<i64>,
    pub created_at: i64,
    pub old_client_states: Vec<String>,
}

impl Default for GetUser {
    fn default() -> Self {
        Self {
            uid: Default::default(),
            client_state: "616161".to_owned(),
            generation: 1234,
            node: Default::default(),
            keys_changed_at: Some(1234),
            created_at: Default::default(),
            old_client_states: Default::default(),
        }
    }
}

#[cfg(test)]
#[derive(Default, QueryableByName)]
pub struct PostNode {
    #[sql_type = "Bigint"]
    pub id: i64,
}

#[cfg(test)]
#[derive(Default, QueryableByName)]
pub struct PostService {
    #[sql_type = "Integer"]
    pub id: i32,
}

#[derive(Default, QueryableByName)]
pub struct PostUser {
    #[sql_type = "Bigint"]
    pub id: i64,
}

pub type ReplaceUsers = ();
pub type ReplaceUser = ();
pub type PutUser = ();

#[derive(Default, QueryableByName)]
pub struct GetNodeId {
    #[sql_type = "Bigint"]
    pub id: i64,
}

#[cfg(test)]
pub type SetUserCreatedAt = ();

#[cfg(test)]
pub type GetUsers = Vec<GetUser>;

pub type Check = bool;
