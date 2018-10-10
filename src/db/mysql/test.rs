use std::{collections::HashMap, result::Result as StdResult};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection, QueryDsl, RunQueryDsl,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use db::mysql::{
    models::{run_embedded_migrations, MysqlDb, Result, DEFAULT_BSO_TTL},
    pool::MysqlDbPool,
    schema::collections,
};
use db::{error::DbErrorKind, params, Sorting};
use env_logger;
use settings::{Secrets, ServerLimits, Settings};
use web::auth::HawkIdentifier;

// distant future (year 2099) timestamp for tests
pub const MAX_TIMESTAMP: u64 = 4070937600000;

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
        database_url: settings.database_url,
        database_pool_max_size: Some(1),
        database_use_test_transactions: true,
        limits: ServerLimits::default(),
        master_secret: Secrets::default(),
    };

    run_embedded_migrations(&settings)?;
    let pool = MysqlDbPool::new(&settings)?;
    pool.get()
}

fn pbso<'a>(
    user_id: u32,
    cid: i32,
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i32>,
    ttl: Option<u32>,
) -> params::PutBso<'a> {
    params::PutBso {
        user_id: HawkIdentifier::new_legacy(user_id as u64),
        collection_id: cid,
        id: bid.to_owned(),
        payload: payload.map(|payload| payload.to_owned().into()),
        sortindex,
        ttl,
    }
}

fn postbso(
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

fn gbso(user_id: u32, cid: i32, bid: &str) -> params::GetBso {
    params::GetBso {
        user_id: HawkIdentifier::new_legacy(user_id as u64),
        collection_id: cid,
        id: bid.to_owned(),
    }
}

fn hid(user_id: u32) -> HawkIdentifier {
    HawkIdentifier::new_legacy(user_id as u64)
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
        .load(&db.conn)?
        .into_iter()
        .collect();
    assert_eq!(results.len(), cols.len());
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

#[test]
fn bso_successfully_updates_single_values() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "testBSO";
    let sortindex = 1;
    let ttl = 3600 * 1000;
    let bso1 = pbso(
        uid,
        cid,
        bid,
        Some("initial value"),
        Some(sortindex),
        Some(ttl),
    );
    db.put_bso_sync(&bso1)?;

    let payload = "Updated payload";
    let bso2 = pbso(uid, cid, bid, Some(payload), None, None);
    db.put_bso_sync(&bso2)?;

    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(bso.modified, db.session.timestamp);
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX: go version assumes ttl was updated here?
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.session.timestamp + ttl as i64);

    let sortindex = 2;
    let bso2 = pbso(uid, cid, bid, None, Some(sortindex), None);
    db.put_bso_sync(&bso2)?;
    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(bso.modified, db.session.timestamp);
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX:
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.session.timestamp + ttl as i64);
    Ok(())
}

#[test]
fn bso_modified_not_changed_on_ttl_touch() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "testBSO";
    let timestamp = db.session.timestamp;

    let bso1 = pbso(uid, cid, bid, Some("hello"), Some(1), Some(10));
    db.session.timestamp -= 100;
    let modified1 = db.session.timestamp;
    db.put_bso_sync(&bso1)?;
    db.session.timestamp = timestamp;

    let bso2 = pbso(uid, cid, bid, None, None, Some(15));
    db.put_bso_sync(&bso2)?;
    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    // ttl has changed
    assert_eq!(bso.expiry, timestamp + 15);
    // modified has not changed
    assert_eq!(bso.modified, modified1);
    Ok(())
}

#[test]
fn put_bso_updates() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "1";
    let bso1 = pbso(uid, cid, bid, Some("initial"), None, None);
    db.put_bso_sync(&bso1)?;

    let bso2 = pbso(uid, cid, bid, Some("Updated"), Some(100), None);
    db.put_bso_sync(&bso2)?;

    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(Some(bso.payload.into()), bso2.payload);
    assert_eq!(bso.sortindex, bso2.sortindex);
    assert_eq!(bso.modified, db.session.timestamp);
    Ok(())
}

