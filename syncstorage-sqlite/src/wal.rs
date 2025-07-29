use diesel::{
    connection::SimpleConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    sqlite::SqliteConnection,
};

// For e2e tests only
#[derive(Debug)]
pub struct WALTransactionCustomizer;

impl CustomizeConnection<SqliteConnection, PoolError> for WALTransactionCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), PoolError> {
        (|| {
            conn.batch_execute("PRAGMA journal_mode = WAL;")?;
            conn.batch_execute("PRAGMA synchronous = NORMAL;")?;
            conn.batch_execute("PRAGMA foreign_keys = ON;")?;
            conn.batch_execute("PRAGMA busy_timeout = 10000;")?;
            Ok(())
        })()
        .map_err(PoolError::QueryError)
    }
}
