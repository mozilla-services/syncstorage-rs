use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

use google_cloud_rust_raw::spanner::v1::type_pb::{StructType, Type, TypeCode};
use protobuf::{
    well_known_types::{ListValue, Value},
    RepeatedField,
};
use syncstorage_db_common::{
    error::{DbError, DbErrorKind},
    params, results,
    util::to_rfc3339,
    UserIdentifier, BATCH_LIFETIME, DEFAULT_BSO_TTL,
};
use uuid::Uuid;

use super::models::{Result, SpannerDb, PRETOUCH_TS};
use super::support::{as_type, null_value, struct_type_field, IntoSpannerValue};
use crate::web::tags::Tags;

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
    let new_batch = results::CreateBatch {
        size: db
            .check_quota(&params.user_id, &params.collection, collection_id)
            .await?,
        id: batch_id,
    };

    let (sqlparams, mut sqlparam_types) = params! {
        "fxa_uid" => params.user_id.fxa_uid.clone(),
        "fxa_kid" => params.user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
        "batch_id" => new_batch.id.clone(),
        "expiry" => to_rfc3339(timestamp + BATCH_LIFETIME)?,
    };
    sqlparam_types.insert("expiry".to_owned(), as_type(TypeCode::TIMESTAMP));
    db.sql(
        "INSERT INTO batches (fxa_uid, fxa_kid, collection_id, batch_id, expiry)
         VALUES (@fxa_uid, @fxa_kid, @collection_id, @batch_id, @expiry)",
    )?
    .params(sqlparams)
    .param_types(sqlparam_types)
    .execute_dml_async(&db.conn)
    .await?;

    do_append_async(
        db,
        params.user_id,
        collection_id,
        new_batch.clone(),
        params.bsos,
        &params.collection,
    )
    .await?;
    Ok(new_batch)
}

pub async fn validate_async(db: &SpannerDb, params: params::ValidateBatch) -> Result<bool> {
    let exists = get_async(db, params.into()).await?;
    Ok(exists.is_some())
}

// Append a collection to a pending batch (`create_batch` creates a new batch)
pub async fn append_async(db: &SpannerDb, params: params::AppendToBatch) -> Result<()> {
    let mut metrics = db.metrics.clone();
    metrics.start_timer("storage.spanner.append_items_to_batch", None);
    let collection_id = db.get_collection_id_async(&params.collection).await?;

    let current_size = db
        .check_quota(&params.user_id, &params.collection, collection_id)
        .await?;
    let mut batch = params.batch;
    if let Some(size) = current_size {
        batch.size = Some(size + batch.size.unwrap_or(0));
    }

    // confirm that this batch exists or has not yet been committed.
    let exists = validate_async(
        db,
        params::ValidateBatch {
            user_id: params.user_id.clone(),
            collection: params.collection.clone(),
            id: batch.id.clone(),
        },
    )
    .await?;
    if !exists {
        // NOTE: db tests expects this but it doesn't seem necessary w/ the
        // handler validating the batch before appends
        Err(DbErrorKind::BatchNotFound)?
    }

    do_append_async(
        db,
        params.user_id,
        collection_id,
        batch,
        params.bsos,
        &params.collection,
    )
    .await?;
    Ok(())
}

pub async fn get_async(
    db: &SpannerDb,
    params: params::GetBatch,
) -> Result<Option<results::GetBatch>> {
    let collection_id = db.get_collection_id_async(&params.collection).await?;
    let (sqlparams, sqlparam_types) = params! {
        "fxa_uid" => params.user_id.fxa_uid.clone(),
        "fxa_kid" => params.user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
        "batch_id" => params.id.clone(),
    };
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
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_async(&db.conn)?
        .one_or_none()
        .await?
        .map(move |_| params::Batch { id: params.id });
    Ok(batch)
}