#[test]
fn get_bsos_limit_offset() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let size = 12;
    let timestamp = db.session.timestamp;
    for i in 0..size {
        let bso = pbso(
            uid,
            cid,
            &i.to_string(),
            Some(&format!("payload-{}", i)),
            Some(i),
            Some(DEFAULT_BSO_TTL),
        );
        db.session.timestamp = timestamp + i as i64 * 10;
        db.put_bso_sync(&bso)?;
    }
    db.session.timestamp = timestamp;

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 0, 0)?;
    assert!(bsos.bsos.is_empty());
    assert!(bsos.more);
    assert_eq!(bsos.offset, 0);

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, -1, 0)?;
    assert_eq!(bsos.bsos.len(), size as usize);
    assert!(!bsos.more);
    assert_eq!(bsos.offset, 0);

    let newer = 0;
    let limit = 5;
    let offset = 0;
    // XXX: validation?
    /*
    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, -1, 0)?;
    .. etc
    */

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Newest,
        limit,
        offset,
    )?;
    assert_eq!(bsos.bsos.len(), 5 as usize);
    assert!(bsos.more);
    assert_eq!(bsos.offset, 5);
    assert_eq!(bsos.bsos[0].id, "11");
    assert_eq!(bsos.bsos[4].id, "7");

    let bsos2 = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos.offset,
    )?;
    assert_eq!(bsos2.bsos.len(), 5 as usize);
    assert!(bsos2.more);
    assert_eq!(bsos2.offset, 10);
    assert_eq!(bsos2.bsos[0].id, "6");
    assert_eq!(bsos2.bsos[4].id, "2");

    let bsos3 = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos2.offset,
    )?;
    assert_eq!(bsos3.bsos.len(), 2 as usize);
    assert!(!bsos3.more);
    assert_eq!(bsos3.offset, 0);
    assert_eq!(bsos3.bsos[0].id, "1");
    assert_eq!(bsos3.bsos[1].id, "0");
    Ok(())
}

#[test]
fn get_bsos_newer() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let timestamp = db.session.timestamp;
    // XXX: validation
    //db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0).is_err()

    for i in (0..=2).rev() {
        let pbso = pbso(
            uid,
            cid,
            &format!("b{}", i),
            Some("a"),
            Some(1),
            Some(DEFAULT_BSO_TTL),
        );
        db.session.timestamp = timestamp - i;
        db.put_bso_sync(&pbso)?;
    }
    db.session.timestamp = timestamp;

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 3,
        Sorting::Newest,
        10,
        0,
    )?;
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b2");

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 2,
        Sorting::Newest,
        10,
        0,
    )?;
    assert_eq!(bsos.bsos.len(), 2);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 1,
        Sorting::Newest,
        10,
        0,
    )?;
    assert_eq!(bsos.bsos.len(), 1);
    assert_eq!(bsos.bsos[0].id, "b0");

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64,
        Sorting::Newest,
        10,
        0,
    )?;
    assert_eq!(bsos.bsos.len(), 0);
    Ok(())
}

#[test]
fn get_bsos_sort() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let timestamp = db.session.timestamp;
    // XXX: validation again
    //db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0).is_err()

    for (revi, sortindex) in [1, 0, 2].iter().enumerate().rev() {
        let pbso = pbso(
            uid,
            cid,
            &format!("b{}", revi),
            Some("a"),
            Some(*sortindex),
            Some(DEFAULT_BSO_TTL),
        );
        db.session.timestamp = timestamp - revi as i64;
        db.put_bso_sync(&pbso)?;
    }
    db.session.timestamp = timestamp;

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Newest, 10, 0)?;
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b2");

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Oldest, 10, 0)?;
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b1");
    assert_eq!(bsos.bsos[2].id, "b0");

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 10, 0)?;
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b0");
    assert_eq!(bsos.bsos[2].id, "b1");
    Ok(())
}

#[test]
fn delete_bsos_in_correct_collection() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let payload = "data";
    db.put_bso_sync(&pbso(uid, 1, "b1", Some(payload), None, None))?;
    db.put_bso_sync(&pbso(uid, 2, "b1", Some(payload), None, None))?;
    db.delete_bsos_sync(uid, 1, &["b1"])?;
    let bso = db.get_bso_sync(&gbso(uid, 2, "b1"))?;
    assert!(bso.is_some());
    Ok(())
}

#[test]
fn get_storage_modified() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    db.create_collection("col1")?;
    let col2 = db.create_collection("col2")?;
    db.create_collection("col3")?;

    db.session.timestamp += 100000;
    db.touch_collection(uid, col2)?;

    let m = db.get_storage_modified_sync(uid)?;
    assert_eq!(m, db.session.timestamp);
    Ok(())
}

#[test]
fn get_collection_id() -> Result<()> {
    let db = db()?;
    db.get_collection_id("bookmarks")?;
    Ok(())
}

#[test]
fn create_collection() -> Result<()> {
    let db = db()?;

    let name = "NewCollection";
    let cid = db.create_collection(name)?;
    assert_ne!(cid, 0);
    let cid2 = db.get_collection_id(name)?;
    assert_eq!(cid2, cid);
    Ok(())
}

#[test]
fn touch_collection() -> Result<()> {
    let db = db()?;

    let cid = db.create_collection("test")?;
    db.touch_collection(1, cid)?;
    Ok(())
}

