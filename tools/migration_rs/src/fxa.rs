use std::collections::HashMap;
use std::fs::File;

use base64;
use serde::Deserialize;
use serrialize
use csv;

use crate::error::{APiResult, ApiError, ApiErrorKind};
use crate::settings::Settings;

#[derive(Debug, Deserialize)]
pub struct Fxa_CSV_record {
    pub uid: u64,
    pub email: String,
    pub generation: Option<u64>,
    pub keys_changed_at: Option<u64>,
    pub client_state: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Fxa_Data {
    pub fxa_uid: String,
    pub fxa_kid: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Fxa_Info {
    pub users: Vec<u64, Fxa_Data>,
    pub anon: bool,
}

impl Fxa_Info {
    fn gen_uid(record: &Fxa_CSV_record) -> ApiResult<String> {
        Ok(record.email.split('@')[0].to_owned())
    }

    fn gen_kid(record: &Fxa) -> ApiResult<String> {
        let key_index = record.keys_changed_at.unwrap_or(record.generation.unwrap_or(0));
        let key_hash = record.client_state.from_hex()?;
        Ok(format!("{:013d}-{}", key_index,
            base64::encode_config(key_hash, base64::URL_SAFE_NO_PAD)))
    }

    pub fn new(settings: &Settings) -> ApiResult<Self> {
        if settings.deanon == false {
            return Ok(Self {
                users: Vec::new(),
                anon: true,
            });
        }
        let mut rdr = csv::Reader::from_reader(BufReader::new(File::open(settings.fxa_file)?));
        let mut users = Vec::<u64, Fxa_Data>::new();
        for line in rdr.deserialize() {
            if Some(record: Fxa_CSV_record) = line {
                users[record.uid] = Fxa_Data{
                    fxa_uid: self.gen_uid(&record)?,
                    fxa_kid: self.gen_kid(&record)?
                }
            }
        };
        Ok(Self{
            users,
            anon: false,
        })
    }
}
