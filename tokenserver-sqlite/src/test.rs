use syncserver_common::Metrics;
use tokenserver_db_common::{DbPool, DbResult, params};
use tokenserver_settings::Settings;

use crate::TokenserverSqlitePool;

fn unique_db_path(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir()
        .join(format!("{name}_{nanos}.sqlite3"))
        .to_string_lossy()
        .into_owned()
}

fn test_settings(database_url: String) -> Settings {
    Settings {
        database_url,
        database_pool_max_size: 1,
        run_migrations: true,
        enabled: true,
        ..Settings::default()
    }
}

/// End-to-end smoke test against a real (temp file) SQLite db: migrations,
/// service/node bootstrap, and a full user lifecycle.
#[tokio::test]
async fn lifecycle_smoke_test() -> DbResult<()> {
    let _ = env_logger::try_init();
    let database_url = unique_db_path("tokenserver_sqlite_test");
    let settings = test_settings(database_url);

    let mut pool = TokenserverSqlitePool::new(&settings, &Metrics::noop(), false)?;
    pool.init().await?;

    let mut db = pool.get().await?;

    // The "sync-1.5" service row is seeded by the init migration.
    let service_id = db
        .get_service_id(params::GetServiceId {
            service: "sync-1.5".to_owned(),
        })
        .await?
        .id;

    let added = db
        .insert_sync15_node(params::Sync15Node {
            node: "https://node1".to_owned(),
            capacity: 100,
        })
        .await?;
    assert!(added, "expected the node to be newly inserted");

    let node_id = db
        .get_node_id(params::GetNodeId {
            service_id,
            node: "https://node1".to_owned(),
        })
        .await?
        .id;

    let best_node = db
        .get_best_node(params::GetBestNode {
            service_id,
            capacity_release_rate: None,
        })
        .await?;
    assert_eq!(best_node.node, "https://node1");

    let post_user = db
        .post_user(params::PostUser {
            service_id,
            email: "test@example.com".to_owned(),
            generation: 1,
            client_state: "abcdef".to_owned(),
            created_at: 0,
            node_id,
            keys_changed_at: None,
        })
        .await?;

    let users = db
        .get_users(params::GetUsers {
            service_id,
            email: "test@example.com".to_owned(),
        })
        .await?;
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].uid, post_user.uid);

    db.add_user_to_node(params::AddUserToNode {
        service_id,
        node: "https://node1".to_owned(),
    })
    .await?;

    db.replace_user(params::ReplaceUser {
        service_id,
        uid: post_user.uid,
        replaced_at: 1,
    })
    .await?;

    assert!(db.check().await?);

    Ok(())
}
