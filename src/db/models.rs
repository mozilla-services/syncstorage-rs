use std::collections::HashMap;
use std::ops::Deref;

use diesel::{
    delete, dsl::sql, insert_into, replace_into, result::Error as DieselError, sql_query,
    sql_types::Integer, sqlite::SqliteConnection, update, Connection, ConnectionError,
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl, Table,
};

use super::schema::{bso, collections, keyvalues};
use super::util::{last_insert_rowid, ms_since_epoch};

// The default expiry is to never expire. Use 100 years which should be enough
// (in milliseconds)
pub const DEFAULT_BSO_TTL: i64 = 100 * 365 * 24 * 60 * 60 * 1000;

// 2099 ... somebody else's problem by then (I hope)
pub const MAX_TIMESTAMP: i64 = 4070822400000;

pub const STORAGE_LAST_MODIFIED: &'static str = "Storage Last Modified";

#[derive(Default, Debug, Copy, Clone)]
pub struct DBConfig {
    pub cache_size: i64,
}

pub struct DBManager {
    path: String,
    pub(super) conn: SqliteConnection,
    config: DBConfig,
}

#[derive(Debug)]
pub enum Sorting {
    None,
    Newest,
    Oldest,
    Index,
}

#[derive(Debug)]
pub struct BSOs {
    pub bsos: Vec<BSO>, // XXX: naming
    pub more: bool,
    pub offset: i64, // XXX: i64?
}

impl DBManager {
    pub fn new(path: &str, config: DBConfig) -> Result<Self, ConnectionError> {
        Ok(Self {
            path: path.to_owned(),
            conn: SqliteConnection::establish(path)?,
            config,
        })
    }

    pub fn init(&self) -> Result<(), DieselError> {
        let pragmas = vec![
            "PRAGMA page_size=4096;".to_owned(),
            "PRAGMA journal_mode=WAL;".to_owned(),
            format!("PRAGMA cache_size={};", &self.config.cache_size),
        ];
        for pragma in pragmas {
            self.conn.execute(&pragma)?;
        }

        let schema_ver = sql_query("PRAGMA schema_version;").execute(&self.conn)?;
        if schema_ver == 0 {
            self.conn.execute(include_str!("schema.sql"))?;
        }
        Ok(())
    }

    pub fn put_bso(&self, bso: &PutBSO) -> Result<(), DieselError> {
        if bso.payload.is_none() && bso.sortindex.is_none() && bso.ttl.is_none() {
            // XXX: go returns an error here (ErrNothingToDo), and is treated
            // as other errors
            return Ok(());
        }

        // XXX: potentially use sqlite 3.24.0 (2018-06-04) new UPSERT (ON
        // CONFLICT DO)?
        self.conn.transaction(|| {
            let exists = bso::table
                .select(sql::<Integer>("1"))
                .filter(bso::collection_id.eq(&bso.collection_id))
                .filter(bso::id.eq(&bso.id))
                .get_result::<i32>(&self.conn)
                .optional()?
                .is_some();

            if exists {
                update(bso::table)
                    .filter(bso::collection_id.eq(&bso.collection_id))
                    .filter(bso::id.eq(&bso.id))
                    .set(&bso.as_changeset())
                    .execute(&self.conn)?;
            } else {
                let payload = bso.payload.as_ref().map(Deref::deref).unwrap_or_default();
                let sortindex = bso.sortindex.unwrap_or_default();
                let ttl = bso.ttl.unwrap_or(DEFAULT_BSO_TTL);
                insert_into(bso::table)
                    .values((
                        bso::collection_id.eq(&bso.collection_id),
                        bso::id.eq(&bso.id),
                        bso::sortindex.eq(&sortindex),
                        bso::payload.eq(payload),
                        bso::payload_size.eq(payload.len() as i64),
                        bso::last_modified.eq(bso.last_modified),
                        bso::expiry.eq(bso.last_modified + ttl),
                    ))
                    .execute(&self.conn)?;
            }
            self.touch_collection_and_storage(bso.collection_id, bso.last_modified)
        })
    }

