use std::collections::HashMap;

use diesel::{sql_query, sql_types::Integer, QueryDsl, RunQueryDsl};

use db::models::{DBConfig, DBManager, PutBSO, Sorting, DEFAULT_BSO_TTL, MAX_TIMESTAMP};
use db::schema::collections;
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
fn db_init() {
    for size in &[0, -1, -10, 1, 100] {
        let db = DBManager::new(":memory:", DBConfig { cache_size: *size }).unwrap();
        db.init().unwrap();
        let result: Vec<Pragma> = sql_query("PRAGMA cache_size;").load(&db.conn).unwrap();
        assert_eq!(*size, result.first().unwrap().cache_size as i64);
    }
}

#[test]
fn static_collection_id() {
    let db = db();

    // ensure DB actually has predefined common collections
    let cols: Vec<(i64, _)> = vec![
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

    let results: HashMap<i64, String> = collections::table
        .select((collections::id, collections::name))
        .load(&db.conn)
        .unwrap()
        .into_iter()
        .collect();
    assert_eq!(results.len(), cols.len());
    for (id, name) in &cols {
        assert_eq!(results.get(id).unwrap(), name);
    }

    for (id, name) in &cols {
        let result = db.get_collection_id(name).unwrap();
        assert_eq!(result, *id);
    }

    let cid = db.create_collection("col1").unwrap();
    assert_eq!(cid, 100);
}

#[test]
fn bso_successfully_updates_single_values() {
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
fn bso_modified_not_changed_on_ttl_touch() {
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
fn put_bso_updates() {
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

#[test]
fn get_bsos_limit_offset() {
    let db = db();

    let cid = 1;
    let size = 12;
    for i in 0..size {
        let mut bso = pbso(
            cid,
            &i.to_string(),
            Some(&format!("payload-{}", i)),
            Some(i),
            Some(DEFAULT_BSO_TTL),
        );
        bso.last_modified += i * 10;
        db.put_bso(&bso).unwrap();
    }

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 0, 0)
        .unwrap();
    assert!(bsos.bsos.is_empty());
    assert!(bsos.more);
    assert_eq!(bsos.offset, 0);

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, -1, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), size as usize);
    assert!(!bsos.more);
    assert_eq!(bsos.offset, 0);

    let newer = 0;
    let limit = 5;
    let offset = 0;
    // XXX: validation?
    /*
    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, -1, 0).unwrap();
    .. etc
    */

    let bsos = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Newest,
        limit,
        offset,
    ).unwrap();
    assert_eq!(bsos.bsos.len(), 5 as usize);
    assert!(bsos.more);
    assert_eq!(bsos.offset, 5);
    assert_eq!(bsos.bsos[0].id, "11");
    assert_eq!(bsos.bsos[4].id, "7");

    let bsos2 = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos.offset,
    ).unwrap();
    assert_eq!(bsos2.bsos.len(), 5 as usize);
    assert!(bsos2.more);
    assert_eq!(bsos2.offset, 10);
    assert_eq!(bsos2.bsos[0].id, "6");
    assert_eq!(bsos2.bsos[4].id, "2");

    let bsos3 = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos2.offset,
    ).unwrap();
    assert_eq!(bsos3.bsos.len(), 2 as usize);
    assert!(!bsos3.more);
    assert_eq!(bsos3.offset, 0);
    assert_eq!(bsos3.bsos[0].id, "1");
    assert_eq!(bsos3.bsos[1].id, "0");
}

#[test]
fn get_bsos_newer() {
    let db = db();

    let cid = 1;
    let modified = ms_since_epoch();
    // XXX: validation
    //db.get_bsos(cid, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0).is_err()

    for i in (0..=2).rev() {
        let mut pbso = pbso(
            cid,
            &format!("b{}", i),
            Some("a"),
            Some(1),
            Some(DEFAULT_BSO_TTL),
        );
        pbso.last_modified = modified - i;
        db.put_bso(&pbso).unwrap();
    }

    let bsos = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        modified - 3,
        Sorting::Newest,
        10,
        0,
    ).unwrap();
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b2");

    let bsos = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        modified - 2,
        Sorting::Newest,
        10,
        0,
    ).unwrap();
    assert_eq!(bsos.bsos.len(), 2);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");

    let bsos = db.get_bsos(
        cid,
        &[],
        MAX_TIMESTAMP,
        modified - 1,
        Sorting::Newest,
        10,
        0,
    ).unwrap();
    assert_eq!(bsos.bsos.len(), 1);
    assert_eq!(bsos.bsos[0].id, "b0");

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, modified, Sorting::Newest, 10, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), 0);
}

#[test]
fn get_bsos_sort() {
    let db = db();

    let cid = 1;
    let modified = ms_since_epoch();
    // XXX: validation again
    //db.get_bsos(cid, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0).is_err()

    for (revi, sortindex) in [1, 0, 2].iter().enumerate().rev() {
        let mut pbso = pbso(
            cid,
            &format!("b{}", revi),
            Some("a"),
            Some(*sortindex),
            Some(DEFAULT_BSO_TTL),
        );
        pbso.last_modified = modified - revi as i64;
        db.put_bso(&pbso).unwrap();
    }

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Newest, 10, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b2");

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Oldest, 10, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b0");

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 10, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b0");
    assert_eq!(bsos.bsos[2].id, "b1");
}

