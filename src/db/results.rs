// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Result types for database methods.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub type GetCollections = HashMap<String, u64>;
pub type GetCollectionCounts = HashMap<String, u64>;
pub type GetCollectionUsage = HashMap<String, u32>;
pub type GetQuota = Vec<u32>;
pub type DeleteAll = ();
pub type GetCollection = Vec<GetCollectionItem>;
pub type DeleteCollection = Option<DeleteBsos>;
pub type DeleteBso = ();
pub type PutBso = u64;

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GetCollectionItem {
    Id(String),
    Bso(GetBso),
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GetBso {
    pub id: String,
    pub modified: u64,
    pub payload: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortindex: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct DeleteBsos {
    modified: u64,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct PostCollection {
    pub modified: u64,
    pub success: Vec<String>,
    pub failed: HashMap<String, String>,
}
