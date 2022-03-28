use async_trait::async_trait;
use diesel::{
    backend::Backend,
    helper_types::Limit,
    insertable::CanInsertInSingleQuery,
    mysql::Mysql,
    query_builder::{AstPass, InsertStatement, QueryFragment, QueryId},
    query_dsl::methods::{ExecuteDsl, LimitDsl, LoadQuery, LockingDsl},
    result::{Error, QueryResult},
    Expression, RunQueryDsl, Table,
};

use super::models::{Conn, MysqlDb};
use crate::{
    db::{self, error::DbErrorKind},
    error::{ApiErrorKind, ApiResult},
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

#[async_trait(?Send)]
pub trait RunAsyncQueryDsl: RunQueryDsl<Conn> + Send + 'static {
    async fn execute(self, db: MysqlDb) -> ApiResult<usize>
    where
        Self: ExecuteDsl<Conn>,
    {
        db::blocking_thread(move || RunQueryDsl::execute(self, &db.conn)).await
    }

    async fn load<U>(self, db: MysqlDb) -> ApiResult<Vec<U>>
    where
        U: Send + 'static,
        Self: LoadQuery<Conn, U>,
    {
        db::blocking_thread(move || RunQueryDsl::load(self, &db.conn)).await
    }

    async fn get_result<U>(self, db: MysqlDb) -> ApiResult<U>
    where
        U: Send + 'static,
        Self: LoadQuery<Conn, U>,
    {
        db::blocking_thread(move || RunQueryDsl::get_result(self, &db.conn)).await
    }

    async fn get_results<U>(self, db: MysqlDb) -> ApiResult<Vec<U>>
    where
        U: Send + 'static,
        Self: LoadQuery<Conn, U>,
    {
        db::blocking_thread(move || RunQueryDsl::get_results(self, &db.conn)).await
    }

    async fn first<U>(self, db: MysqlDb) -> ApiResult<U>
    where
        U: Send + 'static,
        Self: LimitDsl,
        Limit<Self>: LoadQuery<Conn, U> + Send + 'static,
    {
        db::blocking_thread(move || RunQueryDsl::first(self, &db.conn)).await
    }
}

impl<T: RunQueryDsl<Conn> + Send + 'static> RunAsyncQueryDsl for T {}

pub trait OptionalExtension<T> {
    fn optional(self) -> ApiResult<Option<T>>;
}

impl<T> OptionalExtension<T> for ApiResult<T> {
    fn optional(self) -> ApiResult<Option<T>> {
        match self {
            Ok(result) => Ok(Some(result)),
            Err(err) => match err.kind() {
                ApiErrorKind::Db(db_error)
                    if matches!(db_error.kind(), DbErrorKind::DieselQuery(Error::NotFound)) =>
                {
                    Ok(None)
                }
                _ => Err(err),
            },
        }
    }
}
