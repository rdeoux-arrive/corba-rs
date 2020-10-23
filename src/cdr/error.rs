use serde::{de, ser};
use std::fmt::{self, Display};
use std::str::Utf8Error;
use std::{error, io, result};

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // Error constructed via the custom() method
    Message(String),
    // I/O error
    Io(io::Error),
    // Invalid boolean value (should be 0 or 1)
    Bool(u8),
    // Nul string lenght (should be > 0 because of the string terminator)
    StringLength,
    // Invalid string terminator (should be zero)
    StringTerminator(u8),
    // Invalid UTF-8 string
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
            Self::Io(err) => write!(formatter, "{}", err),
            Self::Bool(_) => write!(formatter, "Invalid boolean value"),
            Self::StringLength => write!(formatter, "Nul string lenght"),
            Self::StringTerminator(_) => write!(formatter, "Invalid string terminator"),
            Self::Utf8(err) => write!(formatter, "{}", err),
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8(err)
    }
}
