use super::{
    models::{Result, SpannerDb},
    support::{as_value, SpannerType},
};
use crate::db::{params, results, util::to_rfc3339, DbError, DbErrorKind, BATCH_LIFETIME};
use protobuf::well_known_types::ListValue;
use serde_json::json;

/// Serialize results into strings separated by newlines
fn results_to_batch_string(results: Vec<ListValue>) -> String {
    if results.is_empty() {
        "".to_string()
    } else {
        let batch_strings: Vec<String> = results
            .iter()
            .map(|result| {
                result.get_values().to_vec()[1]
                    .get_string_value()
                    .to_string()
            })
            .filter(|result| !result.is_empty())
            .collect();
        batch_strings.join("\n")
    }
}

/// Deserialize a batch string into bsos
fn batch_string_to_bsos(bsos: &str) -> Result<Vec<params::PostCollectionBso>> {
    bsos.lines()
        .map(|line| {
            serde_json::from_str(line).map_err(|e| {
                DbError::internal(&format!("Couldn't deserialize batch::load_bsos bso: {}", e))
            })
        })
        .collect()
}

pub fn create(db: &SpannerDb, params: params::CreateBatch) -> Result<results::CreateBatch> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp()?.as_i64();
    if params.bsos.is_empty() {
        db.sql("INSERT INTO batches (userid, collection, id, bsos, expiry, timestamp) VALUES (@userid, @collectionid, @bsoid, @bsos, @expiry, @timestamp)")?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => to_rfc3339(timestamp)?,
                "timestamp" => to_rfc3339(timestamp)?,
                "bsos" => "".to_string(),
                "expiry" => to_rfc3339(timestamp + BATCH_LIFETIME)?,
            })
            .param_types(param_types! {
                "bsoid" => SpannerType::Timestamp,
                "expiry" => SpannerType::Timestamp,
                "timestamp" => SpannerType::Timestamp,
            })
            .execute(&db.conn)?;
    }
    for (i, bso) in (&params.bsos).iter().enumerate() {
        let bsos = json!({
            "id": bso.id,
            "sortindex": bso.sortindex,
            "payload": bso.payload,
            "ttl": bso.ttl,
        })
        .to_string();

        db.sql("INSERT INTO batches (userid, collection, id, bsos, expiry, timestamp) VALUES (@userid, @collectionid, @bsoid, @bsos, @expiry, @timestamp)")?
            .params(params! {
                "userid" => user_id.to_string(),
                "collectionid" => collection_id.to_string(),
                "bsoid" => to_rfc3339(timestamp + i as i64)?,
                "timestamp" => to_rfc3339(timestamp)?,
                "bsos" => bsos,
                "expiry" => to_rfc3339(timestamp + BATCH_LIFETIME)?,
            })
            .param_types(param_types! {
                "bsoid" => SpannerType::Timestamp,
                "expiry" => SpannerType::Timestamp,
                "timestamp" => SpannerType::Timestamp,
            })
            .execute(&db.conn)?;
    }

    Ok(timestamp)
}

pub fn validate(db: &SpannerDb, params: params::ValidateBatch) -> Result<bool> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = db.sql("SELECT expiry FROM batches WHERE userid = @userid AND collection = @collectionid AND timestamp = @timestamp AND expiry > @expiry")?
        .params(params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
            "timestamp" => to_rfc3339(params.id)?,
            "expiry" => to_rfc3339(db.timestamp()?.as_i64())?,
        })
        .param_types(param_types! {
            "timestamp" => SpannerType::Timestamp,
            "expiry" => SpannerType::Timestamp,
        })
        .execute(&db.conn)?
        .all_or_none();
    Ok(exists.is_some())
}

