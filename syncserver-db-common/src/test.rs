use diesel::{
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection,
};

#[cfg(feature = "mysql")]
use diesel::mysql::MysqlConnection;
#[cfg(feature = "sqlite")]
use diesel::sqlite::SqliteConnection;

#[derive(Debug)]
pub struct TestTransactionCustomizer;

#[cfg(feature = "mysql")]
impl CustomizeConnection<MysqlConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> Result<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}

#[cfg(feature = "sqlite")]
impl CustomizeConnection<SqliteConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}
