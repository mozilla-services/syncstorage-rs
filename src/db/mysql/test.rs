use std::{collections::HashMap, result::Result as StdResult};

use diesel::{
    // expression_methods::TextExpressionMethods, // See note below about `not_like` becoming swedish
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection,
    ExpressionMethods,
    QueryDsl,
    RunQueryDsl,
};
use url::Url;

use crate::db::{
    mysql::{models::MysqlDb, pool::MysqlDbPool, schema::collections},
    Db,
};
use crate::error::ApiResult;
use crate::server::metrics;
use crate::settings::{test_settings, Settings};

#[derive(Debug)]
pub struct TestTransactionCustomizer;

impl CustomizeConnection<MysqlConnection, PoolError> for TestTransactionCustomizer {
    fn on_acquire(&self, conn: &mut MysqlConnection) -> StdResult<(), PoolError> {
        conn.begin_test_transaction().map_err(PoolError::QueryError)
    }
}

pub fn db(settings: &Settings) -> ApiResult<MysqlDb> {
    let _ = env_logger::try_init();
    // inherit SYNC_DATABASE_URL from the env

    let pool = MysqlDbPool::new(settings, &metrics::Metrics::noop())?;
    pool.get_sync()
}

#[actix_rt::test]
async fn static_collection_id() {
    let settings = test_settings();
    if Url::parse(&settings.database_url).unwrap().scheme() != "mysql" {
        // Skip this test if we're not using mysql
        return;
    }
    let db = db(&settings).expect("failed to instantiate test database");

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
    // Note: not sure why but as of 11/02/20, `.not_like("xxx%")` is apparently
    // swedish-ci. Commenting that out for now.
    let results: HashMap<i32, String> = collections::table
        .select((collections::id, collections::name))
        .filter(collections::name.ne(""))
        //.filter(collections::name.not_like("xxx%")) // from most integration tests
        .filter(collections::name.ne("xxx_col2")) // from server::test
        .filter(collections::name.ne("col2")) // from older intergration tests
        .load(&db.inner.conn)
        .expect("failed to load collections from database")
        .into_iter()
        .collect();
    assert_eq!(results.len(), cols.len(), "mismatched columns");
    for (id, name) in &cols {
        assert_eq!(results.get(id).unwrap(), name);
    }

    for (id, name) in &cols {
        let result = db
            .get_collection_id(name.to_string())
            .await
            .expect("failed to get collection id");
        assert_eq!(result, *id);
    }

    let cid = db
        .get_or_create_collection_id("col1".to_string())
        .await
        .expect("failed to get or create collection id");
    assert!(cid >= 100);
}
