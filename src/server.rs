use crate::cdr::{self, Deserializer, Error};
use crate::giop::{MessageHeader, Version};
use crate::giop10::{self, SystemExceptionReplyBody};
use crate::giop11;
use crate::giop12::{self, TargetAddress};
use crate::iop;
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use std::io::{BufWriter, Cursor, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::thread::spawn;

pub struct Request<'a, S>
where
    S: Read + Write,
{
    pub target: TargetAddress,
    pub operation: String,
    pub body: Deserializer<&'a [u8]>,
    stream: &'a mut S,
    version: Version,
    byte_order: bool,
    request_id: u32,
}

pub trait Handler<S>
where
    S: Read + Write,
{
    fn handle(&mut self, request: Request<S>) -> cdr::Result<()>;
}

impl<'a, S> Request<'a, S>
where
    S: Read + Write,
{
    #[must_use]
    pub fn byte_order(&self) -> bool {
        self.byte_order
    }

    pub fn reply_ok<T>(self, body: T) -> cdr::Result<()>
    where
        T: Serialize,
    {
        info!("send reply to request #{}", self.request_id);
        match self.version {
            Version::GIOP10 | Version::GIOP11 => {
                let header = giop10::ReplyHeader {
                    service_context: Vec::<iop::ServiceContext>::default(),
                    request_id: self.request_id,
                    reply_status: giop10::ReplyStatusType::NoException,
                };
                self.reply(header, body)
            }
            _ => {
                let header = giop12::ReplyHeader {
                    request_id: self.request_id,
                    reply_status: giop12::ReplyStatusType::NoException,
                    service_context: Vec::<iop::ServiceContext>::default(),
                };
                self.reply(header, body)
            }
        }
    }

    pub fn reply_system_exception(self, err: SystemExceptionReplyBody) -> cdr::Result<()> {
        warn!("send system exception to request #{}", self.request_id);
        match self.version {
            Version::GIOP10 | Version::GIOP11 => {
                let header = giop10::ReplyHeader {
                    service_context: Vec::<iop::ServiceContext>::default(),
                    request_id: self.request_id,
                    reply_status: giop10::ReplyStatusType::SystemException,
                };
                self.reply(header, err)
            }
            _ => {
                let header = giop12::ReplyHeader {
                    request_id: self.request_id,
                    reply_status: giop12::ReplyStatusType::SystemException,
                    service_context: Vec::<iop::ServiceContext>::default(),
                };
                self.reply(header, err)
            }
        }
    }

    pub fn reply_user_exception<T>(self, exception_id: &str, err: T) -> cdr::Result<()>
    where
        T: Serialize,
    {
        info!("send user exception to request #{}", self.request_id);
        match self.version {
            Version::GIOP10 | Version::GIOP11 => {
                let header = giop10::ReplyHeader {
                    service_context: Vec::<iop::ServiceContext>::default(),
                    request_id: self.request_id,
                    reply_status: giop10::ReplyStatusType::UserException,
                };
                self.reply(header, (exception_id, err))
            }
            _ => {
                let header = giop12::ReplyHeader {
                    request_id: self.request_id,
                    reply_status: giop12::ReplyStatusType::UserException,
                    service_context: Vec::<iop::ServiceContext>::default(),
                };
                self.reply(header, (exception_id, err))
            }
        }
    }

    pub fn reply<T, U>(mut self, header: T, body: U) -> cdr::Result<()>
    where
        T: Serialize,
        U: Serialize,
    {
        let cursor = Cursor::new(Vec::new());
        let mut ser = match self.version {
            Version::GIOP10 => giop10::MessageHeader::new(giop10::MsgType::Reply)
                .with_byte_order(self.byte_order)
                .serialize(cursor),
            _ => giop11::MessageHeader::new(self.version.clone(), giop11::MsgType::Reply)
                .with_flags(if self.byte_order { 1 } else { 0 })
                .serialize(cursor),
        }?;
        let header_size = ser.writer().position();
        header.serialize(&mut ser)?;
        body.serialize(&mut ser)?;
        let message_size = (ser.writer().position() - header_size) as u32;
        debug!("send {} bytes", header_size as u32 + message_size);
        let buf = BufWriter::with_capacity(
            header_size as usize + message_size as usize,
            &mut self.stream,
        );
        let mut ser = match self.version {
            Version::GIOP10 => giop10::MessageHeader::new(giop10::MsgType::Reply)
                .with_byte_order(self.byte_order)
                .with_message_size(message_size)
                .serialize(buf),
            _ => giop11::MessageHeader::new(self.version, giop11::MsgType::Reply)
                .with_flags(if self.byte_order { 1 } else { 0 })
                .with_message_size(message_size)
                .serialize(buf),
        }?;
        header.serialize(&mut ser)?;
        body.serialize(&mut ser)?;
        ser.writer_mut().flush().map_err(Error::Io)
    }
}

pub fn serve<A, F, H>(addr: A, factory: F) -> cdr::Result<()>
where
    A: ToSocketAddrs,
    F: Fn(SocketAddr) -> H,
    H: 'static + Send + Handler<TcpStream>,
{
    let listener = TcpListener::bind(addr)?;
    if let Ok(addr) = listener.local_addr() {
        info!("server listening on {}", addr);
    }
    loop {
        let (stream, addr) = listener.accept()?;
        info!("new client: {}", addr);
        let handler = factory(addr);
        spawn(move || {
            if let Err(err) = serve_client(stream, handler) {
                error!("client {} dead: {}", addr, err)
            }
        });
    }
}

fn serve_client<S, T>(stream: S, mut handler: T) -> cdr::Result<()>
where
    S: Read + Write,
    T: Handler<S>,
{
    let mut stream = stream;

    loop {
        let (header, de) = MessageHeader::deserialize(&mut stream)?;
        match header {
            MessageHeader::V1_0(header) => {
                debug!(
                    "received GIOP header 1.0 type {}, {} bytes",
                    header.message_type, header.message_size
                );
                let byte_order = header.byte_order;
                let mut buf = vec![0; header.message_size as usize];
                let mut stream = de.into_inner();
                match stream.read_exact(buf.as_mut()) {
                    Ok(()) => {
                        if header.message_type == giop10::MsgType::Request as u8 {
                            let mut de =
                                Deserializer::from_reader(buf.as_slice()).with_le(byte_order);
                            let request = giop10::RequestHeader::deserialize(&mut de)?;
                            info!(
                                "received request #{}: {}",
                                request.request_id, request.operation
                            );
                            handler.handle(Request {
                                target: TargetAddress::ObjectKey(request.object_key),
                                operation: request.operation,
                                body: de,
                                stream,
                                version: Version::GIOP10,
                                byte_order,
                                request_id: request.request_id,
                            })?;
                        } else {
                            error!("unsupported message type: {}", header.message_type);
                            let header = giop10::MessageHeader::new(giop10::MsgType::MessageError);
                            header.serialize(&mut stream)?;
                        }
                    }
                    Err(err) => error!("failed to read message: {}", err),
                }
            }
            MessageHeader::V1_1(header)
            | MessageHeader::V1_2(header)
            | MessageHeader::V1_3(header) => {
                debug!(
                    "received GIOP header 1.{} type {}, {} bytes",
                    header.giop_version.minor(),
                    header.message_type,
                    header.message_size
                );
                let byte_order = header.flags & 1 != 0;
                let mut buf = vec![0; header.message_size as usize];
                let mut stream = de.into_inner();
                match stream.read_exact(buf.as_mut()) {
                    Ok(()) => {
                        let mut de = Deserializer::from_reader(buf.as_slice()).with_le(byte_order);
                        if header.message_type == giop11::MsgType::Request as u8 {
                            if let Version::GIOP11 = header.giop_version {
                                let request = giop11::RequestHeader::deserialize(&mut de)?;
                                info!(
                                    "received request #{}: {}",
                                    request.request_id, request.operation
                                );
                                handler.handle(Request {
                                    target: TargetAddress::ObjectKey(request.object_key),
                                    operation: request.operation,
                                    body: de,
                                    stream,
                                    version: header.giop_version,
                                    byte_order,
                                    request_id: request.request_id,
                                })
                            } else {
                                let request = giop12::RequestHeader::deserialize(&mut de)?;
                                info!(
                                    "received request #{}: {}",
                                    request.request_id, request.operation
                                );
                                handler.handle(Request {
                                    target: request.target,
                                    operation: request.operation,
                                    body: de,
                                    stream,
                                    version: header.giop_version,
                                    byte_order,
                                    request_id: request.request_id,
                                })
                            }?;
                        } else {
                            error!("unsupported message type: {}", header.message_type);
                            let header = giop11::MessageHeader::new(
                                header.giop_version,
                                giop11::MsgType::MessageError,
                            );
                            header.serialize(&mut stream)?;
                        }
                    }
                    Err(err) => error!("failed to read message: {}", err),
                }
            }
        };
    }
}
