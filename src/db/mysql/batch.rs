use diesel::{
    self, dsl::sql, insert_into, sql_types::Integer, update, ExpressionMethods, OptionalExtension,
    QueryDsl, RunQueryDsl, TextExpressionMethods,
};
use serde_json;

use super::{
    models::{MysqlDb, Result},
    schema::batches,
};
use crate::db::{params, results, DbError, DbErrorKind};

/// Rough guesstimate of the maximum reasonable life span of a batch.
pub const BATCH_LIFETIME: i64 = 2 * 60 * 60 * 1000; // 2 hours, in milliseconds

pub fn create(db: &MysqlDb, params: params::CreateBatch) -> Result<results::CreateBatch> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let timestamp = db.timestamp().as_i64();
    let bsos = bsos_to_batch_string(&params.bsos)?;
    insert_into(batches::table)
        .values((
            batches::user_id.eq(&user_id),
            batches::collection_id.eq(&collection_id),
            batches::id.eq(&timestamp),
            batches::bsos.eq(&bsos),
            batches::expiry.eq(timestamp + BATCH_LIFETIME),
        ))
        .execute(&db.conn)?;
    Ok(timestamp)
}

pub fn validate(db: &MysqlDb, params: params::ValidateBatch) -> Result<bool> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let exists = batches::table
        .select(sql::<Integer>("1"))
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&params.id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .get_result::<i32>(&db.conn)
        .optional()?;
    Ok(exists.is_some())
}

pub fn append(db: &MysqlDb, params: params::AppendToBatch) -> Result<()> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    let bsos = bsos_to_batch_string(&params.bsos)?;
    let affected_rows = update(batches::table)
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&params.id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .set(batches::bsos.eq(batches::bsos.concat(&bsos)))
        .execute(&db.conn)?;
    if affected_rows == 1 {
        Ok(())
    } else {
        Err(DbErrorKind::BatchNotFound.into())
    }
}

pub fn get(db: &MysqlDb, params: params::GetBatch) -> Result<Option<results::GetBatch>> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    Ok(batches::table
        .select((batches::id, batches::bsos, batches::expiry))
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&params.id))
        .filter(batches::expiry.gt(&db.timestamp().as_i64()))
        .get_result(&db.conn)
        .optional()?)
}

fn delete(db: &MysqlDb, params: params::DeleteBatch) -> Result<()> {
    let user_id = params.user_id.legacy_id as i32;
    let collection_id = db.get_collection_id(&params.collection)?;
    diesel::delete(batches::table)
        .filter(batches::user_id.eq(&user_id))
        .filter(batches::collection_id.eq(&collection_id))
        .filter(batches::id.eq(&params.id))
        .execute(&db.conn)?;
    Ok(())
}

/// Commits a batch to the bsos table, deleting the batch when succesful
pub fn commit(db: &MysqlDb, params: params::CommitBatch) -> Result<results::CommitBatch> {
    let bsos = batch_string_to_bsos(&params.batch.bsos)?;
    let result = db.post_bsos_sync(params::PostBsos {
        user_id: params.user_id.clone(),
        collection: params.collection.clone(),
        bsos,
        failed: Default::default(),
    });
    delete(
        db,
        params::DeleteBatch {
            user_id: params.user_id,
            collection: params.collection,
            id: params.batch.id,
        },
    )?;
    result
}

/// Deserialize a batch string into bsos
fn batch_string_to_bsos(bsos: &str) -> Result<Vec<params::PostCollectionBso>> {
    bsos.lines()
        .map(|line| {
            serde_json::from_str(line).map_err(|e| {
                DbError::internal(&format!("Couldn't deserialize batch::load_bsos bso: {}", e))
            })
        })
        .collect()
}

/// Serialize bsos into strings separated by newlines
fn bsos_to_batch_string(bsos: &[params::PostCollectionBso]) -> Result<String> {
    let batch_strings: Result<Vec<String>> = bsos
        .iter()
        .map(|bso| {
            serde_json::to_string(bso).map_err(|e| {
                DbError::internal(&format!("Couldn't serialize batch::create bso: {}", e))
            })
        })
        .collect();
    batch_strings.map(|bs| {
        format!(
            "{}{}",
            bs.join("\n"),
            if bsos.is_empty() { "" } else { "\n" }
        )
    })
}

#[macro_export]
macro_rules! batch_db_method {
    ($name:ident, $batch_name:ident, $type:ident) => {
        pub fn $name(&self, params: params::$type) -> Result<results::$type> {
            batch::$batch_name(self, params)
        }
    }
}

