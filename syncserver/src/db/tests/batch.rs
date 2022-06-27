use log::debug;
use syncserver_db_common::{
    error::DbErrorIntrospect, params, results, util::SyncTimestamp, BATCH_LIFETIME,
};
use syncserver_settings::Settings;

use super::support::{db_pool, gbso, hid, pbso, postbso, test_db, Result};

fn cb(user_id: u32, coll: &str, bsos: Vec<params::PostCollectionBso>) -> params::CreateBatch {
    params::CreateBatch {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        bsos,
    }
}

fn vb(user_id: u32, coll: &str, id: String) -> params::ValidateBatch {
    params::ValidateBatch {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id,
    }
}

fn ab(
    user_id: u32,
    coll: &str,
    batch: results::CreateBatch,
    bsos: Vec<params::PostCollectionBso>,
) -> params::AppendToBatch {
    params::AppendToBatch {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        batch,
        bsos,
    }
}

fn gb(user_id: u32, coll: &str, id: String) -> params::GetBatch {
    params::GetBatch {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id,
    }
}

#[tokio::test]
async fn create_delete() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let new_batch = db.create_batch(cb(uid, coll, vec![])).await?;
    assert!(
        db.validate_batch(vb(uid, coll, new_batch.id.clone()))
            .await?
    );

    db.delete_batch(params::DeleteBatch {
        user_id: hid(uid),
        collection: coll.to_owned(),
        id: new_batch.id.clone(),
    })
    .await?;
    assert!(!db.validate_batch(vb(uid, coll, new_batch.id)).await?);
    Ok(())
}

#[tokio::test]
async fn expiry() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let new_batch = with_delta!(db, -(BATCH_LIFETIME + 11), {
        db.create_batch(cb(uid, coll, vec![])).await
    })?;
    assert!(
        !db.validate_batch(vb(uid, coll, new_batch.id.clone()))
            .await?
    );
    let result = db.get_batch(gb(uid, coll, new_batch.id.clone())).await?;
    assert!(result.is_none());

    let bsos = vec![postbso("b0", Some("payload 0"), Some(10), None)];
    let result = db.append_to_batch(ab(uid, coll, new_batch, bsos)).await;
    let is_batch_not_found = result.unwrap_err().is_batch_not_found();
    assert!(is_batch_not_found, "Expected BatchNotFound");
    Ok(())
}

#[tokio::test]
async fn update() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let new_batch = db.create_batch(cb(uid, coll, vec![])).await?;
    assert!(db
        .get_batch(gb(uid, coll, new_batch.id.clone()))
        .await?
        .is_some());

    let bsos = vec![
        postbso("b0", Some("payload 0"), Some(10), None),
        postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
    ];
    db.append_to_batch(ab(uid, coll, new_batch.clone(), bsos))
        .await?;

    assert!(db.get_batch(gb(uid, coll, new_batch.id)).await?.is_some());
    Ok(())
}

#[tokio::test]
async fn append_commit() -> Result<()> {
    let pool = db_pool(None).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let bsos1 = vec![
        postbso("b0", Some("payload 0"), Some(10), None),
        postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
    ];
    let new_batch = db.create_batch(cb(uid, coll, bsos1)).await?;

    let bsos2 = vec![postbso("b2", Some("payload 2"), None, Some(1000))];
    db.append_to_batch(ab(uid, coll, new_batch.clone(), bsos2))
        .await?;

    let batch = db.get_batch(gb(uid, coll, new_batch.id)).await?.unwrap();
    let modified = db
        .commit_batch(params::CommitBatch {
            user_id: hid(uid),
            collection: coll.to_owned(),
            batch,
        })
        .await?;

    assert!(db.get_bso(gbso(uid, coll, "b0")).await?.is_some());
    assert!(db.get_bso(gbso(uid, coll, "b2")).await?.is_some());

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: hid(uid),
            collection: coll.to_owned(),
        })
        .await?;
    assert_eq!(modified, ts);

    let bso = db.get_bso(gbso(uid, coll, "b1")).await?.unwrap();
    assert_eq!(bso.sortindex, Some(1_000_000_000));
    assert_eq!(bso.payload, "payload 1");
    Ok(())
}

