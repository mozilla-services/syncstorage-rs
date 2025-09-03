use diesel::{
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection,
};
use diesel_async::{AsyncConnection, AsyncMysqlConnection};

#[derive(Debug)]
pub struct TestTransactionCustomizer;

impl CustomizeConnection<MysqlConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> Result<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}

pub async fn test_transaction_hook(
    conn: &mut AsyncMysqlConnection,
) -> deadpool::managed::HookResult<diesel_async::pooled_connection::PoolError> {
    conn.begin_test_transaction().await.map_err(|e| {
        deadpool::managed::HookError::Backend(
            diesel_async::pooled_connection::PoolError::QueryError(e),
        )
    })
}
