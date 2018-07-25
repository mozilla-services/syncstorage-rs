#[cfg(test)]
use std::{thread, time};

use diesel::{sql_query, sql_types::Integer, RunQueryDsl};

use db::models::{DBConfig, DBManager, PutBSO, Sorting, DEFAULT_BSO_TTL, MAX_TIMESTAMP};
use db::util::ms_since_epoch;

#[derive(QueryableByName)]
struct Pragma {
    #[sql_type = "Integer"]
    cache_size: i32,
}

pub fn db() -> DBManager {
    let db = DBManager::new(":memory:", DBConfig::default()).unwrap();
    db.init().unwrap();
    db
}

fn pbso(
    cid: i64,
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i64>,
    ttl: Option<i64>,
) -> PutBSO {
    PutBSO {
        collection_id: cid,
        id: bid.to_owned(),
        payload: payload.map(&str::to_owned),
        sortindex,
        ttl,
        last_modified: ms_since_epoch(),
    }
}

#[test]
fn test_init() {
    for size in &[0, -1, -10, 1, 100] {
        let db = DBManager::new(":memory:", DBConfig { cache_size: *size }).unwrap();
        db.init().unwrap();
        let result: Vec<Pragma> = sql_query("PRAGMA cache_size;").load(&db.conn).unwrap();
        assert_eq!(*size, result.first().unwrap().cache_size as i64);
    }
}

#[test]
fn test_static_collection_id() {
    /*
        let db = db();
        // ensure DB actually has predefined common collections
        let cols = vec![
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
         */
}

#[test]
fn test_bso_successfully_updates_single_values() {
    let db = db();
    let cid = 1;
    let bid = "testBSO";
    let sortindex = 1;
    let ttl = 3600 * 1000;

    let bso1 = pbso(cid, bid, Some("initial value"), Some(sortindex), Some(ttl));
    db.put_bso(&bso1).unwrap();

    let payload = "Updated payload";
    let bso2 = pbso(cid, bid, Some(payload), None, None);
    db.put_bso(&bso2).unwrap();

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(bso.last_modified, bso2.last_modified);
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX: go version assumes ttl was updated here?
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, bso1.last_modified + ttl);

    let sortindex = 2;
    let bso2 = pbso(cid, bid, None, Some(sortindex), None);
    db.put_bso(&bso2).unwrap();
    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(bso.last_modified, bso2.last_modified);
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX:
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, bso1.last_modified + ttl);
}

#[test]
fn test_bso_modified_not_changed_on_ttl_touch() {
    let db = db();
    let cid = 1;
    let bid = "testBSO";

    let mut bso1 = pbso(cid, bid, Some("hello"), Some(1), Some(10));
    bso1.last_modified = ms_since_epoch() - 100;
    db.put_bso(&bso1).unwrap();

    let bso2 = pbso(cid, bid, None, None, Some(15));
    db.put_bso(&bso2).unwrap();
    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    // ttl has changed
    assert_eq!(bso.expiry, bso2.last_modified + 15);
    // modified has not changed
    assert_eq!(bso.last_modified, bso1.last_modified);
}

#[test]
fn test_put_bso_updates() {
    let db = db();
    let cid = 1;
    let bid = "1";

    let bso1 = pbso(cid, bid, Some("initial"), None, None);
    db.put_bso(&bso1).unwrap();

    let bso2 = pbso(cid, bid, Some("Updated"), Some(100), None);
    db.put_bso(&bso2).unwrap();

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(Some(bso.payload), bso2.payload);
    assert_eq!(bso.sortindex, bso2.sortindex);
    assert_eq!(bso.last_modified, bso2.last_modified);
}

/*
#[test]
fn test_get_bsos_limit_offset() {
}

#[test]
fn test_get_bsos_newer() {
}

#[test]
fn test_get_bsos_sort() {
}

#[test]
fn test_delete_bsos_in_correct_collection() {
}
*/

#[test]
fn test_last_modified() {
    let db = db();
    db.create_collection("col1").unwrap();
    let col2 = db.create_collection("col2").unwrap();
    db.create_collection("col3").unwrap();

    let modified = ms_since_epoch() + 100000;
    db.touch_collection_and_storage(col2, modified).unwrap();

    let m = db.last_modified().unwrap();
    assert_eq!(m, modified);
}

#[test]
fn test_get_collection_id() {
    let db = db();
    db.get_collection_id("bookmarks").unwrap();
}

