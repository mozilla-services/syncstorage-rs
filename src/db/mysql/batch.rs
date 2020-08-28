
use diesel::{
    self,
    dsl::sql,
    insert_into,
    result::{DatabaseErrorKind::UniqueViolation, Error as DieselError},
    sql_types::{BigInt, Integer},
    sql_query,
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, JoinOnDsl,
};

use super::{
    models::{MysqlDb, Result},
    schema::{batch_upload_items, batch_uploads},
};

use crate::{
    db::{params, results, DbError, DbErrorKind, BATCH_LIFETIME},
    web::extractors::HawkIdentifier,
};

const MAXTTL: i32 = 2_100_000_000;

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

    db.touch_collection(user_id as u32, collection_id)?;

    do_append(
        db,
        batch_id.clone(),
        params.user_id,
        collection_id,
        params.bsos,
        false,
    )?;

    Ok(encode_id(batch_id))
}

pub fn validate(db: &MysqlDb, params: params::ValidateBatch) -> Result<bool> {
    eprintln!("validate... {:?}", &params.id);
    let batch_id = decode_id(&params.id)?;
    // Avoid hitting the db for batches that are obviously too old.  Recall
    // that the batchid is a millisecond timestamp.
    eprintln!("validate! {:?}", batch_id);
    if (batch_id / 1000 + BATCH_LIFETIME) < db.timestamp().as_i64() {
        return Ok(false);
    }

    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    eprintln!("userid collid {:?} {:?}", user_id, collection_id);
    let exists = batch_uploads::table
        .select(sql::<Integer>("1"))
        .filter(batch_uploads::batch_id.eq(&batch_id))
        .filter(batch_uploads::user_id.eq(&user_id))
        .filter(batch_uploads::collection_id.eq(&collection_id))
        .get_result::<i32>(&db.conn)
        .optional()?;
    eprintln!("exists {:?}", exists);
    Ok(exists.is_some())
}

pub fn append(db: &MysqlDb, params: params::AppendToBatch) -> Result<()> {
    let batch_id = decode_id(&params.id)?;
    let collection_id = db.get_collection_id(&params.collection)?;
    // XXX: spanner impl does a validate_async + triggers a BatchNotFound for
    // db-tests
    do_append(db, batch_id, params.user_id, collection_id, params.bsos, true)?;
    Ok(())
}

/*
#[derive(Debug, Default, Queryable)]
pub struct Batch {
    pub id: i64,
    pub bsos: String,
//    pub expiry: i64,
}
*/

