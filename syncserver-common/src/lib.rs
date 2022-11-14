use hkdf::Hkdf;
use sha2::Sha256;

// header statics must be lower case, numbers and symbols per the RFC spec. This reduces chance of error.
pub static X_LAST_MODIFIED: &str = "x-last-modified";
pub static X_WEAVE_TIMESTAMP: &str = "x-weave-timestamp";
pub static X_WEAVE_NEXT_OFFSET: &str = "x-weave-next-offset";
pub static X_WEAVE_RECORDS: &str = "x-weave-records";
pub static X_WEAVE_BYTES: &str = "x-weave-bytes";
pub static X_WEAVE_TOTAL_RECORDS: &str = "x-weave-total-records";
pub static X_WEAVE_TOTAL_BYTES: &str = "x-weave-total-bytes";
pub static X_VERIFY_CODE: &str = "x-verify-code";

// max load size in bytes
pub const MAX_SPANNER_LOAD_SIZE: usize = 100_000_000;

/// Helper function for [HKDF](https://tools.ietf.org/html/rfc5869) expansion to 32 bytes.
pub fn hkdf_expand_32(info: &[u8], salt: Option<&[u8]>, key: &[u8]) -> Result<[u8; 32], String> {
    let mut result = [0u8; 32];
    let hkdf = Hkdf::<Sha256>::new(salt, key);
    hkdf.expand(info, &mut result)
        .map_err(|e| format!("HKDF Error: {:?}", e))?;
    Ok(result)
}

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

pub trait ReportableError {
    fn error_backtrace(&self) -> String;
    fn is_sentry_event(&self) -> bool;
    fn metric_label(&self) -> Option<String>;
}

/// Types that implement this trait can represent internal errors.
pub trait InternalError {
    /// Constructs an internal error with the given error message.
    fn internal_error(message: String) -> Self;
}
