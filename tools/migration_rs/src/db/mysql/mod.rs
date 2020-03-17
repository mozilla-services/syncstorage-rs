use mysql_async;

use crate::settings::Settings;

#[derive(Clone)]
pub struct MysqlDb {
    pool: mysql_async::Pool,
}

impl MysqlDb {
    pub fn new(settings: &Settings) -> Result<Self> {
        pool = mysql_async::Pool::new(settings.dsns.mysql);
        Ok(Self { pool })
    }
}
