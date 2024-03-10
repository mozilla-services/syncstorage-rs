use diesel::{
    backend::Backend,
    insertable::CanInsertInSingleQuery,
    query_builder::{AstPass, InsertStatement, QueryFragment, QueryId},
    result::QueryResult,
    sqlite::Sqlite,
    Expression, RunQueryDsl, Table,
};

#[derive(Debug, Clone, Copy, QueryId)]
pub struct LockInShareMode;

impl QueryFragment<Sqlite> for LockInShareMode {
    fn walk_ast(&self, mut out: AstPass<'_, Sqlite>) -> QueryResult<()> {
        out.push_sql(" LOCK IN SHARE MODE");
        Ok(())
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
