use std::collections::HashMap;

use google_cloud_rust_raw::spanner::v1::{
    spanner::ExecuteSqlRequest,
    type_pb::{StructType_Field, Type, TypeCode},
};

use protobuf::{
    well_known_types::{ListValue, NullValue, Struct, Value},
    RepeatedField,
};
use syncstorage_db_common::{
    params, results, util::to_rfc3339, util::SyncTimestamp, UserIdentifier, DEFAULT_BSO_TTL,
};

pub use super::stream::StreamedResultSetAsync;
use crate::{error::DbError, pool::Conn, DbResult};

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
        payload: row[2].take_string_value(),
        modified,
        expiry: SyncTimestamp::from_rfc3339(row[4].get_string_value())
            .map_err(|e| DbError::integrity(e.to_string()))?
            .as_i64(),
    })
}

pub fn bso_to_insert_row(
    user_id: &UserIdentifier,
    collection_id: i32,
    bso: params::PostCollectionBso,
    now: SyncTimestamp,
) -> DbResult<ListValue> {
    let sortindex = bso
        .sortindex
        .map(|sortindex| sortindex.into_spanner_value())
        .unwrap_or_else(null_value);
    let ttl = bso.ttl.unwrap_or(DEFAULT_BSO_TTL);
    let expiry = to_rfc3339(now.as_i64() + (i64::from(ttl) * 1000))?;

    let mut row = ListValue::new();
    row.set_values(RepeatedField::from_vec(vec![
        user_id.fxa_uid.clone().into_spanner_value(),
        user_id.fxa_kid.clone().into_spanner_value(),
        collection_id.into_spanner_value(),
        bso.id.into_spanner_value(),
        sortindex,
        bso.payload.unwrap_or_default().into_spanner_value(),
        now.as_rfc3339()?.into_spanner_value(),
        expiry.into_spanner_value(),
    ]));
    Ok(row)
}

pub fn bso_to_update_row(
    user_id: &UserIdentifier,
    collection_id: i32,
    bso: params::PostCollectionBso,
    now: SyncTimestamp,
) -> DbResult<(Vec<&'static str>, ListValue)> {
    let mut columns = vec!["fxa_uid", "fxa_kid", "collection_id", "bso_id"];
    let mut values = vec![
        user_id.fxa_uid.clone().into_spanner_value(),
        user_id.fxa_kid.clone().into_spanner_value(),
        collection_id.into_spanner_value(),
        bso.id.into_spanner_value(),
    ];

    let modified = bso.payload.is_some() || bso.sortindex.is_some();
    if let Some(sortindex) = bso.sortindex {
        columns.push("sortindex");
        values.push(sortindex.into_spanner_value());
    }
    if let Some(payload) = bso.payload {
        columns.push("payload");
        values.push(payload.into_spanner_value());
    }
    if modified {
        columns.push("modified");
        values.push(now.as_rfc3339()?.into_spanner_value());
    }
    if let Some(ttl) = bso.ttl {
        columns.push("expiry");
        let expiry = now.as_i64() + (i64::from(ttl) * 1000);
        values.push(to_rfc3339(expiry)?.into_spanner_value());
    }

    let mut row = ListValue::new();
    row.set_values(RepeatedField::from_vec(values));
    Ok((columns, row))
}
