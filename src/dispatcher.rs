//! `Dispatcher` is a command dispatching actor that distributes commands to the appropriate
//! actor for a given user. If an actor for that user is no longer active, it creates and
//! initializes the actor before dispatching the command.
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::io::Error;

use actix::{Actor, Addr, Context, SyncContext, Handler, Message};

use db::models::{DBConfig, DBManager};

// Messages that can be sent to the user
#[derive(Default)]
pub struct CollectionInfo {
    pub user_id: String,
}

impl Message for CollectionInfo {
    type Result = Result<HashMap<String, String>, Error>;
}

pub struct DBExecutor {
    pub db_handles: Arc<RwLock<HashMap<String, Mutex<DBManager>>>>,
}

impl Handler<CollectionInfo> for DBExecutor {
    type Result = Result<HashMap<String, String>, Error>;

    fn handle(&mut self, msg: CollectionInfo, _: &mut Self::Context) -> Self::Result {
        Ok(HashMap::new())
    }
}

impl Actor for DBExecutor {
    type Context = SyncContext<Self>;
}
