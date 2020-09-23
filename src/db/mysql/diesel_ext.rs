use diesel::{
    backend::Backend,
    insertable::CanInsertInSingleQuery,
    mysql::Mysql,
    query_builder::{AstPass, InsertStatement, QueryFragment, QueryId},
    query_dsl::methods::LockingDsl,
    result::QueryResult,
    Expression, RunQueryDsl, Table,
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

/// Emit 'ON DUPLICATE KEY UPDATE'
pub trait IntoDuplicateValueClause {
    type ValueClause;

    fn into_value_clause(self) -> Self::ValueClause;
}

pub trait OnDuplicateKeyUpdateDsl<T, U, Op, Ret> {
    fn on_duplicate_key_update<X>(self, expression: X) -> OnDuplicateKeyUpdate<T, U, Op, Ret, X>
    where
        X: Expression;
}

impl<T, U, Op, Ret> OnDuplicateKeyUpdateDsl<T, U, Op, Ret> for InsertStatement<T, U, Op, Ret> {
    fn on_duplicate_key_update<X>(self, expression: X) -> OnDuplicateKeyUpdate<T, U, Op, Ret, X>
    where
        X: Expression,
    {
        OnDuplicateKeyUpdate(Box::new(self), expression)
    }
}

#[derive(Debug, Clone)]
pub struct OnDuplicateKeyUpdate<T, U, Op, Ret, X>(Box<InsertStatement<T, U, Op, Ret>>, X);

impl<T, U, Op, Ret, DB, X> QueryFragment<DB> for OnDuplicateKeyUpdate<T, U, Op, Ret, X>
where
    DB: Backend,
    T: Table,
    T::FromClause: QueryFragment<DB>,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
    X: Expression,
{
    fn walk_ast(&self, mut out: AstPass<'_, DB>) -> QueryResult<()> {
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(" ON DUPLICATE KEY UPDATE ");
        //self.1.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, U, Op, Ret, DB, X> RunQueryDsl<DB> for OnDuplicateKeyUpdate<T, U, Op, Ret, X> {}

impl<T, U, Op, Ret, X> QueryId for OnDuplicateKeyUpdate<T, U, Op, Ret, X> {
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
