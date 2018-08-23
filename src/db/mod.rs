// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, you can obtain one at https://mozilla.org/MPL/2.0/.

//! Generic db abstration.

pub mod mock;
pub mod params;
pub mod results;

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use futures::future::Future;

type DbFuture<T> = Box<Future<Item = T, Error = DbError>>;

pub trait Db: Send {
    fn new() -> Box<Db>
    where
        Self: Sized;

    fn get_collections(&self, params: params::GetCollections) -> DbFuture<results::GetCollections>;

    fn get_collection_counts(
        &self,
        params: params::GetCollectionCounts,
    ) -> DbFuture<results::GetCollectionCounts>;

    fn get_collection_usage(
        &self,
        params: params::GetCollectionUsage,
    ) -> DbFuture<results::GetCollectionUsage>;

    fn get_quota(&self, params: params::GetQuota) -> DbFuture<results::GetQuota>;

    fn delete_all(&self, params: params::DeleteAll) -> DbFuture<results::DeleteAll>;

    fn delete_collection(
        &self,
        params: params::DeleteCollection,
    ) -> DbFuture<results::DeleteCollection>;

    fn get_collection(&self, params: params::GetCollection) -> DbFuture<results::GetCollection>;

    fn post_collection(&self, params: params::PostCollection) -> DbFuture<results::PostCollection>;

    fn delete_bso(&self, params: params::DeleteBso) -> DbFuture<results::DeleteBso>;

    fn get_bso(&self, params: params::GetBso) -> DbFuture<results::GetBso>;

    fn put_bso(&self, params: params::PutBso) -> DbFuture<results::PutBso>;
}

// HACK: temporary placeholder until we have proper error-handling
#[derive(Debug)]
pub struct DbError(String);

impl Display for DbError {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "db error: {}", &self.0)
    }
}

impl Error for DbError {}
