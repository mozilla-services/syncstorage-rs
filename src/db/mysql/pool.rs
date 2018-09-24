use std::{collections::HashMap, sync::RwLock};

use diesel::{
    mysql::MysqlConnection,
    r2d2::{ConnectionManager, Pool},
};

use super::models::{MysqlDb, Result};
#[cfg(test)]
use super::test::TestTransactionCustomizer;
use db::{error::DbError, STD_COLLS};
use settings::Settings;

pub struct MysqlDbPool {
    pool: Pool<ConnectionManager<MysqlConnection>>,
    /// In-memory cache of collection_ids and their names
    coll_cache: CollectionCache,
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
            coll_cache: Default::default(),
        })
    }

    pub fn get(&self) -> Result<MysqlDb> {
        Ok(MysqlDb::new(self.pool.get()?, &self.coll_cache))
    }
}

pub struct CollectionCache {
    pub by_name: RwLock<HashMap<String, i32>>,
    pub by_id: RwLock<HashMap<i32, String>>,
}

impl CollectionCache {
    pub fn put(&self, id: i32, name: String) -> Result<()> {
        // XXX: should probably either lock both simultaneously during
        // writes or use an RwLock alternative
        self.by_name
            .write()
            .map_err(|_| DbError::internal("by_name write"))?
            .insert(name.clone(), id);
        self.by_id
            .write()
            .map_err(|_| DbError::internal("by_id write"))?
            .insert(id, name);
        Ok(())
    }

    pub fn get_id(&self, name: &str) -> Result<Option<i32>> {
        Ok(self
            .by_name
            .read()
            .map_err(|_| DbError::internal("by_name read"))?
            .get(name)
            .cloned())
    }

    pub fn get_name(&self, id: i32) -> Result<Option<String>> {
        Ok(self
            .by_id
            .read()
            .map_err(|_| DbError::internal("by_id read"))?
            .get(&id)
            .cloned())
    }
}

impl Default for CollectionCache {
    fn default() -> Self {
        Self {
            by_name: RwLock::new(
                STD_COLLS
                    .iter()
                    .map(|(k, v)| ((*v).to_owned(), *k))
                    .collect(),
            ),
            by_id: RwLock::new(
                STD_COLLS
                    .iter()
                    .map(|(k, v)| (*k, (*v).to_owned()))
                    .collect(),
            ),
        }
    }
}
