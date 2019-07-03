use std::{collections::HashMap, result::Result as StdResult};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection, QueryDsl, RunQueryDsl,
};
use env_logger;

use crate::db::mysql::{
    models::{MysqlDb, Result},
    pool::MysqlDbPool,
    schema::collections,
};
use crate::db::params;
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::extractors::HawkIdentifier;

#[derive(Debug)]
pub struct TestTransactionCustomizer;

impl CustomizeConnection<MysqlConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> StdResult<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}

pub fn db() -> Result<MysqlDb> {
    let _ = env_logger::try_init();
    // inherit SYNC_DATABASE_URL from the env
    let settings = Settings::with_env_and_config_file(&None).unwrap();
    let settings = Settings {
        debug: true,
        port: 8000,
        host: settings.host,
        database_url: settings.database_url,
        database_pool_max_size: Some(1),
        database_use_test_transactions: true,
        limits: ServerLimits::default(),
        master_secret: Secrets::default(),
    };

    let pool = MysqlDbPool::new(&settings)?;
    pool.get_sync()
}

pub fn postbso(
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i32>,
    ttl: Option<u32>,
) -> params::PostCollectionBso {
    params::PostCollectionBso {
        id: bid.to_owned(),
        payload: payload.map(&str::to_owned),
        sortindex,
        ttl,
    }
}

pub fn gbso(user_id: u32, coll: &str, bid: &str) -> params::GetBso {
    params::GetBso {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id: bid.to_owned(),
    }
}

pub fn hid(user_id: u32) -> HawkIdentifier {
    HawkIdentifier::new_legacy(u64::from(user_id))
}

#[test]
fn static_collection_id() -> Result<()> {
    let db = db()?;

    // ensure DB actually has predefined common collections
    let cols: Vec<(i32, _)> = vec![
        (1, "clients"),
        (2, "crypto"),
        (3, "forms"),
        (4, "history"),
        (5, "keys"),
        (6, "meta"),
        (7, "bookmarks"),
        (8, "prefs"),
        (9, "tabs"),
        (10, "passwords"),
        (11, "addons"),
        (12, "addresses"),
        (13, "creditcards"),
    ];

    let results: HashMap<i32, String> = collections::table
        .select((collections::id, collections::name))
        .load(&db.inner.conn)?
        .into_iter()
        .collect();
    assert_eq!(results.len(), cols.len(), "mismatched columns");
    for (id, name) in &cols {
        assert_eq!(results.get(id).unwrap(), name);
    }

    for (id, name) in &cols {
        let result = db.get_collection_id(name)?;
        assert_eq!(result, *id);
    }

    let cid = db.create_collection("col1")?;
    assert!(cid >= 100);
    Ok(())
}
