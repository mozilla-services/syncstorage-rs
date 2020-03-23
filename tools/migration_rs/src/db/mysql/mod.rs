use std::str::FromStr;
use mysql_async::{self, params};
use mysql_async::prelude::Queryable;

use crate::db::collections::Collections;
use crate::error::{ApiErrorKind, ApiResult};
use crate::settings::Settings;
use crate::fxa::FxaInfo;
use crate::db::{User, UserData};

#[derive(Clone)]
pub struct MysqlDb {
    settings: Settings,
    pub pool: mysql_async::Pool,
}

impl MysqlDb {
    pub fn new(settings: &Settings) -> ApiResult<Self> {
        let pool = mysql_async::Pool::new(settings.dsns.mysql.clone().unwrap());
        Ok(Self {settings: settings.clone(), pool})
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
        match cursor
            .map_and_drop(|row| {
                let id: u8 = row.get(0).unwrap();
                let collection_name:String = row.get(1).unwrap();
                if base.get(&collection_name).is_none() {
                    base.set(&collection_name, id);
                }
            }).await {
                Ok(_) => {
                    Ok(base.clone())
                },
                Err(e) => {
                    Err(ApiErrorKind::Internal(format!("failed to get collections {}", e)).into())
                }
            }
    }

    pub async fn get_user_ids(&self, bso_num: u8) -> ApiResult<Vec<u64>> {
        let mut results: Vec<u64> = Vec::new();
        // return the list if they're already specified in the options.
        if let Some(user) = self.settings.user.clone() {
            for uid in user.user_id {
                results.push(u64::from_str(&uid).map_err(|e| ApiErrorKind::Internal(format!("Invalid UID option found {} {}", uid, e)))?);
            }
            return Ok(results)
        }

        let sql = "SELECT DISTINCT userid FROM :bso";
        let conn: mysql_async::Conn = match self
            .pool
            .get_conn()
            .await {
                Ok(v) => v,
                Err(e) => {
                    return Err(ApiErrorKind::Internal(format!("Could not get connection: {}", e)).into())
                }
            };
        let cursor = match conn.prep_exec(sql, params!{
            "bso" => bso_num
        }).await {
            Ok(v) => v,
            Err(e) => {
                return Err(ApiErrorKind::Internal(format!("Could not get users: {}",e)).into())
            }
        };
        match cursor.map_and_drop(|row| {
            let uid:String = mysql_async::from_row(row);
            if let Ok(v) = u64::from_str(&uid) {
                v
            } else {
                panic!("Invalid UID found in database {}", uid);
            }
        }).await {
            Ok(_) => {Ok(results)}
            Err(e)=> {Err(ApiErrorKind::Internal(format!("Bad UID found in database {}", e)).into())}
        }
    }

    pub async fn get_user_data(&self, user: &User, bso: u8) -> ApiResult<Vec<UserData>> {
        Err(ApiErrorKind::Internal("TODO".to_owned()).into())
    }
}
