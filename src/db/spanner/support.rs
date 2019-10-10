use std::collections::HashMap;

use protobuf::well_known_types::Struct;
use protobuf::{
    well_known_types::{ListValue, Value},
    RepeatedField,
};

use super::models::{Conn, Result};
use crate::db::{results, util::SyncTimestamp, DbError, DbErrorKind};

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

#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq, Debug, Hash)]
pub enum SpannerType {
    TypeCodeUnspecified,
    Bool,
    Int64,
    Float64,
    Timestamp,
    Date,
    String,
    Bytes,
    Array,
    Struct,
}

impl Into<Type> for SpannerType {
    fn into(self) -> Type {
        let mut t = Type::new();
        let code = match self {
            SpannerType::TypeCodeUnspecified => TypeCode::TYPE_CODE_UNSPECIFIED,
            SpannerType::Bool => TypeCode::BOOL,
            SpannerType::Int64 => TypeCode::INT64,
            SpannerType::Float64 => TypeCode::FLOAT64,
            SpannerType::Timestamp => TypeCode::TIMESTAMP,
            SpannerType::Date => TypeCode::DATE,
            SpannerType::String => TypeCode::STRING,
            SpannerType::Bytes => TypeCode::BYTES,
            SpannerType::Array => TypeCode::ARRAY,
            SpannerType::Struct => TypeCode::STRUCT,
        };
        t.set_code(code);
        t
    }
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

    pub fn all_or_none(&mut self) -> Option<Vec<ListValue>> {
        if self.result.rows.is_empty() {
            None
        } else {
            Some(self.result.rows.clone().into_vec())
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
