pub mod mysql;
pub mod spanner;
pub mod collections;

use crate::error::{ApiError, ApiResult};
use crate::settings::Settings;

pub const DB_THREAD_POOL_SIZE: usize = 50;

pub struct Dbs {
    mysql: mysql::MysqlDb,
    spanner: spanner::SpannerPool,
}

impl Dbs {
    pub fn connect(settings: &Settings) -> ApiResult<Dbs> {
        Ok(Self {
            mysql: mysql::MysqlDb::new(&settings)?,
            spanner: spanner::SpannerPool::new(&settings)?,
        })
    }
}
