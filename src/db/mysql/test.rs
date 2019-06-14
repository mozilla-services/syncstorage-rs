use std::{collections::HashMap, result::Result as StdResult};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{CustomizeConnection, Error as PoolError},
    Connection, QueryDsl, RunQueryDsl,
};
use env_logger;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use crate::db::mysql::{
    models::{MysqlDb, Result, DEFAULT_BSO_TTL},
    pool::MysqlDbPool,
    schema::collections,
};
use crate::db::util::SyncTimestamp;
use crate::db::{params, DbErrorKind, Sorting};
use crate::settings::{Secrets, ServerLimits, Settings};
use crate::web::extractors::{BsoQueryParams, HawkIdentifier};

// distant future (year 2099) timestamp for tests
pub const MAX_TIMESTAMP: u64 = 4_070_937_600_000;

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

fn pbso(
    user_id: u32,
    coll: &str,
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i32>,
    ttl: Option<u32>,
) -> params::PutBso {
    params::PutBso {
        user_id: HawkIdentifier::new_legacy(u64::from(user_id)),
        collection: coll.to_owned(),
        id: bid.to_owned(),
        payload: payload.map(|payload| payload.to_owned()),
        sortindex,
        ttl,
    }
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

#[allow(clippy::too_many_arguments)]
fn gbsos(
    user_id: u32,
    coll: &str,
    bids: &[&str],
    older: u64,
    newer: u64,
    sort: Sorting,
    limit: i64,
    offset: i64,
) -> params::GetBsos {
    params::GetBsos {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        params: BsoQueryParams {
            ids: bids.iter().map(|id| id.to_owned().into()).collect(),
            older: Some(SyncTimestamp::from_milliseconds(older)),
            newer: Some(SyncTimestamp::from_milliseconds(newer)),
            sort,
            limit: Some(limit as u32),
            offset: Some(offset as u64),
            full: true,
        },
    }
}

fn dbso(user_id: u32, coll: &str, bid: &str) -> params::DeleteBso {
    params::DeleteBso {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id: bid.to_owned(),
    }
}

fn dbsos(user_id: u32, coll: &str, bids: &[&str]) -> params::DeleteBsos {
    params::DeleteBsos {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        ids: bids.iter().map(|id| id.to_owned().into()).collect(),
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

#[test]
fn bso_successfully_updates_single_values() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "testBSO";
    let sortindex = 1;
    let ttl = 3600 * 1000;
    let bso1 = pbso(
        uid,
        coll,
        bid,
        Some("initial value"),
        Some(sortindex),
        Some(ttl),
    );
    db.put_bso_sync(bso1)?;

    let payload = "Updated payload";
    let bso2 = pbso(uid, coll, bid, Some(payload), None, None);
    db.put_bso_sync(bso2)?;

    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX: go version assumes ttl was updated here?
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));

    let sortindex = 2;
    let bso2 = pbso(uid, coll, bid, None, Some(sortindex), None);
    db.put_bso_sync(bso2)?;
    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX:
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));
    Ok(())
}

#[test]
fn bso_modified_not_changed_on_ttl_touch() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "testBSO";
    let timestamp = db.timestamp().as_i64();

    let bso1 = pbso(uid, coll, bid, Some("hello"), Some(1), Some(10));
    db.with_delta(-100, |db| db.put_bso_sync(bso1))?;

    let bso2 = pbso(uid, coll, bid, None, None, Some(15));
    db.put_bso_sync(bso2)?;
    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    // ttl has changed
    assert_eq!(bso.expiry, timestamp + (15 * 1000));
    // modified has not changed
    assert_eq!(bso.modified.as_i64(), timestamp - 100);
    Ok(())
}

#[test]
fn put_bso_updates() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "1";
    let bso1 = pbso(uid, coll, bid, Some("initial"), None, None);
    db.put_bso_sync(bso1)?;

    let payload = "Updated";
    let sortindex = 200;
    let bso2 = pbso(uid, coll, bid, Some(payload), Some(sortindex), None);
    db.put_bso_sync(bso2)?;

    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    assert_eq!(Some(bso.payload), Some(payload.to_owned()));
    assert_eq!(bso.sortindex, Some(sortindex));
    assert_eq!(bso.modified, db.timestamp());
    Ok(())
}

