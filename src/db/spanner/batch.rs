use std::str::FromStr;

use googleapis_raw::spanner::v1::type_pb::TypeCode;
use uuid::Uuid;

use super::support::null_value;
use super::{
    models::{Result, SpannerDb, DEFAULT_BSO_TTL, PRETOUCH_TS},
    support::as_value,
};
use crate::{
    db::{params, results, util::to_rfc3339, DbError, DbErrorKind, BATCH_LIFETIME},
    web::extractors::HawkIdentifier,
};

pub fn create(db: &SpannerDb, params: params::CreateBatch) -> Result<results::CreateBatch> {
    let batch_id = Uuid::new_v4().to_simple().to_string();
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp()?.as_i64();

    // Ensure a parent record exists in user_collections before writing to batches
    // (INTERLEAVE IN PARENT user_collections)
    pretouch_collection(db, &params.user_id, collection_id)?;

    db.sql(
        "INSERT INTO batches (fxa_uid, fxa_kid, collection_id, batch_id, expiry)
         VALUES (@fxa_uid, @fxa_kid, @collection_id, @batch_id, @expiry)",
    )?
    .params(params! {
        "fxa_uid" => params.user_id.fxa_uid.clone(),
        "fxa_kid" => params.user_id.fxa_kid.clone(),
        "collection_id" => collection_id.to_string(),
        "batch_id" => batch_id.clone(),
        "expiry" => to_rfc3339(timestamp + BATCH_LIFETIME)?,
    })
    .param_types(param_types! {
        "expiry" => TypeCode::TIMESTAMP,
    })
    .execute(&db.conn)?;

    do_append(
        db,
        params.user_id,
        collection_id,
        batch_id.clone(),
        params.bsos,
    )?;
    Ok(batch_id)
}

pub fn validate(db: &SpannerDb, params: params::ValidateBatch) -> Result<bool> {
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = db
        .sql(
            "SELECT 1
               FROM batches
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND batch_id = @batch_id
                AND expiry > CURRENT_TIMESTAMP()",
        )?
        .params(params! {
            "fxa_uid" => params.user_id.fxa_uid,
            "fxa_kid" => params.user_id.fxa_kid,
            "collection_id" => collection_id.to_string(),
            "batch_id" => params.id,
        })
        .execute(&db.conn)?
        .one_or_none()?;
    Ok(exists.is_some())
}

pub fn append(db: &SpannerDb, params: params::AppendToBatch) -> Result<()> {
    let mut timer = db.metrics.clone();
    timer.start_timer("syncstorage.storage.spanner.append_items_to_batch", None);

    let exists = validate(
        db,
        params::ValidateBatch {
            user_id: params.user_id.clone(),
            collection: params.collection.clone(),
            id: params.id.clone(),
        },
    )?;
    if !exists {
        // NOTE: db_tests expects this but it doesn't seem necessary w/ the
        // handler validating the batch before appends
        Err(DbErrorKind::BatchNotFound)?
    }

    let collection_id = db.get_collection_id(&params.collection)?;
    do_append(db, params.user_id, collection_id, params.id, params.bsos)?;
    Ok(())
}

pub fn get(db: &SpannerDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let collection_id = db.get_collection_id(&params.collection)?;
    let batch = db
        .sql(
            "SELECT 1
               FROM batches
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND batch_id = @batch_id
                AND expiry > CURRENT_TIMESTAMP()",
        )?
        .params(params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "batch_id" => params.id.clone(),
        })
        .execute(&db.conn)?
        .one_or_none()?
        .map(move |_| {
            params::Batch {
                id: params.id,
                // XXX: we don't use bsos/expiry (but they're currently needed
                // for mysql/diesel compat). converting expiry back to i64 is
                // maybe suspicious
                bsos: "".to_owned(),
                expiry: 0,
            }
        });
    Ok(batch)
}

pub fn delete(db: &SpannerDb, params: params::DeleteBatch) -> Result<()> {
    let collection_id = db.get_collection_id(&params.collection)?;
    // Also deletes child batch_bsos rows (INTERLEAVE IN PARENT batches ON
    // DELETE CASCADE)
    db.sql(
        "DELETE FROM batches
          WHERE fxa_uid = @fxa_uid
            AND fxa_kid = @fxa_kid
            AND collection_id = @collection_id
            AND batch_id = @batch_id",
    )?
    .params(params! {
        "fxa_uid" => params.user_id.fxa_uid.clone(),
        "fxa_kid" => params.user_id.fxa_kid.clone(),
        "collection_id" => collection_id.to_string(),
        "batch_id" => params.id,
    })
    .execute(&db.conn)?;
    Ok(())
}

