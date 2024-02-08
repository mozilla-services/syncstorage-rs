use serde::{Deserialize, Serialize};

#[cfg(not(feature = "py"))]
mod native;
#[cfg(feature = "py")]
mod py;

#[cfg(feature = "py")]
pub type Verifier = py::Verifier;

#[cfg(not(feature = "py"))]
pub type Verifier<J> = native::Verifier<J>;

/// The information extracted from a valid OAuth token.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct VerifyOutput {
    #[serde(rename = "user")]
    pub fxa_uid: String,
    pub generation: Option<i64>,
}
