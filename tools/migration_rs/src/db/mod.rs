pub mod mysql;
pub mod spanner;
pub mod collections;

use futures::executor::block_on;

use crate::error::{ApiError, ApiResult};
use crate::settings::Settings;
use crate::fxa::{FxaInfo, FxaData};
use crate::db::collections::Collections;

pub const DB_THREAD_POOL_SIZE: usize = 50;

pub struct Dbs {
    mysql: mysql::MysqlDb,
    spanner: spanner::Spanner,
}

pub struct Bso {
    col_name: String,
    col_id: u64,
    bso_id: u64,
    expiry: u64,
    modify: u64,
    payload: Vec<u8>,
    sort_index: Option<u64>,
}

pub struct User {
    uid: u64,
    fxa_data: FxaData,
}

pub struct UserData {
    collections: Option<Collections>,
    bsos: Option<Vec<Bso>>
}

impl Dbs {
    pub fn connect(settings: &Settings) -> ApiResult<Dbs> {
        Ok(Self {
            mysql: mysql::MysqlDb::new(&settings)?,
            spanner: spanner::Spanner::new(&settings)?,
        })
    }

    pub fn get_users(&self, bso_num:u8, fxa: &FxaInfo) -> ApiResult<Vec<User>> {
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

    pub fn move_user(&self, user: &User, bso: u8) -> ApiResult<()> {
        let userdata = block_on(self.mysql.get_user_data(user, bso)).unwrap();
        for user in userdata {
            //TOOD add abort stuff
            //TODO add divvy up
            // TODO: finish update_user
            match block_on(self.spanner.update_user(user)){
                Ok(_) => {},
                /*
                Err(ApiError.kind(ApiErrorKind::AlreadyExists)) ||
                Err(ApiError.kind(ApiErrorKind::InvalidArgument)) => {
                    // already exists, so skip
                },
                */
                Err(e) => {panic!("Unknown Error: {}", e)}
            };
        }
        Ok(())
    }
}
