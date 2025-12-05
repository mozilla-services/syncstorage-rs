#![allow(unused_variables)] // XXX:
use async_trait::async_trait;
use diesel::{delete, ExpressionMethods};
use diesel_async::RunQueryDsl;

use syncstorage_db_common::{params, results, BatchDb, Db};

use super::PgDb;
use crate::schema::{batch_bsos, batches};
use crate::{DbError, DbResult};

#[async_trait(?Send)]
impl BatchDb for PgDb {
    type Error = DbError;

    async fn create_batch(
        &mut self,
        params: params::CreateBatch,
    ) -> Result<results::CreateBatch, Self::Error> {
        todo!()
    }

    async fn validate_batch(
        &mut self,
        params: params::ValidateBatch,
    ) -> Result<results::ValidateBatch, Self::Error> {
        todo!()
    }

    async fn append_to_batch(
        &mut self,
        params: params::AppendToBatch,
    ) -> Result<results::AppendToBatch, Self::Error> {
        todo!()
    }

    async fn get_batch(
        &mut self,
        params: params::GetBatch,
    ) -> Result<Option<results::GetBatch>, Self::Error> {
        todo!()
    }

    async fn commit_batch(
        &mut self,
        params: params::CommitBatch,
    ) -> Result<results::CommitBatch, Self::Error> {
        todo!()
    }

    async fn delete_batch(
        &mut self,
        params: params::DeleteBatch,
    ) -> DbResult<results::DeleteBatch> {
        let user_id = params.user_id.legacy_id as i64;
        let collection_id = self.get_collection_id(&params.collection).await?;
        delete(batches::table)
            .filter(batches::batch_id.eq(&params.id))
            .filter(batches::user_id.eq(&user_id))
            .filter(batches::collection_id.eq(&collection_id))
            .execute(&mut self.conn)
            .await?;
        delete(batch_bsos::table)
            .filter(batch_bsos::batch_id.eq(&params.id))
            .filter(batch_bsos::user_id.eq(&user_id))
            .execute(&mut self.conn)
            .await?;
        Ok(())
    }
}
