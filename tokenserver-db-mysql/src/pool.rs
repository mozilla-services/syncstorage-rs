use diesel::{mysql::MysqlConnection, Connection};
use diesel_logger::LoggingConnection;
use tokenserver_db_common::error::DbResult;

embed_migrations!();

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(database_url: &str) -> DbResult<()> {
    let conn = MysqlConnection::establish(database_url)?;
    #[cfg(debug_assertions)]
    // XXX: this doesn't show the DDL statements
    // https://github.com/shssoichiro/diesel-logger/issues/1
    embedded_migrations::run(&LoggingConnection::new(conn))?;
    #[cfg(not(debug_assertions))]
    embedded_migrations::run(&conn)?;
    Ok(())
}
