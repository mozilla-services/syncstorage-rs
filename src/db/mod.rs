//! Generic db abstration.

pub mod error;
#[macro_use]
pub mod mock;
pub mod mysql;
pub mod params;
pub mod results;
pub mod util;

use std::{collections::HashMap, ops::Deref};

use futures::future::Future;

pub use self::error::DbError;

lazy_static! {
    static ref STD_COLLS: HashMap<i32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(1, "clients");
        m.insert(2, "crypto");
        m.insert(3, "forms");
        m.insert(4, "history");
        m.insert(5, "keys");
        m.insert(6, "meta");
        m.insert(7, "bookmarks");
        m.insert(8, "prefs");
        m.insert(9, "tabs");
        m.insert(10, "passwords");
        m.insert(11, "addons");
        m.insert(12, "addresses");
        m.insert(13, "creditcards");
        m
    };
    static ref STD_COLLS_IDS: HashMap<&'static str, i32> =
        STD_COLLS.iter().map(|(k, v)| (*v, *k)).collect();
}

fn get_std_collection_name(id: i32) -> Option<&'static str> {
    STD_COLLS.get(&id).map(Deref::deref)
}

fn get_std_collection_id(name: &str) -> Option<i32> {
    STD_COLLS_IDS.get(&name).map(|i| *i)
}

type DbFuture<T> = Box<Future<Item = T, Error = DbError>>;

// XXX: add a DbPool trait

pub trait Db: Send {
    // XXX: add a generic fn transaction(&self, f)

    fn get_collection_id(
        &self,
        params: &params::GetCollectionId,
    ) -> DbFuture<results::GetCollectionId>;

    fn get_collections(&self, params: &params::GetCollections)
        -> DbFuture<results::GetCollections>;

    fn get_collection_counts(
        &self,
        params: &params::GetCollectionCounts,
    ) -> DbFuture<results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: &params::GetCollectionUsage,
    ) -> DbFuture<results::GetCollectionUsage>;

    fn get_quota(&self, params: &params::GetQuota) -> DbFuture<results::GetQuota>;

    fn delete_all(&self, params: &params::DeleteAll) -> DbFuture<results::DeleteAll>;

    fn delete_collection(
        &self,
        params: &params::DeleteCollection,
    ) -> DbFuture<results::DeleteCollection>;

    fn get_collection(&self, params: &params::GetCollection) -> DbFuture<results::GetCollection>;

    fn post_collection(&self, params: &params::PostCollection)
        -> DbFuture<results::PostCollection>;

    fn delete_bso(&self, params: &params::DeleteBso) -> DbFuture<results::DeleteBso>;

    fn get_bso(&self, params: &params::GetBso) -> DbFuture<results::GetBso>;

    fn put_bso(&self, params: &params::PutBso) -> DbFuture<results::PutBso>;
}

#[derive(Debug)]
pub enum Sorting {
    None,
    Newest,
    Oldest,
    Index,
}
