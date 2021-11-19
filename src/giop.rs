use crate::cdr::{self, Deserializer, Serializer};
use crate::{giop10, giop11, giop12, giop13};
use alloc::format;
use serde::{de, ser, Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Magicn;

impl<'de> Deserialize<'de> for Magicn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let v = <[u8; 4]>::deserialize(deserializer)?;
        if &v != b"GIOP" {
            return Err(de::Error::invalid_value(de::Unexpected::Bytes(&v), &"GIOP"));
        }
        Ok(Self)
    }
}

impl Serialize for Magicn {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        b"GIOP".serialize(serializer)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Version {
    GIOP10,
    GIOP11,
    GIOP12,
    GIOP13,
}

impl Version {
    #[must_use]
    pub const fn minor(&self) -> u8 {
        match self {
            Self::GIOP10 => 0,
            Self::GIOP11 => 1,
            Self::GIOP12 => 2,
            Self::GIOP13 => 3,
        }
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let (major, minor) = <(u8, u8)>::deserialize(deserializer)?;
        let v = match (major, minor) {
            (1, 0) => Self::GIOP10,
            (1, 1) => Self::GIOP11,
            (1, 2) => Self::GIOP12,
            (1, 3) => Self::GIOP13,
            (major, minor) => {
                return Err(de::Error::custom(format!(
                    "Unsupported GIOP version: {}.{}",
                    major, minor
                )))
            }
        };
        Ok(v)
    }
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct;
        let mut st = serializer.serialize_struct("Version", 2)?;
        st.serialize_field("major", &1_u8)?;
        st.serialize_field("minor", &self.minor())?;
        st.end()
    }
}

#[derive(Clone, Debug)]
pub enum MessageHeader {
    V1_0(giop10::MessageHeader),
    V1_1(giop11::MessageHeader),
    V1_2(giop12::MessageHeader),
    V1_3(giop13::MessageHeader),
}

impl MessageHeader {
    pub fn deserialize<'a>(buf: &'a [u8]) -> cdr::Result<(Self, Deserializer<'a>)> {
        let mut de = Deserializer::from(buf);
        let magic = Magicn::deserialize(&mut de)?;
        let giop_version = Version::deserialize(&mut de)?;
        match giop_version {
            Version::GIOP10 => {
                let byte_order = bool::deserialize(&mut de)?;
                let mut de = de.with_le(byte_order);
                let message_type = u8::deserialize(&mut de)?;
                let message_size = u32::deserialize(&mut de)?;
                Ok((
                    Self::V1_0(giop10::MessageHeader {
                        byte_order,
                        message_type,
                        message_size,
                    }),
                    de,
                ))
            }
            giop_version => {
                let flags = u8::deserialize(&mut de)?;
                let mut de = de.with_le(flags & 1 != 0);
                let message_type = u8::deserialize(&mut de)?;
                let message_size = u32::deserialize(&mut de)?;
                let header = giop11::MessageHeader {
                    magic,
                    giop_version,
                    flags,
                    message_type,
                    message_size,
                };
                Ok((
                    match header.giop_version {
                        Version::GIOP10 => unreachable!(),
                        Version::GIOP11 => Self::V1_1(header),
                        Version::GIOP12 => Self::V1_2(header),
                        Version::GIOP13 => Self::V1_3(header),
                    },
                    de,
                ))
            }
        }
    }

    pub fn serialize<T>(self) -> cdr::Result<Serializer<T>>
    where
        T: Extend<u8> + Default,
    {
        match self {
            Self::V1_0(header) => header.serialize(),
            Self::V1_1(header) | Self::V1_2(header) | Self::V1_3(header) => header.serialize(),
        }
    }
}

pub enum ReplyHeader {
    V1_0(giop10::ReplyHeader),
    V1_2(giop12::ReplyHeader),
}

#[derive(Clone, Debug)]
pub enum RequestHeader {
    V1_0(giop10::RequestHeader),
    V1_1(giop11::RequestHeader),
    V1_2(giop12::RequestHeader),
}
