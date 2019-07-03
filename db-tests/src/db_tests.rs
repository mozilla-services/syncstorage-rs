use std::collections::HashMap;

use env_logger;
use futures::compat::Future01CompatExt;
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use codegen::async_test;
use syncstorage::db::mysql::{models::DEFAULT_BSO_TTL, pool::MysqlDbPool};
use syncstorage::db::util::SyncTimestamp;
use syncstorage::db::{params, Db, DbPool, Sorting};
use syncstorage::error::ApiError;
use syncstorage::settings::{Secrets, ServerLimits, Settings};
use syncstorage::web::extractors::{BsoQueryParams, HawkIdentifier};

// distant future (year 2099) timestamp for tests
const MAX_TIMESTAMP: u64 = 4_070_937_600_000;

type Result<T> = std::result::Result<T, ApiError>;

async fn db() -> Result<Box<dyn Db>> {
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

    // XXX: let pool = syncstorage::db:pool_from_settings(&settings)?;
    let pool = MysqlDbPool::new(&settings)?;
    pool.get().compat().await
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

fn gbso(user_id: u32, coll: &str, bid: &str) -> params::GetBso {
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

fn hid(user_id: u32) -> HawkIdentifier {
    HawkIdentifier::new_legacy(u64::from(user_id))
}

macro_rules! with_delta {
    ($db:expr, $delta:expr, $body:block) => {{
        let ts = $db.timestamp().as_i64();
        $db.set_timestamp(SyncTimestamp::_from_i64(ts + $delta).unwrap());
        let result = $body;
        $db.set_timestamp(SyncTimestamp::_from_i64(ts).unwrap());
        result
    }};
}

#[async_test]
async fn bso_successfully_updates_single_values() -> Result<()> {
    let db = db().await?;

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
    db.put_bso(bso1).compat().await?;

    let payload = "Updated payload";
    let bso2 = pbso(uid, coll, bid, Some(payload), None, None);
    db.put_bso(bso2).compat().await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX: go version assumes ttl was updated here?
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));

    let sortindex = 2;
    let bso2 = pbso(uid, coll, bid, None, Some(sortindex), None);
    db.put_bso(bso2).compat().await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    // XXX:
    //assert_eq!(bso.expiry, modified + ttl);
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));
    Ok(())
}

#[async_test]
async fn bso_modified_not_changed_on_ttl_touch() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bid = "testBSO";
    let timestamp = db.timestamp().as_i64();

    let bso1 = pbso(uid, coll, bid, Some("hello"), Some(1), Some(10));
    with_delta!(db, -100, { db.put_bso(bso1).compat().await })?;

    let bso2 = pbso(uid, coll, bid, None, None, Some(15));
    db.put_bso(bso2).compat().await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    // ttl has changed
    assert_eq!(bso.expiry, timestamp + (15 * 1000));
    // modified has not changed
    assert_eq!(bso.modified.as_i64(), timestamp - 100);
    Ok(())
}

#[async_test]
async fn put_bso_updates() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bid = "1";
    let bso1 = pbso(uid, coll, bid, Some("initial"), None, None);
    db.put_bso(bso1).compat().await?;

    let payload = "Updated";
    let sortindex = 200;
    let bso2 = pbso(uid, coll, bid, Some(payload), Some(sortindex), None);
    db.put_bso(bso2).compat().await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    assert_eq!(Some(bso.payload), Some(payload.to_owned()));
    assert_eq!(bso.sortindex, Some(sortindex));
    assert_eq!(bso.modified, db.timestamp());
    Ok(())
}

#[async_test]
async fn get_bsos_limit_offset() -> Result<()> {
    let db = db().await?;

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
        with_delta!(&db, i64::from(i) * 10, { db.put_bso(bso).compat().await })?;
    }

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Index,
            0,
            0,
        ))
        .compat()
        .await?;
    assert!(bsos.items.is_empty());
    assert_eq!(bsos.offset, Some(0));

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Index,
            -1,
            0,
        ))
        .compat()
        .await?;
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

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            newer,
            Sorting::Newest,
            limit,
            offset,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 5 as usize);
    assert_eq!(bsos.offset, Some(5));
    assert_eq!(bsos.items[0].id, "11");
    assert_eq!(bsos.items[4].id, "7");

    let bsos2 = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            newer,
            Sorting::Index,
            limit,
            bsos.offset.unwrap(),
        ))
        .compat()
        .await?;
    assert_eq!(bsos2.items.len(), 5 as usize);
    assert_eq!(bsos2.offset, Some(10));
    assert_eq!(bsos2.items[0].id, "6");
    assert_eq!(bsos2.items[4].id, "2");

    let bsos3 = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            newer,
            Sorting::Index,
            limit,
            bsos2.offset.unwrap(),
        ))
        .compat()
        .await?;
    assert_eq!(bsos3.items.len(), 2 as usize);
    assert_eq!(bsos3.offset, None);
    assert_eq!(bsos3.items[0].id, "1");
    assert_eq!(bsos3.items[1].id, "0");
    Ok(())
}

