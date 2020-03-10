use std::{collections::HashMap, str::FromStr};

use googleapis_raw::spanner::v1::type_pb::{StructType, Type, TypeCode};
use protobuf::{
    well_known_types::{ListValue, Value},
    RepeatedField,
};
use uuid::Uuid;

use super::support::{null_value, struct_type_field};
use super::{
    models::{Result, SpannerDb, DEFAULT_BSO_TTL, PRETOUCH_TS},
    support::as_value,
};
use crate::{
    db::{params, results, util::to_rfc3339, DbError, DbErrorKind, BATCH_LIFETIME},
    web::extractors::HawkIdentifier,
};

pub async fn create_async(
    db: &SpannerDb,
    params: params::CreateBatch,
) -> Result<results::CreateBatch> {
    let batch_id = Uuid::new_v4().to_simple().to_string();
    let collection_id = db.get_collection_id_async(&params.collection).await?;
    let timestamp = db.timestamp()?.as_i64();

    // Ensure a parent record exists in user_collections before writing to batches
    // (INTERLEAVE IN PARENT user_collections)
    pretouch_collection_async(db, &params.user_id, collection_id).await?;

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
    .execute_dml_async(&db.conn)
    .await?;

    do_append_async(
        db,
        params.user_id,
        collection_id,
        batch_id.clone(),
        params.bsos,
    )
    .await?;
    Ok(batch_id)
}

pub async fn validate_async(db: &SpannerDb, params: params::ValidateBatch) -> Result<bool> {
    let collection_id = db.get_collection_id_async(&params.collection).await?;
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
        .execute_async(&db.conn)?
        .one_or_none()
        .await?;
    Ok(exists.is_some())
}

pub async fn append_async(db: &SpannerDb, params: params::AppendToBatch) -> Result<()> {
    let mut metrics = db.metrics.clone();
    metrics.start_timer("storage.spanner.append_items_to_batch", None);

    let exists = validate_async(
        db,
        params::ValidateBatch {
            user_id: params.user_id.clone(),
            collection: params.collection.clone(),
            id: params.id.clone(),
        },
    )
    .await?;
    if !exists {
        // NOTE: db tests expects this but it doesn't seem necessary w/ the
        // handler validating the batch before appends
        Err(DbErrorKind::BatchNotFound)?
    }

    let collection_id = db.get_collection_id_async(&params.collection).await?;
    do_append_async(db, params.user_id, collection_id, params.id, params.bsos).await?;
    Ok(())
}

