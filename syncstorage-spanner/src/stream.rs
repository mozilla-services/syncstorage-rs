use std::{collections::VecDeque, mem};

use futures::stream::{StreamExt, StreamFuture};
use google_cloud_rust_raw::spanner::v1::{
    result_set::{PartialResultSet, ResultSetMetadata, ResultSetStats},
    type_pb::{StructType_Field, Type, TypeCode},
};
use grpcio::ClientSStreamReceiver;
use protobuf::well_known_types::Value;

use crate::{error::DbError, support::IntoSpannerValue, DbResult};

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
