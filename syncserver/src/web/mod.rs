//! Web authentication, handlers, and middleware
pub mod auth;
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod middleware;
pub mod tags;

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
