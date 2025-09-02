use std::{fmt::Debug, marker::PhantomData};

use diesel::{
    backend::Backend,
    insertable::CanInsertInSingleQuery,
    query_builder::{AstPass, InsertStatement, QueryFragment, QueryId},
    result::QueryResult,
    Expression, QuerySource, RunQueryDsl,
};

#[allow(dead_code)] // Not really dead, Rust can't see it.
#[derive(Debug, Clone)]
pub struct OnDuplicateKeyUpdate<T, U, Op, Ret, DB, X>(
    Box<InsertStatement<T, U, Op, Ret>>,
    X,
    PhantomData<DB>,
)
where
    DB: Backend,
    T: QuerySource,
    T::FromClause: QueryFragment<DB> + Clone + Debug,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
    X: Expression;

impl<T, U, Op, Ret, DB, X> QueryFragment<DB> for OnDuplicateKeyUpdate<T, U, Op, Ret, DB, X>
where
    DB: Backend,
    T: QuerySource,
    T::FromClause: QueryFragment<DB> + Clone + Debug,
    InsertStatement<T, U, Op, Ret>: QueryFragment<DB>,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
    X: Expression,
{
    fn walk_ast<'b>(&'b self, mut out: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(out.reborrow())?;
        out.push_sql(" ON DUPLICATE KEY UPDATE ");
        //self.1.walk_ast(out.reborrow())?;
        Ok(())
    }
}

impl<T, U, Op, Ret, DB, X> RunQueryDsl<DB> for OnDuplicateKeyUpdate<T, U, Op, Ret, DB, X>
where
    DB: Backend,
    T: QuerySource,
    T::FromClause: QueryFragment<DB> + Clone + Debug,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
    X: Expression,
{
}

impl<T, U, Op, Ret, DB, X> QueryId for OnDuplicateKeyUpdate<T, U, Op, Ret, DB, X>
where
    DB: Backend,
    T: QuerySource,
    T::FromClause: QueryFragment<DB> + Clone + Debug,
    U: QueryFragment<DB> + CanInsertInSingleQuery<DB>,
    Op: QueryFragment<DB>,
    Ret: QueryFragment<DB>,
    X: Expression,
{
    type QueryId = ();

    const HAS_STATIC_QUERY_ID: bool = false;
}