#[tokio::test]
async fn quota_test_create_batch() -> Result<()> {
    let mut settings = Settings::test_settings().syncstorage;

    if !settings.enable_quota {
        debug!("[test] Skipping test");
        return Ok(());
    }

    let limit = 300;
    settings.limits.max_quota_limit = limit;

    let pool = db_pool(Some(settings.clone())).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let filler = (0..limit - 10).map(|_| "#").collect::<Vec<_>>().concat();

    // create too many records.
    let bsos1 = vec![postbso("b0", Some(filler.as_ref()), None, None)];
    let bsos2 = vec![postbso("b1", Some(filler.as_ref()), None, None)];

    let new_batch = db.create_batch(cb(uid, coll, bsos1)).await?;
    let batch = db.get_batch(gb(uid, coll, new_batch.id)).await?.unwrap();
    db.commit_batch(params::CommitBatch {
        user_id: hid(uid),
        collection: coll.to_owned(),
        batch,
    })
    .await?;

    let result = db.create_batch(cb(uid, coll, bsos2)).await;
    if settings.enforce_quota {
        assert!(result.is_err());
    } else {
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn quota_test_append_batch() -> Result<()> {
    let mut settings = Settings::test_settings().syncstorage;

    if !settings.enable_quota {
        debug!("[test] Skipping test");
        return Ok(());
    }

    let limit = 300;
    settings.limits.max_quota_limit = limit;

    let pool = db_pool(Some(settings.clone())).await?;
    let db = test_db(pool).await?;

    let uid = 1;
    let coll = "clients";
    let filler = (0..limit / 3).map(|_| "#").collect::<Vec<_>>().concat();

    // create too many records.
    let bsos1 = vec![postbso("b0", Some(filler.as_ref()), None, None)];
    let bsos2 = vec![postbso("b1", Some(filler.as_ref()), None, None)];
    let bsos3 = vec![postbso("b2", Some(filler.as_ref()), None, None)];

    let new_batch = db.create_batch(cb(uid, coll, bsos1)).await?;
    let batch = db
        .get_batch(gb(uid, coll, new_batch.id.clone()))
        .await?
        .unwrap();
    db.commit_batch(params::CommitBatch {
        user_id: hid(uid),
        collection: coll.to_owned(),
        batch,
    })
    .await?;
    let id2 = db.create_batch(cb(uid, coll, bsos2)).await?;
    let result = db.append_to_batch(ab(uid, coll, id2.clone(), bsos3)).await;
    if settings.enforce_quota {
        assert!(result.is_err())
    } else {
        assert!(result.is_ok())
    }
    Ok(())
}

#[tokio::test]
async fn test_append_async_w_null() -> Result<()> {
    let settings = Settings::test_settings().syncstorage;
    let pool = db_pool(Some(settings)).await?;
    let db = test_db(pool).await?;
    // Remember: TTL is seconds to live, not an expiry date
    let ttl_0 = 86_400;
    let ttl_1 = 86_400;
    let bid_0 = "b0";
    let bid_1 = "b1";

    let uid = 1;
    let coll = "clients";
    let payload_0 = "payload 0";
    let payload_1 = "payload 1";
    let first_bso = pbso(uid, coll, bid_0, Some(payload_0), Some(10), Some(ttl_0));
    db.put_bso(first_bso).await?;
    let second_bso = pbso(uid, coll, bid_1, Some(payload_1), Some(10), Some(ttl_1));
    db.put_bso(second_bso).await?;

    let tomorrow = ttl_0 + 86_400;
    let new_batch = db.create_batch(cb(uid, coll, vec![])).await?;
    // update the single bso twice, leaving payload the same.
    db.append_to_batch(ab(
        uid,
        coll,
        new_batch.clone(),
        vec![postbso(bid_0, None, Some(15), None)],
    ))
    .await?;
    db.append_to_batch(ab(
        uid,
        coll,
        new_batch.clone(),
        vec![postbso(bid_0, None, None, Some(tomorrow))],
    ))
    .await?;

    // update the second bso to ensure that the first is unaltered
    db.append_to_batch(ab(
        uid,
        coll,
        new_batch.clone(),
        vec![postbso(bid_1, None, Some(20), None)],
    ))
    .await?;

    let batch = db
        .get_batch(gb(uid, coll, new_batch.id.clone()))
        .await?
        .unwrap();
    db.commit_batch(params::CommitBatch {
        user_id: hid(uid),
        collection: coll.to_owned(),
        batch,
    })
    .await?;
    let bso_0 = db.get_bso(gbso(uid, coll, bid_0)).await?.unwrap();
    let bso_1 = db.get_bso(gbso(uid, coll, bid_1)).await?.unwrap();

    assert!(bso_0.payload == payload_0);
    assert!(bso_0.sortindex == Some(15));

    assert!(bso_1.payload == payload_1);
    assert!(bso_1.sortindex == Some(20));

    Ok(())
}
