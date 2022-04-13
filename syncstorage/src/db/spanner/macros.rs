macro_rules! params {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(params!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { params!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = params!(@count $($key),*);
            let mut _value_map = ::std::collections::HashMap::with_capacity(_cap);
            let mut _type_map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                _type_map.insert($key.to_owned(), IntoSpannerValue::spanner_type(&$value));
                _value_map.insert($key.to_owned(), IntoSpannerValue::into_spanner_value($value));
            )*
            (_value_map, _type_map)
        }
    };
}

#[test]
fn test_params_macro() {
    use crate::db::spanner::support::IntoSpannerValue;
    use google_cloud_rust_raw::spanner::v1::type_pb::{Type, TypeCode};
    use protobuf::{
        well_known_types::{ListValue, Value},
        RepeatedField,
    };
    use std::collections::HashMap;

    let (sqlparams, sqlparam_types) = params! {
        "String param" => "I am a String".to_owned(),
        "i32 param" => 100i32,
        "u32 param" => 100u32,
        "Vec<String> param" => vec!["I am a String".to_owned()],
        "Vec<i32> param" => vec![100i32],
        "Vec<u32> param" => vec![100u32],
    };

    let mut expected_sqlparams = HashMap::new();
    let string_value = {
        let mut t = Value::new();
        t.set_string_value("I am a String".to_owned());
        t
    };
    expected_sqlparams.insert("String param".to_owned(), string_value.clone());

    let i32_value = {
        let mut t = Value::new();
        t.set_string_value(100i32.to_string());
        t
    };
    expected_sqlparams.insert("i32 param".to_owned(), i32_value.clone());

    let u32_value = {
        let mut t = Value::new();
        t.set_string_value(100u32.to_string());
        t
    };
    expected_sqlparams.insert("u32 param".to_owned(), u32_value.clone());

    let string_vec_value = {
        let mut list = ListValue::new();
        list.set_values(RepeatedField::from_vec(vec![string_value]));
        let mut value = Value::new();
        value.set_list_value(list);
        value
    };
    expected_sqlparams.insert("Vec<String> param".to_owned(), string_vec_value);

    let i32_vec_value = {
        let mut list = ListValue::new();
        list.set_values(RepeatedField::from_vec(vec![i32_value]));
        let mut value = Value::new();
        value.set_list_value(list);
        value
    };
    expected_sqlparams.insert("Vec<i32> param".to_owned(), i32_vec_value);

    let u32_vec_value = {
        let mut list = ListValue::new();
        list.set_values(RepeatedField::from_vec(vec![u32_value]));
        let mut value = Value::new();
        value.set_list_value(list);
        value
    };
    expected_sqlparams.insert("Vec<u32> param".to_owned(), u32_vec_value);

    let mut expected_sqlparam_types = HashMap::new();

    let string_type = {
        let mut t = Type::new();
        t.set_code(TypeCode::STRING);
        t
    };
    expected_sqlparam_types.insert("String param".to_owned(), string_type);

    let i32_type = {
        let mut t = Type::new();
        t.set_code(TypeCode::INT64);
        t
    };
    expected_sqlparam_types.insert("i32 param".to_owned(), i32_type);

    let u32_type = {
        let mut t = Type::new();
        t.set_code(TypeCode::INT64);
        t
    };
    expected_sqlparam_types.insert("u32 param".to_owned(), u32_type);

    let string_vec_type = {
        let mut element_type = Type::new();
        element_type.set_code(TypeCode::STRING);

        let mut vec_type = Type::new();
        vec_type.set_code(TypeCode::ARRAY);
        vec_type.set_array_element_type(element_type);

        vec_type
    };
    expected_sqlparam_types.insert("Vec<String> param".to_owned(), string_vec_type);

    let i32_vec_type = {
        let mut element_type = Type::new();
        element_type.set_code(TypeCode::INT64);

        let mut vec_type = Type::new();
        vec_type.set_code(TypeCode::ARRAY);
        vec_type.set_array_element_type(element_type);

        vec_type
    };
    expected_sqlparam_types.insert("Vec<i32> param".to_owned(), i32_vec_type);

    let u32_vec_type = {
        let mut element_type = Type::new();
        element_type.set_code(TypeCode::INT64);

        let mut vec_type = Type::new();
        vec_type.set_code(TypeCode::ARRAY);
        vec_type.set_array_element_type(element_type);

        vec_type
    };
    expected_sqlparam_types.insert("Vec<u32> param".to_owned(), u32_vec_type);

    assert_eq!(expected_sqlparams, sqlparams);
    assert_eq!(expected_sqlparam_types, sqlparam_types);
}