#[test]
fn test_get_collection_modified() {
    let db = db();

    let name = "test";
    let cid = db.create_collection(name).unwrap();
    let cols = db.info_collections().unwrap();
    assert!(cols.contains_key(name));

    let modified = db.get_collection_modified(cid).unwrap();
    assert_eq!(Some(&modified), cols.get(name));
}

#[test]
fn test_create_collection() {
    let db = db();

    let name = "NewCollection";
    let cid = db.create_collection(name).unwrap();
    assert_ne!(cid, 0);
    let cid2 = db.get_collection_id(name).unwrap();
    assert_eq!(cid2, cid);
}

#[test]
fn test_touch_collection() {
    let db = db();

    let cid = db.create_collection("test").unwrap();
    db.touch_collection_and_storage(cid, ms_since_epoch())
        .unwrap();
}

#[test]
fn test_delete_collection() {
    let db = db();

    let cname = "NewConnection";
    let cid = db.create_collection(cname).unwrap();
    for bid in 1..=3 {
        db.put_bso(&pbso(cid, &bid.to_string(), Some("test"), None, None))
            .unwrap();
    }
    let modified = db.delete_collection(cid).unwrap();
    let modified2 = db.last_modified().unwrap();
    assert_eq!(modified2, modified);

    // make sure BSOs are deleted
    for bid in 1..=3 {
        let result = db.get_bso(cid, &bid.to_string()).unwrap();
        assert!(result.is_none());
    }

    let cmodified = db.get_collection_modified(cid).unwrap();
    assert_eq!(cmodified, 0);
}

#[test]
fn test_info_collections() {
    let db = db();

    let name = "bookmarks";
    let id = db.get_collection_id(name).unwrap();
    let modified = ms_since_epoch();
    db.touch_collection_and_storage(id, modified).unwrap();
    let cols = db.info_collections().unwrap();
    assert!(cols.contains_key(name));
    assert_eq!(cols.get(name), Some(&modified));
}

/*
#[test]
fn test_info_collection_usage() {
    let db = db();
}

#[test]
fn test_info_collection_counts() {
    let db = db();
}
*/

#[test]
fn test_put_bso() {
    let db = db();
    let cid = 1;
    let bid = "b0";

    let bso1 = pbso(cid, bid, Some("foo"), Some(1), Some(DEFAULT_BSO_TTL));
    db.put_bso(&bso1).unwrap();
    let modified = db.get_collection_modified(cid).unwrap();
    assert_eq!(bso1.last_modified, modified);

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(&bso.payload, "foo");
    assert_eq!(bso.sortindex, Some(1));

    // sleep a bit so we have a least a 100th of a millisecond difference
    // between the operations
    thread::sleep(time::Duration::from_millis(19));

    let bso2 = pbso(cid, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    db.put_bso(&bso2).unwrap();
    let modified = db.get_collection_modified(cid).unwrap();
    assert_eq!(bso2.last_modified, modified);

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(&bso.payload, "bar");
    assert_eq!(bso.sortindex, Some(2));
}

/*
#[test]
fn test_post_bsos() {
}
 */

#[test]
fn test_get_bso() {
    let db = db();

    let cid = 1;
    let bid = "b0";
    let payload = "a";
    db.put_bso(&pbso(cid, bid, Some(payload), None, None))
        .unwrap();

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(bso.id, bid);
    assert_eq!(bso.payload, payload);

    let result = db.get_bso(cid, "nope").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_get_bsos() {
    let db = db();

    let cid = 1;
    let payload = "Hello";
    // XXX: document this
    let sortindexes = vec![1, 3, 4, 2, 0];
    for i in sortindexes.iter().rev() {
        let mut bso = pbso(
            cid,
            &format!("b{}", i.to_string()),
            Some(payload),
            Some(*i),
            None,
        );
        bso.last_modified += i;
        db.put_bso(&bso).unwrap();
    }

    let bsos = db.get_bsos(
        cid,
        &vec!["b0", "b2", "b4"],
        MAX_TIMESTAMP,
        0,
        Sorting::Newest,
        10,
        0,
    ).unwrap();
    /*
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(&bsos.bsos[0], "b0");
    assert_eq!(&bsos.bsos[0], "b2");
    assert_eq!(&bsos.bsos[0], "b4");
    */
}

/*
#[test]
fn test_get_bso_modified() {
}
*/

// more..
