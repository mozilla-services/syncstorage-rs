use std::str::FromStr;

use syncserver_common::Metrics;
use syncserver_db_common::{params, util::SyncTimestamp, Db, Sorting, UserIdentifier};
use syncserver_settings::Settings as SyncserverSettings;
use syncstorage_settings::Settings as SyncstorageSettings;

use crate::db::DbPool;
use crate::error::ApiResult;
use crate::{db::pool_from_settings, error::ApiError};

pub type Result<T> = std::result::Result<T, ApiError>;

#[cfg(test)]
pub async fn db_pool(settings: Option<SyncstorageSettings>) -> Result<Box<dyn DbPool>> {
    let _ = env_logger::try_init();
    // The default for SYNC_SYNCSTORAGE__DATABASE_USE_TEST_TRANSACTIONS is
    // false, but we want the mysql default to be true, so let's check
    // explicitly the env var because we can't rely on the default value or
    // the env var passed through to settings.
    let use_test_transactions = std::env::var("SYNC_SYNCSTORAGE__DATABASE_USE_TEST_TRANSACTIONS")
        .unwrap_or_else(|_| "true".to_string())
        .eq("true");

    // inherit SYNC_SYNCSTORAGE__DATABASE_URL from the env
    let mut settings = settings.unwrap_or_else(|| SyncserverSettings::test_settings().syncstorage);
    settings.database_use_test_transactions = use_test_transactions;

    let metrics = Metrics::noop();
    let pool = pool_from_settings(&settings, &metrics).await?;
    Ok(pool)
}

pub async fn test_db<E>(pool: &dyn DbPool<Error = E>) -> ApiResult<Box<dyn Db<'_, Error = E>>> {
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
        user_id: UserIdentifier::new_legacy(u64::from(user_id)),
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
        ids: bids.iter().map(|id| id.to_owned().into()).collect(),
        older: Some(SyncTimestamp::from_milliseconds(older)),
        newer: Some(SyncTimestamp::from_milliseconds(newer)),
        sort,
        limit: Some(limit as u32),
        offset: Some(params::Offset::from_str(offset).unwrap_or_default()),
        full: true,
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

pub fn hid(user_id: u32) -> UserIdentifier {
    UserIdentifier::new_legacy(u64::from(user_id))
}
