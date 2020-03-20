//! Application settings objects and initialization
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;

use structopt::StructOpt;
use url::Url;

use crate::error::{ApiError, ApiErrorKind};

static DEFAULT_CHUNK_SIZE: u64 = 1_500_000;
static DEFAULT_READ_CHUNK: u64 = 1_000;
static DEFAULT_OFFSET: u64 = 0;
static DEFAULT_START_BSO: u64 = 0;
static DEFAULT_END_BSO: u64 = 19;
static DEFAULT_FXA_FILE: &str = "users.csv";
static DEFAULT_SPANNER_POOL_SIZE: usize = 32;

#[derive(Clone, Debug)]
pub struct Dsns {
    pub mysql: Option<String>,
    pub spanner: Option<String>,
}

impl Default for Dsns {
    fn default() -> Self {
        Dsns {
            mysql: None,
            spanner: None,
        }
    }
}
impl Dsns {
    fn from_str(raw: &str) -> Result<Self, ApiError> {
        let mut result = Self::default();
        let buffer = BufReader::new(File::open(raw)?);
        for line in buffer.lines().map(|l| l.unwrap()) {
            let url = Url::parse(&line).expect("Invalid DSN url");
            match url.scheme() {
                "mysql" => result.mysql = Some(line),
                "spanner" => result.spanner = Some(line),
                _ => {}
            }
        }
        Ok(result)
    }
}

#[derive(Clone, Debug)]
pub struct User {
    pub bso: String,
    pub user_id: Vec<String>,
}

impl User {
    fn from_str(raw: &str) -> Result<User, ApiError> {
        let parts: Vec<&str> = raw.splitn(2, ':').collect();
        if parts.len() == 1 {
            return Err(ApiErrorKind::Internal("bad user option".to_owned()).into());
        }
        let bso = String::from(parts[0]);
        let s_ids = parts[1].split(',').collect::<Vec<&str>>();
        let mut user_id: Vec<String> = Vec::new();
        for id in s_ids {
            user_id.push(id.to_owned());
        }

        Ok(User { bso, user_id })
    }
}

#[derive(Clone, Debug)]
pub struct Abort {
    pub bso: String,
    pub count: u64,
}

impl Abort {
    fn from_str(raw: &str) -> Result<Self, ApiError> {
        let parts: Vec<&str> = raw.splitn(2, ':').collect();
        if parts.len() == 1 {
            return Err(ApiErrorKind::Internal("Bad abort option".to_owned()).into());
        }
        Ok(Abort {
            bso: String::from(parts[0]),
            count: u64::from_str(parts[1]).expect("Bad count for Abort"),
        })
    }
}

#[derive(Clone, Debug)]
pub struct UserRange {
    pub offset: u64,
    pub limit: u64,
}

impl UserRange {
    fn from_str(raw: &str) -> Result<Self, ApiError> {
        let parts: Vec<&str> = raw.splitn(2, ':').collect();
        if parts.len() == 1 {
            return Err(ApiErrorKind::Internal("Bad user range option".to_owned()).into());
        }
        Ok(UserRange {
            offset: u64::from_str(parts[0]).expect("Bad offset"),
            limit: u64::from_str(parts[1]).expect("Bad limit"),
        })
    }
}

#[derive(StructOpt, Clone, Debug)]
#[structopt(name = "env")]
pub struct Settings {
    #[structopt(long, parse(try_from_str=Dsns::from_str), env = "MIGRATE_DSNS")]
    pub dsns: Dsns,
    #[structopt(long, env = "MIGRATE_DEBUG")]
    pub debug: bool,
    #[structopt(short, env = "MIGRATE_VERBOSE")]
    pub verbose: bool,
    #[structopt(long)]
    pub quiet: bool,
    #[structopt(long)]
    pub full: bool,
    #[structopt(long)]
    pub deanon: bool,
    #[structopt(long)]
    pub skip_collections: bool,
    #[structopt(long)]
    pub dryrun: bool,
    #[structopt(long, parse(from_flag = std::ops::Not::not))]
    pub human_logs: bool,
    pub fxa_file: String,
    pub chunk_limit: Option<u64>,
    pub offset: Option<u64>,
    pub start_bso: Option<u64>,
    pub end_bso: Option<u64>,
    pub readchunk: Option<u64>,
    pub spanner_pool_size: Option<usize>,
    #[structopt(long, parse(try_from_str=User::from_str))]
    pub user: Option<User>,
    #[structopt(long, parse(try_from_str=Abort::from_str))]
    pub abort: Option<Abort>,
    #[structopt(long, parse(try_from_str=UserRange::from_str))]
    pub user_range: Option<UserRange>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            dsns: Dsns::default(),
            debug: false,
            verbose: false,
            quiet: false,
            full: false,
            deanon: false,
            skip_collections: false,
            dryrun: false,
            human_logs: true,
            chunk_limit: Some(DEFAULT_CHUNK_SIZE),
            offset: Some(DEFAULT_OFFSET),
            start_bso: Some(DEFAULT_START_BSO),
            end_bso: Some(DEFAULT_END_BSO),
            readchunk: Some(DEFAULT_READ_CHUNK),
            spanner_pool_size: Some(DEFAULT_SPANNER_POOL_SIZE),
            fxa_file: DEFAULT_FXA_FILE.to_owned(),
            user: None,
            abort: None,
            user_range: None,
        }
    }
}
