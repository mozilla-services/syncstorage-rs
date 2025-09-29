use serde::Deserialize;
use std::{num::ParseIntError, str::FromStr};
use syncstorage_db::{params, SyncTimestamp};

#[derive(Debug, Default, Clone, Copy, Deserialize, Eq, PartialEq, Validate)]
#[serde(default)]
pub struct Offset {
    pub timestamp: Option<SyncTimestamp>,
    pub offset: u64,
}

impl From<Offset> for params::Offset {
    fn from(offset: Offset) -> Self {
        Self {
            timestamp: offset.timestamp,
            offset: offset.offset,
        }
    }
}

impl FromStr for Offset {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // issue559: Disable ':' support for now: simply parse as i64 as
        // previously (it was u64 previously but i64's close enough)
        let result = Offset {
            timestamp: None,
            offset: s.parse::<u64>()?,
        };
        /*
        let result = match s.chars().position(|c| c == ':') {
            None => Offset {
                timestamp: None,
                offset: s.parse::<u64>()?,
            },
            Some(_colon_position) => {
                let mut parts = s.split(':');
                let timestamp_string = parts.next().unwrap_or("0");
                let timestamp = SyncTimestamp::from_milliseconds(timestamp_string.parse::<u64>()?);
                let offset = parts.next().unwrap_or("0").parse::<u64>()?;
                Offset {
                    timestamp: Some(timestamp),
                    offset,
                }
            }
        };
        */
        Ok(result)
    }
}
