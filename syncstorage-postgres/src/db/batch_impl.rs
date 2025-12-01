#![allow(unused_variables)] // XXX:
use async_trait::async_trait;
use syncstorage_db_common::{params, results, BatchDb};

use super::PgDb;
use crate::DbError;

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
    ) -> Result<results::DeleteBatch, Self::Error> {
        todo!()
    }
}
