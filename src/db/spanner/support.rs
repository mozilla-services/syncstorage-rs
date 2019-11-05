use std::collections::HashMap;

use protobuf::{
    well_known_types::{ListValue, NullValue, Struct, Value},
    RepeatedField,
};

use super::models::{Conn, Result};
use crate::db::{results, util::SyncTimestamp, DbError, DbErrorKind};

#[cfg(not(any(test, feature = "db_test")))]
use crate::{
    db::{params, spanner::models::DEFAULT_BSO_TTL, util::to_rfc3339},
    web::extractors::HawkIdentifier,
};

use googleapis_raw::spanner::v1::type_pb::{Type, TypeCode};

type ParamValue = protobuf::well_known_types::Value;

type ParamType = googleapis_raw::spanner::v1::type_pb::Type;

pub type ExecuteSqlRequest = googleapis_raw::spanner::v1::spanner::ExecuteSqlRequest;

type ResultSet = googleapis_raw::spanner::v1::result_set::ResultSet;

type ResultSetMetadata = googleapis_raw::spanner::v1::result_set::ResultSetMetadata;

type ResultSetStats = googleapis_raw::spanner::v1::result_set::ResultSetStats;

pub fn as_value(string_value: String) -> protobuf::well_known_types::Value {
    let mut value = Value::new();
    value.set_string_value(string_value);
    value
}

pub fn as_type(v: TypeCode) -> Type {
    let mut t = Type::new();
    t.set_code(v);
    t
}
pub fn as_list_value(
    string_values: impl Iterator<Item = String>,
) -> protobuf::well_known_types::Value {
    let mut list = ListValue::new();
    list.set_values(RepeatedField::from_vec(
        string_values.map(as_value).collect(),
    ));
    let mut value = Value::new();
    value.set_list_value(list);
    value
}

pub fn null_value() -> protobuf::well_known_types::Value {
    let mut value = Value::new();
    value.set_null_value(NullValue::NULL_VALUE);
    value
}

#[derive(Default)]
pub struct ExecuteSqlRequestBuilder {
    execute_sql: ExecuteSqlRequest,
    params: Option<HashMap<String, ParamValue>>,
    param_types: Option<HashMap<String, ParamType>>,
}

impl ExecuteSqlRequestBuilder {
    pub fn new(execute_sql: ExecuteSqlRequest) -> Self {
        ExecuteSqlRequestBuilder {
            execute_sql,
            ..Default::default()
        }
    }

    pub fn params(mut self, params: HashMap<String, ParamValue>) -> Self {
        self.params = Some(params);
        self
    }

    pub fn param_types(mut self, param_types: HashMap<String, ParamType>) -> Self {
        self.param_types = Some(param_types);
        self
    }

    pub fn execute(self, spanner: &Conn) -> Result<SyncResultSet> {
        let mut request = self.execute_sql;
        request.set_session(spanner.session.get_name().to_owned());
        if let Some(params) = self.params {
            let mut paramss = Struct::new();
            paramss.set_fields(params);
            request.set_params(paramss);
        }
        if let Some(param_types) = self.param_types {
            request.set_param_types(param_types);
        }
        let result = spanner.client.execute_sql(&request)?;
        Ok(SyncResultSet { result })
    }
}

#[derive(Debug)]
pub struct SyncResultSet {
    result: ResultSet,
}

impl SyncResultSet {
    #[allow(dead_code)]
    pub fn metadata(&self) -> Option<&ResultSetMetadata> {
        self.result.metadata.as_ref()
    }

    pub fn stats(&self) -> Option<&ResultSetStats> {
        self.result.stats.as_ref()
    }

    pub fn one(&mut self) -> Result<Vec<Value>> {
        if let Some(result) = self.one_or_none()? {
            Ok(result)
        } else {
            Err(DbError::internal("No rows matched the given query."))?
        }
    }

    pub fn one_or_none(&mut self) -> Result<Option<Vec<Value>>> {
        let result = self.next();
        if result.is_none() {
            Ok(None)
        } else if self.next().is_some() {
            Err(DbError::internal("Execpted one result; got more."))?
        } else {
            Ok(result)
        }
    }

    pub fn affected_rows(self: &SyncResultSet) -> Result<i64> {
        let stats = self
            .stats()
            .ok_or_else(|| DbError::internal("Expected result_set stats"))?;
        let row_count_exact = stats.get_row_count_exact();
        Ok(row_count_exact)
    }
}

impl Iterator for SyncResultSet {
    type Item = Vec<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        let rows = &mut self.result.rows;
        if rows.is_empty() {
            None
        } else {
            let row = rows.remove(0);
            Some(row.get_values().to_vec())
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.result.rows.len();
        (len, Some(len))
    }
}

pub fn bso_from_row(row: Vec<Value>) -> Result<results::GetBso> {
    Ok(results::GetBso {
        id: row[0].get_string_value().to_owned(),
        modified: SyncTimestamp::from_rfc3339(&row[1].get_string_value())?,
        payload: row[2].get_string_value().to_owned(),
        sortindex: if row[3].has_null_value() {
            None
        } else {
            Some(
                row[3]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?,
            )
        },
        expiry: SyncTimestamp::from_rfc3339(&row[4].get_string_value())?.as_i64(),
    })
}

#[cfg(not(any(test, feature = "db_test")))]
pub fn bso_to_insert_row(
    user_id: &HawkIdentifier,
    collection_id: i32,
    bso: params::PostCollectionBso,
    now: SyncTimestamp,
) -> Result<ListValue> {
    let sortindex = bso
        .sortindex
        .map(|sortindex| as_value(sortindex.to_string()))
        .unwrap_or_else(null_value);
    let ttl = bso.ttl.unwrap_or(DEFAULT_BSO_TTL);
    let expiry = to_rfc3339(now.as_i64() + (i64::from(ttl) * 1000))?;

    let mut row = ListValue::new();
    row.set_values(RepeatedField::from_vec(vec![
        as_value(user_id.fxa_uid.clone()),
        as_value(user_id.fxa_kid.clone()),
        as_value(collection_id.to_string()),
        as_value(bso.id),
        sortindex,
        as_value(bso.payload.unwrap_or_default()),
        as_value(now.as_rfc3339()?),
        as_value(expiry),
    ]));
    Ok(row)
}

#[cfg(not(any(test, feature = "db_test")))]
pub fn bso_to_update_row(
    user_id: &HawkIdentifier,
    collection_id: i32,
    bso: params::PostCollectionBso,
    now: SyncTimestamp,
) -> Result<(Vec<&'static str>, ListValue)> {
    let mut columns = vec!["fxa_uid", "fxa_kid", "collection_id", "bso_id"];
    let mut values = vec![
        as_value(user_id.fxa_uid.clone()),
        as_value(user_id.fxa_kid.clone()),
        as_value(collection_id.to_string()),
        as_value(bso.id),
    ];

    let modified = bso.payload.is_some() || bso.sortindex.is_some();
    if let Some(sortindex) = bso.sortindex {
        columns.push("sortindex");
        values.push(as_value(sortindex.to_string()));
    }
    if let Some(payload) = bso.payload {
        columns.push("payload");
        values.push(as_value(payload));
    }
    if modified {
        columns.push("modified");
        values.push(as_value(now.as_rfc3339()?));
    }
    if let Some(ttl) = bso.ttl {
        columns.push("expiry");
        let expiry = now.as_i64() + (i64::from(ttl) * 1000);
        values.push(as_value(to_rfc3339(expiry)?));
    }

    let mut row = ListValue::new();
    row.set_values(RepeatedField::from_vec(values));
    Ok((columns, row))
}
