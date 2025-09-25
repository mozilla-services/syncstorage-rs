use std::{collections::HashMap, sync::Arc};

use diesel::{
    // expression_methods::TextExpressionMethods, // See note below about `not_like` becoming swedish
    ExpressionMethods,
    QueryDsl,
};
use diesel_async::RunQueryDsl;
use syncserver_common::{BlockingThreadpool, Metrics};
use syncserver_settings::Settings as SyncserverSettings;
use syncstorage_db_common::DbPool;
use syncstorage_settings::Settings as SyncstorageSettings;
use url::Url;

use crate::{models::MysqlDb, pool::MysqlDbPool, schema::collections, DbResult};

async fn db(settings: &SyncstorageSettings) -> DbResult<MysqlDb> {
    let _ = env_logger::try_init();
    // inherit SYNC_SYNCSTORAGE__DATABASE_URL from the env

    let mut pool = MysqlDbPool::new(
        settings,
        &Metrics::noop(),
        Arc::new(BlockingThreadpool::new(512)),
    )?;
    pool.init().await?;
    pool.get_mysql_db().await
}

#[tokio::test]
async fn static_collection_id() -> DbResult<()> {
    let settings = SyncserverSettings::test_settings().syncstorage;
    if Url::parse(&settings.database_url).unwrap().scheme() != "mysql" {
        // Skip this test if we're not using mysql
        return Ok(());
    }
    let mut db = db(&settings).await?;

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
    // The integration tests can create collections that start
    // with `xxx%`. We should not include those in our counts for local
    // unit tests.
    let results: HashMap<i32, String> = collections::table
        .select((collections::id, collections::name))
        .filter(collections::name.ne(""))
        .filter(collections::name.ne("xxx_col2")) // from server::test
        .filter(collections::name.ne("col2")) // from older intergration tests
        .load(&mut db.conn)
        .await?
        .into_iter()
        .collect();
    assert_eq!(results.len(), cols.len(), "mismatched columns");
    for (id, name) in &cols {
        assert_eq!(results.get(id).unwrap(), name);
    }

    for (id, name) in &cols {
        let result = db.get_collection_id(name).await?;
        assert_eq!(result, *id);
    }

    let cid = db.get_or_create_collection_id("col1").await?;
    assert!(cid >= 100);
    Ok(())
}
