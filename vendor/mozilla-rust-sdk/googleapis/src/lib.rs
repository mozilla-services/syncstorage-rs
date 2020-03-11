use std::error;
use std::fmt;

pub mod spanner;

#[derive(Debug)]
pub struct Error(grpcio::Error);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl error::Error for Error {}

impl From<grpcio::Error> for Error {
    fn from(err: grpcio::Error) -> Error {
        Error(err)
    }
}

pub type Result<T> = std::result::Result<T, crate::Error>;
