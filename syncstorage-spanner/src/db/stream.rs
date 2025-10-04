use std::{collections::VecDeque, mem};

use futures::{stream::StreamFuture, Stream, StreamExt};
use google_cloud_rust_raw::spanner::v1::{
    result_set::{PartialResultSet, ResultSetMetadata, ResultSetStats},
    type_pb::{StructType_Field, Type, TypeCode},
};
use grpcio::ClientSStreamReceiver;
use protobuf::well_known_types::Value;

use super::support::IntoSpannerValue;
use crate::{error::DbError, DbResult};

pub struct StreamedResultSetAsync<T = ClientSStreamReceiver<PartialResultSet>> {
    /// Stream from execute_streaming_sql
    stream: Option<StreamFuture<T>>,

    metadata: Option<ResultSetMetadata>,
    stats: Option<ResultSetStats>,

    /// Fully-processed rows
    rows: VecDeque<Vec<Value>>,
    /// Accumulated values for incomplete row
    current_row: Vec<Value>,
    /// Incomplete value
    pending_chunk: Option<Value>,
}

impl<T> StreamedResultSetAsync<T>
where
    T: Stream<Item = grpcio::Result<PartialResultSet>> + Unpin,
{
    pub fn new(stream: T) -> Self {
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
        let result = self.try_next().await?;
        if result.is_none() {
            Ok(None)
        } else if self.try_next().await?.is_some() {
            Err(DbError::internal(
                "Expected one result; got more.".to_owned(),
            ))
        } else {
            Ok(result)
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
    // this is easier for now and we can refactor into the trait later.
    pub async fn try_next(&mut self) -> DbResult<Option<Vec<Value>>> {
        while self.rows.is_empty() {
            // Note: Iteration may continue after an error. We may want to
            // stop afterwards instead for safety sake (it's not really
            // recoverable)
            if !self.consume_next().await? {
                return Ok(None);
            }
        }
        Ok(self.rows.pop_front())
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

#[cfg(test)]
mod tests {
    use futures::stream;
    use google_cloud_rust_raw::spanner::v1::{
        result_set::{PartialResultSet, ResultSetMetadata},
        type_pb::{StructType, StructType_Field, Type, TypeCode},
    };
    use grpcio::Error::GoogleAuthenticationFailed;
    use protobuf::well_known_types::Value;

    use super::StreamedResultSetAsync;
    use crate::error::DbErrorKind;

    fn simple_part() -> PartialResultSet {
        let mut field_type = Type::default();
        field_type.set_code(TypeCode::INT64);

        let mut field = StructType_Field::default();
        field.set_name("foo".to_owned());
        field.set_field_type(field_type);

        let mut row_type = StructType::default();
        row_type.set_fields(vec![field].into());

        let mut metadata = ResultSetMetadata::default();
        metadata.set_row_type(row_type);

        let mut part = PartialResultSet::default();
        part.set_metadata(metadata);

        let mut value = Value::default();
        value.set_string_value("22".to_owned());
        part.set_values(vec![value].into());

        part
    }

    #[actix_web::test]
    async fn consume_next_err() {
        let mut s = StreamedResultSetAsync::new(stream::iter([
            Ok(simple_part()),
            Err(GoogleAuthenticationFailed),
        ]));
        assert!(s.consume_next().await.unwrap());
        let err = s.consume_next().await.unwrap_err();
        assert!(matches!(
            err.kind,
            DbErrorKind::Grpc(GoogleAuthenticationFailed)
        ));
    }

    #[actix_web::test]
    async fn one_or_none_err_propagate() {
        let mut s = StreamedResultSetAsync::new(stream::iter([
            Ok(simple_part()),
            Err(GoogleAuthenticationFailed),
        ]));
        let err = s.one_or_none().await.unwrap_err();
        // Note:resolves historic Sentry error. Uncomment dbg! for debugging only.
        // See: https://github.com/mozilla-services/syncstorage-rs/issues/1384
        //dbg!(&err);
        assert!(matches!(
            err.kind,
            DbErrorKind::Grpc(GoogleAuthenticationFailed)
        ));
    }
}
