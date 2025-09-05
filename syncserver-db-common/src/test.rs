use deadpool::managed::{HookError, HookResult};
use diesel::{mysql::MysqlConnection, r2d2::CustomizeConnection, Connection};
use diesel_async::{pooled_connection::PoolError, AsyncConnection};

#[derive(Debug)]
pub struct TestTransactionCustomizer;

impl CustomizeConnection<MysqlConnection, diesel::r2d2::Error> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> Result<(), diesel::r2d2::Error> {
        conn.begin_test_transaction()
            .map_err(diesel::r2d2::Error::QueryError)
    }
}

pub async fn test_transaction_hook<T>(conn: &mut T) -> HookResult<PoolError>
where
    T: AsyncConnection,
{
    conn.begin_test_transaction()
        .await
        .map_err(|e| HookError::Backend(PoolError::QueryError(e)))
}