pub fn commit(db: &SpannerDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    let mut timer = db.metrics.clone();
    timer.start_timer("syncstorage.storage.spanner.apply_batch", None);
    let collection_id = db.get_collection_id(&params.collection)?;

    // Ensure a parent record exists in user_collections before writing to bsos
    // (INTERLEAVE IN PARENT user_collections)
    let timestamp = db.touch_collection(&params.user_id, collection_id)?;

    let as_rfc3339 = timestamp.as_rfc3339()?;
    // First, UPDATE existing rows in the bsos table with any new values
    // supplied in this batch
    db.sql(include_str!("batch_commit_update.sql"))?
        .params(params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "batch_id" => params.batch.id.clone(),
            "timestamp" => as_rfc3339.clone(),
        })
        .param_types(param_types! {
            "timestamp" => TypeCode::TIMESTAMP,
        })
        .execute(&db.conn)?;

    // Then INSERT INTO SELECT remaining rows from this batch into the bsos
    // table (that didn't already exist there)
    db.sql(include_str!("batch_commit_insert.sql"))?
        .params(params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "batch_id" => params.batch.id.clone(),
            "timestamp" => as_rfc3339,
            "default_bso_ttl" => DEFAULT_BSO_TTL.to_string(),
        })
        .param_types(param_types! {
            "timestamp" => TypeCode::TIMESTAMP,
            "default_bso_ttl" => TypeCode::INT64,
        })
        .execute(&db.conn)?;

    delete(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )?;
    // XXX: returning results::PostBsos here isn't needed
    Ok(results::PostBsos {
        modified: timestamp,
        success: Default::default(),
        failed: Default::default(),
    })
}

pub fn do_append(
    db: &SpannerDb,
    user_id: HawkIdentifier,
    collection_id: i32,
    batch_id: String,
    bsos: Vec<params::PostCollectionBso>,
) -> Result<()> {
    for bso in bsos {
        let mut sqlparams = params! {
            "fxa_uid" => user_id.fxa_uid.clone(),
            "fxa_kid" => user_id.fxa_kid.clone(),
            "collection_id" => collection_id.to_string(),
            "batch_id" => batch_id.clone(),
            "batch_bso_id" => bso.id,
        };

        sqlparams.insert(
            "sortindex".to_string(),
            bso.sortindex
                .map(|sortindex| as_value(sortindex.to_string()))
                .unwrap_or_else(null_value),
        );
        sqlparams.insert(
            "payload".to_string(),
            bso.payload.map(as_value).unwrap_or_else(null_value),
        );
        sqlparams.insert(
            "ttl".to_string(),
            bso.ttl
                .map(|ttl| as_value(ttl.to_string()))
                .unwrap_or_else(null_value),
        );

        db.sql(
            "INSERT INTO batch_bsos (fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id,
                                     sortindex, payload, ttl)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @batch_id, @batch_bso_id,
                     @sortindex, @payload, @ttl)",
        )?
        .params(sqlparams)
        .execute(&db.conn)?;
    }
    Ok(())
}

/// Ensure a parent row exists in user_collections prior to creating a child
/// row in the batches table.
///
/// When no parent exists, a "tombstone" like ("pre birth stone"?) value for
/// modified is inserted, which is explicitly ignored by other queries.
///
/// For the special case of a user creating a batch for a collection with no
/// prior data.
fn pretouch_collection(db: &SpannerDb, user_id: &HawkIdentifier, collection_id: i32) -> Result<()> {
    let mut sqlparams = params! {
        "fxa_uid" => user_id.fxa_uid.clone(),
        "fxa_kid" => user_id.fxa_kid.clone(),
        "collection_id" => collection_id.to_string(),
    };
    let result = db
        .sql(
            "SELECT 1
               FROM user_collections
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id",
        )?
        .params(sqlparams.clone())
        .execute(&db.conn)?
        .one_or_none()?;
    if result.is_none() {
        sqlparams.insert("modified".to_owned(), as_value(PRETOUCH_TS.to_owned()));
        db.sql(
            "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)",
        )?
        .params(sqlparams)
        .param_types(param_types! {
            "modified" => TypeCode::TIMESTAMP,
        })
        .execute(&db.conn)?;
    }
    Ok(())
}

pub fn validate_batch_id(id: &str) -> Result<()> {
    Uuid::from_str(id)
        .map(|_| ())
        .map_err(|e| DbError::internal(&format!("Invalid batch_id: {}", e)))
}
