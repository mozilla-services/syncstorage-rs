use diesel::{
    sql_types::{Bigint, Nullable, Text},
    QueryableByName,
};
use serde::{Deserialize, Serialize};

#[cfg(test)]
use diesel::sql_types::Integer;

#[derive(Clone, Debug, Default, Deserialize, QueryableByName, Serialize)]
pub struct GetUser {
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

#[cfg(test)]
#[derive(Default, QueryableByName)]
pub struct PostUser {
    #[sql_type = "Bigint"]
    pub uid: i64,
}
