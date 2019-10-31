macro_rules! params {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(params!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { params!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = params!(@count $($key),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                _map.insert($key.to_owned(), as_value($value));
            )*
            _map
        }
    };
}

macro_rules! param_types {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(param_types!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { param_types!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = param_types!(@count $($key),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                _map.insert($key.to_owned(), crate::db::spanner::support::as_type($value));
            )*
            _map
        }
    };
}
