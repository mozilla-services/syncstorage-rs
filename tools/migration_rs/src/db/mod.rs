pub mod mysql;

use futures::future::LocalBoxFuture;

use crate::error::{ApiError, Result};
use crate::settings::Settings;

pub const DB_THREAD_POOL_SIZE: usize = 50;

pub struct Dbs {
    mysql: mysql::MysqlDb,
}

impl Dbs {
    pub fn connect(settings: &Settings) -> Result(Dbs) {
        Ok(Self {
            mysql: mysql::MysqlDb::new(&settings),
            //TODO: Get spanner db
        })
    }
}
