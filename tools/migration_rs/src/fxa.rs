use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use base64;
use csv;
use serde::{self, Deserialize};

use crate::error::{ApiError, ApiErrorKind, ApiResult};
use crate::settings::Settings;

#[derive(Debug, Deserialize)]
pub struct FxaCSVRecord {
    pub uid: u64,
    pub email: String,
    pub generation: Option<u64>,
    pub keys_changed_at: Option<u64>,
    pub client_state: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FxaData {
    pub fxa_uid: String,
    pub fxa_kid: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct FxaInfo {
    pub users: HashMap<u64, FxaData>,
    pub anon: bool,
}

impl FxaInfo {
    fn gen_uid(record: &FxaCSVRecord) -> ApiResult<String> {
        let parts: Vec<&str> = record.email.splitn(2, '@').collect();
        Ok(parts[0].to_owned())
    }

    fn gen_kid(record: &FxaCSVRecord) -> ApiResult<String> {
        let key_index = record
            .keys_changed_at
            .unwrap_or(record.generation.unwrap_or(0));
        let key_hash: Vec<u8> = match hex::decode(record.client_state.clone()) {
            Ok(v) => v,
            Err(e) => {
                return Err(ApiErrorKind::Internal(format!("Invalid client state {}", e)).into())
            }
        };
        Ok(format!(
            "{:013}-{}",
            key_index,
            base64::encode_config(&key_hash, base64::URL_SAFE_NO_PAD)
        ))
    }

    pub fn new(settings: &Settings) -> ApiResult<Self> {
        if settings.deanon == false {
            return Ok(Self {
                users: HashMap::new(),
                anon: true,
            });
        }
        let mut rdr = csv::Reader::from_reader(BufReader::new(File::open(&settings.fxa_file)?));
        let mut users = HashMap::<u64, FxaData>::new();
        for line in rdr.deserialize::<FxaCSVRecord>() {
            if let Ok(record) = line {
                users.insert(record.uid, FxaData {
                    fxa_uid: FxaInfo::gen_uid(&record)?,
                    fxa_kid: FxaInfo::gen_kid(&record)?,
                });
            }
        }
        Ok(Self { users, anon: false })
    }
}
