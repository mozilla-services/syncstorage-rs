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
}

impl NodeType {
    pub fn spanner() -> Self {
        Self::Spanner
    }
}

impl Default for NodeType {
    fn default() -> Self {
        Self::Spanner
    }
}