#[async_test]
async fn get_bsos_newer() -> Result<()> {
    let db = db().await?;

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
        with_delta!(&db, -i * 10, { db.put_bso(pbso).compat().await })?;
    }

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            timestamp as u64 - 30,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b2");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            timestamp as u64 - 20,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 2);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            timestamp as u64 - 10,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 1);
    assert_eq!(bsos.items[0].id, "b0");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            timestamp as u64,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 0);
    Ok(())
}

#[async_test]
async fn get_bsos_sort() -> Result<()> {
    let db = db().await?;

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
        with_delta!(&db, -(revi as i64) * 10, {
            db.put_bso(pbso).compat().await
        })?;
    }

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b2");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Oldest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b1");
    assert_eq!(bsos.items[2].id, "b0");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Index,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b0");
    assert_eq!(bsos.items[2].id, "b1");
    Ok(())
}

#[async_test]
async fn delete_bsos_in_correct_collection() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let payload = "data";
    db.put_bso(pbso(uid, "clients", "b1", Some(payload), None, None))
        .compat()
        .await?;
    db.put_bso(pbso(uid, "crypto", "b1", Some(payload), None, None))
        .compat()
        .await?;
    db.delete_bsos(dbsos(uid, "clients", &["b1"]))
        .compat()
        .await?;
    let bso = db.get_bso(gbso(uid, "crypto", "b1")).compat().await?;
    assert!(bso.is_some());
    Ok(())
}

#[async_test]
async fn get_storage_timestamp() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    db.create_collection("col1".to_owned()).compat().await?;
    let col2 = db.create_collection("col2".to_owned()).compat().await?;
    db.create_collection("col3".to_owned()).compat().await?;

    with_delta!(&db, 100_000, {
        db.touch_collection(params::TouchCollection {
            user_id: hid(uid),
            collection_id: col2,
        })
        .compat()
        .await?;
        let m = db.get_storage_timestamp(hid(uid)).compat().await?;
        assert_eq!(m, db.timestamp());
        Ok(())
    })
}

#[async_test]
async fn get_collection_id() -> Result<()> {
    let db = db().await?;
    db.get_collection_id("bookmarks".to_owned())
        .compat()
        .await?;
    Ok(())
}

#[async_test]
async fn create_collection() -> Result<()> {
    let db = db().await?;

    let name = "NewCollection";
    let cid = db.create_collection(name.to_owned()).compat().await?;
    assert_ne!(cid, 0);
    let cid2 = db.get_collection_id(name.to_owned()).compat().await?;
    assert_eq!(cid2, cid);
    Ok(())
}

#[async_test]
async fn touch_collection() -> Result<()> {
    let db = db().await?;

    let cid = db.create_collection("test".to_owned()).compat().await?;
    db.touch_collection(params::TouchCollection {
        user_id: hid(1),
        collection_id: cid,
    })
    .compat()
    .await?;
    Ok(())
}

#[async_test]
async fn delete_collection() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "NewCollection";
    for bid in 1..=3 {
        db.put_bso(pbso(uid, coll, &bid.to_string(), Some("test"), None, None))
            .compat()
            .await?;
    }
    let ts = db
        .delete_collection(params::DeleteCollection {
            user_id: hid(uid),
            collection: coll.to_owned(),
        })
        .compat()
        .await?;
    let ts2 = db.get_storage_timestamp(hid(uid)).compat().await?;
    assert_eq!(ts2, ts);

    // make sure BSOs are deleted
    for bid in 1..=3 {
        let result = db
            .get_bso(gbso(uid, coll, &bid.to_string()))
            .compat()
            .await?;
        assert!(result.is_none());
    }

    let result = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .compat()
        .await;
    assert!(result.unwrap_err().is_colllection_not_found());
    Ok(())
}