pub async fn get_async(
    db: &SpannerDb,
    params: params::GetBatch,
) -> Result<Option<results::GetBatch>> {
    let collection_id = db.get_collection_id_async(&params.collection).await?;
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
        .execute_async(&db.conn)?
        .one_or_none()
        .await?
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

pub async fn delete_async(db: &SpannerDb, params: params::DeleteBatch) -> Result<()> {
    let collection_id = db.get_collection_id_async(&params.collection).await?;
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
    .execute_dml_async(&db.conn)
    .await?;
    Ok(())
}

pub async fn commit_async(
    db: &SpannerDb,
    params: params::CommitBatch,
) -> Result<results::CommitBatch> {
    let mut metrics = db.metrics.clone();
    metrics.start_timer("storage.spanner.apply_batch", None);
    let collection_id = db.get_collection_id_async(&params.collection).await?;

    // Ensure a parent record exists in user_collections before writing to bsos
    // (INTERLEAVE IN PARENT user_collections)
    let timestamp = db
        .touch_collection_async(&params.user_id, collection_id)
        .await?;

    let as_rfc3339 = timestamp.as_rfc3339()?;
    {
        // First, UPDATE existing rows in the bsos table with any new values
        // supplied in this batch
        let mut timer2 = db.metrics.clone();
        timer2.start_timer("storage.spanner.apply_batch_update", None);
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
            .execute_dml_async(&db.conn)
            .await?;
    }

    {
        // Then INSERT INTO SELECT remaining rows from this batch into the bsos
        // table (that didn't already exist there)
        let mut timer3 = db.metrics.clone();
        timer3.start_timer("storage.spanner.apply_batch_insert", None);
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
            .execute_dml_async(&db.conn)
            .await?;
    }

    delete_async(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )
    .await?;
    // XXX: returning results::PostBsos here isn't needed
    Ok(results::PostBsos {
        modified: timestamp,
        success: Default::default(),
        failed: Default::default(),
    })
}

pub async fn do_append_async(
    db: &SpannerDb,
    user_id: HawkIdentifier,
    collection_id: i32,
    batch_id: String,
    bsos: Vec<params::PostCollectionBso>,
) -> Result<()> {
    // Pass an array of struct objects as @values (for UNNEST), e.g.:
    // [("<fxa_uid>", "<fxa_kid>", 101, "ba1", "bso1", NULL, "payload1", NULL),
    //  ("<fxa_uid>", "<fxa_kid>", 101, "ba1", "bso2", NULL, "payload2", NULL)]
    // https://cloud.google.com/spanner/docs/structs#creating_struct_objects
    let rows: Vec<_> = bsos
        .into_iter()
        .map(|bso| {
            let sortindex = bso
                .sortindex
                .map(|sortindex| as_value(sortindex.to_string()))
                .unwrap_or_else(null_value);
            let payload = bso.payload.map(as_value).unwrap_or_else(null_value);
            let ttl = bso
                .ttl
                .map(|ttl| as_value(ttl.to_string()))
                .unwrap_or_else(null_value);

            let mut row = ListValue::new();
            row.set_values(RepeatedField::from_vec(vec![
                as_value(user_id.fxa_uid.clone()),
                as_value(user_id.fxa_kid.clone()),
                as_value(collection_id.to_string()),
                as_value(batch_id.clone()),
                as_value(bso.id),
                sortindex,
                payload,
                ttl,
            ]));
            let mut value = Value::new();
            value.set_list_value(row);
            value
        })
        .collect();

    let mut list_values = ListValue::new();
    list_values.set_values(RepeatedField::from_vec(rows));
    let mut values = Value::new();
    values.set_list_value(list_values);

    // values' type is an ARRAY of STRUCTs
    let mut param_type = Type::new();
    param_type.set_code(TypeCode::ARRAY);
    let mut array_type = Type::new();
    array_type.set_code(TypeCode::STRUCT);

    // STRUCT requires definition of all its field types
    let mut struct_type = StructType::new();
    let fields = vec![
        ("fxa_uid", TypeCode::STRING),
        ("fxa_kid", TypeCode::STRING),
        ("collection_id", TypeCode::INT64),
        ("batch_id", TypeCode::STRING),
        ("batch_bso_id", TypeCode::STRING),
        ("sortindex", TypeCode::INT64),
        ("payload", TypeCode::STRING),
        ("ttl", TypeCode::INT64),
    ]
    .into_iter()
    .map(|(name, field_type)| struct_type_field(name, field_type))
    .collect();
    struct_type.set_fields(RepeatedField::from_vec(fields));
    array_type.set_struct_type(struct_type);
    param_type.set_array_element_type(array_type);

    let mut sqlparams = HashMap::new();
    sqlparams.insert("values".to_owned(), values);
    let mut sqlparam_types = HashMap::new();
    sqlparam_types.insert("values".to_owned(), param_type);
    db.sql(
        "INSERT INTO batch_bsos (fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id,
                                 sortindex, payload, ttl)
         SELECT * FROM UNNEST(@values)",
    )?
    .params(sqlparams)
    .param_types(sqlparam_types)
    .execute_dml_async(&db.conn)
    .await?;

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
async fn pretouch_collection_async(
    db: &SpannerDb,
    user_id: &HawkIdentifier,
    collection_id: i32,
) -> Result<()> {
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
        .execute_async(&db.conn)?
        .one_or_none()
        .await?;
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
        .execute_dml_async(&db.conn)
        .await?;
    }
    Ok(())
}

pub fn validate_batch_id(id: &str) -> Result<()> {
    Uuid::from_str(id)
        .map(|_| ())
        .map_err(|e| DbError::internal(&format!("Invalid batch_id: {}", e)))
}
