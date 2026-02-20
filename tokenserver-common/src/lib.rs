mod error;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub use error::{ErrorLocation, TokenserverError};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ToSchema)]
pub enum NodeType {
    #[serde(rename = "mysql")]
    MySql,
    #[serde(rename = "spanner")]
    Spanner,
    #[serde(rename = "postgres")]
    Postgres,
}

impl NodeType {
    pub fn spanner() -> Self {
        Self::Spanner
    }

    pub fn postgres() -> Self {
        Self::Postgres
    }
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Spanner
    }
}
