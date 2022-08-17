#[macro_export]
macro_rules! from_error {
    ($from:ty, $to:ty, $to_kind:expr) => {
        impl From<$from> for $to {
            fn from(inner: $from) -> $to {
                $to_kind(inner).into()
            }
        }
    };
}

#[macro_export]
macro_rules! impl_fmt_display {
    ($error:ty, $kind:ty) => {
        impl fmt::Display for $error {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.kind, formatter)
            }
        }
    };
}

pub trait ErrorBacktrace {
    fn error_backtrace(&self) -> String;
}
