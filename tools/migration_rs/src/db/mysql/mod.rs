use std::str::FromStr;
use mysql_async::{self, params};
use mysql_async::prelude::Queryable;

use crate::db::collections::{Collection, Collections};
use crate::error::{ApiErrorKind, ApiResult};
use crate::settings::Settings;
use crate::db::{User, Bso};

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

    // take the existing set of collections, return a list of any "new"
    // unrecognized collections.
    pub async fn merge_collections(&self, base: &mut Collections) -> ApiResult<Collections> {
        let conn = self.pool.get_conn().await.unwrap();
        let mut new_collections = Collections::empty();

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
                let id: u16 = row.get(0).unwrap();
                // Only add "new" items
                let collection_name:String = row.get(1).unwrap();
                if base.get(&collection_name).is_none() {
                    let new = Collection{
                        collection: id,
                        name: collection_name.clone(),
                        last_modified: 0,
                    };
                    new_collections.set(&collection_name, new.clone());
                    base.set(&collection_name, new.clone());
                }
            }).await {
                Ok(_) => {
                    Ok(new_collections)
                },
                Err(e) => {
                    Err(ApiErrorKind::Internal(format!("failed to get collections {}", e)).into())
                }
            }
    }

    pub async fn get_user_ids(&self, bso_num: &u8) -> ApiResult<Vec<u64>> {
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

    pub async fn get_user_collections(&self, user: &User, bso_num: &u8) -> ApiResult<Vec<Collection>> {
        // fetch the collections and bso info for a given user.alloc
        // COLLECTIONS
        let bso_sql = "
        SELECT
            collections.name, user_collections.collection, user_collections.last_modified
        FROM
            collections, user_collections
        WHERE
            user_collections.userid = :userid and collections.collectionid = user_collections.collection;
        ";
        let conn: mysql_async::Conn = match self
            .pool
            .get_conn()
            .await {
                Ok(v) => v,
                Err(e) => {
                    return Err(ApiErrorKind::Internal(format!("Could not get connection: {}", e)).into())
                }
            };
        let cursor = match conn.prep_exec(bso_sql, params!{
            "bso_num" => bso_num,
            "user_id" => user.uid,
        }).await {
            Ok(v) => v,
            Err(e) => {
                return Err(ApiErrorKind::Internal(format!("Could not get users: {}",e)).into())
            }
        };
        let (_cursor, result) = cursor.map_and_drop(|row| {
            let (name, collection, last_modified) = mysql_async::from_row(row);
            Collection {
                name,
                collection,
                last_modified,
            }
        }).await.unwrap();

        Ok(result)
    }

    pub async fn get_user_bsos(&self, user: &User, bso_num: &u8) -> ApiResult<Vec<Bso>> {
        // BSOs
        let bso_sql = "
        SELECT
            collections.name, bso.collection,
            bso.id, bso.ttl, bso.modified, bso.payload, bso.sortindex
        FROM
            :bso_num as bso,
            collections
        WHERE
            bso.userid = :user_id
                and collections.collectionid = bso.collection
                and bso.ttl > unix_timestamp()
        ORDER BY
            bso.collection, bso.id";
        let conn: mysql_async::Conn = match self
            .pool
            .get_conn()
            .await {
                Ok(v) => v,
                Err(e) => {
                    return Err(ApiErrorKind::Internal(format!("Could not get connection: {}", e)).into())
                }
            };
        let cursor = match conn.prep_exec(bso_sql, params!{
            "bso_num" => bso_num,
            "user_id" => user.uid,
        }).await {
            Ok(v) => v,
            Err(e) => {
                return Err(ApiErrorKind::Internal(format!("Could not get users: {}",e)).into())
            }
        };
        let (_cursor, result) = cursor.map_and_drop(|row| {
            let (col_name, col_id, bso_id, expiry, modify, payload, sort_index) = mysql_async::from_row(row);
            Bso{
                col_name,
                col_id,
                bso_id,
                expiry,
                modify,
                payload,
                sort_index
            }

        }).await.unwrap();

        Ok(result)
    }
}