#[test]
fn get_bsos_limit_offset() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let size = 12;
    for i in 0..size {
        let bso = pbso(
            uid,
            coll,
            &i.to_string(),
            Some(&format!("payload-{}", i)),
            Some(i),
            Some(DEFAULT_BSO_TTL),
        );
        db.with_delta(i64::from(i) * 10, |db| db.put_bso_sync(bso))?;
    }

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Index,
        0,
        0,
    ))?;
    assert!(bsos.items.is_empty());
    assert_eq!(bsos.offset, Some(0));

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Index,
        -1,
        0,
    ))?;
    assert_eq!(bsos.items.len(), size as usize);
    assert_eq!(bsos.offset, None);

    let newer = 0;
    let limit = 5;
    let offset = 0;
    // XXX: validation?
    /*
    let bsos = db.get_bsos_sync(gbsos(uid, coll, &[], MAX_TIMESTAMP, 0, Sorting::Index, -1, 0))?;
    .. etc
    */

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Newest,
        limit,
        offset,
    ))?;
    assert_eq!(bsos.items.len(), 5 as usize);
    assert_eq!(bsos.offset, Some(5));
    assert_eq!(bsos.items[0].id, "11");
    assert_eq!(bsos.items[4].id, "7");

    let bsos2 = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos.offset.unwrap(),
    ))?;
    assert_eq!(bsos2.items.len(), 5 as usize);
    assert_eq!(bsos2.offset, Some(10));
    assert_eq!(bsos2.items[0].id, "6");
    assert_eq!(bsos2.items[4].id, "2");

    let bsos3 = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        newer,
        Sorting::Index,
        limit,
        bsos2.offset.unwrap(),
    ))?;
    assert_eq!(bsos3.items.len(), 2 as usize);
    assert_eq!(bsos3.offset, None);
    assert_eq!(bsos3.items[0].id, "1");
    assert_eq!(bsos3.items[1].id, "0");
    Ok(())
}

#[test]
fn get_bsos_newer() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let timestamp = db.timestamp().as_i64();
    // XXX: validation
    //db.get_bsos_sync(gbsos(uid, coll, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0)).is_err()

    for i in (0..=2).rev() {
        let pbso = pbso(
            uid,
            coll,
            &format!("b{}", i),
            Some("a"),
            Some(1),
            Some(DEFAULT_BSO_TTL),
        );
        db.with_delta(-i * 10, |db| db.put_bso_sync(pbso))?;
    }

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 30,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b2");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 20,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 2);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64 - 10,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 1);
    assert_eq!(bsos.items[0].id, "b0");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        timestamp as u64,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 0);
    Ok(())
}

#[test]
fn get_bsos_sort() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    // XXX: validation again
    //db.get_bsos_sync(gbsos(uid, coll, &[], MAX_TIMESTAMP, -1, Sorting::None, 10, 0)).is_err()

    for (revi, sortindex) in [1, 0, 2].iter().enumerate().rev() {
        let pbso = pbso(
            uid,
            coll,
            &format!("b{}", revi),
            Some("a"),
            Some(*sortindex),
            Some(DEFAULT_BSO_TTL),
        );
        db.with_delta(-(revi as i64) * 10, |db| db.put_bso_sync(pbso))?;
    }

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b2");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Oldest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b0");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Index,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b0");
    assert_eq!(bsos.items[2].id, "b1");
    Ok(())
}

#[test]
fn delete_bsos_in_correct_collection() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let payload = "data";
    db.put_bso_sync(pbso(uid, "clients", "b1", Some(payload), None, None))?;
    db.put_bso_sync(pbso(uid, "crypto", "b1", Some(payload), None, None))?;
    db.delete_bsos_sync(dbsos(uid, "clients", &["b1"]))?;
    let bso = db.get_bso_sync(gbso(uid, "crypto", "b1"))?;
    assert!(bso.is_some());
    Ok(())
}

#[test]
fn get_storage_timestamp() -> Result<()> {
    let db = db()?;

    let uid = 1;
    db.create_collection("col1")?;
    let col2 = db.create_collection("col2")?;
    db.create_collection("col3")?;

    db.with_delta(100_000, |db| {
        db.touch_collection(uid, col2)?;
        let m = db.get_storage_timestamp_sync(hid(uid))?;
        assert_eq!(m, db.timestamp());
        Ok(())
    })
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
    let coll = "NewCollection";
    for bid in 1..=3 {
        db.put_bso_sync(pbso(uid, coll, &bid.to_string(), Some("test"), None, None))?;
    }
    let ts = db.delete_collection_sync(params::DeleteCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })?;
    let ts2 = db.get_storage_timestamp_sync(hid(uid))?;
    assert_eq!(ts2, ts);

    // make sure BSOs are deleted
    for bid in 1..=3 {
        let result = db.get_bso_sync(gbso(uid, coll, &bid.to_string()))?;
        assert!(result.is_none());
    }

    let result = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
    });
    match result.unwrap_err().kind() {
        DbErrorKind::CollectionNotFound => (),
        _ => panic!("Expected CollectionNotFound"),
    };
    Ok(())
}