    // XXX: limit/offset i64?
    pub fn get_bsos(
        &self,
        collection_id: i64,
        mut ids: &[&str],
        older: i64,
        newer: i64,
        sort: Sorting,
        limit: i64,
        offset: i64,
    ) -> Result<BSOs, DieselError> {
        // XXX: ensure offset/limit/newer are valid

        // XXX: should error out (400 Bad Request) when more than 100
        // are provided (move to validation layer)
        if ids.len() > 100 {
            // spec says only 100 ids at a time
            ids = &ids[0..100];
        }

        let mut query = bso::table
            .select(bso::table::all_columns())
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::last_modified.lt(&older))
            .filter(bso::last_modified.gt(&newer))
            .filter(bso::expiry.gt(ms_since_epoch()))
            .into_boxed();

        if !ids.is_empty() {
            query = query.filter(bso::id.eq_any(ids));
        }

        query = match sort {
            Sorting::Index => query.order(bso::sortindex.desc()),
            Sorting::Newest => query.order(bso::last_modified.desc()),
            Sorting::Oldest => query.order(bso::last_modified.asc()),
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
        let mut bsos = query.load::<BSO>(&self.conn)?;

        let (more, next_offset) = if limit >= 0 && bsos.len() > limit as usize {
            bsos.pop();
            (true, limit + offset)
        } else {
            (false, 0)
        };

        Ok(BSOs {
            bsos,
            more,
            offset: next_offset,
        })
    }

    pub fn get_bso(&self, collection_id: i64, bso_id: &str) -> Result<Option<BSO>, DieselError> {
        bso::table
            .select(bso::table::all_columns())
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(&bso_id))
            .filter(bso::expiry.ge(&ms_since_epoch()))
            .first::<BSO>(&self.conn)
            .optional()
    }

    pub fn get_bso_modified(&self, collection_id: i64, bso_id: &str) -> Result<i64, DieselError> {
        let last_modified = bso::table
            .select(bso::last_modified)
            .filter(bso::collection_id.eq(&collection_id))
            .filter(bso::id.eq(&bso_id))
            .filter(bso::expiry.ge(&ms_since_epoch()))
            .first(&self.conn)
            .optional()?;
        // XXX: returning 0 when not found or prefer Option?
        Ok(last_modified.unwrap_or(0))
    }

    pub fn delete_bso(&self, collection_id: i64, bso_id: &str) -> Result<i64, DieselError> {
        self.delete_bsos(collection_id, &[bso_id])
    }

    pub fn delete_bsos(&self, collection_id: i64, bso_id: &[&str]) -> Result<i64, DieselError> {
        self.conn.transaction(|| {
            delete(bso::table)
                .filter(bso::collection_id.eq(&collection_id))
                .filter(bso::id.eq_any(bso_id))
                .execute(&self.conn)?;

            let last_modified = ms_since_epoch();
            self.touch_collection_and_storage(collection_id, last_modified)?;
            Ok(last_modified)
        })
    }

    pub fn get_collection_modified(&self, collection_id: i64) -> Result<i64, DieselError> {
        let last_modified = collections::table
            .select(collections::last_modified)
            .filter(collections::id.eq(&collection_id))
            .first(&self.conn)
            .optional()?;
        // XXX: returning 0 when not found or prefer Option?
        Ok(last_modified.unwrap_or(0))
    }

    pub fn get_collection_id(&self, name: &str) -> Result<i64, DieselError> {
        match name {
            "clients" => return Ok(1),
            "crypto" => return Ok(2),
            "forms" => return Ok(3),
            "history" => return Ok(4),
            "keys" => return Ok(5),
            "meta" => return Ok(6),
            "bookmarks" => return Ok(7),
            "prefs" => return Ok(8),
            "tabs" => return Ok(9),
            "passwords" => return Ok(10),
            "addons" => return Ok(11),
            "addresses" => return Ok(12),
            "creditcards" => return Ok(13),
            _ => collections::table
                .select(collections::id)
                .filter(collections::name.eq(name))
                .first(&self.conn),
        }
    }

    /// Implied that this is called within a transaction
    pub fn touch_collection_and_storage(
        &self,
        collection_id: i64,
        last_modified: i64,
    ) -> Result<(), DieselError> {
        self.touch_collection(collection_id, last_modified)?;
        self.touch_storage(last_modified)
    }

    fn touch_collection(&self, collection_id: i64, last_modified: i64) -> Result<(), DieselError> {
        update(collections::table)
            .filter(collections::id.eq(&collection_id))
            .set(collections::last_modified.eq(&last_modified))
            .execute(&self.conn)
            .map(|_| ())
    }

    fn touch_storage(&self, last_modified: i64) -> Result<(), DieselError> {
        self.set_key(STORAGE_LAST_MODIFIED, &last_modified.to_string())
    }

    pub(super) fn set_key(&self, key: &'static str, value: &str) -> Result<(), DieselError> {
        // XXX: go code ignored these errors..
        replace_into(keyvalues::table)
            .values((keyvalues::key.eq(key), keyvalues::value.eq(&value)))
            .execute(&self.conn)
            .map(|_| ())
    }

    pub(super) fn get_key(&self, key: &'static str) -> Result<Option<String>, DieselError> {
        keyvalues::table
            .select(keyvalues::value)
            .filter(keyvalues::key.eq(key))
            .first(&self.conn)
            .optional()
    }

    pub fn create_collection(&self, name: &str) -> Result<i64, DieselError> {
        // XXX: CollectionNameOk
        let collection_id = self.conn.transaction(|| {
            insert_into(collections::table)
                .values((
                    collections::name.eq(name),
                    collections::last_modified.eq(ms_since_epoch()),
                ))
                .execute(&self.conn)?;
            collections::table
                .select(last_insert_rowid)
                .first(&self.conn)
        })?;
        Ok(collection_id)
    }

    pub fn delete_collection(&self, collection_id: i64) -> Result<i64, DieselError> {
        self.conn.transaction(|| {
            delete(bso::table.filter(bso::collection_id.eq(&collection_id))).execute(&self.conn)?;

            self.touch_collection(collection_id, 0)?;
            let last_modified = ms_since_epoch();
            self.touch_storage(last_modified)?;
            Ok(last_modified)
        })
    }

    pub fn info_collections(&self) -> Result<HashMap<String, i64>, DieselError> {
        Ok(collections::table
            .select((collections::name, collections::last_modified))
            .filter(collections::last_modified.ne(0))
            .load::<(String, i64)>(&self.conn)?
            .into_iter()
            .collect())
    }

    pub fn last_modified(&self) -> Result<i64, DieselError> {
        Ok(self.get_key(STORAGE_LAST_MODIFIED)?
            .map_or(0, |last_modified| last_modified.parse().unwrap()))
    }
}

