use futures_await_test::async_test;
use log::debug;

use super::support::{db, gbso, hid, postbso, Result};
use crate::{
    db::{error::DbErrorKind, params, util::SyncTimestamp, BATCH_LIFETIME},
    error::ApiErrorKind,
};

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
    id: String,
    bsos: Vec<params::PostCollectionBso>,
) -> params::AppendToBatch {
    params::AppendToBatch {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id,
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

#[async_test]
async fn create_delete() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let id = db.create_batch(cb(uid, coll, vec![])).await?;
    assert!(db.validate_batch(vb(uid, coll, id.clone())).await?);

    db.delete_batch(params::DeleteBatch {
        user_id: hid(uid),
        collection: coll.to_owned(),
        id: id.clone(),
    })
    .await?;
    assert!(!db.validate_batch(vb(uid, coll, id)).await?);
    Ok(())
}

#[async_test]
async fn expiry() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let id = with_delta!(db, -(BATCH_LIFETIME + 11), {
        db.create_batch(cb(uid, coll, vec![])).await
    })?;
    assert!(!db.validate_batch(vb(uid, coll, id.clone())).await?);
    let result = db.get_batch(gb(uid, coll, id.clone())).await?;
    assert!(result.is_none());

    let bsos = vec![postbso("b0", Some("payload 0"), Some(10), None)];
    let result = db.append_to_batch(ab(uid, coll, id, bsos)).await;
    let is_batch_not_found = match result.unwrap_err().kind() {
        ApiErrorKind::Db(dbe) => match dbe.kind() {
            DbErrorKind::BatchNotFound => true,
            _ => false,
        },
        _ => false,
    };
    assert!(is_batch_not_found, "Expected BatchNotFound");
    Ok(())
}

#[async_test]
async fn update() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let id = db.create_batch(cb(uid, coll, vec![])).await?;
    assert!(db.get_batch(gb(uid, coll, id.clone())).await?.is_some());
    // XXX: now bogus under spanner
    //assert_eq!(batch.bsos, "".to_owned());

    let bsos = vec![
        postbso("b0", Some("payload 0"), Some(10), None),
        postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
    ];
    db.append_to_batch(ab(uid, coll, id.clone(), bsos)).await?;

    assert!(db.get_batch(gb(uid, coll, id)).await?.is_some());
    // XXX: now bogus under spanner
    //assert_ne!(batch.bsos, "".to_owned());
    Ok(())
}

#[async_test]
async fn append_commit() -> Result<()> {
    let db = db().await?;

    let uid = 1;
    let coll = "clients";
    let bsos1 = vec![
        postbso("b0", Some("payload 0"), Some(10), None),
        postbso("b1", Some("payload 1"), Some(1_000_000_000), None),
    ];
    let id = db.create_batch(cb(uid, coll, bsos1)).await?;

    let bsos2 = vec![postbso("b2", Some("payload 2"), None, Some(1000))];
    db.append_to_batch(ab(uid, coll, id.clone(), bsos2)).await?;

    let batch = db.get_batch(gb(uid, coll, id)).await?.unwrap();
    let result = db
        .commit_batch(params::CommitBatch {
            user_id: hid(uid),
            collection: coll.to_owned(),
            batch,
        })
        .await?;

    debug!("result: {:?}", &result);
    assert!(db.get_bso(gbso(uid, coll, "b0")).await?.is_some());
    assert!(db.get_bso(gbso(uid, coll, "b2")).await?.is_some());

    let ts = db
        .get_collection_timestamp(params::GetCollectionTimestamp {
            user_id: hid(uid),
            collection: coll.to_owned(),
        })
        .await?;
    assert_eq!(result.modified, ts);

    let bso = db.get_bso(gbso(uid, coll, "b1")).await?.unwrap();
    assert_eq!(bso.sortindex, Some(1_000_000_000));
    assert_eq!(bso.payload, "payload 1");
    Ok(())
}