pub fn select_max_id(db: &SpannerDb, params: params::ValidateBatch) -> Result<i64> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = db.sql("SELECT UNIX_MILLIS(id) FROM batches WHERE userid = @userid AND collection = @collectionid AND timestamp = @timestamp AND expiry > @expiry ORDER BY id DESC")?
        .params(params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
            "timestamp" => to_rfc3339(params.id)?,
            "expiry" => to_rfc3339(db.timestamp()?.as_i64())?,
        })
        .param_types(param_types! {
            "timestamp" => SpannerType::Timestamp,
            "expiry" => SpannerType::Timestamp,
        })
        .execute(&db.conn)?
        .all_or_none();
    if let Some(exists) = exists {
        return Ok(exists[0].get_values().to_vec()[0]
            .get_string_value()
            .to_string()
            .parse::<i64>()
            .unwrap());
    }
    Err(DbError::internal("No rows matched the given query."))
}

pub fn append(db: &SpannerDb, params: params::AppendToBatch) -> Result<()> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = params.id;
    if let Ok(max_id) = select_max_id(
        db,
        params::ValidateBatch {
            id: timestamp,
            user_id: params.user_id.clone(),
            collection: params.collection.clone(),
        },
    ) {
        let mut i = max_id + 1;
        for bso in &params.bsos {
            let bsos = json!({
                "id": bso.id,
                "sortindex": bso.sortindex,
                "payload": bso.payload,
                "ttl": bso.ttl,
            })
            .to_string();
            db.sql("INSERT INTO batches (userid, collection, id, bsos, expiry, timestamp) VALUES (@userid, @collectionid, @bsoid, @bsos, @expiry, @timestamp)")?
                .params(params! {
                    "userid" => user_id.to_string(),
                    "collectionid" => collection_id.to_string(),
                    "bsoid" => to_rfc3339(params.id + i)?,
                    "timestamp" => to_rfc3339(params.id)?,
                    "expiry" => to_rfc3339(timestamp + BATCH_LIFETIME)?,
                    "bsos" => bsos,
                })
                .param_types(param_types! {
                    "bsoid" => SpannerType::Timestamp,
                    "timestamp" => SpannerType::Timestamp,
                    "expiry" => SpannerType::Timestamp,
                })
                .execute(&db.conn)?;
            i += 1;
        }
        Ok(())
    } else {
        Err(DbErrorKind::BatchNotFound.into())
    }
}

pub fn get(db: &SpannerDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp()?.as_i64();

    let result = db.sql("SELECT id, bsos, expiry FROM batches WHERE userid = @userid AND collection = @collectionid AND timestamp = @bsoid AND expiry > @expiry")?
        .params(params! {
            "userid" => user_id.to_string(),
            "collectionid" => collection_id.to_string(),
            "bsoid" => to_rfc3339(params.id)?,
            "expiry" => to_rfc3339(timestamp)?,
        })
        .param_types(param_types! {
            "bsoid" => SpannerType::Timestamp,
            "expiry" => SpannerType::Timestamp,
        })
        .execute(&db.conn)?.all_or_none();
    if let Some(result) = result {
        Ok(Some(params::Batch {
            id: params.id,
            bsos: results_to_batch_string(result),
            // XXX: we don't really use expiry (but it's probably needed for
            // mysql/diesel compat). converting it back to i64 is maybe
            // suspicious
            expiry: 0,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete(db: &SpannerDb, params: params::DeleteBatch) -> Result<()> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;

    db.sql(
        "DELETE FROM batches WHERE userid = @userid AND collection = @collectionid AND timestamp = @bsoid",
    )?
    .params(params! {
        "userid" => user_id.to_string(),
        "collectionid" => collection_id.to_string(),
        "bsoid" => to_rfc3339(params.id)?,
    })
    .param_types(param_types! {
        "bsoid" => SpannerType::Timestamp,
    })
    .execute(&db.conn)?;
    Ok(())
}

pub fn commit(db: &SpannerDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    let bsos = batch_string_to_bsos(&params.batch.bsos)?;
    let result = db.post_bsos_sync(params::PostBsos {
        user_id: params.user_id.clone(),
        collection: params.collection.clone(),
        bsos,
        failed: Default::default(),
    });
    delete(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )?;
    result
}