#[test]
fn delete_collection() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cname = "NewConnection";
    let cid = db.create_collection(cname)?;
    for bid in 1..=3 {
        db.put_bso_sync(&pbso(uid, cid, &bid.to_string(), Some("test"), None, None))?;
    }
    let modified = db.delete_collection_sync(uid, cid)?;
    let modified2 = db.get_storage_modified_sync(uid)?;
    assert_eq!(modified2, modified);

    // make sure BSOs are deleted
    for bid in 1..=3 {
        let result = db.get_bso_sync(&gbso(uid, cid, &bid.to_string()))?;
        assert!(result.is_none());
    }

    let result = db.get_collection_modified_sync(uid, cid);
    match result.unwrap_err().kind() {
        DbErrorKind::CollectionNotFound => assert!(true),
        _ => assert!(false),
    };
    Ok(())
}

#[test]
fn get_collections_modified() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let name = "test";
    let cid = db.create_collection(name)?;
    db.touch_collection(uid, cid)?;
    let cols = db.get_collections_modified_sync(&params::GetCollections { user_id: hid(uid) })?;
    assert!(cols.contains_key(name));
    assert_eq!(cols.get(name), Some(&db.session.timestamp));

    let modified = db.get_collection_modified_sync(uid, cid)?;
    assert_eq!(Some(&modified), cols.get(name));
    Ok(())
}

#[test]
fn get_collection_sizes() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let mut expected = HashMap::new();
    let mut rng = thread_rng();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        let cid = db.get_collection_id(coll)?;

        for i in 0..100 {
            let size = 50 + rng.gen_range(0, 100);
            let payload = rng
                .sample_iter(&Alphanumeric)
                .take(size)
                .collect::<String>();
            db.put_bso_sync(&pbso(
                uid,
                cid,
                &format!("b{}", i),
                Some(&payload),
                None,
                None,
            ))?;
            *expected.entry(coll.to_owned()).or_insert(0) += size as i64;
        }
    }

    let sizes = db.get_collection_sizes_sync(hid(uid))?;
    assert_eq!(sizes, expected);
    let total = db.get_storage_size_sync(hid(uid))?;
    assert_eq!(total, expected.values().sum::<i64>() as u64);
    Ok(())
}

#[test]
fn get_collection_counts() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let mut expected = HashMap::new();
    let mut rng = thread_rng();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        let cid = db.get_collection_id(coll)?;
        let count = 5 + rng.gen_range(0, 99);
        expected.insert(coll.to_owned(), count);
        for i in 0..count {
            db.put_bso_sync(&pbso(uid, cid, &format!("b{}", i), Some("x"), None, None))?;
        }
    }

    let counts = db.get_collection_counts_sync(hid(uid))?;
    assert_eq!(counts, expected);
    Ok(())
}

#[test]
fn put_bso() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "b0";
    let bso1 = pbso(uid, cid, bid, Some("foo"), Some(1), Some(DEFAULT_BSO_TTL));
    db.put_bso_sync(&bso1)?;
    let modified = db.get_collection_modified_sync(uid, cid)?;
    assert_eq!(modified, db.session.timestamp);

    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(&bso.payload, "foo");
    assert_eq!(bso.sortindex, Some(1));

    let bso2 = pbso(uid, cid, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    db.session.timestamp += 19;
    db.put_bso_sync(&bso2)?;
    let modified = db.get_collection_modified_sync(uid, cid)?;
    assert_eq!(modified, db.session.timestamp);

    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(&bso.payload, "bar");
    assert_eq!(bso.sortindex, Some(2));
    Ok(())
}

#[test]
fn post_bsos() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let result = db.post_bsos_sync(&params::PostCollection {
        user_id: hid(uid),
        collection_id: cid,
        bsos: vec![
            postbso("b0", Some("payload 0"), Some(10), None),
            postbso("b1", Some("payload 1"), Some(1000000000), None),
            postbso("b2", Some("payload 2"), Some(100), None),
        ],
    })?;

    assert!(result.success.contains(&"b0".to_owned()));
    assert!(result.success.contains(&"b2".to_owned()));
    // XXX: validation?
    //assert!(!result.success.contains(&"b1".to_owned()));
    //assert!(!result.failed.contains_key("b1"));
    //assert!(!result.failed.contains_key("b1"));

    let modified = db.get_collection_modified_sync(uid, cid)?;
    // XXX: casts
    assert_eq!(result.modified, modified as u64);

    let result2 = db.post_bsos_sync(&params::PostCollection {
        user_id: hid(uid),
        collection_id: cid,
        bsos: vec![
            postbso("b0", Some("updated 0"), Some(11), Some(100000)),
            postbso("b2", Some("updated 2"), Some(22), Some(10000)),
        ],
    })?;

    assert_eq!(result2.success.len(), 2);
    assert_eq!(result2.failed.len(), 0);
    assert!(result2.success.contains(&"b0".to_owned()));
    assert!(result2.success.contains(&"b2".to_owned()));

    let bso = db.get_bso_sync(&gbso(uid, cid, "b0"))?.unwrap();
    assert_eq!(bso.sortindex, Some(11));
    assert_eq!(bso.payload, "updated 0");
    let bso = db.get_bso_sync(&gbso(uid, cid, "b2"))?.unwrap();
    assert_eq!(bso.sortindex, Some(22));
    assert_eq!(bso.payload, "updated 2");

    let modified = db.get_collection_modified_sync(uid, cid)?;
    assert_eq!(result2.modified, modified as u64);
    Ok(())
}

