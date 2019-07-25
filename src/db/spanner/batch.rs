use std::collections::HashMap;

use super::models::{Result, SpannerDb};

use crate::db::{params, results, util::SyncTimestamp, DbError, DbErrorKind};

use google_spanner1::ExecuteSqlRequest;

/// Rough guesstimate of the maximum reasonable life span of a batch.
pub const BATCH_LIFETIME: i64 = 2 * 60 * 60 * 1000; // 2 hours, in milliseconds

/// Serialize bsos into strings separated by newlines
fn bsos_to_batch_string(bsos: &[params::PostCollectionBso]) -> Result<String> {
    let batch_strings: Result<Vec<String>> = bsos
        .iter()
        .map(|bso| {
            serde_json::to_string(bso).map_err(|e| {
                DbError::internal(&format!("Couldn't serialize batch::create bso: {}", e))
            })
        })
        .collect();
    batch_strings.map(|bs| {
        format!(
            "{}{}",
            bs.join("\n"),
            if bsos.is_empty() { "" } else { "\n" }
        )
    })
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
    let timestamp = db.timestamp().as_i64();
    let bsos = bsos_to_batch_string(&params.bsos)?;

    let spanner = &db.conn;
    let session = spanner.session.name.as_ref().unwrap();
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("INSERT INTO batches (user_id, collection_id, id, bsos, expiry) VALUES (@userid, @collectionid, @bsosid, @bsos, @expiry)".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsosid".to_string(), timestamp.to_string());
    sqlparams.insert("bsos".to_string(), bsos);
    sqlparams.insert(
        "expiry".to_string(),
        (timestamp + BATCH_LIFETIME).to_string(),
    );
    sql.params = Some(sqlparams);

    let results = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    match results {
        Ok(results) => Ok(timestamp),
        // TODO Return the correct error
        Err(e) => Err(DbErrorKind::CollectionNotFound.into()),
    }
}

pub fn validate(db: &SpannerDb, params: params::ValidateBatch) -> Result<bool> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp().as_i64();

    let spanner = &db.conn;
    let session = spanner.session.name.as_ref().unwrap();
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("SELECT 1 FROM batches WHERE user_id = @userid AND collection_id = @collectionid AND id = @bsoid AND expiry > @expiry".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsoid".to_string(), params.id.to_string());
    sqlparams.insert("expiry".to_string(), timestamp.to_string());
    sql.params = Some(sqlparams);

    let results = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    match results {
        Ok(results) => Ok(results.1.rows.is_some()),
        // TODO Return the correct error
        Err(e) => Err(DbErrorKind::CollectionNotFound.into()),
    }
}

pub fn append(db: &SpannerDb, params: params::AppendToBatch) -> Result<()> {
    // TODO
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let bsos = bsos_to_batch_string(&params.bsos)?;
    let timestamp = db.timestamp().as_i64();

    let spanner = &db.conn;
    let session = spanner.session.name.as_ref().unwrap();
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("SELECT 1 FROM batches WHERE user_id = @userid AND collection_id = @collectionid AND id = @bsoid AND expiry > @expiry".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsoid".to_string(), params.id.to_string());
    sqlparams.insert("expiry".to_string(), timestamp.to_string());
    sql.params = Some(sqlparams);

    let results = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    if let Ok(results) = results {
        if results.1.rows.is_some() {
            return Err(DbErrorKind::BatchNotFound.into());
        }
    }
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("UPDATE batches SET bsos = @bsos WHERE user_id = @userid AND collection_id = @collectionid AND id = @bsoid AND expiry > @expiry".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsoid".to_string(), params.id.to_string());
    sqlparams.insert("expiry".to_string(), timestamp.to_string());
    sqlparams.insert("bsos".to_string(), bsos);
    sql.params = Some(sqlparams);

    let results = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    match results {
        Ok(results) => Ok(()),
        // TODO Return the correct error
        Err(e) => Err(DbErrorKind::CollectionNotFound.into()),
    }
}

pub fn get(db: &SpannerDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp().as_i64();

    let spanner = &db.conn;
    let session = spanner.session.name.as_ref().unwrap();
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("SELECT (id, bsos, expiry) FROM batches WHERE user_id = @userid AND collection_id = @collectionid AND id = @bsoid AND expiry > @expiry".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsoid".to_string(), params.id.to_string());
    sqlparams.insert("expiry".to_string(), timestamp.to_string());
    sql.params = Some(sqlparams);

    let result = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    match result {
        Ok(result) => match result.1.rows {
            Some(rows) => Ok(Some(params::Batch {
                id: params.id,
                bsos: rows[0][1].clone(),
                expiry: rows[0][2].parse().unwrap(),
            })),
            None => Ok(None),
        },
        // TODO Return the correct error
        Err(e) => Err(DbErrorKind::CollectionNotFound.into()),
    }
}

fn delete(db: &SpannerDb, params: params::DeleteBatch) -> Result<()> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;

    let spanner = &db.conn;
    let session = spanner.session.name.as_ref().unwrap();
    let mut sql = ExecuteSqlRequest::default();
    sql.sql = Some("DELETE FROM batches WHERE user_id = @userid AND collection_id = @collectionid AND id = @bsoid".to_string());
    let mut sqlparams = HashMap::new();
    sqlparams.insert("userid".to_string(), user_id.to_string());
    sqlparams.insert("collectionid".to_string(), collection_id.to_string());
    sqlparams.insert("bsoid".to_string(), params.id.to_string());
    sql.params = Some(sqlparams);

    let result = spanner
        .hub
        .projects()
        .instances_databases_sessions_execute_sql(sql, session)
        .doit();
    match result {
        Ok(_result) => Ok(()),
        // TODO Return the correct error
        Err(e) => Err(DbErrorKind::CollectionNotFound.into()),
    }
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