#[async_test]
async fn get_collection_timestamps() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "test";
    let cid = db.create_collection(coll.to_owned()).compat().await?;
    db.touch_collection(params::TouchCollection {
        user_id: hid(uid),
        collection_id: cid,
    })
    .compat()
    .await?;
    let cols = db.get_collection_timestamps(hid(uid)).compat().await?;
    assert!(cols.contains_key(coll));
    assert_eq!(cols.get(coll), Some(&db.timestamp()));

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .compat()
        .await?;
    assert_eq!(Some(&ts), cols.get(coll));
    Ok(())
}

#[async_test]
async fn get_collection_usage() -> Result<()> {
    let db = db().await?;

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
            db.put_bso(pbso(
                uid,
                coll,
                &format!("b{}", i),
                Some(&payload),
                None,
                None,
            ))
            .compat()
            .await?;
            *expected.entry(coll.to_owned()).or_insert(0) += size as i64;
        }
    }

    let sizes = db.get_collection_usage(hid(uid)).compat().await?;
    assert_eq!(sizes, expected);
    let total = db.get_storage_usage(hid(uid)).compat().await?;
    assert_eq!(total, expected.values().sum::<i64>() as u64);
    Ok(())
}

#[async_test]
async fn get_collection_counts() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let mut expected = HashMap::new();
    let mut rng = thread_rng();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        let count = 5 + rng.gen_range(0, 99);
        expected.insert(coll.to_owned(), count);
        for i in 0..count {
            db.put_bso(pbso(uid, coll, &format!("b{}", i), Some("x"), None, None))
                .compat()
                .await?;
        }
    }

    let counts = db.get_collection_counts(hid(uid)).compat().await?;
    assert_eq!(counts, expected);
    Ok(())
}

#[async_test]
async fn put_bso() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "NewCollection";
    let bid = "b0";
    let bso1 = pbso(uid, coll, bid, Some("foo"), Some(1), Some(DEFAULT_BSO_TTL));
    db.put_bso(bso1).compat().await?;
    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .compat()
        .await?;
    assert_eq!(ts, db.timestamp());

    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    assert_eq!(&bso.payload, "foo");
    assert_eq!(bso.sortindex, Some(1));

    let bso2 = pbso(uid, coll, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    with_delta!(&db, 19, {
        db.put_bso(bso2).compat().await?;
        let ts = db
            .get_collection_timestamp(params::GetCollectionTimestamp {
                user_id: uid.into(),
                collection: coll.to_string(),
            })
            .compat()
            .await?;
        assert_eq!(ts, db.timestamp());

        let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
        assert_eq!(&bso.payload, "bar");
        assert_eq!(bso.sortindex, Some(2));
        Ok(())
    })
}

#[async_test]
async fn post_bsos() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "NewCollection";
    let result = db
        .post_bsos(params::PostBsos {
            user_id: hid(uid),
            collection: coll.to_owned(),
            bsos: vec![
                postbso("b0", Some("payload 0"), Some(10), None),
                postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
                postbso("b2", Some("payload 2"), Some(100), None),
            ],
            failed: Default::default(),
        })
        .compat()
        .await?;

    assert!(result.success.contains(&"b0".to_owned()));
    assert!(result.success.contains(&"b2".to_owned()));
    // XXX: validation?
    //assert!(!result.success.contains(&"b1".to_owned()));
    //assert!(!result.failed.contains_key("b1"));
    //assert!(!result.failed.contains_key("b1"));

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .compat()
        .await?;
    // XXX: casts
    assert_eq!(result.modified, ts);

    let result2 = db
        .post_bsos(params::PostBsos {
            user_id: hid(uid),
            collection: coll.to_owned(),
            bsos: vec![
                postbso("b0", Some("updated 0"), Some(11), Some(100_000)),
                postbso("b2", Some("updated 2"), Some(22), Some(10000)),
            ],
            failed: Default::default(),
        })
        .compat()
        .await?;

    assert_eq!(result2.success.len(), 2);
    assert_eq!(result2.failed.len(), 0);
    assert!(result2.success.contains(&"b0".to_owned()));
    assert!(result2.success.contains(&"b2".to_owned()));

    let bso = db.get_bso(gbso(uid, coll, "b0")).compat().await?.unwrap();
    assert_eq!(bso.sortindex, Some(11));
    assert_eq!(bso.payload, "updated 0");
    let bso = db.get_bso(gbso(uid, coll, "b2")).compat().await?.unwrap();
    assert_eq!(bso.sortindex, Some(22));
    assert_eq!(bso.payload, "updated 2");

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .compat()
        .await?;
    assert_eq!(result2.modified, ts);
    Ok(())
}