#[test]
fn get_bso() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "b0";
    let payload = "a";
    db.put_bso_sync(&pbso(uid, cid, bid, Some(payload), None, None))?;

    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?.unwrap();
    assert_eq!(bso.id, bid);
    assert_eq!(bso.payload, payload);

    let result = db.get_bso_sync(&gbso(uid, cid, "nope"))?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn get_bsos() -> Result<()> {
    let mut db = db()?;

    let uid = 1;
    let cid = 1;
    let timestamp = db.session.timestamp;
    let sortindexes = vec![1, 3, 4, 2, 0];
    for (i, (revi, sortindex)) in sortindexes.iter().enumerate().rev().enumerate() {
        let bso = pbso(
            uid,
            cid,
            // XXX: to_string?
            &format!("b{}", revi.to_string()),
            Some("Hello"),
            Some(*sortindex),
            None,
        );
        db.session.timestamp = timestamp + i as i64 * 10;
        db.put_bso_sync(&bso)?;
    }
    db.session.timestamp = timestamp;

    let bsos = db.get_bsos_sync(
        uid,
        cid,
        &vec!["b0", "b2", "b4"],
        MAX_TIMESTAMP,
        0,
        Sorting::Newest,
        10,
        0,
    )?;
    assert_eq!(bsos.bsos.len(), 3);
    assert_eq!(bsos.bsos[0].id, "b0");
    assert_eq!(bsos.bsos[1].id, "b2");
    assert_eq!(bsos.bsos[2].id, "b4");

    let bsos = db.get_bsos_sync(uid, cid, &[], MAX_TIMESTAMP, 0, Sorting::Index, 2, 0)?;
    assert_eq!(bsos.bsos.len(), 2);
    assert_eq!(bsos.offset, 2);
    assert!(bsos.more);
    assert_eq!(bsos.bsos[0].id, "b2");
    assert_eq!(bsos.bsos[1].id, "b1");
    Ok(())
}

#[test]
fn get_bso_modified() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "b0";
    let bso = pbso(uid, cid, bid, Some("a"), None, None);
    db.put_bso_sync(&bso)?;
    let modified = db.get_bso_modified_sync(uid, cid, bid)?;
    assert_eq!(modified, db.session.timestamp);
    Ok(())
}

#[test]
fn delete_bso() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bid = "b0";
    db.put_bso_sync(&pbso(uid, cid, bid, Some("a"), None, None))?;
    db.delete_bso_sync(uid, cid, bid)?;
    let bso = db.get_bso_sync(&gbso(uid, cid, bid))?;
    assert!(bso.is_none());
    Ok(())
}

#[test]
fn delete_bsos() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let cid = 1;
    let bids = (0..=2).map(|i| format!("b{}", i));
    for bid in bids.clone() {
        db.put_bso_sync(&pbso(
            uid,
            cid,
            &bid,
            Some("payload"),
            Some(10),
            Some(DEFAULT_BSO_TTL),
        ))?;
    }
    db.delete_bso_sync(uid, cid, "b0")?;
    // deleting non existant bid returns no errors
    db.delete_bso_sync(uid, cid, "bxi0")?;
    db.delete_bsos_sync(uid, cid, &["b1", "b2"])?;
    for bid in bids {
        let bso = db.get_bso_sync(&gbso(uid, cid, &bid))?;
        assert!(bso.is_none());
    }
    Ok(())
}

/*
#[test]
fn usage_stats() -> Result<()> {
    let db = db()?;
    Ok(())
}

#[test]
fn purge_expired() -> Result<()> {
    let db = db()?;
    Ok(())
}

#[test]
fn optimize() -> Result<()> {
    let db = db()?;
    Ok(())
}
*/

#[test]
fn delete_storage() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let bid = "test";
    let cid = db.create_collection("my_collection")?;
    db.put_bso_sync(&pbso(uid, cid, bid, Some("test"), None, None))?;

    db.delete_storage_sync(uid)?;
    let result = db.get_bso_sync(&gbso(uid, cid, bid))?;
    assert!(result.is_none());

    // collection data sticks around
    let cid2 = db.get_collection_id("my_collection")?;
    assert_eq!(cid2, cid);
    Ok(())
}
