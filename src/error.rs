use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    // Generic variants created by internal data structures
    Message(String),

    // Format-specific variants
    ByteSlicingError,
    Eof,
    OutOfRange,
    Syntax(String),
    TrailingBytes,
    Utf8(std::str::Utf8Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => write!(f, "{}", msg),
            _ => unimplemented!(),
        }
    }
}

impl std::error::Error for Error {}
