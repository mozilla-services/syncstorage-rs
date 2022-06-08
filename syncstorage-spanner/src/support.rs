use std::{
    collections::{HashMap, VecDeque},
    mem,
};

use futures::stream::{StreamExt, StreamFuture};
use google_cloud_rust_raw::spanner::v1::{
    result_set::{PartialResultSet, ResultSetMetadata, ResultSetStats},
    spanner::ExecuteSqlRequest,
    type_pb::{StructType_Field, Type, TypeCode},
};
use grpcio::ClientSStreamReceiver;
use protobuf::{
    well_known_types::{ListValue, NullValue, Struct, Value},
    RepeatedField,
};
use syncserver_db_common::{
    params, results, util::to_rfc3339, util::SyncTimestamp, UserIdentifier, DEFAULT_BSO_TTL,
};

use super::{error::DbError, pool::Conn, DbResult};

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
    pub fn execute_async(self, conn: &Conn) -> DbResult<StreamedResultSetAsync> {
        let stream = conn
            .client
            .execute_streaming_sql(&self.prepare_request(conn))?;
        Ok(StreamedResultSetAsync::new(stream))
    }

    /// Execute a DML statement, returning the exact count of modified rows
    pub async fn execute_dml_async(self, conn: &Conn) -> DbResult<i64> {
        let rs = conn
            .client
            .execute_sql_async(&self.prepare_request(conn))?
            .await?;
        Ok(rs.get_stats().get_row_count_exact())
    }
}

pub struct StreamedResultSetAsync {
    /// Stream from execute_streaming_sql
    stream: Option<StreamFuture<ClientSStreamReceiver<PartialResultSet>>>,

    metadata: Option<ResultSetMetadata>,
    stats: Option<ResultSetStats>,

    /// Fully-processed rows
    rows: VecDeque<Vec<Value>>,
    /// Accumulated values for incomplete row
    current_row: Vec<Value>,
    /// Incomplete value
    pending_chunk: Option<Value>,
}

impl StreamedResultSetAsync {
    pub fn new(stream: ClientSStreamReceiver<PartialResultSet>) -> Self {
        Self {
            stream: Some(stream.into_future()),
            metadata: None,
            stats: None,
            rows: Default::default(),
            current_row: vec![],
            pending_chunk: None,
        }
    }

    #[allow(dead_code)]
    pub fn metadata(&self) -> Option<&ResultSetMetadata> {
        self.metadata.as_ref()
    }

    #[allow(dead_code)]
    pub fn stats(&self) -> Option<&ResultSetStats> {
        self.stats.as_ref()
    }

    pub fn fields(&self) -> &[StructType_Field] {
        match self.metadata {
            Some(ref metadata) => metadata.get_row_type().get_fields(),
            None => &[],
        }
    }

    pub async fn one(&mut self) -> DbResult<Vec<Value>> {
        if let Some(result) = self.one_or_none().await? {
            Ok(result)
        } else {
            Err(DbError::internal(
                "No rows matched the given query.".to_owned(),
            ))
        }
    }

    pub async fn one_or_none(&mut self) -> DbResult<Option<Vec<Value>>> {
        let result = self.next_async().await;
        if result.is_none() {
            Ok(None)
        } else if self.next_async().await.is_some() {
            Err(DbError::internal(
                "Expected one result; got more.".to_owned(),
            ))
        } else {
            result.transpose()
        }
    }

    /// Pull and process the next values from the Stream
    ///
    /// Returns false when the stream is finished
    async fn consume_next(&mut self) -> DbResult<bool> {
        let (result, stream) = self
            .stream
            .take()
            .expect("Could not get next stream element")
            .await;

        self.stream = Some(stream.into_future());
        let mut partial_rs = if let Some(result) = result {
            result?
        } else {
            // Stream finished
            return Ok(false);
        };

        if self.metadata.is_none() && partial_rs.has_metadata() {
            // first response
            self.metadata = Some(partial_rs.take_metadata());
        }
        if partial_rs.has_stats() {
            // last response
            self.stats = Some(partial_rs.take_stats());
        }

        let mut values = partial_rs.take_values().into_vec();
        if values.is_empty() {
            // sanity check
            return Ok(true);
        }

        if let Some(pending_chunk) = self.pending_chunk.take() {
            let fields = self.fields();
            let current_row_i = self.current_row.len();
            if fields.len() <= current_row_i {
                return Err(DbError::integrity(
                    "Invalid PartialResultSet fields".to_owned(),
                ));
            }
            let field = &fields[current_row_i];
            values[0] = merge_by_type(pending_chunk, &values[0], field.get_field_type())?;
        }
        if partial_rs.get_chunked_value() {
            self.pending_chunk = values.pop();
        }

        self.consume_values(values);
        Ok(true)
    }

