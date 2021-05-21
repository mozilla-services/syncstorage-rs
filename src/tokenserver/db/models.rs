use diesel::mysql::MysqlConnection;
use diesel::sql_types::Text;
use diesel::{Connection, RunQueryDsl};

use super::results::GetTokenserverUser;
use crate::db::error::{DbError, DbErrorKind};

// TODO: Connecting to the database like this is only temporary. In #1054, we
// will add a more mature database adapter for Tokenserver.
pub fn get_tokenserver_user_sync(
    email: &str,
    database_url: &str,
) -> Result<GetTokenserverUser, DbError> {
    let connection = MysqlConnection::establish(database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let mut user_records = diesel::sql_query(
        r#"
        SELECT users.uid, users.email, users.client_state, users.generation,
            users.keys_changed_at, users.created_at, nodes.node
        FROM users
        JOIN nodes ON nodes.id = users.nodeid
        WHERE users.email = ?
    "#,
    )
    .bind::<Text, _>(&email)
    .load::<GetTokenserverUser>(&connection)?;

    if user_records.is_empty() {
        return Err(DbErrorKind::TokenserverUserNotFound.into());
    }

    user_records.sort_by_key(|user_record| (user_record.generation, user_record.created_at));
    let user_record = user_records[0].clone();

    Ok(user_record)
}

// todo add params::UpdateTokenserverUser, collection_data! macro?
pub update_tokenserver_user_sync(id: i64, generation: i64,)
