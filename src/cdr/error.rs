use alloc::string::{String, ToString};
use core::fmt::{self, Display};
use core::str::Utf8Error;
use serde::{de, de::StdError, ser};

pub type Result<T> = ::core::result::Result<T, Error>;

/// Serialization/deserialization error
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// Error constructed via the custom() method
    Message(String),
    /// No more bytes at the end of the buffer
    Truncated,
    /// Invalid boolean value (should be 0 or 1)
    Bool(u8),
    /// Nul string length (should be > 0 because of the string terminator)
    StringLength,
    /// Invalid string terminator (should be zero)
    StringTerminator(u8),
    /// Invalid UTF-8 string
    Utf8(Utf8Error),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Self::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Message(msg) => write!(formatter, "{}", msg),
            Self::Truncated => write!(formatter, "Truncated buffer"),
            Self::Bool(_) => write!(formatter, "Invalid boolean value"),
            Self::StringLength => write!(formatter, "Nul string lenght"),
            Self::StringTerminator(_) => write!(formatter, "Invalid string terminator"),
            Self::Utf8(err) => write!(formatter, "{}", err),
        }
    }
}

impl StdError for Error {}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}
