use async_trait::async_trait;
use diesel::{
    self, delete,
    dsl::{now, sql},
    insert_into, sql_query,
    sql_types::{BigInt, Integer, Nullable, Text, Timestamptz, Uuid as SqlUuid},
    ExpressionMethods, OptionalExtension, QueryDsl,
};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use syncstorage_db_common::{
    params, results, BatchDb, Db, UserIdentifier, BATCH_LIFETIME, DEFAULT_BSO_TTL,
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
        let expiry =
            self.timestamp().as_datetime()? + chrono::TimeDelta::milliseconds(BATCH_LIFETIME);

        self.ensure_user_collection(user_id, collection_id).await?;
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
            .filter(batches::expiry.gt(now))
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
        Ok(is_valid.then_some(results::GetBatch { id: params.id }))
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
        let default_ttl_seconds = DEFAULT_BSO_TTL as i64;
        let ts_datetime = timestamp.as_datetime()?;

        sql_query(
            "UPDATE bsos
             SET
                 sortindex = COALESCE(src.sortindex, bsos.sortindex),
                 payload = COALESCE(src.payload, bsos.payload),
                 modified = $3,
                 expiry = COALESCE(
                     CASE
                         WHEN src.ttl IS NOT NULL THEN $3 + (src.ttl || ' seconds')::INTERVAL
                         ELSE NULL
                     END,
                     bsos.expiry
                 )
             FROM (
                 SELECT batch_bso_id, sortindex, payload, ttl
                 FROM batch_bsos
                 WHERE user_id = $1 AND collection_id = $2 AND batch_id = $5
             ) AS src
             WHERE bsos.user_id = $1
               AND bsos.collection_id = $2
               AND bsos.bso_id = src.batch_bso_id",
        )
        .bind::<BigInt, _>(user_id)
        .bind::<Integer, _>(collection_id)
        .bind::<Timestamptz, _>(ts_datetime)
        .bind::<BigInt, _>(default_ttl_seconds)
        .bind::<SqlUuid, _>(&batch_id)
        .execute(&mut self.conn)
        .await?;

        sql_query(
            "INSERT INTO bsos (user_id, collection_id, bso_id, sortindex, payload, modified, expiry)
             SELECT
                 $1,
                 $2,
                 batch_bso_id,
                 sortindex,
                 COALESCE(payload, ''::TEXT),
                 $3,
                 CASE
                     WHEN ttl IS NOT NULL THEN $3 + (ttl || ' seconds')::INTERVAL
                     ELSE $3 + ($4 || ' seconds')::INTERVAL
                 END
             FROM batch_bsos
             WHERE user_id = $1 AND batch_id = $5
               AND NOT EXISTS (
                   SELECT 1 FROM bsos
                   WHERE bsos.user_id = $1
                     AND bsos.collection_id = $2
                     AND bsos.bso_id = batch_bsos.batch_bso_id
               )"
        )
        .bind::<BigInt, _>(user_id)
        .bind::<Integer, _>(collection_id)
        .bind::<Timestamptz, _>(ts_datetime)
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
        let batch_id = validate_batch_id(&params.id)?;
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
        let user_id_i64 = user_id.legacy_id as i64;

        sql_query(
            "INSERT INTO batch_bsos (user_id, collection_id, batch_id, batch_bso_id, sortindex, payload, ttl)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (user_id, collection_id, batch_id, batch_bso_id) DO UPDATE SET
                 sortindex = COALESCE(EXCLUDED.sortindex, batch_bsos.sortindex),
                 payload = COALESCE(EXCLUDED.payload, batch_bsos.payload),
                 ttl = COALESCE(EXCLUDED.ttl, batch_bsos.ttl)"
        )
        .bind::<BigInt, _>(user_id_i64)
        .bind::<Integer, _>(collection_id)
        .bind::<SqlUuid, _>(&batch_id)
        .bind::<Text, _>(&bso.id)
        .bind::<Nullable<Integer>, _>(sortindex)
        .bind::<Nullable<Text>, _>(&bso.payload)
        .bind::<Nullable<BigInt>, _>(ttl)
        .execute(&mut db.conn)
        .await?;
    }

    Ok(())
}

pub fn validate_batch_id(id: &str) -> DbResult<Uuid> {
    Uuid::parse_str(id).map_err(|e| DbError::internal(format!("Invalid batch_id: {}", e)))
}
