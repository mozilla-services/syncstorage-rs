use std::collections::HashSet;

use diesel::{
    self,
    dsl::sql,
    insert_into,
    result::{DatabaseErrorKind::UniqueViolation, Error as DieselError},
    sql_query,
    sql_types::{BigInt, Integer},
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl,
};
use syncserver_db_common::{
    error::{DbError, DbErrorKind},
    params, results, DbResult, UserIdentifier, BATCH_LIFETIME,
};

use super::{
    models::MysqlDb,
    schema::{batch_upload_items, batch_uploads},
};

const MAXTTL: i32 = 2_100_000_000;

pub fn create(db: &MysqlDb, params: params::CreateBatch) -> DbResult<results::CreateBatch> {
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    // Careful, there's some weirdness here!
    //
    // Sync timestamps are in seconds and quantized to two decimal places, so
    // when we convert one to a bigint in milliseconds, the final digit is
    // always zero. But we want to use the lower digits of the batchid for
    // sharding writes via (batchid % num_tables), and leaving it as zero would
    // skew the sharding distribution.
    //
    // So we mix in the lowest digit of the uid to improve the distribution
    // while still letting us treat these ids as millisecond timestamps.  It's
    // yuck, but it works and it keeps the weirdness contained to this single
    // line of code.
    let batch_id = db.timestamp().as_i64() + (user_id % 10);
    insert_into(batch_uploads::table)
        .values((
            batch_uploads::batch_id.eq(&batch_id),
            batch_uploads::user_id.eq(&user_id),
            batch_uploads::collection_id.eq(&collection_id),
        ))
        .execute(&db.conn)
        .map_err(|e| -> DbError {
            match e {
                // The user tried to create two batches with the same timestamp
                DieselError::DatabaseError(UniqueViolation, _) => DbErrorKind::Conflict.into(),
                _ => e.into(),
            }
        })?;

    do_append(db, batch_id, params.user_id, collection_id, params.bsos)?;
    Ok(results::CreateBatch {
        id: encode_id(batch_id),
        size: None,
    })
}

pub fn validate(db: &MysqlDb, params: params::ValidateBatch) -> DbResult<bool> {
    let batch_id = decode_id(&params.id)?;
    // Avoid hitting the db for batches that are obviously too old.  Recall
    // that the batchid is a millisecond timestamp.
    if (batch_id + BATCH_LIFETIME) < db.timestamp().as_i64() {
        return Ok(false);
    }

    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = batch_uploads::table
        .select(sql::<Integer>("1"))
        .filter(batch_uploads::batch_id.eq(&batch_id))
        .filter(batch_uploads::user_id.eq(&user_id))
        .filter(batch_uploads::collection_id.eq(&collection_id))
        .get_result::<i32>(&db.conn)
        .optional()?;
    Ok(exists.is_some())
}

pub fn append(db: &MysqlDb, params: params::AppendToBatch) -> DbResult<()> {
    let exists = validate(
        db,
        params::ValidateBatch {
            user_id: params.user_id.clone(),
            collection: params.collection.clone(),
            id: params.batch.id.clone(),
        },
    )?;

    if !exists {
        return Err(DbErrorKind::BatchNotFound.into());
    }

    let batch_id = decode_id(&params.batch.id)?;
    let collection_id = db.get_collection_id(&params.collection)?;
    do_append(db, batch_id, params.user_id, collection_id, params.bsos)?;
    Ok(())
}

pub fn get(db: &MysqlDb, params: params::GetBatch) -> DbResult<Option<results::GetBatch>> {
    let is_valid = validate(
        db,
        params::ValidateBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.id.clone(),
        },
    )?;
    let batch = if is_valid {
        Some(results::GetBatch { id: params.id })
    } else {
        None
    };
    Ok(batch)
}

pub fn delete(db: &MysqlDb, params: params::DeleteBatch) -> DbResult<()> {
    let batch_id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    diesel::delete(batch_uploads::table)
        .filter(batch_uploads::batch_id.eq(&batch_id))
        .filter(batch_uploads::user_id.eq(&user_id))
        .filter(batch_uploads::collection_id.eq(&collection_id))
        .execute(&db.conn)?;
    diesel::delete(batch_upload_items::table)
        .filter(batch_upload_items::batch_id.eq(&batch_id))
        .filter(batch_upload_items::user_id.eq(&user_id))
        .execute(&db.conn)?;
    Ok(())
}

