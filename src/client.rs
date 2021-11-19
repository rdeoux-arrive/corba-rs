use crate::cdr;
use crate::giop::MessageHeader;
use crate::giop::Version;
use crate::giop12::{
    self, MsgType, ReplyHeader, ReplyStatusType, RequestHeader, SystemExceptionReplyBody,
    TargetAddress,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Cursor, Read, Write};

#[derive(Debug)]
pub enum Error<T> {
    Cdr(cdr::Error),
    Message,
    System(SystemExceptionReplyBody),
    User(T),
}

impl<T> From<cdr::Error> for Error<T> {
    fn from(err: cdr::Error) -> Self {
        Self::Cdr(err)
    }
}

impl<T> From<SystemExceptionReplyBody> for Error<T> {
    fn from(err: SystemExceptionReplyBody) -> Self {
        Self::System(err)
    }
}

pub struct Client<T>
where
    T: Read + Write,
{
    request_id: u32,
    byte_order: bool,
    stream: T,
}

impl<T> Client<T>
where
    T: Read + Write,
{
    pub fn with_byte_order(self, byte_order: bool) -> Self {
        Self {
            request_id: self.request_id,
            byte_order,
            stream: self.stream,
        }
    }

    pub fn send<S>(
        &mut self,
        oneway: bool,
        target: TargetAddress,
        operation: &str,
        body: S,
    ) -> cdr::Result<()>
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
        let mut cursor = Cursor::new(Vec::new());
        to_writer(&mut cursor, self.byte_order, &req.message)?;
        let header_size = cursor.position();
        to_writer(&mut cursor, self.byte_order, &req.request)?;
        to_writer(&mut cursor, self.byte_order, &req.body)?;
        req.message.message_size = (cursor.position() - header_size) as u32;
        debug!(
            "send {} bytes",
            header_size as u32 + req.message.message_size
        );
        let mut stream = BufWriter::with_capacity(cursor.position() as usize, &mut self.stream);
        to_writer(&mut stream, self.byte_order, &req)?;
        stream.flush()?;
        self.request_id += 1;
        Ok(())
    }

    pub fn recv<'de, O, E>(&mut self) -> Result<O, Error<E>>
    where
        O: Deserialize<'de>,
        E: Deserialize<'de>,
    {
        let (header, mut de) = MessageHeader::deserialize(&mut self.stream)?;
        match header {
            MessageHeader::V1_2(header) => {
                if header.message_type == MsgType::Reply as u8 {
                    let reply = ReplyHeader::deserialize(&mut de)?;
                    match reply.reply_status {
                        ReplyStatusType::NoException => {
                            let body = <_>::deserialize(&mut de)?;
                            Ok(body)
                        }
                        ReplyStatusType::SystemException => {
                            let err = SystemExceptionReplyBody::deserialize(&mut de)?;
                            Err(Error::System(err))
                        }
                        ReplyStatusType::UserException => {
                            let err = E::deserialize(&mut de)?;
                            Err(Error::User(err))
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    Err(Error::Message)
                }
            }
            _ => Err(Error::Message),
        }
    }
}

impl<T> From<T> for Client<T>
where
    T: Read + Write,
{
    fn from(stream: T) -> Self {
        Self {
            request_id: 0,
            byte_order: false,
            stream,
        }
    }
}
