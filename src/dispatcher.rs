//! `Dispatcher` is a command dispatching actor that distributes commands to the appropriate
//! actor for a given user. If an actor for that user is no longer active, it creates and
//! initializes the actor before dispatching the command.
use std::sync::{Arc, Mutex, RwLock};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use actix::{Actor, Addr, Context, SyncContext, Handler, Message};

use db::models::{BSO, DBConfig, DBManager};

// Messages that can be sent to the user
#[derive(Default)]
pub struct CollectionInfo {
    pub user_id: String,
}

impl Message for CollectionInfo {
    type Result = Result<HashMap<String, String>, Error>;
}

#[derive(Default)]
pub struct GetBso {
    pub user_id: String,
    pub collection: String,
    pub bso_id: String,
}

impl Message for GetBso {
    type Result = Result<Option<BSO>, Error>;
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

impl Handler<GetBso> for DBExecutor {
    type Result = Result<Option<BSO>, Error>;

    fn handle(&mut self, msg: GetBso, _: &mut Self::Context) -> Self::Result {
        self
            .db_handles
            .read()
            .map_err(|error| Error::new(ErrorKind::Other, "db handles lock error"))
            .and_then(|db_handles| {
                db_handles
                    .get(&msg.user_id)
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, "unknown user"))
                    .and_then(|mutex| {
                        mutex
                            .lock()
                            .map_err(|error| Error::new(ErrorKind::Other, "db manager mutex error"))
                            .and_then(|db_manager| {
                                db_manager
                                    .get_collection_id(&msg.collection)
                                    .map_err(|error| Error::new(ErrorKind::Other, error))
                                    .and_then(|collection_id| {
                                        db_manager
                                            .get_bso(collection_id, &msg.bso_id)
                                            .map_err(|error| Error::new(ErrorKind::Other, error))
                                    })
                            })
                    })
            })
    }
}

impl Actor for DBExecutor {
    type Context = SyncContext<Self>;
}
