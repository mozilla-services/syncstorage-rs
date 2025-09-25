use deadpool::managed::{HookError, HookResult};
use diesel_async::{pooled_connection::PoolError, AsyncConnection};

pub async fn test_transaction_hook<T>(conn: &mut T) -> HookResult<PoolError>
where
    T: AsyncConnection,
{
    conn.begin_test_transaction()
        .await
        .map_err(|e| HookError::Backend(PoolError::QueryError(e)))
}
