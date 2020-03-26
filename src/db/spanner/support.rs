use std::{
    collections::{HashMap, VecDeque},
    mem,
    result::Result as StdResult,
};

use futures::compat::{Compat01As03, Future01CompatExt, Stream01CompatExt};
use futures::stream::{StreamExt, StreamFuture};
use googleapis_raw::spanner::v1::{
    result_set::{PartialResultSet, ResultSetMetadata, ResultSetStats},
    spanner::ExecuteSqlRequest,
    type_pb::{StructType_Field, Type, TypeCode},
};
use grpcio::ClientSStreamReceiver;
use protobuf::{
    well_known_types::{ListValue, NullValue, Struct, Value},
    RepeatedField,
};

use super::models::{Conn, Result};
use crate::db::{results, util::SyncTimestamp, DbError, DbErrorKind};

use crate::{
    db::{params, spanner::models::DEFAULT_BSO_TTL, util::to_rfc3339},
    web::extractors::HawkIdentifier,
};

pub fn as_value(string_value: String) -> Value {
    let mut value = Value::new();
    value.set_string_value(string_value);
    value
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
    pub fn execute_async(self, conn: &Conn) -> Result<StreamedResultSetAsync> {
        let stream = conn
            .client
            .execute_streaming_sql(&self.prepare_request(conn))?;
        Ok(StreamedResultSetAsync::new(stream))
    }

    /// Execute a DML statement, returning the exact count of modified rows
    pub async fn execute_dml_async(self, conn: &Conn) -> Result<i64> {
        let rs = conn
            .client
            .execute_sql_async(&self.prepare_request(conn))?
            .compat()
            .await?;
        Ok(rs.get_stats().get_row_count_exact())
    }
}

pub struct StreamedResultSetAsync {
    /// Stream from execute_streaming_sql
    stream: Option<StreamFuture<Compat01As03<ClientSStreamReceiver<PartialResultSet>>>>,

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
            stream: Some(stream.compat().into_future()),
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

    pub async fn one(&mut self) -> Result<Vec<Value>> {
        if let Some(result) = self.one_or_none().await? {
            Ok(result)
        } else {
            Err(DbError::internal("No rows matched the given query."))?
        }
    }

    pub async fn one_or_none(&mut self) -> Result<Option<Vec<Value>>> {
        let result = self.next_async().await;
        if result.is_none() {
            Ok(None)
        } else if self.next_async().await.is_some() {
            Err(DbError::internal("Expected one result; got more."))?
        } else {
            result.transpose()
        }
    }

    /// Pull and process the next values from the Stream
    ///
    /// Returns false when the stream is finished
    async fn consume_next(&mut self) -> Result<bool> {
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
                Err(DbErrorKind::Integrity(
                    "Invalid PartialResultSet fields".to_owned(),
                ))?;
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
                let current_row = mem::replace(&mut self.current_row, vec![]);
                self.rows.push_back(current_row);
            }
        }
    }

    // We could implement Stream::poll_next instead of this, but
    // this is easier for now and we can refactor into the trait later
    pub async fn next_async(&mut self) -> Option<Result<Vec<Value>>> {
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

fn merge_by_type(lhs: Value, rhs: &Value, field_type: &Type) -> Result<Value> {
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
        TypeCode::ARRAY
        | TypeCode::FLOAT64
        | TypeCode::STRUCT
        | TypeCode::TYPE_CODE_UNSPECIFIED
        | TypeCode::BOOL => unsupported_merge(field_type),
    }
}

fn unsupported_merge(field_type: &Type) -> Result<Value> {
    Err(DbError::internal(&format!(
        "merge not supported, type: {:?}",
        field_type
    )))
}

fn merge_string(mut lhs: Value, rhs: &Value) -> Result<Value> {
    if !lhs.has_string_value() || !rhs.has_string_value() {
        Err(DbError::internal("merge_string has no string value"))?
    }
    let mut merged = lhs.take_string_value();
    merged.push_str(rhs.get_string_value());
    Ok(as_value(merged))
}

pub fn bso_from_row(mut row: Vec<Value>) -> Result<results::GetBso> {
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
                    .map_err(|e| DbErrorKind::Integrity(e.to_string()))?,
            )
        },
        payload: row[2].take_string_value(),
        modified,
        expiry: SyncTimestamp::from_rfc3339(&row[4].get_string_value())?.as_i64(),
    })
}

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

#[derive(Clone)]
pub struct MapAndThenIterator<I, F> {
    iter: I,
    f: F,
}

impl<A, B, E, I, F> Iterator for MapAndThenIterator<I, F>
where
    F: FnMut(A) -> StdResult<B, E>,
    I: Iterator<Item = StdResult<A, E>>,
{
    type Item = StdResult<B, E>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|result| result.and_then(&mut self.f))
    }
}

pub trait MapAndThenTrait {
    /// Return an iterator adaptor that applies the provided closure to every
    /// Result::Ok value. Result::Err values are unchanged.
    ///
    /// The closure can be used for control flow based on result values
    fn map_and_then<F, A, B, E>(self, func: F) -> MapAndThenIterator<Self, F>
    where
        Self: Sized + Iterator<Item = StdResult<A, E>>,
        F: FnMut(A) -> StdResult<B, E>,
    {
        MapAndThenIterator {
            iter: self,
            f: func,
        }
    }
}

impl<I, T, E> MapAndThenTrait for I where I: Sized + Iterator<Item = StdResult<T, E>> {}
