//! Web authentication, handlers, and middleware
pub mod auth;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod tags;

// header statics must be lower case, numbers and symbols per the RFC spec. This reduces chance of error.
pub static X_LAST_MODIFIED: &str = "x-last-modified";
pub static X_WEAVE_TIMESTAMP: &str = "x-weave-timestamp";
pub static X_WEAVE_NEXT_OFFSET: &str = "x-weave-next-offset";
pub static X_WEAVE_RECORDS: &str = "x-weave-records";
pub static X_WEAVE_BYTES: &str = "x-weave-bytes";
pub static X_WEAVE_TOTAL_RECORDS: &str = "x-weave-total-records";
pub static X_WEAVE_TOTAL_BYTES: &str = "x-weave-total-bytes";
pub static X_VERIFY_CODE: &str = "x-verify-code";

// Known DockerFlow commands for Ops callbacks
pub const DOCKER_FLOW_ENDPOINTS: [&str; 4] = [
    "/__heartbeat__",
    "/__lbheartbeat__",
    "/__version__",
    "/__error__",
];

#[macro_export]
macro_rules! label {
    ($string:expr) => {
        Some($string.to_string())
    };
}
