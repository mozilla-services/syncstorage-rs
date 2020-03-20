use std::fs::File;
use std::io::{BufReader, Error};

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
    let dbs = db::Dbs::connect(&settings).unwrap();
    // TODO:read in fxa_info file (todo: make db?)
    let fxa = fxa::FxaInfo::new(&settings).unwrap();
    // TODO: dbs.reconcile_collections()?.await;
    // let users = dbs.get_users(&settings, &fxa)?.await;
    // for bso in [start..end] {
    //      dbs.move_user(&fxa, &users, &bso)?;
    //  }
}
