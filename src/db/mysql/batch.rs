use diesel::{
    self,
    dsl::sql,
    insert_into,
    result::{DatabaseErrorKind::UniqueViolation, Error as DieselError},
    sql_types::Integer,
    update, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, TextExpressionMethods,
};
use serde_json;

use super::{
    models::{MysqlDb, Result},
    schema::batches,
};
use crate::db::{params, results, DbError, DbErrorKind, BATCH_LIFETIME};

pub fn create(db: &MysqlDb, params: params::CreateBatch) -> Result<results::CreateBatch> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp().as_i64();
    let bsos = bsos_to_batch_string(&params.bsos)?;
    insert_into(batches::table)
        .values((
            batches::user_id.eq(&user_id),
            batches::collection_id.eq(&collection_id),
            batches::id.eq(&timestamp),
            batches::bsos.eq(&bsos),
            batches::expiry.eq(timestamp + BATCH_LIFETIME),
        ))
        .execute(&db.conn)
        .map_err(|e| -> DbError {
            match e {
                // The user tried to create two batches with the same timestamp
                DieselError::DatabaseError(UniqueViolation, _) => DbErrorKind::Conflict.into(),
                _ => e.into(),
            }
        })?;
    Ok(encode_id(timestamp))
}

pub fn validate(db: &MysqlDb, params: params::ValidateBatch) -> Result<bool> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = batches::table
        .select(sql::<Integer>("1"))
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .get_result::<i32>(&db.conn)
        .optional()?;
    Ok(exists.is_some())
}

pub fn append(db: &MysqlDb, params: params::AppendToBatch) -> Result<()> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let bsos = bsos_to_batch_string(&params.bsos)?;
    let affected_rows = update(batches::table)
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .set(batches::bsos.eq(batches::bsos.concat(&bsos)))
        .execute(&db.conn)?;
    if affected_rows == 1 {
        Ok(())
    } else {
        Err(DbErrorKind::BatchNotFound.into())
    }
}

#[derive(Debug, Default, Queryable)]
pub struct Batch {
    pub id: i64,
    pub bsos: String,
    pub expiry: i64,
}

pub fn get(db: &MysqlDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    Ok(batches::table
        .select((batches::id, batches::bsos, batches::expiry))
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .get_result::<Batch>(&db.conn)
        .optional()?
        .map(|batch| results::GetBatch {
            id: encode_id(batch.id),
            bsos: batch.bsos,
            expiry: batch.expiry,
        }))
}

pub fn delete(db: &MysqlDb, params: params::DeleteBatch) -> Result<()> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    diesel::delete(batches::table)
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&id))
        .execute(&db.conn)?;
    Ok(())
}

/// Commits a batch to the bsos table, deleting the batch when succesful
pub fn commit(db: &MysqlDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    let bsos = batch_string_to_bsos(&params.batch.bsos)?;
    let mut metrics = db.metrics.clone();
    metrics.start_timer("storage.sql.apply_batch", None);
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

pub fn validate_batch_id(id: &str) -> Result<()> {
    decode_id(id).map(|_| ())
}

fn encode_id(id: i64) -> String {
    base64::encode(&id.to_string())
}

fn decode_id(id: &str) -> Result<i64> {
    let bytes = base64::decode(id).unwrap_or_else(|_| id.as_bytes().to_vec());
    let decoded = std::str::from_utf8(&bytes).unwrap_or(id);
    decoded
        .parse::<i64>()
        .map_err(|e| DbError::internal(&format!("Invalid batch_id: {}", e)))
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

#[macro_export]
macro_rules! batch_db_method {
    ($name:ident, $batch_name:ident, $type:ident) => {
        pub fn $name(&self, params: params::$type) -> Result<results::$type> {
            batch::$batch_name(self, params)
        }
    }
}
