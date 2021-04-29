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
                _value_map.insert($key.to_owned(), ToSpannerValue::to_spanner_value(&$value));
                _type_map.insert($key.to_owned(), ToSpannerValue::spanner_type(&$value));
            )*
            (_value_map, _type_map)
        }
    };
}
