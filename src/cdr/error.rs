use alloc::string::{String, ToString};
use core::fmt::{self, Display};
use core::{num::TryFromIntError, str::Utf8Error};
use serde::{de, de::StdError, ser};

pub type Result<T> = ::core::result::Result<T, Error>;

/// Serialization/deserialization error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Error constructed via the `custom()` method
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
    /// Length or size that does not fit in 32 bits
    TooLarge,
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
            Self::Message(msg) => write!(formatter, "{msg}"),
            Self::Truncated => write!(formatter, "Truncated buffer"),
            Self::Bool(_) => write!(formatter, "Invalid boolean value"),
            Self::StringLength => write!(formatter, "Nul string lenght"),
            Self::StringTerminator(_) => write!(formatter, "Invalid string terminator"),
            Self::Utf8(err) => write!(formatter, "{err}"),
            Self::TooLarge => write!(formatter, "Length does not fit in 32 bits"),
        }
    }
}

impl StdError for Error {}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::TooLarge
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}
