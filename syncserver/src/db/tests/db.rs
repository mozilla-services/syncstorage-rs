#![allow(clippy::cognitive_complexity)]
use std::collections::HashMap;

use lazy_static::lazy_static;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use syncserver_settings::Settings;
use syncstorage_db_common::{
    params, util::SyncTimestamp, Sorting, UserIdentifier, DEFAULT_BSO_TTL,
};

use super::support::{db_pool, dbso, dbsos, gbso, gbsos, hid, pbso, postbso, test_db, Result};

// distant future (year 2099) timestamp for tests
const MAX_TIMESTAMP: u64 = 4_070_937_600_000;

lazy_static! {
    static ref UID: u32 = thread_rng().gen_range(0..10000);
}

#[tokio::test]
async fn bso_successfully_updates_single_values() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
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
    db.put_bso(bso1).await?;

    let payload = "Updated payload";
    let bso2 = pbso(uid, coll, bid, Some(payload), None, None);
    db.put_bso(bso2).await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));

    let sortindex = 2;
    let bso2 = pbso(uid, coll, bid, None, Some(sortindex), None);
    db.put_bso(bso2).await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    assert_eq!(bso.modified, db.timestamp());
    assert_eq!(bso.payload, payload);
    assert_eq!(bso.sortindex, Some(sortindex));
    assert_eq!(bso.expiry, db.timestamp().as_i64() + i64::from(ttl * 1000));
    Ok(())
}

#[tokio::test]
async fn bso_modified_not_changed_on_ttl_touch() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let bid = "testBSO";
    let timestamp = db.timestamp().as_i64();

    let bso1 = pbso(uid, coll, bid, Some("hello"), Some(1), Some(10));
    with_delta!(db, -100, { db.put_bso(bso1).await })?;

    let bso2 = pbso(uid, coll, bid, None, None, Some(15));
    db.put_bso(bso2).await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    // ttl has changed
    assert_eq!(bso.expiry, timestamp + (15 * 1000));
    // modified has not changed
    assert_eq!(bso.modified.as_i64(), timestamp - 100);
    Ok(())
}

#[tokio::test]
async fn put_bso_updates() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let bid = "1";
    let bso1 = pbso(uid, coll, bid, Some("initial"), None, None);
    db.put_bso(bso1).await?;

    let payload = "Updated";
    let sortindex = 200;
    let bso2 = pbso(uid, coll, bid, Some(payload), Some(sortindex), None);
    db.put_bso(bso2).await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    assert_eq!(Some(bso.payload), Some(payload.to_owned()));
    assert_eq!(bso.sortindex, Some(sortindex));
    assert_eq!(bso.modified, db.timestamp());
    Ok(())
}

#[tokio::test]
async fn get_bsos_limit_offset() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
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
        with_delta!(&db, i64::from(i) * 10, { db.put_bso(bso).await })?;
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
            "0",
        ))
        .await?;
    assert!(bsos.items.is_empty());
    assert_eq!(bsos.offset, Some("0".to_string()));

    let bsos = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            0,
            Sorting::Index,
            -1,
            "0",
        ))
        .await?;
    assert_eq!(bsos.items.len(), size as usize);
    assert_eq!(bsos.offset, None);

    let newer = 0;
    let limit = 5;
    let offset = "0".to_owned();
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
            &offset,
        ))
        .await?;
    assert_eq!(bsos.items.len(), 5);
    if let Some(ref offset) = bsos.offset {
        if !offset.chars().any(|c| c == ':') {
            assert_eq!(offset, &"5".to_string());
        }
    }

    assert_eq!(bsos.items[0].id, "11");
    assert_eq!(bsos.items[4].id, "7");

    let bsos2 = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            newer,
            Sorting::Newest,
            limit,
            &bsos.offset.unwrap(),
        ))
        .await?;
    assert_eq!(bsos2.items.len(), 5);
    if let Some(ref offset) = bsos2.offset {
        if !offset.chars().any(|c| c == ':') {
            assert_eq!(offset, &"10".to_owned());
        }
    }
    assert_eq!(bsos2.items[0].id, "6");
    assert_eq!(bsos2.items[4].id, "2");

    let bsos3 = db
        .get_bsos(gbsos(
            uid,
            coll,
            &[],
            MAX_TIMESTAMP,
            newer,
            Sorting::Newest,
            limit,
            &bsos2.offset.unwrap(),
        ))
        .await?;
    assert_eq!(bsos3.items.len(), 2);
    assert_eq!(bsos3.offset, None);
    assert_eq!(bsos3.items[0].id, "1");
    assert_eq!(bsos3.items[1].id, "0");
    Ok(())
}