/// Commits a batch to the bsos table, deleting the batch when succesful
pub fn commit(db: &MysqlDb, params: params::CommitBatch) -> DbResult<results::CommitBatch> {
    let batch_id = decode_id(&params.batch.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp();
    sql_query(include_str!("batch_commit.sql"))
        .bind::<BigInt, _>(user_id as i64)
        .bind::<Integer, _>(&collection_id)
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<BigInt, _>((MAXTTL as i64) * 1000) // XXX:
        .bind::<BigInt, _>(&batch_id)
        .bind::<BigInt, _>(user_id as i64)
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .execute(&db.conn)?;

    db.update_collection(user_id as u32, collection_id)?;

    delete(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )?;
    Ok(timestamp)
}

pub fn do_append(
    db: &MysqlDb,
    batch_id: i64,
    user_id: UserIdentifier,
    _collection_id: i32,
    bsos: Vec<params::PostCollectionBso>,
) -> DbResult<()> {
    fn exist_idx(user_id: u64, batch_id: i64, bso_id: &str) -> String {
        // Construct something that matches the key for batch_upload_items
        format!(
            "{batch_id}-{user_id}-{bso_id}",
            batch_id = batch_id,
            user_id = user_id,
            bso_id = bso_id,
        )
    }

    // It's possible for the list of items to contain a duplicate key entry.
    // This means that we can't really call `ON DUPLICATE` here, because that's
    // more about inserting one item at a time. (e.g. it works great if the
    // values contain a key that's already in the database, less so if the
    // the duplicate is in the value set we're inserting.
    #[derive(Debug, QueryableByName)]
    #[table_name = "batch_upload_items"]
    struct ExistsResult {
        batch_id: i64,
        id: String,
    }

    #[derive(AsChangeset)]
    #[table_name = "batch_upload_items"]
    struct UpdateBatches {
        payload: Option<String>,
        payload_size: Option<i64>,
        ttl_offset: Option<i32>,
    }

    let mut existing = HashSet::new();

    // pre-load the "existing" hashset with any batched uploads that are already in the table.
    for item in sql_query(
        "SELECT userid as user_id, batch as batch_id, id FROM batch_upload_items WHERE userid=? AND batch=?;",
    )
    .bind::<BigInt, _>(user_id.legacy_id as i64)
    .bind::<BigInt, _>(batch_id)
    .get_results::<ExistsResult>(&db.conn)?
    {
        existing.insert(exist_idx(
            user_id.legacy_id,
            item.batch_id,
            &item.id.to_string(),
        ));
    }

    for bso in bsos {
        let payload_size = bso.payload.as_ref().map(|p| p.len() as i64);
        let exist_idx = exist_idx(user_id.legacy_id, batch_id, &bso.id);

        if existing.contains(&exist_idx) {
            diesel::update(
                batch_upload_items::table
                    .filter(batch_upload_items::user_id.eq(user_id.legacy_id as i64))
                    .filter(batch_upload_items::batch_id.eq(batch_id)),
            )
            .set(&UpdateBatches {
                payload: bso.payload,
                payload_size,
                ttl_offset: bso.ttl.map(|ttl| ttl as i32),
            })
            .execute(&db.conn)?;
        } else {
            diesel::insert_into(batch_upload_items::table)
                .values((
                    batch_upload_items::batch_id.eq(&batch_id),
                    batch_upload_items::user_id.eq(user_id.legacy_id as i64),
                    batch_upload_items::id.eq(bso.id.clone()),
                    batch_upload_items::sortindex.eq(bso.sortindex),
                    batch_upload_items::payload.eq(bso.payload),
                    batch_upload_items::payload_size.eq(payload_size),
                    batch_upload_items::ttl_offset.eq(bso.ttl.map(|ttl| ttl as i32)),
                ))
                .execute(&db.conn)?;
            // make sure to include the key into our table check.
            existing.insert(exist_idx);
        }
    }

    Ok(())
}

pub fn validate_batch_id(id: &str) -> DbResult<()> {
    decode_id(id).map(|_| ())
}

fn encode_id(id: i64) -> String {
    base64::encode(&id.to_string())
}

fn decode_id(id: &str) -> DbResult<i64> {
    let bytes = base64::decode(id).unwrap_or_else(|_| id.as_bytes().to_vec());
    let decoded = std::str::from_utf8(&bytes).unwrap_or(id);
    decoded
        .parse::<i64>()
        .map_err(|e| DbError::internal(&format!("Invalid batch_id: {}", e)))
}

#[macro_export]
macro_rules! batch_db_method {
    ($name:ident, $batch_name:ident, $type:ident) => {
        pub fn $name(&self, params: params::$type) -> DbResult<results::$type> {
            batch::$batch_name(self, params)
        }
    };
}
