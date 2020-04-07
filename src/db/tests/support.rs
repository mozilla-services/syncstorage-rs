use std::str::FromStr;

use env_logger;

use crate::{
    db::{params, pool_from_settings, util::SyncTimestamp, Db, Sorting},
    error::ApiError,
    server::metrics,
    settings::{Secrets, ServerLimits, Settings},
    web::extractors::{BsoQueryParams, HawkIdentifier, Offset},
};

pub type Result<T> = std::result::Result<T, ApiError>;

pub async fn db() -> Result<Box<dyn Db>> {
    let _ = env_logger::try_init();
    // inherit SYNC_DATABASE_URL from the env
    let settings = Settings::with_env_and_config_file(&None).unwrap();
    let settings = Settings {
        debug: true,
        port: 8000,
        host: settings.host,
        database_url: settings.database_url,
        database_pool_max_size: Some(1),
        database_use_test_transactions: true,
        limits: ServerLimits::default(),
        master_secret: Secrets::default(),
        ..Default::default()
    };

    let metrics = metrics::Metrics::noop();
    let pool = pool_from_settings(&settings, &metrics)?;
    let db = pool.get().await?;
    // Spanner won't have a timestamp until lock_for_xxx are called: fill one
    // in for it
    db.set_timestamp(SyncTimestamp::default());
    Ok(db)
}

macro_rules! with_delta {
    ($db:expr, $delta:expr, $body:block) => {{
        let ts = $db.timestamp().as_i64();
        $db.set_timestamp(SyncTimestamp::_from_i64(ts + $delta).unwrap());
        let result = $body;
        $db.set_timestamp(SyncTimestamp::_from_i64(ts).unwrap());
        result
    }};
}

pub fn pbso(
    user_id: u32,
    coll: &str,
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i32>,
    ttl: Option<u32>,
) -> params::PutBso {
    params::PutBso {
        user_id: HawkIdentifier::new_legacy(u64::from(user_id)),
        collection: coll.to_owned(),
        id: bid.to_owned(),
        payload: payload.map(|payload| payload.to_owned()),
        sortindex,
        ttl,
    }
}

pub fn postbso(
    bid: &str,
    payload: Option<&str>,
    sortindex: Option<i32>,
    ttl: Option<u32>,
) -> params::PostCollectionBso {
    params::PostCollectionBso {
        id: bid.to_owned(),
        payload: payload.map(&str::to_owned),
        sortindex,
        ttl,
    }
}

pub fn gbso(user_id: u32, coll: &str, bid: &str) -> params::GetBso {
    params::GetBso {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id: bid.to_owned(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn gbsos(
    user_id: u32,
    coll: &str,
    bids: &[&str],
    older: u64,
    newer: u64,
    sort: Sorting,
    limit: i64,
    offset: &str,
) -> params::GetBsos {
    params::GetBsos {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        params: BsoQueryParams {
            ids: bids.iter().map(|id| id.to_owned().into()).collect(),
            older: Some(SyncTimestamp::from_milliseconds(older)),
            newer: Some(SyncTimestamp::from_milliseconds(newer)),
            sort,
            limit: Some(limit as u32),
            offset: Some(Offset::from_str(offset).unwrap_or_default()),
            full: true,
        },
    }
}

pub fn dbso(user_id: u32, coll: &str, bid: &str) -> params::DeleteBso {
    params::DeleteBso {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        id: bid.to_owned(),
    }
}

pub fn dbsos(user_id: u32, coll: &str, bids: &[&str]) -> params::DeleteBsos {
    params::DeleteBsos {
        user_id: hid(user_id),
        collection: coll.to_owned(),
        ids: bids.iter().map(|id| id.to_owned().into()).collect(),
    }
}

pub fn hid(user_id: u32) -> HawkIdentifier {
    HawkIdentifier::new_legacy(u64::from(user_id))
}
