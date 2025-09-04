use diesel::{
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection,
};
use diesel_async::AsyncConnection;

#[derive(Debug)]
pub struct TestTransactionCustomizer;

impl CustomizeConnection<MysqlConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> Result<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}

pub async fn test_transaction_hook<T>(
    conn: &mut T,
) -> deadpool::managed::HookResult<diesel_async::pooled_connection::PoolError>
where
    T: AsyncConnection,
{
    conn.begin_test_transaction().await.map_err(|e| {
        deadpool::managed::HookError::Backend(
            diesel_async::pooled_connection::PoolError::QueryError(e),
        )
    })
}