#[tokio::test]
async fn get_bsos_newer() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let timestamp = db.timestamp().as_i64();

    for i in (0..=2).rev() {
        let pbso = pbso(
            uid,
            coll,
            &format!("b{}", i),
            Some("a"),
            Some(1),
            Some(DEFAULT_BSO_TTL),
        );
        with_delta!(&db, -i * 10, { db.put_bso(pbso).await })?;
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
            "0",
        ))
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
            "0",
        ))
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
            "0",
        ))
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
            "0",
        ))
        .await?;
    assert_eq!(bsos.items.len(), 0);
    Ok(())
}

#[tokio::test]
async fn get_bsos_sort() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";

    for (revi, sortindex) in [1, 0, 2].iter().enumerate().rev() {
        let pbso = pbso(
            uid,
            coll,
            &format!("b{}", revi),
            Some("a"),
            Some(*sortindex),
            Some(DEFAULT_BSO_TTL),
        );
        with_delta!(&db, -(revi as i64) * 10, { db.put_bso(pbso).await })?;
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
            "0",
        ))
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
            "0",
        ))
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
            "0",
        ))
        .await?;
    assert_eq!(bsos.items.len(), 3);
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b0");
    assert_eq!(bsos.items[2].id, "b1");
    Ok(())
}

#[tokio::test]
async fn delete_bsos_in_correct_collection() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let payload = "data";
    db.put_bso(pbso(uid, "clients", "b1", Some(payload), None, None))
        .await?;
    db.put_bso(pbso(uid, "crypto", "b1", Some(payload), None, None))
        .await?;
    db.delete_bsos(dbsos(uid, "clients", &["b1"])).await?;
    let bso = db.get_bso(gbso(uid, "crypto", "b1")).await?;
    assert!(bso.is_some());
    Ok(())
}

#[tokio::test]
async fn get_storage_timestamp() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    db.create_collection("NewCollection1".to_owned()).await?;
    let col2 = db.create_collection("NewCollection2".to_owned()).await?;
    db.create_collection("NewCollection3".to_owned()).await?;

    with_delta!(&db, 100_000, {
        db.update_collection(params::UpdateCollection {
            user_id: hid(uid),
            collection_id: col2,
            collection: "NewCollection2".to_owned(),
        })
        .await?;
        let m = db.get_storage_timestamp(hid(uid)).await?;
        assert_eq!(m, db.timestamp());
        Ok(())
    })
}

#[tokio::test]
async fn get_collection_id() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;
    db.get_collection_id("bookmarks".to_owned()).await?;
    Ok(())
}

#[tokio::test]
async fn create_collection() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let name = "NewCollection";
    let cid = db.create_collection(name.to_owned()).await?;
    assert_ne!(cid, 0);
    let cid2 = db.get_collection_id(name.to_owned()).await?;
    assert_eq!(cid2, cid);
    Ok(())
}

#[tokio::test]
async fn update_collection() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;
    let collection = "test".to_owned();

    let cid = db.create_collection(collection.clone()).await?;
    db.update_collection(params::UpdateCollection {
        user_id: hid(1),
        collection_id: cid,
        collection,
    })
    .await?;
    Ok(())
}

#[tokio::test]
async fn delete_collection() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "NewCollection";
    for bid in 1u8..=3 {
        db.put_bso(pbso(uid, coll, &bid.to_string(), Some("test"), None, None))
            .await?;
    }
    let ts = db
        .delete_collection(params::DeleteCollection {
            user_id: hid(uid),
            collection: coll.to_owned(),
        })
        .await?;
    let ts2 = db.get_storage_timestamp(hid(uid)).await?;
    assert_eq!(ts2, ts);

    // make sure BSOs are deleted
    for bid in 1u8..=3 {
        let result = db.get_bso(gbso(uid, coll, &bid.to_string())).await?;
        assert!(result.is_none());
    }

    let result = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .await;
    assert!(result.unwrap_err().is_collection_not_found());
    Ok(())
}

