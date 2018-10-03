//! Parameter types for database methods.
use std::borrow::Cow;

use web::auth::HawkIdentifier;

macro_rules! data {
    ($name:ident {$($property:ident: $type:ty,)*}) => {
        #[derive(Debug)]
        pub struct $name {
            $(pub $property: $type,)*
        }
    }
}

macro_rules! uid_data {
    ($($name:ident,)+) => ($(
        data! {
            $name {
                user_id: HawkIdentifier,
            }
        }
    )+)
}

macro_rules! collection_data {
    ($($name:ident {$($property:ident: $type:ty,)*},)+) => ($(
        data! {
            $name {
                user_id: HawkIdentifier,
                collection_id: i32,
                $($property: $type,)*
            }
        }
    )+)
}

macro_rules! bso_data {
    ($($name:ident {$($property:ident: $type:ty,)*},)+) => ($(
        data! {
            $name {
                user_id: HawkIdentifier,
                collection_id: i32,
                id: String,
                $($property: $type,)*
            }
        }
    )+)
}

uid_data! {
    GetCollections,
    GetCollectionCounts,
    GetCollectionUsage,
    GetQuota,
    DeleteAll,
}

pub type GetCollectionId = str;

collection_data! {
    DeleteCollection {
        bso_ids: Vec<String>,
    },
    GetCollection {},
    PostCollection {
        bsos: Vec<PostCollectionBso>,
    },
}

bso_data! {
    DeleteBso {},
    GetBso {},
}

pub struct PutBso<'a> {
    pub user_id: HawkIdentifier,
    pub collection_id: i32,
    pub id: String,
    pub modified: i64,
    pub sortindex: Option<i32>,
    pub payload: Option<Cow<'a, str>>,
    pub ttl: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostCollectionBso {
    pub id: String,
    pub sortindex: Option<i32>,
    pub payload: Option<String>,
    pub ttl: Option<u32>,
}
