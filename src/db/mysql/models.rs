use std::{self, ops::Deref};

use diesel::{
    insert_into,
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool, PooledConnection},
    sql_types::{BigInt, Integer, Text},
    update, Connection, ExpressionMethods, OptionalExtension,
};
use diesel::{sql_query, QueryDsl, RunQueryDsl};
#[cfg(test)]
use diesel_logger::LoggingConnection;
use futures::future;

use super::schema::{bso, collections, user_collections};
#[cfg(test)]
use super::test::TestTransactionCustomizer;
use db::{
    error::DbError, get_std_collection_id, get_std_collection_name, params, results,
    util::ms_since_epoch, Db, DbFuture, Sorting,
};
use settings::Settings;

embed_migrations!();

pub type Result<T> = std::result::Result<T, DbError>;

// The ttl to use for rows that are never supposed to expire (in seconds)
pub const DEFAULT_BSO_TTL: u32 = 2100000000;

no_arg_sql_function!(last_insert_id, Integer);

/// Run the diesel embedded migrations
///
/// Mysql DDL statements implicitly commit which could disrupt MysqlPool's
/// begin_test_transaction during tests. So this runs on its own separate conn.
pub fn run_embedded_migrations(settings: &Settings) -> Result<()> {
    let conn = MysqlConnection::establish(&settings.database_url).unwrap();
    Ok(embedded_migrations::run(&conn)?)
}

pub struct MysqlDbPool {
    pool: Pool<ConnectionManager<MysqlConnection>>,
}

// XXX: to become a db::DbPool trait
impl MysqlDbPool {
    pub fn new(settings: &Settings) -> Result<Self> {
        let manager = ConnectionManager::<MysqlConnection>::new(settings.database_url.as_ref());
        let mut builder = Pool::builder().max_size(settings.database_pool_max_size.unwrap_or(10));
        #[cfg(test)]
        {
            if settings.database_use_test_transactions {
                builder = builder.connection_customizer(Box::new(TestTransactionCustomizer));
            }
        }
        Ok(Self {
            pool: builder.build(manager)?,
        })
    }

    pub fn get(&self) -> Result<MysqlDb> {
        Ok(MysqlDb {
            #[cfg(not(test))]
            conn: self.pool.get()?,
            #[cfg(test)]
            conn: LoggingConnection::new(self.pool.get()?),
        })
    }
}

pub struct MysqlDb {
    #[cfg(not(test))]
    pub(super) conn: PooledConnection<ConnectionManager<MysqlConnection>>,
    #[cfg(test)]
    pub(super) conn: LoggingConnection<PooledConnection<ConnectionManager<MysqlConnection>>>,
}

impl MysqlDb {
    pub fn get_collection_id_sync(
        &self,
        params: &params::GetCollectionId,
    ) -> Result<results::GetCollectionId> {
        let id = if let Some(id) = get_std_collection_id(params) {
            id
        } else {
            sql_query("SELECT id FROM collections WHERE name = ?")
                .bind::<Text, _>(params)
                .get_result::<IdResult>(&self.conn)?
                .id
        };
        Ok(id)
    }

    pub fn get_collections_sync(
        &self,
        params: &params::GetCollections,
    ) -> Result<results::GetCollections> {
        sql_query("SELECT collection_id, modified FROM user_collections WHERE user_id = ?")
            .bind::<Integer, _>(params.user_id as i32)
            .load::<UserCollectionsResult>(&self.conn)?
            .into_iter()
            .map(|cr| {
                self.get_collection_name(cr.id)
                    .map(|name| (name, cr.modified))
            })
            .collect()
    }

    pub fn create_collection_sync(&self, name: &str) -> Result<i32> {
        // XXX: handle concurrent attempts at inserts
        let collection_id = self.conn.transaction(|| {
            sql_query("INSERT INTO collections (name) VALUES (?)")
                .bind::<Text, _>(name)
                .execute(&self.conn)?;
            collections::table.select(last_insert_id).first(&self.conn)
        })?;
        Ok(collection_id)
    }

