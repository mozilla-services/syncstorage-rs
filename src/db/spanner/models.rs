
use crate::db::error::DbError;

pub type Result<T> = std::result::Result<T, DbError>;
