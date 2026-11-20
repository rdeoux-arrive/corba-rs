use crate::cdr::{self, Deserializer, Serializer};
use crate::corba::OctetSeq;
use crate::giop::{Magicn, Version};
use crate::iop::ObjectKey;
use crate::iop::ServiceContextList;
use alloc::string::String;
use serde::{Deserialize, Serialize};

pub use crate::giop10::{ReplyHeader, SystemExceptionReplyBody};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MsgType {
    Request,
    Reply,
    CancelRequest,
    LocateRequest,
    LocateReply,
    CloseConnection,
    MessageError,
    Fragment, // GIOP 1.1 addition
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MessageHeader {
    pub magic: Magicn,
    pub giop_version: Version,
    pub flags: u8, // GIOP 1.1 change
    pub message_type: u8,
    pub message_size: u32,
}

impl MessageHeader {
    #[must_use]
    pub fn new(giop_version: Version, message_type: MsgType) -> Self {
        assert!(
            Version::GIOP10 != giop_version,
            "giop_version should not be {}",
            stringify!(Version::GIOP10)
        );
        Self {
            magic: Magicn,
            giop_version,
            flags: 0,
            message_type: message_type as u8,
            message_size: 0,
        }
    }

    #[must_use]
    pub const fn with_flags(self, flags: u8) -> Self {
        Self {
            magic: self.magic,
            giop_version: self.giop_version,
            flags,
            message_type: self.message_type,
            message_size: self.message_size,
        }
    }

    #[must_use]
    pub const fn with_message_size(self, message_size: u32) -> Self {
        Self {
            magic: self.magic,
            giop_version: self.giop_version,
            flags: self.flags,
            message_type: self.message_type,
            message_size,
        }
    }

    pub fn deserialize(buf: &[u8]) -> cdr::Result<(Self, Deserializer<'_>)> {
        let mut de = Deserializer::from(buf);
        let (magic, giop_version, flags) = <(Magicn, Version, u8)>::deserialize(&mut de)?;
        let mut de = de.with_le(flags & 1 != 0);
        let (message_type, message_size) = <(u8, u32)>::deserialize(&mut de)?;
        Ok((
            Self {
                magic,
                giop_version,
                flags,
                message_type,
                message_size,
            },
            de,
        ))
    }

    pub fn serialize<T>(&self) -> cdr::Result<Serializer<T>>
    where
        T: Extend<u8> + Default,
    {
        let mut serializer = Serializer::default().with_le(self.flags & 1 != 0);
        Serialize::serialize(&self, &mut serializer)?;
        Ok(serializer)
    }
}

pub type RequestReserved = [i8; 3];

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestHeader {
    pub service_context: ServiceContextList,
    pub request_id: u32,
    pub response_expected: bool,
    pub reserved: RequestReserved, // Added in GIOP 1.1
    pub object_key: ObjectKey,
    pub operation: String,
    pub requesting_principal: OctetSeq,
}