pub async fn delete_async(db: &SpannerDb, params: params::DeleteBatch) -> Result<()> {
    let collection_id = db.get_collection_id_async(&params.collection).await?;
    let (sqlparams, sqlparam_types) = params! {
        "fxa_uid" => params.user_id.fxa_uid.clone(),
        "fxa_kid" => params.user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
        "batch_id" => params.id,
    };
    // Also deletes child batch_bsos rows (INTERLEAVE IN PARENT batches ON
    // DELETE CASCADE)
    db.sql(
        "DELETE FROM batches
          WHERE fxa_uid = @fxa_uid
            AND fxa_kid = @fxa_kid
            AND collection_id = @collection_id
            AND batch_id = @batch_id",
    )?
    .params(sqlparams)
    .param_types(sqlparam_types)
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
        .update_collection_async(&params.user_id, collection_id, &params.collection)
        .await?;

    let as_rfc3339 = timestamp.as_rfc3339()?;
    {
        // First, UPDATE existing rows in the bsos table with any new values
        // supplied in this batch
        let mut timer2 = db.metrics.clone();
        timer2.start_timer("storage.spanner.apply_batch_update", None);
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "batch_id" => params.batch.id.clone(),
            "timestamp" => as_rfc3339.clone(),
        };
        sqlparam_types.insert("timestamp".to_owned(), as_type(TypeCode::TIMESTAMP));
        db.sql(include_str!("batch_commit_update.sql"))?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml_async(&db.conn)
            .await?;
    }

    {
        // Then INSERT INTO SELECT remaining rows from this batch into the bsos
        // table (that didn't already exist there)
        let (sqlparams, mut sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "batch_id" => params.batch.id.clone(),
            "timestamp" => as_rfc3339,
            "default_bso_ttl" => DEFAULT_BSO_TTL,
        };
        sqlparam_types.insert("timestamp".to_owned(), as_type(TypeCode::TIMESTAMP));
        let mut timer3 = db.metrics.clone();
        timer3.start_timer("storage.spanner.apply_batch_insert", None);
        db.sql(include_str!("batch_commit_insert.sql"))?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml_async(&db.conn)
            .await?;
    }

    delete_async(
        db,
        params::DeleteBatch {
            user_id: params.user_id.clone(),
            collection: params.collection,
            id: params.batch.id,
        },
    )
    .await?;
    // XXX: returning results::PostBsos here isn't needed
    // update the quotas for the user's collection
    if db.quota.enabled {
        db.update_user_collection_quotas(&params.user_id, collection_id)
            .await?;
    }
    Ok(timestamp)
}

