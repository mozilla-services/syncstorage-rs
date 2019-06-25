extern crate google_spanner1 as spanner1;
extern crate yup_oauth2 as oauth2;

use crate::db::error::DbError;

use diesel::r2d2::PooledConnection;

use oauth2::ServiceAccountAccess;
use spanner1::Spanner;

use std::sync::Arc;

use super::pool::CollectionCache;
use super::spanner::SpannerConnectionManager;

type Conn = PooledConnection<SpannerConnectionManager>;
pub type Result<T> = std::result::Result<T, DbError>;

#[derive(Clone, Debug)]
pub struct SpannerDb {}

impl SpannerDb {
    pub fn new(
        conn: Conn,
        thread_pool: Arc<::tokio_threadpool::ThreadPool>,
        coll_cache: Arc<CollectionCache>,
    ) -> Self {
        SpannerDb {}
    }
}