#[async_test]
async fn get_bso() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    let payload = "a";
    db.put_bso(pbso(uid, coll, bid, Some(payload), None, None))
        .compat()
        .await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?.unwrap();
    assert_eq!(bso.id, bid);
    assert_eq!(bso.payload, payload);

    let result = db.get_bso(gbso(uid, coll, "nope")).compat().await?;
    assert!(result.is_none());
    Ok(())
}

#[async_test]
async fn get_bsos() -> Result<()> {
    let db = db().await?;

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
        with_delta!(&db, i as i64 * 10, { db.put_bso(bso).compat().await })?;
    }

    let ids = db
        .get_bso_ids(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(ids.items, vec!["b0", "b1", "b2", "b3", "b4"]);

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &["b0", "b2", "b4"],
            MAX_TIMESTAMP,
            0,
            Sorting::Newest,
            10,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b0");
    assert_eq!(bsos.items[1].id, "b2");
    assert_eq!(bsos.items[2].id, "b4");

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Index,
            2,
            0,
        ))
        .compat()
        .await?;
    assert_eq!(bsos.items.len(), 2);
    assert_eq!(bsos.offset, Some(2));
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b1");
    Ok(())
}

#[async_test]
async fn get_bso_timestamp() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    let bso = pbso(uid, coll, bid, Some("a"), None, None);
    db.put_bso(bso).compat().await?;
    let ts = db
        .get_bso_timestamp(params::GetBsoTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
            id: bid.to_string(),
        })
        .compat()
        .await?;
    assert_eq!(ts, db.timestamp());
    Ok(())
}

#[async_test]
async fn delete_bso() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bid = "b0";
    db.put_bso(pbso(uid, coll, bid, Some("a"), None, None))
        .compat()
        .await?;
    db.delete_bso(dbso(uid, coll, bid)).compat().await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).compat().await?;
    assert!(bso.is_none());
    Ok(())
}

#[async_test]
async fn delete_bsos() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bids = (0..=2).map(|i| format!("b{}", i));
    for bid in bids.clone() {
        db.put_bso(pbso(
            uid,
            coll,
            &bid,
            Some("payload"),
            Some(10),
            Some(DEFAULT_BSO_TTL),
        ))
        .compat()
        .await?;
    }
    db.delete_bso(dbso(uid, coll, "b0")).compat().await?;
    // deleting non existant bid errors
    assert!(db
        .delete_bso(dbso(uid, coll, "bxi0"))
        .compat()
        .await
        .unwrap_err()
        .is_bso_not_found());
    db.delete_bsos(dbsos(uid, coll, &["b1", "b2"]))
        .compat()
        .await?;
    for bid in bids {
        let bso = db.get_bso(gbso(uid, coll, &bid)).compat().await?;
        assert!(bso.is_none());
    }
    Ok(())
}

/*
#[async_test]
async fn usage_stats() -> Result<()> {
    let db = db().await?;
    Ok(())
}

#[async_test]
async fn purge_expired() -> Result<()> {
    let db = db().await?;
    Ok(())
}

#[async_test]
async fn optimize() -> Result<()> {
    let db = db().await?;
    Ok(())
}
*/

#[async_test]
async fn delete_storage() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let bid = "test";
    let coll = "my_collection";
    let cid = db.create_collection(coll.to_owned()).compat().await?;
    db.put_bso(pbso(uid, coll, bid, Some("test"), None, None))
        .compat()
        .await?;

    db.delete_storage(hid(uid)).compat().await?;
    let result = db.get_bso(gbso(uid, coll, bid)).compat().await?;
    assert!(result.is_none());

    // collection data sticks around
    let cid2 = db
        .get_collection_id("my_collection".to_owned())
        .compat()
        .await?;
    assert_eq!(cid2, cid);
    Ok(())
}

#[async_test]
async fn lock_for_read() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    db.lock_for_read(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })
    .compat()
    .await?;
    let result = db
        .get_collection_id("NewCollection".to_owned())
        .compat()
        .await;
    assert!(result.unwrap_err().is_colllection_not_found());
    db.commit().compat().await?;
    Ok(())
}

#[async_test]
async fn lock_for_write() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    db.lock_for_write(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })
    .compat()
    .await?;
    db.put_bso(pbso(uid, coll, "1", Some("foo"), None, None))
        .compat()
        .await?;
    db.commit().compat().await?;
    Ok(())
}
