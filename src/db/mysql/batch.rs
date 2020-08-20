use diesel::{
    self,
    dsl::sql,
    expression::{bound::Bound, operators::Eq},
    insert_into,
    result::{DatabaseErrorKind::UniqueViolation, Error as DieselError},
    sql_types::Integer,
    update, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, TextExpressionMethods, JoinOnDsl,
};

use super::{
    diesel_ext::OnDuplicateKeyUpdateDsl,
    models::{MysqlDb, Result},
    schema::{batch_upload_items, batch_uploads},
};

use crate::{
    db::{params, results, DbError, DbErrorKind, BATCH_LIFETIME},
    web::extractors::HawkIdentifier,
};

pub fn create(db: &MysqlDb, params: params::CreateBatch) -> Result<results::CreateBatch> {
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

    do_append(
        db,
        batch_id.clone(),
        params.user_id,
        collection_id,
        params.bsos,
    )?;

    Ok(encode_id(batch_id))
}

pub fn validate(db: &MysqlDb, params: params::ValidateBatch) -> Result<bool> {
    let batch_id = decode_id(&params.id)?;
    // Avoid hitting the db for batches that are obviously too old.  Recall
    // that the batchid is a millisecond timestamp.
    if (batch_id / 1000 + BATCH_LIFETIME) < db.timestamp().as_i64() {
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

pub fn append(db: &MysqlDb, params: params::AppendToBatch) -> Result<()> {
    let batch_id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    // XXX: spanner impl does a validate_async + triggers a BatchNotFound for
    // db-tests
    do_append(db, batch_id, params.user_id, collection_id, params.bsos)?;
    Ok(())
}

#[derive(Debug, Default, Queryable)]
pub struct Batch {
    pub id: i64,
    pub bsos: String,
    pub expiry: i64,
}

pub fn get(db: &MysqlDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    Ok(batch_upload_items::table
        .select((batch_upload_items::batch_id, batch_upload_items::payload, batch_upload_items::ttl_offset))
        .inner_join(batch_uploads::table.on(batch_uploads::batch_id.eq(batch_upload_items::batch_id)))
        .filter(batch_upload_items::user_id.eq(&user_id))
        .filter(batch_uploads::collection_id.eq(&collection_id))
        .filter(batch_upload_items::batch_id.eq(id))
        .filter(batch_upload_items::ttl_offset.gt(db.timestamp().as_i64()))
        .get_result::<Batch>(&db.conn)
        .optional()?
        .map(|batch| results::GetBatch {
            id: encode_id(batch.id),
            bsos: batch.bsos,
            expiry: batch.expiry,
        })
    )
}

pub fn delete(db: &MysqlDb, params: params::DeleteBatch) -> Result<()> {
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
pub fn commit(db: &MysqlDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    unimplemented!();
    /*
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
     */
}

pub fn do_append(
    db: &MysqlDb,
    batch_id: i64,
    user_id: HawkIdentifier,
    collection_id: i32,
    bsos: Vec<params::PostCollectionBso>,
) -> Result<()> {
    // Eq<batch_upload_items::columns::user_id, Option<u64>>
    let mut to_insert: Vec<_> = Vec::new();
    for _ in bsos.into_iter().map(|b: params::PostCollectionBso| {
            let payload = b.payload.unwrap_or(String::new());
            let payload_len = payload.len() as i64;
            to_insert.push((
                batch_upload_items::batch_id.eq(&batch_id),
                batch_upload_items::user_id.eq(user_id.legacy_id as i64),
                batch_upload_items::id.eq(b.id.clone()),
                batch_upload_items::sortindex.eq(b.sortindex.unwrap_or(0)),
                batch_upload_items::payload.eq(payload),
                batch_upload_items::payload_size.eq(payload_len)
            ));
        }) {
        // Do nothing, just consume the iter
    }

    let rows_inserted = insert_into(batch_upload_items::table)
        .values(to_insert)
        .on_duplicate_key_update()
        .execute(&db.conn)?;

    if rows_inserted > 0 {
        Ok(())
    } else {
        Err(DbErrorKind::BatchNotFound.into())
    }
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

#[macro_export]
macro_rules! batch_db_method {
    ($name:ident, $batch_name:ident, $type:ident) => {
        pub fn $name(&self, params: params::$type) -> Result<results::$type> {
            batch::$batch_name(self, params)
        }
    };
}