/// BSO records from the DB
#[derive(Debug, Queryable)]
pub struct BSO {
    pub collection_id: i64,
    pub id: String,
    pub sortindex: Option<i64>,
    pub payload: String,
    pub payload_size: i64,
    pub last_modified: i64,
    pub expiry: i64,
}

/// A PUT of a BSO
#[derive(Clone, Debug)]
pub struct PutBSO {
    pub collection_id: i64,
    pub id: String,
    pub sortindex: Option<i64>,
    pub payload: Option<String>,
    pub last_modified: i64,
    pub ttl: Option<i64>,
}

impl PutBSO {
    fn as_changeset<'a>(&'a self) -> UpdateBSO<'a> {
        UpdateBSO {
            sortindex: self.sortindex,
            expiry: self.ttl.map(|ttl| self.last_modified + ttl),
            payload: self.payload.as_ref(),
            payload_size: self.payload.as_ref().map(|payload| payload.len() as i64),
            last_modified: if self.payload.is_some() || self.sortindex.is_some() {
                Some(self.last_modified)
            } else {
                None
            },
        }
    }
}

/// Formats a BSO for UPDATEs
#[derive(AsChangeset)]
#[table_name = "bso"]
struct UpdateBSO<'a> {
    pub sortindex: Option<i64>,
    pub payload: Option<&'a String>,
    pub payload_size: Option<i64>,
    pub last_modified: Option<i64>,
    pub expiry: Option<i64>,
}
