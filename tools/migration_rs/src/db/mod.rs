pub mod mysql;
pub mod spanner;
pub mod collections;

use futures::executor::block_on;

use crate::error::{ApiResult};
use crate::settings::Settings;
use crate::fxa::{FxaInfo, FxaData};
use crate::db::collections::Collections;

pub struct Dbs {
    settings: Settings,
    mysql: mysql::MysqlDb,
    spanner: spanner::Spanner,
}

pub struct Bso {
    col_name: String,
    col_id: u16,
    bso_id: u64,
    expiry: u64,
    modify: u64,
    payload: String,
    sort_index: Option<u64>,
}

pub struct User {
    uid: u64,
    fxa_data: FxaData,
}

impl Dbs {
    pub fn connect(settings: &Settings) -> ApiResult<Dbs> {
        Ok(Self {
            settings: settings.clone(),
            mysql: mysql::MysqlDb::new(&settings)?,
            spanner: spanner::Spanner::new(&settings)?,
        })
    }

    pub fn get_users(&self, bso_num:&u8, fxa: &FxaInfo) -> ApiResult<Vec<User>> {
        let mut result: Vec<User> = Vec::new();
        for uid in block_on(self.mysql.get_user_ids(bso_num)).unwrap() {
                if let Some(fxa_data) = fxa.get_fxa_data(&uid) {
                result.push(User{
                    uid,
                    fxa_data,
                })
            }
        };
        Ok(result)
    }

    pub fn move_user(&mut self, user: &User, bso_num: &u8, collections: &Collections) -> ApiResult<()> {
        // move user collections
        let user_collections = block_on(self.mysql.get_user_collections(user, bso_num)).unwrap();
        block_on(self.spanner.load_user_collections(user, user_collections)).unwrap();

        // fetch and handle the user BSOs
        let bsos = block_on(self.mysql.get_user_bsos(user, bso_num)).unwrap();
        // divvy up according to the readchunk
        let blocks = bsos.windows(self.settings.readchunk.unwrap_or(1000) as usize);
        for block in blocks {
            // TODO add abort stuff
            match block_on(self.spanner.add_user_bsos(user, block, &collections)){
                Ok(_) => {},
                Err(e) => {panic!("Unknown Error: {}", e)}
            };
        }
        Ok(())
    }
}