#[cfg(test)]
mod test {
    use super::{
        super::test::{db, gbso, hid, postbso},
        *,
    };
    use crate::db::params;

    fn cb(user_id: u32, coll: &str, bsos: Vec<params::PostCollectionBso>) -> params::CreateBatch {
        params::CreateBatch {
            user_id: hid(user_id),
            collection: coll.to_owned(),
            bsos,
        }
    }

    fn vb(user_id: u32, coll: &str, id: i64) -> params::ValidateBatch {
        params::ValidateBatch {
            user_id: hid(user_id),
            collection: coll.to_owned(),
            id,
        }
    }

    fn ab(
        user_id: u32,
        coll: &str,
        id: i64,
        bsos: Vec<params::PostCollectionBso>,
    ) -> params::AppendToBatch {
        params::AppendToBatch {
            user_id: hid(user_id),
            collection: coll.to_owned(),
            id,
            bsos,
        }
    }

    fn gb(user_id: u32, coll: &str, id: i64) -> params::GetBatch {
        params::GetBatch {
            user_id: hid(user_id),
            collection: coll.to_owned(),
            id,
        }
    }

    #[test]
    fn create_delete() -> Result<()> {
        let db = db()?;

        let uid = 1;
        let coll = "clients";
        let id = create(&db, cb(uid, coll, vec![]))?;
        assert!(validate(&db, vb(uid, coll, id))?);
        assert!(!validate(&db, vb(uid, coll, id + 1000))?);

        delete(
            &db,
            params::DeleteBatch {
                user_id: hid(uid),
                collection: coll.to_owned(),
                id,
            },
        )?;
        assert!(!validate(&db, vb(uid, coll, id))?);
        Ok(())
    }

    #[test]
    fn expiry() -> Result<()> {
        let db = db()?;

        let uid = 1;
        let coll = "clients";
        let id = db.with_delta(-(BATCH_LIFETIME + 11), |db| {
            create(&db, cb(uid, coll, vec![]))
        })?;
        assert!(!validate(&db, vb(uid, coll, id))?);
        let result = get(&db, gb(uid, coll, id))?;
        assert!(result.is_none());

        let bsos = vec![postbso("b0", Some("payload 0"), Some(10), None)];
        let result = append(&db, ab(uid, coll, id, bsos));
        match result.unwrap_err().kind() {
            DbErrorKind::BatchNotFound => (),
            _ => assert!(false),
        }
        Ok(())
    }

    #[test]
    fn update() -> Result<()> {
        let db = db()?;

        let uid = 1;
        let coll = "clients";
        let id = create(&db, cb(uid, coll, vec![]))?;
        let batch = get(&db, gb(uid, coll, id))?.unwrap();
        assert_eq!(batch.bsos, "".to_owned());

        let bsos = vec![
            postbso("b0", Some("payload 0"), Some(10), None),
            postbso("b1", Some("payload 1"), Some(1000000000), None),
        ];
        append(&db, ab(uid, coll, id, bsos))?;

        let batch = get(&db, gb(uid, coll, id))?.unwrap();
        assert_ne!(batch.bsos, "".to_owned());
        Ok(())
    }

    #[test]
    fn append_commit() -> Result<()> {
        let db = db()?;

        let uid = 1;
        let coll = "clients";
        let bsos1 = vec![
            postbso("b0", Some("payload 0"), Some(10), None),
            postbso("b1", Some("payload 1"), Some(1000000000), None),
        ];
        let id = create(&db, cb(uid, coll, bsos1))?;

        let bsos2 = vec![postbso("b2", Some("payload 2"), None, Some(1000))];
        append(&db, ab(uid, coll, id, bsos2))?;

        let batch = get(&db, gb(uid, coll, id))?.unwrap();
        let result = commit(
            &db,
            params::CommitBatch {
                user_id: hid(uid),
                collection: coll.to_owned(),
                batch,
            },
        )?;

        assert!(result.success.contains(&"b0".to_owned()));
        assert!(result.success.contains(&"b2".to_owned()));

        let ts = db.get_collection_timestamp_sync(params::GetCollectionTimestamp {
            user_id: hid(uid),
            collection: coll.to_owned(),
        })?;
        assert_eq!(result.modified, ts);

        let bso = db.get_bso_sync(gbso(uid, coll, "b1"))?.unwrap();
        assert_eq!(bso.sortindex, Some(1000000000));
        assert_eq!(bso.payload, "payload 1");
        Ok(())
    }

}