#[tokio::test]
async fn delete_collection_tombstone() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "test";
    let bid1 = "b0";
    let coll2 = "test2";
    let ts1 = with_delta!(db, -100, {
        db.put_bso(pbso(uid, coll, bid1, Some("test"), None, None))
            .await?;
        for bid2 in 1u8..=3 {
            db.put_bso(pbso(
                uid,
                "test2",
                &bid2.to_string(),
                Some("test"),
                None,
                None,
            ))
            .await?;
        }
        db.timestamp()
    });

    let ts2 = db
        .delete_collection(params::DeleteCollection {
            user_id: hid(uid),
            collection: coll2.to_owned(),
        })
        .await?;
    assert!(ts2 > ts1);
    /*
    // TODO: fix mysql returning CollectionNotFound here

    // nothing deleted, storage's timestamp not touched
    let ts3 = db
        .delete_collection(params::DeleteCollection {
            user_id: hid(uid),
            collection: coll2.to_owned(),
        })
        .await?;
    assert_eq!(ts2, ts3);
    */

    let ts_storage = db.get_storage_timestamp(hid(uid)).await?;
    assert_eq!(ts2, ts_storage);

    // make sure coll2 BSOs were deleted
    for bid2 in 1u8..=3 {
        let result = db.get_bso(gbso(uid, coll2, &bid2.to_string())).await?;
        assert!(result.is_none());
    }
    // make sure coll BSOs were *not* deleted
    let result = db.get_bso(gbso(uid, coll, bid1)).await?;
    assert!(result.is_some());
    Ok(())
}

#[tokio::test]
async fn get_collection_timestamps() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "test".to_owned();
    let cid = db.create_collection(coll.clone()).await?;
    db.update_collection(params::UpdateCollection {
        user_id: hid(uid),
        collection_id: cid,
        collection: coll.clone(),
    })
    .await?;
    let cols = db.get_collection_timestamps(hid(uid)).await?;
    assert!(cols.contains_key(&coll));
    assert_eq!(cols.get(&coll), Some(&db.timestamp()));

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.clone(),
        })
        .await?;
    assert_eq!(Some(&ts), cols.get(&coll));
    Ok(())
}

#[tokio::test]
async fn get_collection_timestamps_tombstone() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "test".to_owned();
    let cid = db.create_collection(coll.clone()).await?;
    db.update_collection(params::UpdateCollection {
        user_id: hid(uid),
        collection_id: cid,
        collection: coll.clone(),
    })
    .await?;

    db.delete_collection(params::DeleteCollection {
        user_id: hid(uid),
        collection: coll,
    })
    .await?;
    let cols = db.get_collection_timestamps(hid(uid)).await?;
    assert!(cols.is_empty());
    Ok(())
}

#[tokio::test]
async fn get_collection_usage() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = 5;
    let mut expected = HashMap::new();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        for i in 0..5 {
            let size = 50 + thread_rng().gen_range(0..100);
            let payload = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(size)
                .collect::<Vec<u8>>();
            db.put_bso(pbso(
                uid,
                coll,
                &format!("b{}", i as i32),
                Some(&String::from_utf8_lossy(&payload)),
                None,
                None,
            ))
            .await?;
            *expected.entry(coll.to_owned()).or_insert(0) += size as i64;
        }
    }

    let sizes = db.get_collection_usage(hid(uid)).await?;
    assert_eq!(sizes, expected);
    let sum = expected.values().sum::<i64>();
    let total = db.get_storage_usage(hid(uid)).await?;
    assert_eq!(total, sum as u64);
    let settings = Settings::test_settings();
    if settings.syncstorage.enable_quota {
        let collection_id = db.get_collection_id("bookmarks".to_owned()).await?;
        let quota = db
            .get_quota_usage(params::GetQuotaUsage {
                user_id: UserIdentifier::new_legacy(uid as u64),
                collection: "ignored".to_owned(),
                collection_id,
            })
            .await?;
        assert_eq!(
            &(quota.total_bytes as i64),
            expected.get("bookmarks").unwrap()
        );
        assert_eq!(quota.count, 5); // 3 collections, 5 records
    }
    Ok(())
}

