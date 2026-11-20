use crate::cdr::{self, size_of, to_bytes};
use crate::giop::MessageHeader;
use crate::giop::Version;
use crate::giop12::{
    self, MsgType, ReplyHeader, ReplyStatusType, RequestHeader, SystemExceptionReplyBody,
    TargetAddress,
};
use core::fmt::{self, Display, Formatter};
use embedded_nal::TcpClientStack;
use log::debug;
use serde::{Deserialize, Serialize};

/// Sending error
#[derive(Debug)]
pub enum SendError<Stack> {
    /// Serialization error
    Cdr(cdr::Error),
    /// Network related error
    Stack(Stack),
}

impl<Stack> From<cdr::Error> for SendError<Stack> {
    fn from(err: cdr::Error) -> Self {
        Self::Cdr(err)
    }
}

impl<Stack> Display for SendError<Stack>
where
    Stack: Display,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Cdr(err) => write!(f, "serialization failure: {err}"),
            Self::Stack(err) => write!(f, "network failure: {err}"),
        }
    }
}

pub type SendResult<Stack> = nb::Result<(), SendError<Stack>>;

/// Receive error
#[derive(Debug)]
pub enum RecvError<User, Stack> {
    /// Deserialization error
    Cdr(cdr::Error),
    /// Unexpected message type or version
    Message,
    /// Server-side system exception
    System(SystemExceptionReplyBody),
    /// Server-side user exception
    User(User),
    /// Network related error
    Stack(Stack),
}

impl<User, Stack> From<cdr::Error> for RecvError<User, Stack> {
    fn from(err: cdr::Error) -> Self {
        Self::Cdr(err)
    }
}

impl<User, Stack> From<SystemExceptionReplyBody> for RecvError<User, Stack> {
    fn from(err: SystemExceptionReplyBody) -> Self {
        Self::System(err)
    }
}

impl<User, Stack> Display for RecvError<User, Stack>
where
    User: Display,
    Stack: Display,
{
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Cdr(err) => write!(f, "deserialization failure: {err}"),
            Self::Message => write!(f, "unexpected message type or version"),
            Self::System(err) => write!(f, "system exception: {}", err.exception_id),
            Self::User(err) => write!(f, "user exception: {err}"),
            Self::Stack(err) => write!(f, "network failure: {err}"),
        }
    }
}

pub type RecvResult<T, User, Stack> = nb::Result<T, RecvError<User, Stack>>;

/// CORBA client.
pub struct Client<Stack>
where
    Stack: TcpClientStack,
{
    request_id: u32,
    byte_order: bool,
    stream: Stack::TcpSocket,
}

impl<Stack> Client<Stack>
where
    Stack: TcpClientStack,
{
    /// Create a new CORBA client.
    pub const fn new(stream: Stack::TcpSocket) -> Self {
        Self {
            request_id: 0,
            byte_order: false,
            stream,
        }
    }

    /// Change the byte order of the client.
    #[must_use]
    pub fn with_byte_order(self, byte_order: bool) -> Self {
        Self { byte_order, ..self }
    }

    /// Send a request.
    pub fn send<T>(
        &mut self,
        stack: &mut Stack,
        oneway: bool,
        target: TargetAddress,
        operation: &str,
        body: T,
    ) -> SendResult<Stack::Error>
    where
        T: Serialize,
    {
        #[derive(Serialize)]
        struct Req<T> {
            message: giop12::MessageHeader,
            request: RequestHeader,
            body: T,
        }

        debug!("send request #{}", self.request_id);
        let mut req = Req {
            message: giop12::MessageHeader::new(Version::GIOP12, MsgType::Request)
                .with_flags(u8::from(self.byte_order)),
            request: RequestHeader::new(self.request_id, oneway, target, operation),
            body,
        };
        req.message.message_size =
            size_of((&req.request, &req.body)).map_err(SendError::Cdr)? as u32;
        let bytes = to_bytes(self.byte_order, &req).map_err(SendError::Cdr)?;
        debug!("send {} bytes", bytes.len());
        stack
            .send(&mut self.stream, &bytes)
            .map_err(|err| err.map(SendError::Stack))?;
        self.request_id += 1;
        Ok(())
    }

    /// Try to receive a reply.
    pub fn recv<'de, O, E>(&mut self, stack: &mut Stack) -> RecvResult<O, E, Stack::Error>
    where
        O: Deserialize<'de>,
        E: Deserialize<'de>,
    {
        let mut buffer = [0; 1500];
        let size = stack
            .receive(&mut self.stream, &mut buffer)
            .map_err(|err| err.map(RecvError::Stack))?;
        let (header, mut de) =
            MessageHeader::deserialize(&buffer[..size]).map_err(RecvError::Cdr)?;
        match header {
            MessageHeader::V1_2(header) => {
                if header.message_type == MsgType::Reply as u8 {
                    let reply = ReplyHeader::deserialize(&mut de).map_err(RecvError::Cdr)?;
                    match reply.reply_status {
                        ReplyStatusType::NoException => {
                            let body = <_>::deserialize(&mut de).map_err(RecvError::Cdr)?;
                            Ok(body)
                        }
                        ReplyStatusType::SystemException => {
                            let err = SystemExceptionReplyBody::deserialize(&mut de)
                                .map_err(RecvError::Cdr)?;
                            Err(RecvError::System(err).into())
                        }
                        ReplyStatusType::UserException => {
                            let err = E::deserialize(&mut de).map_err(RecvError::Cdr)?;
                            Err(RecvError::User(err).into())
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    Err(RecvError::Message.into())
                }
            }
            _ => Err(RecvError::Message.into()),
        }
    }
}
