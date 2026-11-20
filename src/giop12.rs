use crate::iop::{IOR, ObjectKey, ServiceContextList, TaggedProfile};
use alloc::borrow::ToOwned;
use alloc::format;
use alloc::string::String;
use core::fmt::{self, Debug, Formatter};
use serde::de::{Error, SeqAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub use crate::giop11::{MessageHeader, MsgType, RequestReserved, SystemExceptionReplyBody};

pub type AddressingDisposition = i16;
pub const KEY_ADDR: i16 = 0;
pub const PROFILE_ADDR: i16 = 1;
pub const REFERENCE_ADDR: i16 = 2;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct IORAddressingInfo {
    pub selected_profile_index: u32,
    pub ior: IOR,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TargetAddress {
    ObjectKey(ObjectKey),
    Profile(TaggedProfile),
    Ior(IORAddressingInfo),
}

impl Debug for TargetAddress {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::ObjectKey(key) => match core::str::from_utf8(key) {
                Ok(key) if key.is_ascii() => f.debug_tuple("ObjectKey").field(&key).finish(),
                _ => f.debug_tuple("ObjectKey").field(&key).finish(),
            },
            Self::Profile(profile) => f.debug_tuple("Profile").field(&profile).finish(),
            Self::Ior(ior) => f.debug_tuple("Ior").field(&ior).finish(),
        }
    }
}

impl<'de> Deserialize<'de> for TargetAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TargetAddressVisitor;

        impl<'de> Visitor<'de> for TargetAddressVisitor {
            type Value = TargetAddress;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("a valid TargetAddress")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let addressing_disposition = seq
                    .next_element()?
                    .ok_or_else(|| Error::missing_field("AddressingPosition"))?;
                match addressing_disposition {
                    KEY_ADDR => {
                        let object_key = seq
                            .next_element()?
                            .ok_or_else(|| Error::missing_field("object_key"))?;
                        Ok(TargetAddress::ObjectKey(object_key))
                    }
                    PROFILE_ADDR => {
                        let profile = seq
                            .next_element()?
                            .ok_or_else(|| Error::missing_field("profile"))?;
                        Ok(TargetAddress::Profile(profile))
                    }
                    REFERENCE_ADDR => {
                        let ior = seq
                            .next_element()?
                            .ok_or_else(|| Error::missing_field("ior"))?;
                        Ok(TargetAddress::Ior(ior))
                    }
                    unsupported => Err(Error::custom(format!(
                        "unsupported enum variant: {unsupported} ({unsupported:#x})"
                    ))),
                }
            }
        }

        const FIELDS: &[&str] = &["AddressingDisposition", "variant"];
        deserializer.deserialize_struct("TargetAddress", FIELDS, TargetAddressVisitor)
    }
}

impl Serialize for TargetAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("TargetAddress", 2)?;
        match self {
            Self::ObjectKey(object_key) => {
                st.serialize_field("AddressingDisposition", &KEY_ADDR)?;
                st.serialize_field("object_key", &object_key)
            }
            Self::Profile(profile) => {
                st.serialize_field("AddressingDisposition", &PROFILE_ADDR)?;
                st.serialize_field("profile", &profile)
            }
            Self::Ior(ior) => {
                st.serialize_field("AddressingDisposition", &REFERENCE_ADDR)?;
                st.serialize_field("ior", &ior)
            }
        }?;
        st.end()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestHeader {
    pub request_id: u32,
    pub response_flags: i8,
    pub reserved: RequestReserved,
    pub target: TargetAddress,
    pub operation: String,
    pub service_context: ServiceContextList, // 1.2 change
}

impl RequestHeader {
    #[must_use]
    pub fn new(request_id: u32, oneway: bool, target: TargetAddress, operation: &str) -> Self {
        Self {
            request_id,
            response_flags: if oneway { 0 } else { 3 },
            reserved: RequestReserved::default(),
            target,
            operation: operation.to_owned(),
            service_context: ServiceContextList::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ReplyStatusType {
    NoException,
    UserException,
    SystemException,
    LocationForward,
    LocationForwardPerm, // new value for 1.2
    NeedsAddressingMode, // new value for 1.2
}

#[derive(Deserialize, Serialize)]
pub struct ReplyHeader {
    pub request_id: u32,
    pub reply_status: ReplyStatusType,
    pub service_context: ServiceContextList, // 1.2 change
}
