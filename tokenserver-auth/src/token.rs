#[cfg(not(feature = "py"))]
mod native;

#[cfg(feature = "py")]
mod py;

#[cfg(feature = "py")]
pub type Tokenlib = py::PyTokenlib;

#[cfg(not(feature = "py"))]
pub type Tokenlib = native::Tokenlib;