#[test]
fn get_collection_timestamps() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "test";
    let cid = db.create_collection(coll)?;
    db.touch_collection(uid, cid)?;
    let cols = db.get_collection_timestamps_sync(hid(uid))?;
    assert!(cols.contains_key(coll));
    assert_eq!(cols.get(coll), Some(&db.timestamp()));

    let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
    })?;
    assert_eq!(Some(&ts), cols.get(coll));
    Ok(())
}

#[test]
fn get_collection_usage() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let mut expected = HashMap::new();
    let mut rng = thread_rng();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        for i in 0..100 {
            let size = 50 + rng.gen_range(0, 100);
            let payload = rng
                .sample_iter(&Alphanumeric)
                .take(size)
                .collect::<String>();
            db.put_bso_sync(pbso(
                uid,
                coll,
                &format!("b{}", i),
                Some(&payload),
                None,
                None,
            ))?;
            *expected.entry(coll.to_owned()).or_insert(0) += size as i64;
        }
    }

    let sizes = db.get_collection_usage_sync(hid(uid))?;
    assert_eq!(sizes, expected);
    let total = db.get_storage_usage_sync(hid(uid))?;
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
        let count = 5 + rng.gen_range(0, 99);
        expected.insert(coll.to_owned(), count);
        for i in 0..count {
            db.put_bso_sync(pbso(uid, coll, &format!("b{}", i), Some("x"), None, None))?;
        }
    }

    let counts = db.get_collection_counts_sync(hid(uid))?;
    assert_eq!(counts, expected);
    Ok(())
}

#[test]
fn put_bso() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "NewCollection";
    let bid = "b0";
    let bso1 = pbso(uid, coll, bid, Some("foo"), Some(1), Some(DEFAULT_BSO_TTL));
    db.put_bso_sync(bso1)?;
    let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
    })?;
    assert_eq!(ts, db.timestamp());

    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    assert_eq!(&bso.payload, "foo");
    assert_eq!(bso.sortindex, Some(1));

    let bso2 = pbso(uid, coll, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    db.with_delta(19, |db| {
        db.put_bso_sync(bso2)?;
        let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })?;
        assert_eq!(ts, db.timestamp());

        let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
        assert_eq!(&bso.payload, "bar");
        assert_eq!(bso.sortindex, Some(2));
        Ok(())
    })
}

#[test]
fn post_bsos() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "NewCollection";
    let result = db.post_bsos_sync(params::PostBsos {
        user_id: hid(uid),
        collection: coll.to_owned(),
        bsos: vec![
            postbso("b0", Some("payload 0"), Some(10), None),
            postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
            postbso("b2", Some("payload 2"), Some(100), None),
        ],
        failed: Default::default(),
    })?;

    assert!(result.success.contains(&"b0".to_owned()));
    assert!(result.success.contains(&"b2".to_owned()));
    // XXX: validation?
    //assert!(!result.success.contains(&"b1".to_owned()));
    //assert!(!result.failed.contains_key("b1"));
    //assert!(!result.failed.contains_key("b1"));

    let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
    })?;
    // XXX: casts
    assert_eq!(result.modified, ts);

    let result2 = db.post_bsos_sync(params::PostBsos {
        user_id: hid(uid),
        collection: coll.to_owned(),
        bsos: vec![
            postbso("b0", Some("updated 0"), Some(11), Some(100_000)),
            postbso("b2", Some("updated 2"), Some(22), Some(10000)),
        ],
        failed: Default::default(),
    })?;

    assert_eq!(result2.success.len(), 2);
    assert_eq!(result2.failed.len(), 0);
    assert!(result2.success.contains(&"b0".to_owned()));
    assert!(result2.success.contains(&"b2".to_owned()));

    let bso = db.get_bso_sync(gbso(uid, coll, "b0"))?.unwrap();
    assert_eq!(bso.sortindex, Some(11));
    assert_eq!(bso.payload, "updated 0");
    let bso = db.get_bso_sync(gbso(uid, coll, "b2"))?.unwrap();
    assert_eq!(bso.sortindex, Some(22));
    assert_eq!(bso.payload, "updated 2");

    let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
    })?;
    assert_eq!(result2.modified, ts);
    Ok(())
}

