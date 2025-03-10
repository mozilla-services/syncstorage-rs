extern crate diesel;
extern crate diesel_migrations;
#[macro_use]
extern crate slog_scope;

use diesel::r2d2::{ConnectionManager, PooledConnection};
#[cfg(feature = "mysql")]
use diesel::MysqlConnection;
#[cfg(feature = "sqlite")]
use diesel::SqliteConnection;

pub mod mock;
mod models;
pub mod params;
mod pool;
pub mod results;

pub use models::{Db, TokenserverDb};
pub use pool::{DbPool, TokenserverPool};

#[cfg(feature = "mysql")]
type Conn = MysqlConnection;
#[cfg(feature = "sqlite")]
type Conn = SqliteConnection;
type PooledConn = PooledConnection<ConnectionManager<Conn>>;

#[cfg(all(feature = "mysql", feature = "sqlite"))]
compile_error!("only one of the \"mysql\" and \"sqlite\" features can be enabled at a time");

#[cfg(not(any(feature = "mysql", feature = "sqlite")))]
compile_error!("exactly one of the \"mysql\", \"spanner\" and \"sqlite\" features must be enabled");
