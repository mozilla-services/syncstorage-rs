use std::collections::HashMap;

use google_cloud_rust_raw::spanner::v1::{
    spanner::ExecuteSqlRequest,
    type_pb::{StructType_Field, Type, TypeCode},
};

use protobuf::{
    RepeatedField,
    well_known_types::{ListValue, NullValue, Struct, Value},
};
use syncstorage_db_common::{results, util::SyncTimestamp};

pub use super::stream::StreamedResultSetAsync;
use crate::{DbResult, error::DbError, pool::Conn};

pub trait IntoSpannerValue {
    const TYPE_CODE: TypeCode;

    fn into_spanner_value(self) -> Value;

    fn spanner_type(&self) -> Type {
        let mut t = Type::new();
        t.set_code(Self::TYPE_CODE);
        t
    }
}

impl IntoSpannerValue for String {
    const TYPE_CODE: TypeCode = TypeCode::STRING;

    fn into_spanner_value(self) -> Value {
        let mut value = Value::new();
        value.set_string_value(self);
        value
    }
}

impl IntoSpannerValue for i32 {
    const TYPE_CODE: TypeCode = TypeCode::INT64;

    fn into_spanner_value(self) -> Value {
        self.to_string().into_spanner_value()
    }
}

impl IntoSpannerValue for u32 {
    const TYPE_CODE: TypeCode = TypeCode::INT64;

    fn into_spanner_value(self) -> Value {
        self.to_string().into_spanner_value()
    }
}

impl<T> IntoSpannerValue for Vec<T>
where
    T: IntoSpannerValue,
    Vec<T>: SpannerArrayElementType,
{
    const TYPE_CODE: TypeCode = TypeCode::ARRAY;

    fn into_spanner_value(self) -> Value {
        let mut list = ListValue::new();
        list.set_values(RepeatedField::from_vec(
            self.into_iter().map(|v| v.into_spanner_value()).collect(),
        ));
        let mut value = Value::new();
        value.set_list_value(list);
        value
    }

    fn spanner_type(&self) -> Type {
        let mut t = Type::new();
        t.set_code(Self::TYPE_CODE);
        t.set_array_element_type(self.array_element_type());
        t
    }
}

pub trait SpannerArrayElementType {
    const ARRAY_ELEMENT_TYPE_CODE: TypeCode;

    fn array_element_type(&self) -> Type {
        let mut t = Type::new();
        t.set_code(Self::ARRAY_ELEMENT_TYPE_CODE);
        t
    }
}

impl SpannerArrayElementType for Vec<String> {
    const ARRAY_ELEMENT_TYPE_CODE: TypeCode = TypeCode::STRING;
}

impl SpannerArrayElementType for Vec<i32> {
    const ARRAY_ELEMENT_TYPE_CODE: TypeCode = TypeCode::INT64;
}

impl SpannerArrayElementType for Vec<u32> {
    const ARRAY_ELEMENT_TYPE_CODE: TypeCode = TypeCode::INT64;
}

pub fn as_type(v: TypeCode) -> Type {
    let mut t = Type::new();
    t.set_code(v);
    t
}

pub fn struct_type_field(name: &str, field_type: TypeCode) -> StructType_Field {
    let mut field = StructType_Field::new();
    field.set_name(name.to_owned());
    field.set_field_type(as_type(field_type));
    field
}

pub fn null_value() -> Value {
    let mut value = Value::new();
    value.set_null_value(NullValue::NULL_VALUE);
    value
}

/// A BSO's payload lives either inline (`payload`) or offloaded to GCS
/// (`payload_link`), never both. Reject a write that sets both. (Neither is
/// allowed: that's a metadata-only update preserving the existing row.)
pub fn validate_payload_exclusive(
    payload: Option<&String>,
    payload_link: Option<&String>,
) -> DbResult<()> {
    if payload.is_some() && payload_link.is_some() {
        return Err(DbError::integrity(
            "a BSO write cannot set both payload and payload_link".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Default)]
pub struct ExecuteSqlRequestBuilder {
    execute_sql: ExecuteSqlRequest,
    params: Option<HashMap<String, Value>>,
    param_types: Option<HashMap<String, Type>>,
}

impl ExecuteSqlRequestBuilder {
    pub fn new(execute_sql: ExecuteSqlRequest) -> Self {
        ExecuteSqlRequestBuilder {
            execute_sql,
            ..Default::default()
        }
    }

    pub fn params(mut self, params: HashMap<String, Value>) -> Self {
        self.params = Some(params);
        self
    }

    pub fn param_types(mut self, param_types: HashMap<String, Type>) -> Self {
        self.param_types = Some(param_types);
        self
    }

    fn prepare_request(self, conn: &Conn) -> ExecuteSqlRequest {
        let mut request = self.execute_sql;
        request.set_session(conn.session.get_name().to_owned());
        if let Some(params) = self.params {
            let mut paramss = Struct::new();
            paramss.set_fields(params);
            request.set_params(paramss);
        }
        if let Some(param_types) = self.param_types {
            request.set_param_types(param_types);
        }
        request
    }

    /// Execute a SQL read statement but return a non-blocking streaming result
    pub fn execute(self, conn: &Conn) -> DbResult<StreamedResultSetAsync> {
        let stream = conn
            .client
            .execute_streaming_sql_opt(&self.prepare_request(conn), conn.session_opt()?)?;
        Ok(StreamedResultSetAsync::new(stream))
    }

    /// Execute a DML statement, returning the exact count of modified rows
    pub async fn execute_dml(self, conn: &Conn) -> DbResult<i64> {
        let rs = conn
            .client
            .execute_sql_async_opt(&self.prepare_request(conn), conn.session_opt()?)?
            .await?;
        Ok(rs.get_stats().get_row_count_exact())
    }
}

pub fn bso_from_row(mut row: Vec<Value>) -> DbResult<results::GetBso> {
    let modified_string = &row[3].get_string_value();
    let modified = SyncTimestamp::from_rfc3339(modified_string)
        .map_err(|e| DbError::integrity(e.to_string()))?;

    // A stored BSO must hold its payload in exactly one place: inline
    // (payload) or offloaded (payload_link). Both set or neither set is
    // corrupt data.
    let payload_is_null = row[2].has_null_value();
    let payload_link_is_null = row[5].has_null_value();
    if payload_is_null == payload_link_is_null {
        return Err(DbError::integrity(
            "bso must have exactly one of payload / payload_link set".to_owned(),
        ));
    }

    Ok(results::GetBso {
        id: row[0].take_string_value(),
        sortindex: if row[1].has_null_value() {
            None
        } else {
            Some(
                row[1]
                    .get_string_value()
                    .parse::<i32>()
                    .map_err(|e| DbError::integrity(e.to_string()))?,
            )
        },
        // NULL when offloaded; the handler resolves it from payload_link.
        payload: if payload_is_null {
            String::new()
        } else {
            row[2].take_string_value()
        },
        modified,
        expiry: SyncTimestamp::from_rfc3339(row[4].get_string_value())
            .map_err(|e| DbError::integrity(e.to_string()))?
            .as_i64(),
        payload_link: if row[5].has_null_value() {
            None
        } else {
            Some(row[5].take_string_value())
        },
    })
}