    fn get_collection_name(&self, id: i32) -> Result<String> {
        // XXX: python caches collection names/ids in memory as they're added
        let name = if let Some(name) = get_std_collection_name(id) {
            name.to_owned()
        } else {
            sql_query("SELECT name FROM collections where id = ?")
                .bind::<Integer, _>(&id)
                .get_result::<NameResult>(&self.conn)?
                .name
        };
        Ok(name)
    }

    pub fn put_bso_sync(&self, bso: &params::PutBso) -> Result<results::PutBso> {
        /*
        if bso.payload.is_none() && bso.sortindex.is_none() && bso.ttl.is_none() {
            // XXX: go returns an error here (ErrNothingToDo), and is treated
            // as other errors
            return Ok(());
        }
        */

        // XXX: consider mysql ON DUPLICATE KEY UPDATE?
        self.conn.transaction(|| {
            let q = r#"
                SELECT 1 as count FROM bso
                WHERE user_id = ? AND collection_id = ? AND id = ?
            "#;
            let exists = sql_query(q)
                .bind::<Integer, _>(bso.user_id as i32) // XXX:
                .bind::<Integer, _>(&bso.collection_id)
                .bind::<Text, _>(&bso.id)
                .get_result::<Count>(&self.conn)
                .optional()?
                .is_some();

            if exists {
                update(bso::table)
                    .filter(bso::user_id.eq(bso.user_id as i32)) // XXX:
                    .filter(bso::collection_id.eq(&bso.collection_id))
                    .filter(bso::id.eq(&bso.id))
                    .set(put_bso_as_changeset(&bso))
                    .execute(&self.conn)?;
            } else {
                let payload = bso.payload.as_ref().map(Deref::deref).unwrap_or_default();
                let sortindex = bso.sortindex;
                let ttl = bso.ttl.map_or(DEFAULT_BSO_TTL, |ttl| ttl);
                insert_into(bso::table)
                    .values((
                        bso::user_id.eq(bso.user_id as i32), // XXX:
                        bso::collection_id.eq(&bso.collection_id),
                        bso::id.eq(&bso.id),
                        bso::sortindex.eq(sortindex),
                        bso::payload.eq(payload),
                        bso::payload_size.eq(payload.len() as i32), // XXX:
                        bso::modified.eq(bso.modified),
                        bso::expiry.eq(bso.modified + ttl as i64),
                    ))
                    .execute(&self.conn)?;
            }
            self.touch_collection(bso.user_id as i32, bso.collection_id, bso.modified)?;
            // XXX:
            Ok(bso.modified as u64)
        })
    }

    // XXX: limit/offset i64?
    pub fn get_bsos_sync(
        &self,
        user_id: u32,
        collection_id: i32,
        mut ids: &[&str],
        older: u64,
        newer: u64,
        sort: Sorting,
        limit: i64,
        offset: i64,
    ) -> Result<results::BSOs> {
        // XXX: ensure offset/limit/newer are valid

        // XXX: should error out (400 Bad Request) when more than 100
        // are provided (move to validation layer)
        if ids.len() > 100 {
            // spec says only 100 ids at a time
            ids = &ids[0..100];
        }

        // XXX: convert to raw SQL for use by other backends
        let mut query = bso::table
            //.select(bso::table::all_columns())
            .select((bso::id, bso::modified, bso::payload, bso::sortindex, bso::expiry))
            .filter(bso::user_id.eq(user_id as i32)) // XXX:
            .filter(bso::collection_id.eq(collection_id as i32)) // XXX:
            .filter(bso::modified.lt(older as i64))
            .filter(bso::modified.gt(newer as i64))
            .filter(bso::expiry.gt(ms_since_epoch()))
            .into_boxed();

        if !ids.is_empty() {
            query = query.filter(bso::id.eq_any(ids));
        }

        query = match sort {
            Sorting::Index => query.order(bso::sortindex.desc()),
            Sorting::Newest => query.order(bso::modified.desc()),
            Sorting::Oldest => query.order(bso::modified.asc()),
            _ => query,
        };

        // fetch an extra row to detect if there are more rows that
        // match the query conditions
        query = query.limit(if limit >= 0 { limit + 1 } else { limit });
        if offset != 0 {
            // XXX: copy over this optimization:
            // https://github.com/mozilla-services/server-syncstorage/blob/a0f8117/syncstorage/storage/sql/__init__.py#L404
            query = query.offset(offset);
        }
        let mut bsos = query.load::<results::GetBso>(&self.conn)?;

        let (more, next_offset) = if limit >= 0 && bsos.len() > limit as usize {
            bsos.pop();
            (true, limit + offset)
        } else {
            (false, 0)
        };

        Ok(results::BSOs {
            bsos,
            more,
            offset: next_offset,
        })
    }