pub fn get(db: &MysqlDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let id = decode_id(&params.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    Ok(batch_upload_items::table
        .select(batch_upload_items::batch_id)
        .inner_join(batch_uploads::table.on(batch_uploads::batch_id.eq(batch_upload_items::batch_id)))
        .filter(batch_upload_items::user_id.eq(&user_id))
        .filter(batch_uploads::collection_id.eq(&collection_id))
       .filter(batch_upload_items::batch_id.eq(id))
       // XXX: this isn't expiry, it's the ttl value. I believe the
       // batch expiry "lives" inside the batch id (batch id is based from a timestamp)
//        .filter(batch_upload_items::ttl_offset.gt(db.timestamp().as_i64()))
        .get_result(&db.conn)
        .optional()?
        .map(|batch_id| results::GetBatch {
            id: encode_id(batch_id),
            bsos: "".to_owned(),
//            expiry: 0, // XXX: FIXME
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

//sql_function!(fn i32_coalesce2(x: Integer, y: Integer) -> Integer);
//sql_function!(fn string_coalesce2(x: String, y: String) -> String);

/// Commits a batch to the bsos table, deleting the batch when succesful
pub fn commit(db: &MysqlDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    let batch_id = decode_id(&params.batch.id)?;
    let user_id = params.user_id.legacy_id as i64;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp();
    let batch_insert_update = r#"
    INSERT INTO bso
        (userid, collection, id, modified, sortindex,
        ttl, payload, payload_size)
    SELECT
        ?, ?, id, ?, sortindex,
        COALESCE(ttl_offset + ?, ?),
        COALESCE(payload, ''),
        COALESCE(payload_size, 0)
    FROM batch_upload_items
    WHERE batch = ? AND userid = ?
    ON DUPLICATE KEY UPDATE
        modified = ?,
        sortindex = COALESCE(batch_upload_items.sortindex,
                             bso.sortindex),
        ttl = COALESCE(batch_upload_items.ttl_offset + ?,
                       bso.ttl),
        payload = COALESCE(batch_upload_items.payload,
                           bso.payload),
        payload_size = COALESCE(batch_upload_items.payload_size,
                                bso.payload_size)
        "#;
    sql_query(batch_insert_update)
        .bind::<BigInt, _>(user_id as i64)
        .bind::<Integer, _>(&collection_id)
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<Integer, _>(MAXTTL)
        .bind::<BigInt, _>(&batch_id)
        .bind::<BigInt, _>(user_id as i64)
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .bind::<BigInt, _>(&db.timestamp().as_i64())
        .execute(&db.conn)?;

    /*
    let select_query = batch_upload_items::table.select((
        params.user_id,
        params.collection,
        batch_upload_items::id,
        timestamp.as_i64(),
        batch_upload_items::sortindex,
        i32coalesce2(batch_upload_items::ttl_offset + timestamp.as_i64() as i32, MAXTTL),
        Stringcoalesce2(params.payload, ""),
        i32coalesce2(params.payload.len(), 0)
    ))
    .filter(batch_upload_items::batch_id.eq(params.batch))
    .filter(batch_upload_items::user_id.eq(params.user_id));

    let result = diesel::insert_into(bso::table)
        .values(select_query)
        .into_columns((
            bso::user_id,
            bso::collection_id,
            bso::id,
            bso::modified,
            bso::sortindex,
            bso::expiry,
            bso::payload,
            bso::payload_size
        ))
        .on_duplicate_key_update((
            bso::modified.eq(timestamp.as_i64()),
            bso::sortindex.eq(i32coalesce2(batch_upload_items::sortindex, bso::sortindex)),
            bso::expiry.eq(i32coalesce2(batch_upload_items::ttl_offset + timestamp.as_i64() as i32, bso::expiry)),
            bso::payload.eq(Stringcoalesce2(batch_upload_items::payload, bso::payload)),
            bso::payload_size.eq(i32coalesce2(batch_upload_items::payload_size, bso::payload_size))
        )).execute(db);
*/
    db.touch_collection(user_id as u32, collection_id)?;

    delete(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )?;
    Ok(results::PostBsos {
        modified: timestamp,
        success: Default::default(),
        failed: Default::default(),
    })
}

pub fn do_append(
    db: &MysqlDb,
    batch_id: i64,
    user_id: HawkIdentifier,
    _collection_id: i32,
    bsos: Vec<params::PostCollectionBso>,
    check_result: bool,
) -> Result<()> {
    // Eq<batch_upload_items::columns::user_id, Option<u64>>
    eprintln!("DO APPEND");
    let mut to_insert = Vec::new();
    for _ in bsos.into_iter().map(|b: params::PostCollectionBso| {
    eprintln!("itermap {:?}", &b.ttl);
        let payload = b.payload.unwrap_or(String::new());
        let payload_len = payload.len() as i64;
        to_insert.push((
            batch_upload_items::batch_id.eq(&batch_id),
            batch_upload_items::user_id.eq(user_id.legacy_id as i64),
            batch_upload_items::id.eq(b.id.clone()),
            batch_upload_items::sortindex.eq(b.sortindex.unwrap_or(0)),
            batch_upload_items::payload.eq(payload),
            batch_upload_items::payload_size.eq(payload_len),
            batch_upload_items::ttl_offset.eq(b.ttl.map(|ttl| {
                ttl as i64
            }).or_else(|| Some(MAXTTL as i64))),
        ));
    }) {
        // Do nothing, just consume the iter
    }

    let rows_inserted = insert_into(batch_upload_items::table)
        .values(to_insert)
        .execute(&db.conn)?;
    if check_result {
        if rows_inserted > 0 {
            Ok(())
        } else {
            Err(DbErrorKind::BatchNotFound.into())
        }
    } else {
        Ok(())
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
