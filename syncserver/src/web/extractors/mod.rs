//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

mod constants;
pub use constants::*;
mod utils;
pub use utils::*;
mod validation;
pub use validation::*;
mod metrics;
pub use metrics::*;

mod hawk_identifier;
pub use hawk_identifier::*;
mod offset;
pub use offset::*;
mod precondition_header;
pub use precondition_header::*;

mod batch_bso_body;
pub use batch_bso_body::*;
mod bso_body;
pub use bso_body::*;
mod bso_bodies;
pub use bso_bodies::*;
mod bso_param;
pub use bso_param::*;
mod bso_query_params;
pub use bso_query_params::*;
mod collection_param;
pub use collection_param::*;

mod batch_request;
pub use batch_request::*;
mod bso_request;
pub use bso_request::*;
mod bso_put_request;
pub use bso_put_request::*;
mod collection_request;
pub use collection_request::*;
mod collection_post_request;
pub use collection_post_request::*;
mod heartbeat_request;
pub use heartbeat_request::*;
mod meta_request;
pub use meta_request::*;
mod test_error_request;
pub use test_error_request::*;

#[cfg(test)]
mod tests;
