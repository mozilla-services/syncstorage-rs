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
