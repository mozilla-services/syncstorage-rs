use std::{collections::HashMap, str::FromStr};

use async_trait::async_trait;
use google_cloud_rust_raw::spanner::v1::type_pb::{StructType, Type, TypeCode};
use protobuf::{
    RepeatedField,
    well_known_types::{ListValue, Value},
};
use syncstorage_db_common::{
    BATCH_LIFETIME, BatchDb, DEFAULT_BSO_TTL, Db, UserIdentifier, params, results, util::to_rfc3339,
};
use uuid::Uuid;

use crate::error::DbError;

use super::{
    PRETOUCH_TS, SpannerDb,
    support::{IntoSpannerValue, as_type, null_value, struct_type_field},
};
use crate::DbResult;

#[async_trait(?Send)]
impl BatchDb for SpannerDb {
    type Error = DbError;

    async fn create_batch(
        &mut self,
        params: params::CreateBatch,
    ) -> DbResult<results::CreateBatch> {
        let batch_id = Uuid::new_v4().simple().to_string();
        let collection_id = self._get_collection_id(&params.collection).await?;
        let timestamp = self.checked_timestamp()?.as_i64();

        // Ensure a parent record exists in user_collections before writing to batches
        // (INTERLEAVE IN PARENT user_collections)
        pretouch_collection(self, &params.user_id, collection_id).await?;
        let new_batch = results::CreateBatch {
            size: self
                .check_quota(&params.user_id, &params.collection)
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
        self.sql(
            "INSERT INTO batches (fxa_uid, fxa_kid, collection_id, batch_id, expiry)
             VALUES (@fxa_uid, @fxa_kid, @collection_id, @batch_id, @expiry)",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;

        do_append(
            self,
            params.user_id,
            collection_id,
            new_batch.clone(),
            params.bsos,
            &params.collection,
        )
        .await?;
        Ok(new_batch)
    }

    async fn validate_batch(&mut self, params: params::ValidateBatch) -> DbResult<bool> {
        let exists = self.get_batch(params.into()).await?;
        Ok(exists.is_some())
    }

    // Append a collection to a pending batch (`create_batch` creates a new batch)
    async fn append_to_batch(&mut self, params: params::AppendToBatch) -> DbResult<()> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.spanner.append_items_to_batch", None);
        let collection_id = self._get_collection_id(&params.collection).await?;

        // `batch.size` is the committed size
        let mut batch = params.batch;
        batch.size = self
            .check_quota(&params.user_id, &params.collection)
            .await?;

        // confirm that this batch exists or has not yet been committed.
        let exists = self
            .validate_batch(params::ValidateBatch {
                user_id: params.user_id.clone(),
                collection: params.collection.clone(),
                id: batch.id.clone(),
            })
            .await?;
        if !exists {
            // NOTE: db tests expects this but it doesn't seem necessary w/ the
            // handler validating the batch before appends
            return Err(DbError::batch_not_found());
        }

        do_append(
            self,
            params.user_id,
            collection_id,
            batch,
            params.bsos,
            &params.collection,
        )
        .await?;
        Ok(())
    }

    async fn get_batch(&mut self, params: params::GetBatch) -> DbResult<Option<results::GetBatch>> {
        let collection_id = self._get_collection_id(&params.collection).await?;
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "batch_id" => params.id.clone(),
        };
        let batch = self
            .sql(
                "SELECT 1
                   FROM batches
                  WHERE fxa_uid = @fxa_uid
                    AND fxa_kid = @fxa_kid
                    AND collection_id = @collection_id
                    AND batch_id = @batch_id
                    AND expiry > CURRENT_TIMESTAMP()",
            )
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute(&self.conn)?
            .one_or_none()
            .await?
            .map(move |_| params::Batch { id: params.id });
        Ok(batch)
    }

