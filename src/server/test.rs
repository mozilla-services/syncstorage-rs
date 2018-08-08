use std::sync::{Arc, Mutex, RwLock};

use actix_web::test::TestServer;
use actix_web::HttpMessage;
use serde_json;

use super::*;
use db::models::{DBConfig, DBManager, BSO};
use db::util::ms_since_epoch;
use handlers::BsoBody;

fn setup() -> TestServer {
    TestServer::build_with_state(move || {
        let db_manager = DBManager::new(":memory:", DBConfig::default()).unwrap();
        db_manager.init().unwrap();

        let mut db_handles = HashMap::new();
        db_handles.insert("deadbeef".to_string(), Mutex::new(db_manager));
        let db_handles = Arc::new(RwLock::new(db_handles));

        let db_executor = SyncArbiter::start(num_cpus::get(), move || DBExecutor {
            db_handles: db_handles.clone(),
        });

        ServerState { db_executor }
    }).start(|app| {
        init_routes!(app);
    })
}

fn test_endpoint(method: http::Method, path: &str, expected_body: &str) {
    let mut server = setup();

    let request = server.client(method, path).finish().unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    assert_eq!(body, expected_body.as_bytes());
}

#[test]
fn collections() {
    test_endpoint(http::Method::GET, "deadbeef/info/collections", "{}");
}

#[test]
fn collection_counts() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_counts", "{}");
}

#[test]
fn collection_usage() {
    test_endpoint(http::Method::GET, "deadbeef/info/collection_usage", "{}");
}

#[test]
fn configuration() {
    test_endpoint(http::Method::GET, "deadbeef/info/configuration", "{}");
}

#[test]
fn quota() {
    test_endpoint(http::Method::GET, "deadbeef/info/quota", "[0,null]");
}

#[test]
fn delete_bso() {
    test_endpoint(
        http::Method::DELETE,
        "deadbeef/storage/bookmarks/wibble",
        "null",
    );
}

#[test]
fn get_bso() {
    let bso_path = format!("storage/bookmarks/test.server.get_bso.{}", ms_since_epoch());
    let good_path = format!("deadbeef/{}", &bso_path);
    let bad_path = format!("baadf00d/{}", &bso_path);

    test_endpoint(http::Method::GET, &good_path, "null");

    let mut server = setup();
    let request = server
        .client(http::Method::GET, &bad_path)
        .finish()
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_server_error());
}

#[test]
fn put_bso() {
    let mut server = setup();

    let start_time = ms_since_epoch();
    let bso_path = format!("storage/bookmarks/{}", start_time);
    let good_path = format!("deadbeef/{}", &bso_path);

    let request = server
        .client(http::Method::PUT, &good_path)
        .json(BsoBody {
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000000),
        })
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let request = server
        .client(http::Method::GET, &good_path)
        .finish()
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_success());

    let body = server.execute(response.body()).unwrap();
    let bso: BSO = serde_json::from_slice(&*body).unwrap();
    assert_eq!(bso.collection_id, 7);
    assert_eq!(bso.id, start_time.to_string());
    assert_eq!(bso.sortindex.unwrap(), 0);
    assert_eq!(bso.payload, "wibble");
    assert_eq!(bso.payload_size, 6);
    assert!(bso.last_modified >= start_time && bso.last_modified <= ms_since_epoch());
    assert_eq!(bso.expiry, bso.last_modified + 31536000000);

    let bad_path = format!("baadf00d/{}", &bso_path);
    let request = server
        .client(http::Method::PUT, &bad_path)
        .json(BsoBody {
            sortindex: Some(0),
            payload: Some("wibble".to_string()),
            ttl: Some(31536000000),
        })
        .unwrap();

    let response = server.execute(request.send()).unwrap();
    assert!(response.status().is_server_error());
}