    pub fn get_bso_sync(&self, params: &params::GetBso) -> Result<Option<results::GetBso>> {
        Ok(sql_query(r#"
               SELECT id, modified, payload, sortindex, expiry FROM bso
               WHERE user_id = ? AND collection_id = ? AND id = ? AND expiry >= ?
           "#)
           .bind::<Integer, _>(params.user_id as i32) // XXX:
           .bind::<Integer, _>(&params.collection_id)
           .bind::<Text, _>(&params.id)
           .bind::<Integer, _>(ms_since_epoch() as i32) // XXX:
           .get_result::<results::GetBso>(&self.conn)
           .optional()?)
    }

    fn touch_collection(&self, user_id: i32, collection_id: i32, modified: i64) -> Result<()> {
        // XXX: ensure transaction
        // The common case will be an UPDATE, so try that first
        let affected_rows = update(user_collections::table)
            .filter(user_collections::user_id.eq(&user_id))
            .filter(user_collections::collection_id.eq(&collection_id))
            .set(user_collections::modified.eq(&modified))
            .execute(&self.conn)?;
        if affected_rows != 1 {
            insert_into(user_collections::table)
                .values((
                    user_collections::user_id.eq(&user_id),
                    user_collections::collection_id.eq(&collection_id),
                    user_collections::modified.eq(&modified),
                ))
                .execute(&self.conn)?;
        }
        Ok(())
    }
}

impl Db for MysqlDb {
    mock_db_method!(get_collection_id, GetCollectionId);
    mock_db_method!(get_collections, GetCollections);
    mock_db_method!(get_collection_counts, GetCollectionCounts);
    mock_db_method!(get_collection_usage, GetCollectionUsage);
    mock_db_method!(get_quota, GetQuota);
    mock_db_method!(delete_all, DeleteAll);
    mock_db_method!(delete_collection, DeleteCollection);
    mock_db_method!(get_collection, GetCollection);
    mock_db_method!(post_collection, PostCollection);
    mock_db_method!(delete_bso, DeleteBso);
    mock_db_method!(get_bso, GetBso);
    mock_db_method!(put_bso, PutBso);
}

#[derive(Debug, QueryableByName)]
struct IdResult {
    #[sql_type = "Integer"]
    id: i32,
}

#[derive(Debug, QueryableByName)]
struct NameResult {
    #[sql_type = "Text"]
    name: String,
}

#[derive(Debug, QueryableByName)]
struct UserCollectionsResult {
    #[sql_type = "Integer"]
    id: i32,
    #[sql_type = "BigInt"]
    modified: i64,
}

#[derive(Debug, QueryableByName)]
struct Count {
    #[sql_type = "BigInt"]
    count: i64,
}

/// Formats a BSO for UPDATEs
#[derive(AsChangeset)]
#[table_name = "bso"]
struct UpdateBSO<'a> {
    pub sortindex: Option<i32>,
    pub payload: Option<&'a String>,
    pub payload_size: Option<i32>,
    pub modified: Option<i64>,
    pub expiry: Option<i64>,
}

fn put_bso_as_changeset<'a>(bso: &'a params::PutBso) -> UpdateBSO<'a> {
    UpdateBSO {
        sortindex: bso.sortindex,
        expiry: bso.ttl.map(|ttl| bso.modified + ttl as i64),
        payload: bso.payload.as_ref(),
        payload_size: bso.payload.as_ref().map(|payload| payload.len() as i32), // XXX:
        modified: if bso.payload.is_some() || bso.sortindex.is_some() {
            Some(bso.modified)
        } else {
            None
        },
    }
}
