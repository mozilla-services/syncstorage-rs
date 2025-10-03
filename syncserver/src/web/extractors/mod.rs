//! Request header/body/query extractors
//!
//! Handles ensuring the header's, body, and query parameters are correct, extraction to
//! relevant types, and failing correctly with the appropriate errors if issues arise.

// Only used within extractors
mod constants;
use constants::*;
mod validation;
use validation::*;

mod utils;
pub(crate) use utils::RequestErrorLocation;
use utils::{get_accepted, urldecode};

mod batch_bso_body;
use batch_bso_body::*;
mod bso_bodies;
use bso_bodies::*;
mod bso_query_params;
use bso_query_params::*;
mod batch_request;
use batch_request::*;

mod bso_body;
pub(crate) use bso_body::*;

mod hawk_identifier;
pub(crate) use hawk_identifier::*;
mod precondition_header;
pub(crate) use precondition_header::*;
mod bso_param;
pub(crate) use bso_param::*;
mod collection_param;
pub(crate) use collection_param::*;

mod metrics;
pub(crate) use metrics::*;
mod bso_request;
pub(crate) use bso_request::*;
mod bso_put_request;
pub(crate) use bso_put_request::*;
mod collection_request;
pub(crate) use collection_request::*;
mod collection_post_request;
pub(crate) use collection_post_request::*;
mod heartbeat_request;
pub(crate) use heartbeat_request::*;
mod meta_request;
pub(crate) use meta_request::*;
mod test_error_request;
pub(crate) use test_error_request::*;

#[cfg(test)]
mod test_utils;
