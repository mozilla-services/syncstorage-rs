use actix_web::error::BlockingError;
use actix_web::web::block;
use actix_web::HttpResponse;
use actix_web_httpauth::extractors::bearer::BearerAuth;

use futures::future::{Future, TryFutureExt};

use crate::error::{ApiError, ApiErrorKind};

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
    auth: BearerAuth,
) -> impl Future<Output = Result<HttpResponse, BlockingError<ApiError>>> {
    block(move || get_sync(&auth).map_err(Into::into)).map_ok(move |result| {
        HttpResponse::Ok()
            .content_type("application/json")
            .body(serde_json::to_string(&result).unwrap())
    })
}

pub fn get_sync(auth: &BearerAuth) -> Result<TokenServerResult, ApiError> {
    // the public rsa components come from
    // https://oauth.accounts.firefox.com/v1/jwks
    // TODO we should fetch it from an environment var instead of hardcoding it.
    let token_data = decode::<Claims>(
        &auth.token(),
        &DecodingKey::from_rsa_components("2lDphW0lNZ4w1m9CfmIhC1AxYG9iwihxBdQZo7_6e0TBAi8_TNaoHHI90G9n5d8BQQnNcF4j2vOs006zlXcqGrP27b49KkN3FmbcOMovvfesMseghaqXqqFLALL9us3Wstt_fV_qV7ceRcJq5Hd_Mq85qUgYSfb9qp0vyePb26KEGy4cwO7c9nCna1a_i5rzUEJu6bAtcLS5obSvmsOOpTLHXojKKOnC4LRC3osdR6AU6v3UObKgJlkk_-8LmPhQZqOXiI_TdBpNiw6G_-eishg8V_poPlAnLNd8mfZBam-_7CdUS4-YoOvJZfYjIoboOuVmUrBjogFyDo72EPTReQ", "AQAB"),
        &Validation::new(Algorithm::RS256),
    ).map_err(|ee| {
        ApiError::from(ApiErrorKind::Internal(format!("Unable to decode token_data: {:}", ee)))
    })?;
    let email = format!("{:}@api.accounts.firefox.com", token_data.claims.sub);

    // TODO pull out of settings instead
    let shared_secret = env::var("SYNC_MASTER_SECRET").expect("SYNC_MASTER_SECRET must be set");
    let database_url =
        env::var("TOKENSERVER_DATABASE_URL").expect("TOKENSERVER_DATABASE_URL must be set");

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
    .bind::<Text, _>(email)
    .load::<TokenserverUser>(&connection)
    .unwrap();
    let (python_result, python_derived_result) = Python::with_gil(|py| {
        let tokenlib = PyModule::from_code(
            py,
            r#"
import tokenlib


def make_token(plaintext, shared_secret):
    return tokenlib.make_token(plaintext, secret=shared_secret)


def get_derived_secret(plaintext, shared_secret):
    return tokenlib.get_derived_secret(plaintext, secret=shared_secret)
"#,
            "main.py",
            "main",
        )
        .map_err(|e| {
            e.print_and_set_sys_last_vars(py);
            e
        })?;
        let thedict = [
            ("node", user_record[0].node.as_ref()),
            ("uid", token_data.claims.sub.as_ref()),
            ("fxa_kid", "asdf"), // userid component of authorization email
            ("fxa_uid", "qwer"),
            ("hashed_device_id", "..."),
            ("hashed_fxa_uid", "..."),
        ]
        .into_py_dict(py);
        // todo don't hardcode
        // we're supposed to check the "duration" query
        // param and use that if present (for testing)
        thedict.set_item("expires", 300).unwrap();
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
