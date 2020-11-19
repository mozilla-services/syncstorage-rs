use actix_web::error::BlockingError;
use actix_web::web::block;
use actix_web::web::Data;
use actix_web::HttpResponse;
use actix_web_httpauth::extractors::bearer::BearerAuth;

use futures::future::{Future, TryFutureExt};

use crate::error::{ApiError, ApiErrorKind};
use crate::server::ServerState;

use diesel::mysql::MysqlConnection;
use diesel::prelude::*;
use diesel::sql_types::*;
use diesel::RunQueryDsl;
use std::env;

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;

#[derive(Debug)]
enum MyError {
    EnvError(env::VarError),
}

impl From<env::VarError> for MyError {
    fn from(error: env::VarError) -> Self {
        MyError::EnvError(error)
    }
}

#[derive(Debug, QueryableByName)]
struct TokenserverUser {
    #[sql_type = "Bigint"]
    uid: i64,
    // This is no longer used. Was for making more than just sync tokens.
    #[sql_type = "Text"]
    pattern: String,
    #[sql_type = "Text"]
    email: String,
    #[sql_type = "Bigint"]
    generation: i64,
    #[sql_type = "Text"]
    client_state: String,
    #[sql_type = "Bigint"]
    created_at: i64,
    #[sql_type = "Nullable<Bigint>"]
    replaced_at: Option<i64>,
    #[sql_type = "Text"]
    node: String,
    #[sql_type = "Nullable<Bigint>"]
    keys_changed_at: Option<i64>,
}

#[derive(serde::Serialize)]
pub struct TokenServerResult {
    id: String,
    key: String,
    uid: String,
    api_endpoint: String,
    duration: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct Claims {
    pub sub: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn get(
    state: Data<ServerState>,
    auth: BearerAuth,
) -> impl Future<Output = Result<HttpResponse, BlockingError<ApiError>>> {
    dbg!(&state.tokenserver_database_url);
    block(move || {
        get_sync(
            &auth,
            state
                .tokenserver_database_url
                .as_ref()
                .expect("tokenserver database url not set")
                .to_string(),
            state
                .tokenserver_jwks_rsa_modulus
                .as_ref()
                .expect("modulus not set")
                .to_string(),
            state
                .tokenserver_jwks_rsa_exponent
                .as_ref()
                .expect("exponent not set")
                .to_string(),
            state.secrets.master_secret.clone()[0].to_string(),
        )
        .map_err(Into::into)
    })
    .map_ok(move |result| {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&result).unwrap())
    })
}

