use crate::cdr::{self, Deserializer, Serializer};
use crate::corba::OctetSeq;
use crate::giop::{Magicn, Version};
use crate::iop::{ObjectKey, ServiceContextList};
use alloc::string::String;
use core::fmt::{self, Formatter};
use serde::{de, ser, Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MsgType {
    Request,
    Reply,
    CancelRequest,
    LocateRequest,
    LocateReply,
    CloseConnection,
    MessageError,
}

#[derive(Clone, Debug)]
pub struct MessageHeader {
    pub byte_order: bool,
    pub message_type: u8,
    pub message_size: u32,
}

impl MessageHeader {
    #[must_use]
    pub const fn new(message_type: MsgType) -> Self {
        Self {
            byte_order: false,
            message_type: message_type as u8,
            message_size: 0,
        }
    }

    #[must_use]
    pub const fn with_byte_order(self, byte_order: bool) -> Self {
        Self {
            byte_order,
            message_type: self.message_type,
            message_size: self.message_size,
        }
    }

    #[must_use]
    pub const fn with_message_size(self, message_size: u32) -> Self {
        Self {
            byte_order: self.byte_order,
            message_type: self.message_type,
            message_size,
        }
    }

    pub fn deserialize(buf: &[u8]) -> cdr::Result<(Self, Deserializer)> {
        let mut de = Deserializer::from(buf);
        let mut header: Self = Deserialize::deserialize(&mut de)?;
        let de = de.with_le(header.byte_order);
        if header.byte_order {
            header.message_size = header.message_size.swap_bytes();
        }
        Ok((header, de))
    }

    pub fn serialize<T>(&self) -> cdr::Result<Serializer<T>>
    where
        T: Extend<u8> + Default,
    {
        let mut serializer = Serializer::default().with_le(self.byte_order);
        Serialize::serialize(&self, &mut serializer)?;
        Ok(serializer)
    }
}

impl<'de> Deserialize<'de> for MessageHeader {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MessageHeader;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str("a GIOP message header")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                seq.next_element::<Magicn>()?
                    .ok_or_else(|| de::Error::missing_field("magic"))?;
                let giop_version = seq
                    .next_element::<Version>()?
                    .ok_or_else(|| de::Error::missing_field("GIOP_version"))?;
                if Version::GIOP10 == giop_version {
                    let byte_order = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::missing_field("byte_order"))?;
                    let message_type = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::missing_field("message_type"))?;
                    let message_size = seq
                        .next_element()?
                        .ok_or_else(|| de::Error::missing_field("message_size"))?;
                    Ok(Self::Value {
                        byte_order,
                        message_type,
                        message_size,
                    })
                } else {
                    Err(de::Error::invalid_value(de::Unexpected::Enum, &"1.0"))
                }
            }
        }

        deserializer.deserialize_struct(
            "MessageHeader",
            &[
                "magic",
                "GIOP_version",
                "byte_order",
                "message_type",
                "message_size",
            ],
            Visitor,
        )
    }
}

impl Serialize for MessageHeader {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        use ser::SerializeStruct;
        let mut st = serializer.serialize_struct("MessageHeader", 5)?;
        st.serialize_field("magic", &Magicn)?;
        st.serialize_field("GIOP_version", &Version::GIOP10)?;
        st.serialize_field("byte_order", &self.byte_order)?;
        st.serialize_field("message_type", &self.message_type)?;
        st.serialize_field("message_size", &self.message_size)?;
        st.end()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RequestHeader {
    pub service_context: ServiceContextList,
    pub request_id: u32,
    pub response_expected: bool,
    pub object_key: ObjectKey,
    pub operation: String,
    pub requesting_principal: OctetSeq,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum ReplyStatusType {
    NoException,
    UserException,
    SystemException,
    LocationForward,
}

#[derive(Serialize)]
pub struct ReplyHeader {
    pub service_context: ServiceContextList,
    pub request_id: u32,
    pub reply_status: ReplyStatusType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemExceptionReplyBody {
    pub exception_id: &'static str,
    pub minor_code_value: u32,
    pub completion_status: u32,
}

macro_rules! exception {
    ($n:ident = $m:ident) => {
        impl From<crate::corba::$n> for SystemExceptionReplyBody {
            fn from(exc: crate::corba::$n) -> Self {
                Self {
                    exception_id: stringify!($m),
                    minor_code_value: exc.minor,
                    completion_status: exc.completed as u32,
                }
            }
        }
    };
}
exception!(Unknown = UNKNOWN);
exception!(BadParam = BAD_PARAM);
exception!(NoMemory = NO_MEMORY);
exception!(ImpLimit = IMP_LIMIT);
exception!(CommFailure = COMM_FAILURE);
exception!(InvObjref = INV_OBJREF);
exception!(NoPermission = NO_PERMISSION);
exception!(Internal = INTERNAL);
exception!(Marshal = MARSHAL);
exception!(Initialize = INITIALIZE);
exception!(NoImplement = NO_IMPLEMENT);
exception!(BadTypecode = BAD_TYPECODE);
exception!(BadOperation = BAD_OPERATION);
exception!(NoResources = NO_RESOURCES);
exception!(NoResponse = NO_RESPONSE);
exception!(PersistStore = PERSIST_STORE);
exception!(BadInvOrder = BAD_INV_ORDER);
exception!(Transient = TRANSIENT);
exception!(FreeMem = FREE_MEM);
exception!(InvIdent = INV_IDENT);
exception!(InvFlag = INV_FLAG);
exception!(IntfRepos = INTF_REPOS);
exception!(BadContext = BAD_CONTEXT);
exception!(ObjAdapter = OBJ_ADAPTER);
exception!(DataConversion = DATA_CONVERSION);
exception!(ObjectNotExist = OBJECT_NOT_EXIST);
exception!(TransactionRequired = TRANSACTION_REQUIRED);
exception!(TransactionRolledback = TRANSACTION_ROLLEDBACK);
exception!(InvalidTransaction = INVALID_TRANSACTION);
exception!(InvPolicy = INV_POLICY);
exception!(CodesetIncompatible = CODESET_INCOMPATIBLE);
exception!(Rebind = REBIND);
exception!(Timeout = TIMEOUT);
exception!(TransactionUnavailable = TRANSACTION_UNAVAILABLE);
exception!(TransactionMode = TRANSACTION_MODE);
exception!(BadQos = BAD_QOS);