#[test]
fn get_bso() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    let payload = "a";
    db.put_bso_sync(pbso(uid, coll, bid, Some(payload), None, None))?;

    let bso = db.get_bso_sync(gbso(uid, coll, bid))?.unwrap();
    assert_eq!(bso.id, bid);
    assert_eq!(bso.payload, payload);

    let result = db.get_bso_sync(gbso(uid, coll, "nope"))?;
    assert!(result.is_none());
    Ok(())
}

#[test]
fn get_bsos() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let sortindexes = vec![1, 3, 4, 2, 0];
    for (i, (revi, sortindex)) in sortindexes.iter().enumerate().rev().enumerate() {
        let bso = pbso(
            uid,
            coll,
            // XXX: to_string?
            &format!("b{}", revi.to_string()),
            Some("Hello"),
            Some(*sortindex),
            None,
        );
        db.with_delta(i as i64 * 10, |db| db.put_bso_sync(bso))?;
    }

    let ids = db.get_bso_ids_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(ids.items, vec!["b0", "b1", "b2", "b3", "b4"]);

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &["b0", "b2", "b4"],
        MAX_TIMESTAMP,
        0,
        Sorting::Newest,
        10,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b2");
    assert_eq!(bsos.items[2].id, "b4");

    let bsos = db.get_bsos_sync(gbsos(
        uid,
        coll,
        &[],
        MAX_TIMESTAMP,
        0,
        Sorting::Index,
        2,
        0,
    ))?;
    assert_eq!(bsos.items.len(), 2);
    assert_eq!(bsos.offset, Some(2));
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b1");
    Ok(())
}

#[test]
fn get_bso_timestamp() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    let bso = pbso(uid, coll, bid, Some("a"), None, None);
    db.put_bso_sync(bso)?;
    let ts = db.get_bso_timestamp_sync(params::GetBsoTimestamp {
        user_id: uid.into(),
        collection: coll.to_string(),
        id: bid.to_string(),
    })?;
    assert_eq!(ts, db.timestamp());
    Ok(())
}

#[test]
fn delete_bso() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    db.put_bso_sync(pbso(uid, coll, bid, Some("a"), None, None))?;
    db.delete_bso_sync(dbso(uid, coll, bid))?;
    let bso = db.get_bso_sync(gbso(uid, coll, bid))?;
    assert!(bso.is_none());
    Ok(())
}

#[test]
fn delete_bsos() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    let bids = (0..=2).map(|i| format!("b{}", i));
    for bid in bids.clone() {
        db.put_bso_sync(pbso(
            uid,
            coll,
            &bid,
            Some("payload"),
            Some(10),
            Some(DEFAULT_BSO_TTL),
        ))?;
    }
    db.delete_bso_sync(dbso(uid, coll, "b0"))?;
    // deleting non existant bid errors
    match db
        .delete_bso_sync(dbso(uid, coll, "bxi0"))
        .unwrap_err()
        .kind()
    {
        DbErrorKind::BsoNotFound => (),
        _ => panic!("Expected BsoNotFound"),
    }
    db.delete_bsos_sync(dbsos(uid, coll, &["b1", "b2"]))?;
    for bid in bids {
        let bso = db.get_bso_sync(gbso(uid, coll, &bid))?;
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
    let coll = "my_collection";
    let cid = db.create_collection(coll)?;
    db.put_bso_sync(pbso(uid, coll, bid, Some("test"), None, None))?;

    db.delete_storage_sync(hid(uid))?;
    let result = db.get_bso_sync(gbso(uid, coll, bid))?;
    assert!(result.is_none());

    // collection data sticks around
    let cid2 = db.get_collection_id("my_collection")?;
    assert_eq!(cid2, cid);
    Ok(())
}

#[test]
fn lock_for_read() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    db.lock_for_read_sync(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })?;
    match db.get_collection_id("NewCollection").unwrap_err().kind() {
        DbErrorKind::CollectionNotFound => (),
        _ => panic!("Expected CollectionNotFound"),
    }
    db.commit_sync()?;
    Ok(())
}

#[test]
fn lock_for_write() -> Result<()> {
    let db = db()?;

    let uid = 1;
    let coll = "clients";
    db.lock_for_write_sync(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })?;
    db.put_bso_sync(pbso(uid, coll, "1", Some("foo"), None, None))?;
    db.commit_sync()?;
    Ok(())
}