pub fn get_sync(
    auth: &BearerAuth,
    database_url: String,
    modulus: String,
    exponent: String,
    shared_secret: String,
) -> Result<TokenServerResult, ApiError> {
    let token_data = decode::<Claims>(
        &auth.token(),
        &DecodingKey::from_rsa_components(&modulus, &exponent),
        &Validation::new(Algorithm::RS256),
    )
    .map_err(|ee| {
        ApiError::from(ApiErrorKind::Internal(format!(
            "Unable to decode token_data: {:}",
            ee
        )))
    })?;
    let email = format!("{:}@api.accounts.firefox.com", token_data.claims.sub);

    let connection = MysqlConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
    let user_record = diesel::sql_query(
        r#"SELECT users.uid, services.pattern, users.email, users.generation,
                       users.client_state, users.created_at, users.replaced_at,
                       nodes.node, users.keys_changed_at from users, services,
                       nodes
                 WHERE users.email = ?
                   AND services.id = users.service
                   AND nodes.id = users.nodeid
                   AND nodes.service = services.id"#,
    )
    .bind::<Text, _>(&email)
    .load::<TokenserverUser>(&connection)
    .unwrap();
    let (python_result, python_derived_result) = Python::with_gil(|py| {
        let tokenlib = PyModule::from_code(
            py,
            r###"
import base64
from hashlib import sha256
import hmac
import tokenlib


def make_token(plaintext, shared_secret):
    return tokenlib.make_token(plaintext, secret=shared_secret)


def get_derived_secret(plaintext, shared_secret):
    return tokenlib.get_derived_secret(plaintext, secret=shared_secret)


def encode_bytes(value):
    """Encode BrowserID's base64 encoding format.

    BrowserID likes to strip padding characters off of base64-encoded strings,
    meaning we can't use the stdlib routines to encode them directly.  This
    is a simple wrapper that strips the padding.
    """
    if isinstance(value, str):
        value = value.encode("ascii")
    return base64.urlsafe_b64encode(value).rstrip(b"=").decode("ascii")


def fxa_metrics_hash(value, hmac_key):
    """Derive FxA metrics id from user's FxA email address or whatever.

    This is used to obfuscate the id before logging it with the metrics
    data, as a simple privacy measure.
    """
    hasher = hmac.new(hmac_key.encode("ascii"), ''.encode("ascii"), sha256)
    hasher.update(value.split("@", 1)[0].encode("ascii"))
    return hasher.hexdigest()


def hash_device_id(fxa_uid, device, secret):
    return fxa_metrics_hash(fxa_uid[:32] + device, secret)[:32]
    "###,
            "main.py",
            "main",
        )
        .map_err(|e| {
            e.print_and_set_sys_last_vars(py);
            e
        })?;
        let client_state_b64 = match tokenlib.call1("encode_bytes", (&user_record[0].client_state,))
        {
            Err(e) => {
                e.print_and_set_sys_last_vars(py);
                return Err(e);
            }
            Ok(x) => x.extract::<String>().unwrap(),
        };
        let hashed_fxa_uid = match tokenlib.call1(
            "fxa_metrics_hash",
            (
                &email,
                env::var("FXA_METRICS_HASH_SECRET").unwrap_or_else(|_| "insecure".to_string()),
            ),
        ) {
            Err(e) => {
                e.print_and_set_sys_last_vars(py);
                return Err(e);
            }
            Ok(x) => x.extract::<String>().unwrap(),
        };
        let device_id = "none".to_string();
        let fxa_metrics_hash_secret =
            env::var("FXA_METRICS_HASH_SECRET").unwrap_or_else(|_| "insecure".to_string());
        let hashed_device_id = match tokenlib.call1(
            "hash_device_id",
            (&hashed_fxa_uid, device_id, &fxa_metrics_hash_secret),
        ) {
            Err(e) => {
                e.print_and_set_sys_last_vars(py);
                return Err(e);
            }
            Ok(x) => x.extract::<String>().unwrap(),
        };

        let fxa_kid = format!(
            "{:013}-{:}",
            user_record[0].keys_changed_at.unwrap_or_else(|| 0),
            client_state_b64
        );
        let thedict = [
            ("node", &user_record[0].node),
            ("fxa_kid", &fxa_kid), // userid component of authorization email
            ("fxa_uid", &token_data.claims.sub),
            ("hashed_device_id", &hashed_device_id),
            ("hashed_fxa_uid", &hashed_fxa_uid),
        ]
        .into_py_dict(py);
        // todo don't hardcode
        // we're supposed to check the "duration" query
        // param and use that if present (for testing)
        thedict.set_item("expires", 300).unwrap(); // todo this needs to be converted to timestamp int (now + value * 1000)
        thedict.set_item("uid", user_record[0].uid).unwrap();
        let result = match tokenlib.call1("make_token", (thedict, &shared_secret)) {
            Err(e) => {
                e.print_and_set_sys_last_vars(py);
                return Err(e);
            }
            Ok(x) => x.extract::<String>().unwrap(),
        };
        let derived_result = match tokenlib.call1("get_derived_secret", (&result, &shared_secret)) {
            Err(e) => {
                e.print_and_set_sys_last_vars(py);
                return Err(e);
            }
            Ok(x) => x.extract::<String>().unwrap(),
        };
        //assert_eq!(result, false);
        Ok((result, derived_result))
    })
    .unwrap();
    let api_endpoint = format!("{:}/1.5/{:}", user_record[0].node, user_record[0].uid);
    Ok(TokenServerResult {
        id: python_result,
        key: python_derived_result,
        uid: token_data.claims.sub,
        api_endpoint,
        duration: "300".to_string(),
    })
}
