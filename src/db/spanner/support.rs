#[cfg(google_grpc)]
use googleapis_raw::spanner::v1::type_pb::TypeCode;
#[cfg(google_grpc)]
use protobuf::well_known_types::Value;

// XXX: or Into<protobuf Value>?
#[cfg(google_grpc)]
pub fn as_value(string_value: String) -> Value {
    let mut value = Value::new();
    value.set_string_value(string_value);
    value
}

#[cfg(not(google_grpc))]
pub fn as_value(string_value: String) -> String {
    string_value
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

#[cfg(google_grpc)]
impl Into<googleapis_raw::spanner::v1::type_pb::Type> for SpannerType {
    fn into(self) -> googleapis_raw::spanner::v1::type_pb::Type {
        let mut t = googleapis_raw::spanner::v1::type_pb::Type::new();
        t.set_code(match self {
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
        });
        t
    }
}

impl Into<google_spanner1::Type> for SpannerType {
    fn into(self) -> google_spanner1::Type {
        let code = match self {
            SpannerType::TypeCodeUnspecified => "TYPE_CODE_UNSPECIFIED",
            SpannerType::Bool => "BOOL",
            SpannerType::Int64 => "INT64",
            SpannerType::Float64 => "FLOAT64",
            SpannerType::Timestamp => "TIMESTAMP",
            SpannerType::Date => "DATE",
            SpannerType::String => "STRING",
            SpannerType::Bytes => "BYTES",
            SpannerType::Array => "ARRAY",
            SpannerType::Struct => "STRUCT",
        };
        google_spanner1::Type {
            code: Some(code.to_owned()),
            ..Default::default()
        }
    }
}
