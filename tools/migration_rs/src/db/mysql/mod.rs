use mysql_async;
use mysql_async::prelude::Queryable;


use futures::executor::block_on;

use crate::db::collections::Collections;
use crate::error::ApiResult;
use crate::settings::Settings;

#[derive(Clone)]
pub struct MysqlDb {
    pub pool: mysql_async::Pool,
}

impl MysqlDb {
    pub fn new(settings: &Settings) -> ApiResult<Self> {
        let pool = mysql_async::Pool::new(settings.dsns.mysql.unwrap());
        Ok(Self { pool })
    }

    pub async fn collections(&self, base: &mut Collections) -> ApiResult<Collections> {
        let conn = self.pool.get_conn().await.unwrap();

        let cursor = conn
            .prep_exec(
                "SELECT
                DISTINCT uc.collection, cc.name
            FROM
                user_collections as uc,
                collections as cc
            WHERE
                uc.collection = cc.collectionid
            ORDER BY
                uc.collection
            ",
                (),
            ).await.unwrap();
        cursor
            .map_and_drop(|row| {
                let id: u8 = row.get(0).unwrap();
                let collection_name:String = row.get(1).unwrap();
                if base.get(&collection_name).is_none() {
                    base.set(&collection_name, id);
                }
            }).await;
        self.pool.disconnect().await.unwrap();

        Ok(base.clone())
    }
}
