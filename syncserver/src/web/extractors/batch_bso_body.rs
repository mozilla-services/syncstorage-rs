use serde::Deserialize;
use serde_json::Value;
use validator::Validate;

use syncstorage_db::params::PostCollectionBso;

use super::{validate_body_bso_id, validate_body_bso_sortindex, validate_body_bso_ttl};

#[derive(Debug, Deserialize, Validate)]
pub struct BatchBsoBody {
    #[validate(custom(function = "validate_body_bso_id"))]
    pub id: String,
    #[validate(custom(function = "validate_body_bso_sortindex"))]
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    #[validate(custom(function = "validate_body_bso_ttl"))]
    pub ttl: Option<u32>,
}

impl BatchBsoBody {
    /// Function to convert valid raw JSON BSO body to a BatchBsoBody
    pub fn from_raw_bso(val: Value) -> Result<BatchBsoBody, String> {
        let map = val.as_object().ok_or("invalid json")?;
        // Verify all the keys are valid. modified/collection are allowed but ignored
        let valid_keys = [
            "id",
            "sortindex",
            "payload",
            "ttl",
            "modified",
            "collection",
        ];
        for key_name in map.keys() {
            if !valid_keys.contains(&key_name.as_str()) {
                return Err(format!("unknown field {}", key_name));
            }
        }
        serde_json::from_value(val)
            .map_err(|_| "invalid json".to_string())
            .and_then(|v: BatchBsoBody| match v.validate() {
                Ok(()) => Ok(v),
                Err(e) => Err(format!("invalid bso: {}", e)),
            })
    }
}

impl From<BatchBsoBody> for PostCollectionBso {
    fn from(b: BatchBsoBody) -> PostCollectionBso {
        PostCollectionBso {
            id: b.id,
            sortindex: b.sortindex,
            payload: b.payload,
            ttl: b.ttl,
        }
    }
}
