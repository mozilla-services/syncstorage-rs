use diesel::{
    mysql::Mysql,
    query_builder::{AstPass, QueryFragment},
    query_dsl::methods::LockingDsl,
    result::QueryResult,
};

/// Emit MySQL <= 5.7's `LOCK IN SHARE MODE`
///
/// MySQL 8 supports `FOR SHARE` as an alias (which diesel natively supports)
pub trait LockInShareModeDsl {
    type Output;

    fn lock_in_share_mode(self) -> Self::Output;
}

impl<T> LockInShareModeDsl for T
where
    T: LockingDsl<LockInShareMode>,
{
    type Output = <T as LockingDsl<LockInShareMode>>::Output;

    fn lock_in_share_mode(self) -> Self::Output {
        self.with_lock(LockInShareMode)
    }
}

#[derive(Debug, Clone, Copy, QueryId)]
pub struct LockInShareMode;

impl QueryFragment<Mysql> for LockInShareMode {
    fn walk_ast(&self, mut out: AstPass<'_, Mysql>) -> QueryResult<()> {
        out.push_sql(" LOCK IN SHARE MODE");
        Ok(())
    }
}
