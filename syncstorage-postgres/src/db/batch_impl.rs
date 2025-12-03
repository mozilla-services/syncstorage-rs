use async_trait::async_trait;
use diesel::{
    self, delete,
    dsl::sql,
    insert_into, sql_query,
    sql_types::{BigInt, Integer, Timestamp, Uuid as SqlUuid},
    upsert::excluded,
    ExpressionMethods, OptionalExtension, QueryDsl,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use syncstorage_db_common::{
    params, results, util::SyncTimestamp, BatchDb, Db, UserIdentifier, BATCH_LIFETIME,
    DEFAULT_BSO_TTL,
};

use super::PgDb;
use crate::{
    schema::{batch_bsos, batches},
    DbError, DbResult,
};

#[async_trait(?Send)]
impl BatchDb for PgDb {
    type Error = DbError;

    async fn create_batch(
        &mut self,
        params: params::CreateBatch,
    ) -> DbResult<results::CreateBatch> {
        let batch_id = Uuid::new_v4();
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;
        let timestamp_ms = self.timestamp().as_i64();
        let expiry = SyncTimestamp::from_i64(timestamp_ms + BATCH_LIFETIME)?.as_naive_datetime()?;

        insert_into(batches::table)
            .values((
                batches::batch_id.eq(&batch_id),
                batches::user_id.eq(user_id),
                batches::collection_id.eq(collection_id),
                batches::expiry.eq(expiry),
            ))
            .execute(&mut self.conn)
            .await?;

        let batch = results::CreateBatch {
            id: batch_id.to_string(),
            size: None,
        };

        do_append(
            self,
            params.user_id,
            collection_id,
            batch.clone(),
            params.bsos,
        )
        .await?;

        Ok(batch)
    }

    async fn validate_batch(
        &mut self,
        params: params::ValidateBatch,
    ) -> DbResult<results::ValidateBatch> {
        let batch_id = Uuid::parse_str(&params.id)
            .map_err(|e| DbError::internal(format!("Invalid batch_id: {}", e)))?;
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;

        let exists = batches::table
            .select(sql::<Integer>("1"))
            .filter(batches::batch_id.eq(&batch_id))
            .filter(batches::user_id.eq(user_id))
            .filter(batches::collection_id.eq(collection_id))
            .filter(batches::expiry.gt(diesel::dsl::now))
            .first::<i32>(&mut self.conn)
            .await
            .optional()?;

        Ok(exists.is_some())
    }

    async fn append_to_batch(
        &mut self,
        params: params::AppendToBatch,
    ) -> DbResult<results::AppendToBatch> {
        let exists = self
            .validate_batch(params::ValidateBatch {
                user_id: params.user_id.clone(),
                collection: params.collection.clone(),
                id: params.batch.id.clone(),
            })
            .await?;

        if !exists {
            return Err(DbError::batch_not_found());
        }

        let collection_id = self.get_or_create_collection_id(&params.collection).await?;

        do_append(
            self,
            params.user_id,
            collection_id,
            params.batch,
            params.bsos,
        )
        .await?;

        Ok(())
    }

    async fn get_batch(&mut self, params: params::GetBatch) -> DbResult<Option<results::GetBatch>> {
        let is_valid = self
            .validate_batch(params::ValidateBatch {
                user_id: params.user_id,
                collection: params.collection,
                id: params.id.clone(),
            })
            .await?;
        let batch = if is_valid {
            Some(results::GetBatch { id: params.id })
        } else {
            None
        };

        Ok(batch)
    }

    async fn commit_batch(
        &mut self,
        params: params::CommitBatch,
    ) -> DbResult<results::CommitBatch> {
        let batch_id = Uuid::parse_str(&params.batch.id)
            .map_err(|e| DbError::internal(format!("Invalid batch_id: {}", e)))?;
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_or_create_collection_id(&params.collection).await?;

        let timestamp = self
            .update_collection(params::UpdateCollection {
                user_id: params.user_id.clone(),
                collection_id,
                collection: params.collection.clone(),
            })
            .await?;
        let naive_datetime = timestamp.as_naive_datetime()?;
        let default_ttl_seconds = DEFAULT_BSO_TTL as i64;

        sql_query(
            "INSERT INTO bsos (user_id, collection_id, bso_id, sortindex, payload, modified, expiry)
             SELECT
                 $1::BIGINT,
                 $2::INTEGER,
                 batch_bso_id,
                 sortindex,
                 COALESCE(payload, ''::TEXT),
                 $3::TIMESTAMP,
                 CASE
                     WHEN ttl IS NOT NULL THEN $3::TIMESTAMP + (ttl || ' seconds')::INTERVAL
                     ELSE $3::TIMESTAMP + ($4::BIGINT || ' seconds')::INTERVAL
                 END
             FROM batch_bsos
             WHERE user_id = $1 AND batch_id = $5
             ON CONFLICT (user_id, collection_id, bso_id) DO UPDATE SET
                 sortindex = COALESCE(EXCLUDED.sortindex, bsos.sortindex),
                 payload = COALESCE(EXCLUDED.payload, bsos.payload),
                 modified = EXCLUDED.modified,
                 expiry = COALESCE(EXCLUDED.expiry, bsos.expiry)"
        )
        .bind::<BigInt, _>(user_id)
        .bind::<Integer, _>(collection_id)
        .bind::<Timestamp, _>(naive_datetime)
        .bind::<BigInt, _>(default_ttl_seconds)
        .bind::<SqlUuid, _>(&batch_id)
        .execute(&mut self.conn)
        .await?;

        self.delete_batch(params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        })
        .await?;

        Ok(timestamp)
    }

    async fn delete_batch(
        &mut self,
        params: params::DeleteBatch,
    ) -> DbResult<results::DeleteBatch> {
        let batch_id = Uuid::parse_str(&params.id)
            .map_err(|e| DbError::internal(format!("Invalid batch_id: {}", e)))?;
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;

        delete(
            batches::table
                .filter(batches::user_id.eq(user_id))
                .filter(batches::collection_id.eq(collection_id))
                .filter(batches::batch_id.eq(&batch_id)),
        )
        .execute(&mut self.conn)
        .await?;

        delete(batch_bsos::table)
            .filter(batch_bsos::batch_id.eq(&batch_id))
            .filter(batch_bsos::user_id.eq(user_id))
            .execute(&mut self.conn)
            .await?;

        Ok(())
    }
}

