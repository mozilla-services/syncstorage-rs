use diesel::{
    sql_types::{Bigint, Nullable, Text},
    QueryableByName,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, QueryableByName, Serialize)]
pub struct GetTokenserverUser {
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
