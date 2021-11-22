use crate::cdr::{self, size_of, to_bytes};
use crate::giop::MessageHeader;
use crate::giop::Version;
use crate::giop12::{
    self, MsgType, ReplyHeader, ReplyStatusType, RequestHeader, SystemExceptionReplyBody,
    TargetAddress,
};
use core::convert::Infallible;
use embedded_nal::TcpClientStack;
use log::debug;
use serde::{Deserialize, Serialize};

/// Client error
#[derive(Debug)]
pub enum Error<T, S> {
    /// Serialization/deserialization error
    Cdr(cdr::Error),
    /// Unexpected message type or version
    Message,
    /// Server-side system exception
    System(SystemExceptionReplyBody),
    /// Server-side user exception
    User(T),
    /// Network related error
    Stack(S),
}

impl<T, S> From<cdr::Error> for Error<T, S> {
    fn from(err: cdr::Error) -> Self {
        Self::Cdr(err)
    }
}

impl<T, S> From<SystemExceptionReplyBody> for Error<T, S> {
    fn from(err: SystemExceptionReplyBody) -> Self {
        Self::System(err)
    }
}

/// CORBA client.
pub struct Client<T>
where
    T: TcpClientStack,
{
    request_id: u32,
    byte_order: bool,
    stream: T::TcpSocket,
}

impl<T> Client<T>
where
    T: TcpClientStack,
{
    /// Create a new CORBA client.
    pub fn new(stream: T::TcpSocket) -> Self {
        Self {
            request_id: 0,
            byte_order: false,
            stream,
        }
    }

    /// Change the byte order of the client.
    pub fn with_byte_order(self, byte_order: bool) -> Self {
        Self { byte_order, ..self }
    }

    /// Send a request.
    pub fn send<S>(
        &mut self,
        stack: &mut T,
        oneway: bool,
        target: TargetAddress,
        operation: &str,
        body: S,
    ) -> nb::Result<(), Error<Infallible, T::Error>>
    where
        S: Serialize,
    {
        #[derive(Serialize)]
        struct Req<T: Serialize> {
            message: giop12::MessageHeader,
            request: RequestHeader,
            body: T,
        }

        debug!("send request #{}", self.request_id);
        let mut req = Req {
            message: giop12::MessageHeader::new(Version::GIOP12, MsgType::Request)
                .with_flags(if self.byte_order { 1 } else { 0 }),
            request: RequestHeader::new(self.request_id, oneway, target, operation),
            body,
        };
        req.message.message_size = size_of((&req.request, &req.body)).map_err(Error::Cdr)? as u32;
        let bytes = to_bytes(self.byte_order, &req).map_err(Error::Cdr)?;
        debug!("send {} bytes", bytes.len());
        stack
            .send(&mut self.stream, &bytes)
            .map_err(|err| err.map(Error::Stack))?;
        self.request_id += 1;
        Ok(())
    }

    /// Try to receive a reply.
    pub fn recv<'de, O, E>(&mut self, stack: &mut T) -> nb::Result<O, Error<E, T::Error>>
    where
        O: Deserialize<'de>,
        E: Deserialize<'de>,
    {
        let mut buffer = [0; 1500];
        let size = stack
            .receive(&mut self.stream, &mut buffer)
            .map_err(|err| err.map(Error::Stack))?;
        let (header, mut de) = MessageHeader::deserialize(&buffer[..size]).map_err(Error::Cdr)?;
        match header {
            MessageHeader::V1_2(header) => {
                if header.message_type == MsgType::Reply as u8 {
                    let reply = ReplyHeader::deserialize(&mut de).map_err(Error::Cdr)?;
                    match reply.reply_status {
                        ReplyStatusType::NoException => {
                            let body = <_>::deserialize(&mut de).map_err(Error::Cdr)?;
                            Ok(body)
                        }
                        ReplyStatusType::SystemException => {
                            let err = SystemExceptionReplyBody::deserialize(&mut de)
                                .map_err(Error::Cdr)?;
                            Err(Error::System(err).into())
                        }
                        ReplyStatusType::UserException => {
                            let err = E::deserialize(&mut de).map_err(Error::Cdr)?;
                            Err(Error::User(err).into())
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    Err(Error::Message.into())
                }
            }
            _ => Err(Error::Message.into()),
        }
    }
}
