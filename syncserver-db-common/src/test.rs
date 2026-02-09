use deadpool::managed::{HookError, HookResult};
use diesel_async::{AsyncConnection, pooled_connection::PoolError};

pub async fn test_transaction_hook<T>(conn: &mut T) -> HookResult<PoolError>
where
    T: AsyncConnection,
{
    conn.begin_test_transaction()
        .await
        .map_err(|e| HookError::Backend(PoolError::QueryError(e)))
}