#[tokio::test]
async fn test_quota() -> Result<()> {
    let settings = Settings::test_settings();

    if !settings.syncstorage.enable_quota {
        debug!("[test] Skipping test");
        return Ok(());
    }

    let pool = db_pool(None).await?;
    let mut db = test_db(pool.as_ref()).await?;

    let uid = 5;
    let coll = "bookmarks";

    let size = 5000;
    let random = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .collect::<Vec<u8>>();
    let payload = String::from_utf8_lossy(&random);
    db.set_quota(false, 0, false);

    // These should work
    db.put_bso(pbso(uid, coll, "100", Some(&payload), None, None))
        .await?;
    db.put_bso(pbso(uid, coll, "101", Some(&payload), None, None))
        .await?;

    db.set_quota(true, size * 2, true);

    // Allow the put, but calculate the quota
    db.put_bso(pbso(uid, coll, "102", Some(&payload), None, None))
        .await?;
    // this should fail, since the user is already over the quota
    let result = db
        .put_bso(pbso(uid, coll, "103", Some(&payload), None, None))
        .await;
    assert!(result.is_err());
    Ok(())
}

#[tokio::test]
async fn get_collection_counts() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = 4;
    let mut expected = HashMap::new();
    let mut rng = thread_rng();

    for &coll in ["bookmarks", "history", "prefs"].iter() {
        let count = 5 + rng.gen_range(0..5);
        expected.insert(coll.to_owned(), count);
        for i in 0..count {
            db.put_bso(pbso(uid, coll, &format!("b{}", i), Some("x"), None, None))
                .await?;
        }
    }

    let counts = db.get_collection_counts(hid(uid)).await?;
    assert_eq!(counts, expected);
    Ok(())
}

#[tokio::test]
async fn put_bso() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "NewCollection";
    let bid = "b0";
    let bso1 = pbso(uid, coll, bid, Some("foo"), Some(1), Some(DEFAULT_BSO_TTL));
    db.put_bso(bso1).await?;
    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .await?;
    assert_eq!(ts, db.timestamp());

    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    assert_eq!(&bso.payload, "foo");
    assert_eq!(bso.sortindex, Some(1));

    let bso2 = pbso(uid, coll, bid, Some("bar"), Some(2), Some(DEFAULT_BSO_TTL));
    with_delta!(&db, 19, {
        db.put_bso(bso2).await?;
        let ts = db
            .get_collection_timestamp(params::GetCollectionTimestamp {
                user_id: uid.into(),
                collection: coll.to_string(),
            })
            .await?;
        assert_eq!(ts, db.timestamp());

        let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
        assert_eq!(&bso.payload, "bar");
        assert_eq!(bso.sortindex, Some(2));
        Ok(())
    })
}

#[tokio::test]
async fn post_bsos() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
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
            for_batch: false,
            failed: Default::default(),
        })
        .await?;

    assert!(result.success.contains(&"b0".to_owned()));
    assert!(result.success.contains(&"b2".to_owned()));
    // NOTE: b1 exceeds BSO_MAX_TTL but the database layer doesn't validate.
    // This is the extractor's responsibility
    //assert!(!result.success.contains(&"b1".to_owned()));

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
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
            for_batch: false,
            failed: Default::default(),
        })
        .await?;

    assert_eq!(result2.success.len(), 2);
    assert_eq!(result2.failed.len(), 0);
    assert!(result2.success.contains(&"b0".to_owned()));
    assert!(result2.success.contains(&"b2".to_owned()));

    let bso = db.get_bso(gbso(uid, coll, "b0")).await?.unwrap();
    assert_eq!(bso.sortindex, Some(11));
    assert_eq!(bso.payload, "updated 0");
    let bso = db.get_bso(gbso(uid, coll, "b2")).await?.unwrap();
    assert_eq!(bso.sortindex, Some(22));
    assert_eq!(bso.payload, "updated 2");

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
        })
        .await?;
    assert_eq!(result2.modified, ts);
    Ok(())
}

#[tokio::test]
async fn get_bso() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let bid = "b0";
    let payload = "a";
    db.put_bso(pbso(uid, coll, bid, Some(payload), None, None))
        .await?;

    let bso = db.get_bso(gbso(uid, coll, bid)).await?.unwrap();
    assert_eq!(bso.id, bid);
    assert_eq!(bso.payload, payload);

    let result = db.get_bso(gbso(uid, coll, "nope")).await?;
    assert!(result.is_none());
    Ok(())
}