#[test]
fn delete_bsos_in_correct_collection() {
    let db = db();

    let payload = "data";
    db.put_bso(&pbso(1, "b1", Some(payload), None, None))
        .unwrap();
    db.put_bso(&pbso(2, "b1", Some(payload), None, None))
        .unwrap();
    db.delete_bsos(1, &["b1"]).unwrap();
    let bso = db.get_bso(2, "b1").unwrap();
    assert!(bso.is_some());
}

#[test]
fn last_modified() {
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
fn get_collection_id() {
    let db = db();
    db.get_collection_id("bookmarks").unwrap();
}

#[test]
fn get_collection_modified() {
    let db = db();

    let name = "test";
    let cid = db.create_collection(name).unwrap();
    let cols = db.info_collections().unwrap();
    assert!(cols.contains_key(name));

    let modified = db.get_collection_modified(cid).unwrap();
    assert_eq!(Some(&modified), cols.get(name));
}

#[test]
fn create_collection() {
    let db = db();

    let name = "NewCollection";
    let cid = db.create_collection(name).unwrap();
    assert_ne!(cid, 0);
    let cid2 = db.get_collection_id(name).unwrap();
    assert_eq!(cid2, cid);
}

#[test]
fn touch_collection() {
    let db = db();

    let cid = db.create_collection("test").unwrap();
    db.touch_collection_and_storage(cid, ms_since_epoch())
        .unwrap();
}

#[test]
fn delete_collection() {
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
fn info_collections() {
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
fn info_collection_usage() {
    let db = db();
}

#[test]
fn info_collection_counts() {
    let db = db();
}
*/

#[test]
fn put_bso() {
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

    let mut bso2 = pbso(cid, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    bso2.last_modified += 19;
    db.put_bso(&bso2).unwrap();
    let modified = db.get_collection_modified(cid).unwrap();
    assert_eq!(bso2.last_modified, modified);

    let bso = db.get_bso(cid, bid).unwrap().unwrap();
    assert_eq!(&bso.payload, "bar");
    assert_eq!(bso.sortindex, Some(2));
}

/*
#[test]
fn post_bsos() {
    let db = db();

    let cid = 1;
    // XXX:
}
*/

#[test]
fn get_bso() {
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
fn get_bsos() {
    let db = db();

    let cid = 1;
    let sortindexes = vec![1, 3, 4, 2, 0];
    for (i, (revi, sortindex)) in sortindexes.iter().enumerate().rev().enumerate() {
        let mut bso = pbso(
            cid,
            &format!("b{}", revi.to_string()),
            Some("Hello"),
            Some(*sortindex),
            None,
        );
        bso.last_modified += i as i64 * 10;
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
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b2");
    assert_eq!(bsos.bsos[2].id, "b4");

    let bsos = db.get_bsos(cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 2, 0)
        .unwrap();
    assert_eq!(bsos.bsos.len(), 2);
    assert_eq!(bsos.offset, 2);
    assert!(bsos.more);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b1");
}

#[test]
fn get_bso_modified() {
    let db = db();

    let cid = 1;
    let bid = "b0";
    let bso = pbso(cid, bid, Some("a"), None, None);
    db.put_bso(&bso).unwrap();
    let modified = db.get_bso_modified(cid, bid).unwrap();
    assert_eq!(modified, bso.last_modified);
}

#[test]
fn delete_bso() {
    let db = db();

    let cid = 1;
    let bid = "b0";
    db.put_bso(&pbso(cid, bid, Some("a"), None, None)).unwrap();
    db.delete_bso(cid, bid).unwrap();
    let bso = db.get_bso(cid, bid).unwrap();
    assert!(bso.is_none());
}

#[test]
fn delete_bsos() {
    let db = db();

    let cid = 1;
    let bids = (0..=2).map(|i| format!("b{}", i));
    for bid in bids.clone() {
        db.put_bso(&pbso(
            cid,
            &bid,
            Some("payload"),
            Some(10),
            Some(DEFAULT_BSO_TTL),
        )).unwrap();
    }
    db.delete_bso(cid, "b0").unwrap();
    // deleting non existant bid returns no errors
    db.delete_bso(cid, "bxi0").unwrap();
    db.delete_bsos(cid, &["b1", "b2"]).unwrap();
    for bid in bids {
        let bso = db.get_bso(cid, &bid).unwrap();
        assert!(bso.is_none());
    }
}

/*
#[test]
fn usage_stats() {
    let db = db();
}

#[test]
fn purge_expired() {
    let db = db();
}

#[test]
fn optimize() {
    let db = db();
}

#[test]
fn delete_everything() {
    let db = db();
}
*/

#[test]
fn get_set_keyvalue() {
    let db = db();
    let value = db.get_key("testing").unwrap();
    assert!(value.is_none());
    db.set_key("testing", "12345").unwrap();
    let value = db.get_key("testing").unwrap();
    assert_eq!(value, Some("12345".to_owned()));
}

/*
#[test]
fn schema_upgrades() {
    let db = db();
}
*/