pub async fn do_append(
    db: &mut PgDb,
    user_id: UserIdentifier,
    collection_id: i32,
    batch: results::CreateBatch,
    bsos: Vec<params::PostCollectionBso>,
) -> DbResult<()> {
    let batch_id = Uuid::parse_str(&batch.id)
        .map_err(|e| DbError::internal(format!("Invalid batch_id in batch: {}", e)))?;

    for bso in bsos {
        let ttl = bso.ttl.map(|t| t as i64);
        let sortindex = bso.sortindex;

        insert_into(batch_bsos::table)
            .values((
                batch_bsos::batch_id.eq(&batch_id),
                batch_bsos::user_id.eq(user_id.legacy_id as i64),
                batch_bsos::collection_id.eq(collection_id),
                batch_bsos::batch_bso_id.eq(&bso.id),
                batch_bsos::sortindex.eq(sortindex),
                batch_bsos::payload.eq(&bso.payload),
                batch_bsos::ttl.eq(ttl),
            ))
            .on_conflict((
                batch_bsos::user_id,
                batch_bsos::collection_id,
                batch_bsos::batch_id,
                batch_bsos::batch_bso_id,
            ))
            .do_update()
            .set((
                batch_bsos::sortindex.eq(excluded(batch_bsos::sortindex)),
                batch_bsos::payload.eq(excluded(batch_bsos::payload)),
                batch_bsos::ttl.eq(excluded(batch_bsos::ttl)),
            ))
            .execute(&mut db.conn)
            .await?;
    }

    Ok(())
}