#[tokio::test]
async fn get_bsos() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = 2;
    let coll = "clients";
    let sortindexes = vec![1, 3, 4, 2, 0];
    for (i, (revi, sortindex)) in sortindexes.iter().enumerate().rev().enumerate() {
        let bso = pbso(
            uid,
            coll,
            // XXX: to_string?
            &format!("b{}", revi),
            Some("Hello"),
            Some(*sortindex),
            None,
        );
        with_delta!(&db, i as i64 * 10, { db.put_bso(bso).await })?;
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
            "0",
        ))
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
            "0",
        ))
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
            "0",
        ))
        .await?;
    assert_eq!(bsos.items.len(), 2);
    assert_eq!(bsos.offset, Some("2".to_string()));
    assert_eq!(bsos.items[0].id, "b2");
    assert_eq!(bsos.items[1].id, "b1");
    Ok(())
}

#[tokio::test]
async fn get_bso_timestamp() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let bid = "b0";
    let bso = pbso(uid, coll, bid, Some("a"), None, None);
    db.put_bso(bso).await?;
    let ts = db
        .get_bso_timestamp(params::GetBsoTimestamp {
            user_id: uid.into(),
            collection: coll.to_string(),
            id: bid.to_string(),
        })
        .await?;
    assert_eq!(ts, db.timestamp());
    Ok(())
}

#[tokio::test]
async fn delete_bso() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    let bid = "b0";
    db.put_bso(pbso(uid, coll, bid, Some("a"), None, None))
        .await?;
    db.delete_bso(dbso(uid, coll, bid)).await?;
    let bso = db.get_bso(gbso(uid, coll, bid)).await?;
    assert!(bso.is_none());
    Ok(())
}

#[tokio::test]
async fn delete_bsos() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
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
        .await?;
    }
    db.delete_bso(dbso(uid, coll, "b0")).await?;
    // deleting non existant bid errors
    assert!(db
        .delete_bso(dbso(uid, coll, "bxi0"))
        .await
        .unwrap_err()
        .is_bso_not_found());
    db.delete_bsos(dbsos(uid, coll, &["b1", "b2"])).await?;
    for bid in bids {
        let bso = db.get_bso(gbso(uid, coll, &bid)).await?;
        assert!(bso.is_none());
    }
    Ok(())
}

/*
#[tokio::test]
async fn usage_stats() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;
    Ok(())
}

#[tokio::test]
async fn purge_expired() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;
    Ok(())
}

#[tokio::test]
async fn optimize() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;
    Ok(())
}
*/

#[tokio::test]
async fn delete_storage() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let bid = "test";
    let coll = "my_collection";
    let cid = db.create_collection(coll.to_owned()).await?;
    db.put_bso(pbso(uid, coll, bid, Some("test"), None, None))
        .await?;

    db.delete_storage(hid(uid)).await?;
    let result = db.get_bso(gbso(uid, coll, bid)).await?;
    assert!(result.is_none());

    // collection data sticks around
    let cid2 = db.get_collection_id("my_collection".to_owned()).await?;
    assert_eq!(cid2, cid);

    let collections = db.get_collection_counts(hid(uid)).await?;
    assert!(collections == HashMap::<String, i64>::new());

    Ok(())
}

#[tokio::test]
async fn collection_cache() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "test";
    let cid = db.create_collection(coll.to_owned()).await?;
    db.update_collection(params::UpdateCollection {
        user_id: hid(uid),
        collection_id: cid,
        collection: coll.to_owned(),
    })
    .await?;

    db.clear_coll_cache().await?;
    let cols = db.get_collection_timestamps(hid(uid)).await?;
    assert!(cols.contains_key(coll));
    Ok(())
}

#[tokio::test]
async fn lock_for_read() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    db.lock_for_read(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })
    .await?;
    let result = db.get_collection_id("NewCollection".to_owned()).await;
    assert!(result.unwrap_err().is_collection_not_found());
    db.commit().await?;
    Ok(())
}

#[tokio::test]
async fn lock_for_write() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    let uid = *UID;
    let coll = "clients";
    db.lock_for_write(params::LockCollection {
        user_id: hid(uid),
        collection: coll.to_owned(),
    })
    .await?;
    db.put_bso(pbso(uid, coll, "1", Some("foo"), None, None))
        .await?;
    db.commit().await?;
    Ok(())
}

#[tokio::test]
async fn heartbeat() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool.as_ref()).await?;

    assert!(db.check().await?);
    Ok(())
}
