use lazy_static::lazy_static;
use regex::Regex;

use crate::server::{BSO_ID_REGEX, COLLECTION_ID_REGEX};

pub const BATCH_MAX_IDS: usize = 100;

// BSO const restrictions
pub const BSO_MAX_TTL: u32 = 999_999_999;
pub const BSO_MAX_SORTINDEX_VALUE: i32 = 999_999_999;
pub const BSO_MIN_SORTINDEX_VALUE: i32 = -999_999_999;

pub const ACCEPTED_CONTENT_TYPES: [&str; 3] =
    ["application/json", "text/plain", "application/newlines"];

lazy_static! {
    pub static ref KNOWN_BAD_PAYLOAD_REGEX: Regex =
        Regex::new(r#"IV":\s*"AAAAAAAAAAAAAAAAAAAAAA=="#).unwrap();
    pub static ref VALID_ID_REGEX: Regex = Regex::new(&format!("^{}$", BSO_ID_REGEX)).unwrap();
    pub static ref VALID_COLLECTION_ID_REGEX: Regex =
        Regex::new(&format!("^{}$", COLLECTION_ID_REGEX)).unwrap();
    pub static ref TRUE_REGEX: Regex = Regex::new("^(?i)true$").unwrap();
}
