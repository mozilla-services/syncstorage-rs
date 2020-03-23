use std::fs::File;
use std::io::{BufReader, Error};
use std::sync::Arc;
use std::ops::Range;

use futures::executor::block_on;

use serde::{de::Deserialize, Serialize};
use std::path::PathBuf;
use structopt::StructOpt;
use url::Url;

mod db;
mod error;
mod logging;
mod settings;
mod fxa;

fn main() {
    let settings = settings::Settings::from_args();

    // TODO: set logging level
    logging::init_logging(settings.human_logs);
    // create the database connections
    let dbs = Arc::new(db::Dbs::connect(&settings).unwrap());
    // TODO:read in fxa_info file (todo: make db?)
    let fxa = fxa::FxaInfo::new(&settings).unwrap();
    // TODO: dbs.reconcile_collections()?.await;
    let collections = db::collections::Collections::new(&settings, &dbs).unwrap();
    // let users = dbs.get_users(&settings, &fxa)?.await;
    let mut start_bso = &settings.start_bso.unwrap_or(0);
    let mut end_bso = &settings.end_bso.unwrap_or(19);
    let suser = &settings.user.clone();
    if let Some(user) = suser {
        start_bso = &user.bso;
        end_bso = &user.bso;
    }

    let range = Range{ start:start_bso.clone(), end:end_bso.clone()};
    for bso in range {
        let users = &dbs.get_users(bso, &fxa).unwrap();
        // divvy up users;
        for user in users {
            dbs.move_user(user, bso).unwrap();
        }
    }
}
