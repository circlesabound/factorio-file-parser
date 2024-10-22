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
            Error::Message(msg) => write!(f, "factorio-file-parser::Error::Message({})", msg),
            Error::ByteSlicingError => write!(f, "factorio-file-parser::Error::ByteSlicingError"),
            Error::Eof => write!(f, "factorio-file-parser::Error::Eof"),
            Error::OutOfRange => write!(f, "factorio-file-parser::Error::OutOfRange"),
            Error::Syntax(msg) => write!(f, "factorio-file-parser::Error::Syntax({})", msg),
            Error::TrailingBytes => write!(f, "factorio-file-parser::Error::TrailingBytes"),
            Error::Utf8(utf8_error) => write!(f, "factorio-file-parser::Error::Utf8({})", utf8_error),
            
        }
    }
}

impl std::error::Error for Error {}
