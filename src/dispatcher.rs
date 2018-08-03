//! `Dispatcher` is a command dispatching actor that distributes commands to the appropriate
//! actor for a given user. If an actor for that user is no longer active, it creates and
//! initializes the actor before dispatching the command.
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

use actix::{Actor, Addr, Context, Handler, Message, SyncContext};

use db::models::{DBConfig, DBManager, PutBSO, BSO};
use db::util::ms_since_epoch;

macro_rules! uid_messages {
    ($($message:ident),+) => ($(
        #[derive(Default)]
        pub struct $message {
            pub user_id: String,
        }

        impl Message for $message {
            type Result = <DBExecutor as Handler<$message>>::Result;
        }
    )+)
}

uid_messages! {
    Collections,
    CollectionCounts,
    CollectionUsage,
    Configuration,
    Quota
}

macro_rules! bso_messages {
    ($($message:ident {$($property:ident: $type:ty),*}),+) => ($(
        #[derive(Clone, Default)]
        pub struct $message {
            pub user_id: String,
            pub collection: String,
            pub bso_id: String,
            $(pub $property: $type),*
        }

        impl Message for $message {
            type Result = <DBExecutor as Handler<$message>>::Result;
        }
    )+)
}

bso_messages! {
    DeleteBso {},
    GetBso {},
    PutBso {
        sortindex: Option<i64>,
        payload: Option<String>,
        ttl: Option<i64>
    }
}

pub struct DBExecutor {
    pub db_handles: Arc<RwLock<HashMap<String, Mutex<DBManager>>>>,
}

impl DBExecutor {
    fn execute<R>(
        &self,
        user_id: &str,
        action: &Fn(MutexGuard<DBManager>) -> Result<R, Error>,
    ) -> Result<R, Error> {
        self.db_handles
            .read()
            .map_err(|error| Error::new(ErrorKind::Other, "db handles lock error"))
            .and_then(|db_handles| {
                db_handles
                    .get(user_id)
                    .ok_or_else(|| Error::new(ErrorKind::NotFound, "unknown user"))
                    .and_then(|mutex| {
                        mutex
                            .lock()
                            .map_err(|error| Error::new(ErrorKind::Other, "db manager mutex error"))
                            .and_then(action)
                    })
            })
    }
}

impl Handler<Collections> for DBExecutor {
    type Result = Result<HashMap<String, String>, Error>;

    fn handle(&mut self, msg: Collections, _: &mut Self::Context) -> Self::Result {
        Ok(HashMap::new())
    }
}

impl Handler<CollectionCounts> for DBExecutor {
    type Result = Result<HashMap<String, u64>, Error>;

    fn handle(&mut self, msg: CollectionCounts, _: &mut Self::Context) -> Self::Result {
        Ok(HashMap::new())
    }
}

impl Handler<CollectionUsage> for DBExecutor {
    type Result = Result<HashMap<String, u32>, Error>;

    fn handle(&mut self, msg: CollectionUsage, _: &mut Self::Context) -> Self::Result {
        Ok(HashMap::new())
    }
}

impl Handler<Configuration> for DBExecutor {
    type Result = Result<HashMap<String, u64>, Error>;

    fn handle(&mut self, msg: Configuration, _: &mut Self::Context) -> Self::Result {
        Ok(HashMap::new())
    }
}

impl Handler<Quota> for DBExecutor {
    type Result = Result<Vec<Option<u32>>, Error>;

    fn handle(&mut self, msg: Quota, _: &mut Self::Context) -> Self::Result {
        Ok(vec![Some(0), None])
    }
}

impl Handler<DeleteBso> for DBExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: DeleteBso, _: &mut Self::Context) -> Self::Result {
        Ok(())
    }
}

impl Handler<GetBso> for DBExecutor {
    type Result = Result<Option<BSO>, Error>;

    fn handle(&mut self, msg: GetBso, _: &mut Self::Context) -> Self::Result {
        self.execute(&msg.user_id, &|db_manager| {
            db_manager
                .get_collection_id(&msg.collection)
                .map_err(|error| Error::new(ErrorKind::Other, error))
                .and_then(|collection_id| {
                    db_manager
                        .get_bso(collection_id, &msg.bso_id)
                        .map_err(|error| Error::new(ErrorKind::Other, error))
                })
        })
    }
}

impl Handler<PutBso> for DBExecutor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: PutBso, _: &mut Self::Context) -> Self::Result {
        self.execute(&msg.user_id, &|db_manager| {
            // error[E0507]: cannot move out of captured outer variable in an `Fn` closure
            let msg = msg.clone();
            db_manager
                .get_collection_id(&msg.collection)
                .map_err(|error| Error::new(ErrorKind::Other, error))
                .and_then(|collection_id| {
                    db_manager
                        .put_bso(&PutBSO {
                            collection_id,
                            id: msg.bso_id,
                            sortindex: msg.sortindex,
                            payload: msg.payload,
                            last_modified: ms_since_epoch(),
                            ttl: msg.ttl,
                        })
                        .map_err(|error| Error::new(ErrorKind::Other, error))
                })
        })
    }
}

impl Actor for DBExecutor {
    type Context = SyncContext<Self>;
}
