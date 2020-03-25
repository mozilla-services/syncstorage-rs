use std::ops::Range;

use structopt::StructOpt;

mod db;
mod error;
mod logging;
mod settings;
mod fxa;

fn main() {
    let settings = settings::Settings::from_args();

    // TODO: set logging level
    logging::init_logging(settings.human_logs).unwrap();
    // create the database connections
    let mut dbs = db::Dbs::connect(&settings).unwrap();
    // TODO:read in fxa_info file (todo: make db?)
    let fxa = fxa::FxaInfo::new(&settings).unwrap();
    // reconcile collections
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
    for bso_num in range {
        let users = &dbs.get_users(&bso_num, &fxa).unwrap();
        // divvy up users;
        for user in users {
            dbs.move_user(user, &bso_num, &collections).unwrap();
        }
    }
}