    async fn delete_batch(&mut self, params: params::DeleteBatch) -> DbResult<()> {
        let collection_id = self._get_collection_id(&params.collection).await?;
        let (sqlparams, sqlparam_types) = params! {
            "fxa_uid" => params.user_id.fxa_uid.clone(),
            "fxa_kid" => params.user_id.fxa_kid.clone(),
            "collection_id" => collection_id,
            "batch_id" => params.id,
        };
        // Also deletes child batch_bsos rows (INTERLEAVE IN PARENT batches ON
        // DELETE CASCADE)
        self.sql(
            "DELETE FROM batches
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND batch_id = @batch_id",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute_dml(&self.conn)
        .await?;
        Ok(())
    }

    async fn commit_batch(
        &mut self,
        params: params::CommitBatch,
    ) -> DbResult<results::CommitBatch> {
        let mut metrics = self.metrics.clone();
        metrics.start_timer("storage.spanner.apply_batch", None);
        let collection_id = self._get_collection_id(&params.collection).await?;

        // Ensure a parent record exists in user_collections before writing to bsos
        // (INTERLEAVE IN PARENT user_collections)
        let timestamp = self
            .update_collection(params::UpdateCollection {
                user_id: params.user_id.clone(),
                collection_id,
                collection: params.collection.clone(),
            })
            .await?;

        {
            let mut timer2 = self.metrics.clone();
            timer2.start_timer("storage.spanner.apply_batch_upsert", None);
            let (sqlparams, mut sqlparam_types) = params! {
                "fxa_uid" => params.user_id.fxa_uid.clone(),
                "fxa_kid" => params.user_id.fxa_kid.clone(),
                "collection_id" => collection_id,
                "batch_id" => params.batch.id.clone(),
                "timestamp" => timestamp.as_rfc3339()?,
                "default_bso_ttl" => DEFAULT_BSO_TTL,
            };
            sqlparam_types.insert("timestamp".to_owned(), as_type(TypeCode::TIMESTAMP));
            // NOTE: This write treats expired and non-expired as existing bsos. We don't filter on
            // `expiry > CURRENT_TIMESTAMP()` to avoid having to delete expired rows before
            // inserting new ones with the same id. Unfortunately, this means updates may resurrect
            // expired bsos (or at least a subset of their fields), or possibly even write new data
            // without an associated ttl to an expired record that will be deleted. This in
            // practice should be a very rare occurrence.
            self.sql(include_str!("batch_commit_upsert.sql"))
                .await?
                .params(sqlparams)
                .param_types(sqlparam_types)
                .execute_dml(&self.conn)
                .await?;
        }

        self.delete_batch(params::DeleteBatch {
            user_id: params.user_id.clone(),
            collection: params.collection,
            id: params.batch.id,
        })
        .await?;
        // XXX: returning results::PostBsos here isn't needed
        // update the quotas for the user's collection
        if self.quota.enabled {
            self.update_user_collection_quotas(&params.user_id, collection_id)
                .await?;
        }
        Ok(timestamp)
    }
}

// Append a collection to an existing, pending batch.
pub async fn do_append(
    db: &mut SpannerDb,
    user_id: UserIdentifier,
    collection_id: i32,
    batch: results::CreateBatch,
    bsos: Vec<params::PostCollectionBso>,
    collection: &str,
) -> DbResult<()> {
    let mut running_size: usize = 0;
    let mut tags = HashMap::new();
    tags.insert(
        "collection".to_owned(),
        db.get_collection_name(collection_id)
            .await
            .unwrap_or_else(|| "UNKNOWN".to_string()),
    );

    // Build an ARRAY<STRUCT> of incoming rows for a `INSERT OR UPDATE`.  COALESCE(new, existing)
    // is used so an update only overwrites fields the request supplied.
    let mut rows: Vec<Value> = Vec::with_capacity(bsos.len());
    // The incoming ids are used to exclude the payloads from the running total.
    let mut incoming_ids: Vec<String> = Vec::with_capacity(bsos.len());
    for bso in bsos {
        if let Some(ref payload) = bso.payload {
            running_size += payload.len();
        }
        incoming_ids.push(bso.id.clone());
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
        rows.push(value);
    }

    if db.quota.enabled
        && let Some(size) = batch.size
    {
        let pending_size =
            pending_batch_size(db, &user_id, collection_id, &batch.id, &incoming_ids).await?;
        let projected_total = size + pending_size + running_size;
        if projected_total >= db.quota.size {
            if db.quota.enforced {
                return Err(db.quota_error(collection));
            } else {
                warn!("Quota at limit for user ({} bytes)", projected_total; "collection"=>collection);
            }
        }
    }

    if rows.is_empty() {
        return Ok(());
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

    let row_count = rows.len();
    let mut list_values = ListValue::new();
    list_values.set_values(RepeatedField::from_vec(rows));
    let mut values = Value::new();
    values.set_list_value(list_values);

    let mut param_type = Type::new();
    param_type.set_code(TypeCode::ARRAY);
    let mut array_type = Type::new();
    array_type.set_code(TypeCode::STRUCT);
    let mut struct_type = StructType::new();
    struct_type.set_fields(RepeatedField::from_vec(fields));
    array_type.set_struct_type(struct_type);
    param_type.set_array_element_type(array_type);

    let mut sqlparams = HashMap::new();
    sqlparams.insert("values".to_owned(), values);
    let mut sqlparam_types = HashMap::new();
    sqlparam_types.insert("values".to_owned(), param_type);
    db.sql(
        "INSERT OR UPDATE INTO batch_bsos
             (fxa_uid, fxa_kid, collection_id, batch_id, batch_bso_id,
              sortindex, payload, ttl)
         SELECT
             incoming.fxa_uid,
             incoming.fxa_kid,
             incoming.collection_id,
             incoming.batch_id,
             incoming.batch_bso_id,
             COALESCE(incoming.sortindex, existing.sortindex),
             COALESCE(incoming.payload, existing.payload),
             COALESCE(incoming.ttl, existing.ttl)
           FROM UNNEST(@values) AS incoming
           LEFT JOIN batch_bsos AS existing
             ON existing.fxa_uid = incoming.fxa_uid
            AND existing.fxa_kid = incoming.fxa_kid
            AND existing.collection_id = incoming.collection_id
            AND existing.batch_id = incoming.batch_id
            AND existing.batch_bso_id = incoming.batch_bso_id",
    )
    .await?
    .params(sqlparams)
    .param_types(sqlparam_types)
    .execute_dml(&db.conn)
    .await?;
    db.metrics
        .count_with_tags("storage.spanner.batch.upsert", row_count as i64, tags);

    Ok(())
}

/// Sum the pending payload bytes, excluding any ids in `incoming_ids`.
async fn pending_batch_size(
    db: &mut SpannerDb,
    user_id: &UserIdentifier,
    collection_id: i32,
    batch_id: &str,
    incoming_ids: &[String],
) -> DbResult<usize> {
    let (mut sqlparams, mut sqlparam_types) = params! {
        "fxa_uid" => user_id.fxa_uid.clone(),
        "fxa_kid" => user_id.fxa_kid.clone(),
        "collection_id" => collection_id,
        "batch_id" => batch_id.to_owned(),
    };
    let incoming_ids = incoming_ids.to_vec();
    sqlparam_types.insert("incoming_ids".to_owned(), incoming_ids.spanner_type());
    sqlparams.insert("incoming_ids".to_owned(), incoming_ids.into_spanner_value());
    let result = db
        .sql(
            "SELECT COALESCE(SUM(BYTE_LENGTH(payload)), 0)
               FROM batch_bsos
              WHERE fxa_uid = @fxa_uid
                AND fxa_kid = @fxa_kid
                AND collection_id = @collection_id
                AND batch_id = @batch_id
                AND batch_bso_id NOT IN UNNEST(@incoming_ids)",
        )
        .await?
        .params(sqlparams)
        .param_types(sqlparam_types)
        .execute(&db.conn)?
        .one()
        .await?;
    let bytes = result[0]
        .get_string_value()
        .parse::<usize>()
        .map_err(|e| DbError::integrity(e.to_string()))?;
    Ok(bytes)
}

/// Ensure a parent row exists in user_collections prior to creating a child
/// row in the batches table.
///
/// When no parent exists, a "tombstone" like ("pre birth stone"?) value for
/// modified is inserted, which is explicitly ignored by other queries.
///
/// For the special case of a user creating a batch for a collection with no
/// prior data.
async fn pretouch_collection(
    db: &mut SpannerDb,
    user_id: &UserIdentifier,
    collection_id: i32,
) -> DbResult<()> {
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
        )
        .await?
        .params(sqlparams.clone())
        .param_types(sqlparam_types.clone())
        .execute(&db.conn)?
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
        db.sql(sql)
            .await?
            .params(sqlparams)
            .param_types(sqlparam_types)
            .execute_dml(&db.conn)
            .await?;
    }
    Ok(())
}

pub fn validate_batch_id(id: &str) -> DbResult<()> {
    Uuid::from_str(id)
        .map(|_| ())
        .map_err(|e| DbError::internal(format!("Invalid batch_id: {}", e)))
}