    fn consume_values(&mut self, values: Vec<Value>) {
        let width = self.fields().len();
        for value in values {
            self.current_row.push(value);
            if self.current_row.len() == width {
                let current_row = mem::take(&mut self.current_row);
                self.rows.push_back(current_row);
            }
        }
    }

    // We could implement Stream::poll_next instead of this, but
    // this is easier for now and we can refactor into the trait later
    pub async fn next_async(&mut self) -> Option<DbResult<Vec<Value>>> {
        while self.rows.is_empty() {
            match self.consume_next().await {
                Ok(true) => (),
                Ok(false) => return None,
                // Note: Iteration may continue after an error. We may want to
                // stop afterwards instead for safety sake (it's not really
                // recoverable)
                Err(e) => return Some(Err(e)),
            }
        }
        Ok(self.rows.pop_front()).transpose()
    }
}

fn merge_by_type(lhs: Value, rhs: &Value, field_type: &Type) -> DbResult<Value> {
    // We only support merging basic string types as that's all we currently use.
    // The python client also supports: float64, array, struct. The go client
    // only additionally supports array (claiming structs are only returned as
    // arrays anyway)
    match field_type.get_code() {
        TypeCode::BYTES
        | TypeCode::DATE
        | TypeCode::INT64
        | TypeCode::STRING
        | TypeCode::TIMESTAMP => merge_string(lhs, rhs),
        _ => unsupported_merge(field_type),
    }
}

fn unsupported_merge(field_type: &Type) -> DbResult<Value> {
    Err(DbError::internal(format!(
        "merge not supported, type: {:?}",
        field_type
    )))
}

fn merge_string(mut lhs: Value, rhs: &Value) -> DbResult<Value> {
    if !lhs.has_string_value() || !rhs.has_string_value() {
        return Err(DbError::internal(
            "merge_string has no string value".to_owned(),
        ));
    }
    let mut merged = lhs.take_string_value();
    merged.push_str(rhs.get_string_value());
    Ok(merged.into_spanner_value())
}

pub fn bso_from_row(mut row: Vec<Value>) -> DbResult<results::GetBso> {
    let modified_string = &row[3].get_string_value();
    let modified = SyncTimestamp::from_rfc3339(modified_string)?;
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
        expiry: SyncTimestamp::from_rfc3339(row[4].get_string_value())?.as_i64(),
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

#[derive(Clone)]
pub struct MapAndThenIterator<I, F> {
    iter: I,
    f: F,
}

impl<A, B, E, I, F> Iterator for MapAndThenIterator<I, F>
where
    F: FnMut(A) -> Result<B, E>,
    I: Iterator<Item = Result<A, E>>,
{
    type Item = Result<B, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|result| result.and_then(&mut self.f))
    }
}

pub trait MapAndThenTrait {
    /// Return an iterator adaptor that applies the provided closure to every
    /// DbResult::Ok value. DbResult::Err values are unchanged.
    ///
    /// The closure can be used for control flow based on result values
    fn map_and_then<F, A, B, E>(self, func: F) -> MapAndThenIterator<Self, F>
    where
        Self: Sized + Iterator<Item = Result<A, E>>,
        F: FnMut(A) -> Result<B, E>,
    {
        MapAndThenIterator {
            iter: self,
            f: func,
        }
    }
}

impl<I, T, E> MapAndThenTrait for I where I: Sized + Iterator<Item = Result<T, E>> {}
