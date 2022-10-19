pub mod error;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