// Append a collection to an existing, pending batch.
pub async fn do_append_async(
    db: &SpannerDb,
    user_id: UserIdentifier,
    collection_id: i32,
    batch: results::CreateBatch,
    bsos: Vec<params::PostCollectionBso>,
    collection: &str,
) -> Result<()> {
    // Pass an array of struct objects as @values (for UNNEST), e.g.:
    // [("<fxa_uid>", "<fxa_kid>", 101, "ba1", "bso1", NULL, "payload1", NULL),
    //  ("<fxa_uid>", "<fxa_kid>", 101, "ba1", "bso2", NULL, "payload2", NULL)]
    // https://cloud.google.com/spanner/docs/structs#creating_struct_objects
    let mut running_size: usize = 0;

    // problem: Append may try to insert a duplicate record into the batch_bsos table.
    // this is because spanner doesn't do upserts easily. An upsert like operation can
    // be performed by carefully crafting a complex protobuf struct. (See
    // https://github.com/mozilla-services/syncstorage-rs/issues/618#issuecomment-680227710
    // for details.)
    // Batch_bso is a temp table and items are eventually rolled into bsos.

    // create a simple key for a HashSet to see if a given record has already been
    // created
    fn exist_idx(collection_id: &str, batch_id: &str, bso_id: &str) -> String {
        format!(
            "{collection_id}::{batch_id}::{bso_id}",
            collection_id = collection_id,
            batch_id = batch_id,
            bso_id = bso_id,
        )
    }

    struct UpdateRecord {
        bso_id: String,
        sortindex: Option<i32>,
        payload: Option<String>,
        ttl: Option<u32>,
    }

    //prefetch the existing batch_bsos for this user's batch.
    let mut existing = HashSet::new();
    let mut collisions = HashSet::new();
    let mut count_collisions = 0;
    let mut tags = Tags::default();
    tags.tags.insert(
        "collection".to_owned(),
        db.get_collection_name(collection_id)
            .await
            .unwrap_or_else(|| "UNKNOWN".to_string()),
    );

    let bso_ids = bsos
        .iter()
        .map(|pbso| pbso.id.clone())
        .collect::<Vec<String>>();
    let (sqlparams, sqlparam_types) = params! {
        "fxa_uid" => user_id.fxa_uid.clone(),
        "fxa_kid" => user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
        "batch_id" => batch.id.clone(),
        "ids" => bso_ids,
    };
    let mut existing_stream = db
        .sql(
            "SELECT batch_bso_id
            FROM batch_bsos
            WHERE fxa_uid=@fxa_uid
                AND fxa_kid=@fxa_kid
                AND collection_id=@collection_id
                AND batch_id=@batch_id
                AND batch_bso_id in UNNEST(@ids);",
        )?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_async(&db.conn)?;
    while let Some(row) = existing_stream.next_async().await {
        let row = row?;
        existing.insert(exist_idx(
            &collection_id.to_string(),
            &batch.id,
            row[0].get_string_value(),
        ));
    }

    db.metrics.count_with_tags(
        "storage.spanner.batch.pre-existing",
        existing.len() as i64,
        Some(tags.clone()),
    );

    // Approach 1:
    // iterate and check to see if the record is in batch_bso table already
    let mut insert: Vec<Value> = Vec::new();
    let mut update: Vec<UpdateRecord> = Vec::new();
    for bso in bsos {
        if let Some(ref payload) = bso.payload {
            running_size += payload.len();
        }
        let exist_idx = exist_idx(&collection_id.to_string(), &batch.id, &bso.id);

        if existing.contains(&exist_idx) {
            // need to update this record
            // reject this record since you can only have one update per batch
            update.push(UpdateRecord {
                bso_id: bso.id,
                sortindex: bso.sortindex,
                payload: bso.payload,
                ttl: bso.ttl,
            });
            // BSOs should only update records that were in previous batches.
            // There is the potential that some may update records in their own batch.
            // This will attempt to record such incidents.
            // TODO: If we consistently see no results for this, it can be safely dropped.
            if collisions.contains(&exist_idx) {
                count_collisions += 1;
            }
        } else {
            let sortindex = bso
                .sortindex
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);
            let payload = bso
                .payload
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);
            let ttl = bso
                .ttl
                .map(IntoSpannerValue::into_spanner_value)
                .unwrap_or_else(null_value);

            // convert to a protobuf structure for direct insertion to
            // avoid some mutation limits.
            let mut row = ListValue::new();
            row.set_values(RepeatedField::from_vec(vec![
                user_id.fxa_uid.clone().into_spanner_value(),
                user_id.fxa_kid.clone().into_spanner_value(),
                collection_id.into_spanner_value(),
                batch.id.clone().into_spanner_value(),
                bso.id.into_spanner_value(),
                sortindex,
                payload,
                ttl,
            ]));
            let mut value = Value::new();
            value.set_list_value(row);
            insert.push(value);
            existing.insert(exist_idx.clone());
            collisions.insert(exist_idx);
        };
    }

    if count_collisions > 0 {
        db.metrics.count_with_tags(
            "storage.spanner.batch.collisions",
            count_collisions,
            Some(tags.clone()),
        );
    }

    if db.quota.enabled {
        if let Some(size) = batch.size {
            if size + running_size >= db.quota.size {
                if db.quota.enforced {
                    return Err(db.quota_error(collection));
                } else {
                    warn!("Quota at limit for user's collection ({} bytes)", size + running_size; "collection"=>collection);
                }
            }
        }
    }

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

    if !insert.is_empty() {
        let mut list_values = ListValue::new();
        let count_inserts = insert.len();
        list_values.set_values(RepeatedField::from_vec(insert));
        let mut values = Value::new();
        values.set_list_value(list_values);

        // values' type is an ARRAY of STRUCTs
        let mut param_type = Type::new();
        param_type.set_code(TypeCode::ARRAY);
        let mut array_type = Type::new();
        array_type.set_code(TypeCode::STRUCT);

        // STRUCT requires definition of all its field types
        let mut struct_type = StructType::new();
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
        db.metrics.count_with_tags(
            "storage.spanner.batch.insert",
            count_inserts as i64,
            Some(tags.clone()),
        );
    }

    // assuming that "update" is rarer than an insert, we can try using the standard API for that.
    if !update.is_empty() {
        for val in update {
            let mut fields = Vec::new();
            let (mut params, mut param_types) = params! {
                "fxa_uid" => user_id.fxa_uid.clone(),
                "fxa_kid" => user_id.fxa_kid.clone(),
                "collection_id" => collection_id,
                "batch_id" => batch.id.clone(),
                "batch_bso_id" => val.bso_id,
            };
            if let Some(sortindex) = val.sortindex {
                fields.push("sortindex");
                param_types.insert("sortindex".to_owned(), sortindex.spanner_type());
                params.insert("sortindex".to_owned(), sortindex.into_spanner_value());
            }
            if let Some(payload) = val.payload {
                fields.push("payload");
                param_types.insert("payload".to_owned(), payload.spanner_type());
                params.insert("payload".to_owned(), payload.into_spanner_value());
            };
            if let Some(ttl) = val.ttl {
                fields.push("ttl");
                param_types.insert("ttl".to_owned(), ttl.spanner_type());
                params.insert("ttl".to_owned(), ttl.into_spanner_value());
            }
            if fields.is_empty() {
                continue;
            };
            let updatable = fields
                .iter()
                .map(|field| format!("{field}=@{field}", field = field))
                .collect::<Vec<String>>()
                .join(", ");
            db.sql(&format!(
                "UPDATE batch_bsos SET {updatable}
                WHERE fxa_uid=@fxa_uid AND fxa_kid=@fxa_kid AND collection_id=@collection_id
                AND batch_id=@batch_id AND batch_bso_id=@batch_bso_id",
                updatable = updatable
            ))?
            .params(params)
            .param_types(param_types.clone())
            .execute_dml_async(&db.conn)
            .await?;
        }
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
async fn pretouch_collection_async(
    db: &SpannerDb,
    user_id: &UserIdentifier,
    collection_id: i32,
) -> Result<()> {
    let (mut sqlparams, mut sqlparam_types) = params! {
        "fxa_uid" => user_id.fxa_uid.clone(),
        "fxa_kid" => user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
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
        .param_types(sqlparam_types.clone())
        .execute_async(&db.conn)?
        .one_or_none()
        .await?;
    if result.is_none() {
        sqlparams.insert(
            "modified".to_owned(),
            PRETOUCH_TS.to_owned().into_spanner_value(),
        );
        sqlparam_types.insert("modified".to_owned(), as_type(TypeCode::TIMESTAMP));
        let sql = if db.quota.enabled {
            "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified, count, total_bytes)
            VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified, 0, 0)"
        } else {
            "INSERT INTO user_collections (fxa_uid, fxa_kid, collection_id, modified)
            VALUES (@fxa_uid, @fxa_kid, @collection_id, @modified)"
        };
        db.sql(sql)?
            .params(sqlparams)
            .param_types(sqlparam_types)
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
