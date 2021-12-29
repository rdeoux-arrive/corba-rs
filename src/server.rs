use crate::cdr::{self, size_of, to_bytes, Deserializer};
use crate::giop::{MessageHeader, Version};
use crate::giop10::{self, SystemExceptionReplyBody};
use crate::giop11;
use crate::giop12::{self, TargetAddress};
use crate::iop;
use alloc::string::String;
use alloc::vec::Vec;
use embedded_nal::{SocketAddr, TcpClientStack, TcpFullStack};
use log::{debug, error, info, warn};
use serde::{Deserialize, Serialize};

enum Error<E> {
    Cdr(cdr::Error),
    TcpStack(E),
}

/// GIOP request
pub struct Request<'a> {
    pub target: TargetAddress,
    pub operation: String,
    pub body: Deserializer<'a>,
    version: Version,
    byte_order: bool,
    request_id: u32,
}

impl Request<'_> {
    /// Byte order of the request
    #[must_use]
    pub const fn byte_order(&self) -> bool {
        self.byte_order
    }

    /// Successfully reply to a request
    pub fn reply_ok<T>(self, body: T) -> cdr::Result<Vec<u8>>
    where
        T: Serialize,
    {
        info!("send reply to request #{}", self.request_id);
        match self.version {
            Version::GIOP10 | Version::GIOP11 => {
                let header = giop10::ReplyHeader {
                    service_context: Vec::new(),
                    request_id: self.request_id,
                    reply_status: giop10::ReplyStatusType::NoException,
                };
                self.reply(header, body)
            }
            _ => {
                let header = giop12::ReplyHeader {
                    request_id: self.request_id,
                    reply_status: giop12::ReplyStatusType::NoException,
                    service_context: Vec::new(),
                };
                self.reply(header, body)
            }
        }
    }

    /// Reply with a system failure
    pub fn reply_system_exception(self, err: SystemExceptionReplyBody) -> cdr::Result<Vec<u8>> {
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

    /// Reply with a user exception
    pub fn reply_user_exception<T>(self, exception_id: &str, err: T) -> cdr::Result<Vec<u8>>
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

    fn reply<T, U>(self, header: T, body: U) -> cdr::Result<Vec<u8>>
    where
        T: Serialize,
        U: Serialize,
    {
        let message_size = size_of((&header, &body))? as u32;
        if Version::GIOP10 == self.version {
            let message_header = giop10::MessageHeader::new(giop10::MsgType::Reply)
                .with_byte_order(self.byte_order)
                .with_message_size(message_size);
            to_bytes(self.byte_order, (message_header, header, body))
        } else {
            let message_header = giop11::MessageHeader::new(self.version, giop11::MsgType::Reply)
                .with_flags(if self.byte_order { 1 } else { 0 })
                .with_message_size(message_size);
            to_bytes(self.byte_order, (message_header, header, body))
        }
    }
}

/// CORBA request handler
pub trait Handler {
    /// Handle a request
    fn handle(&mut self, request: Request) -> cdr::Result<Vec<u8>>;
}

struct Client<S, H>
where
    S: TcpClientStack,
{
    socket: S::TcpSocket,
    addr: SocketAddr,
    handler: H,
    tx: Vec<u8>,
}

impl<'a, S, H> Client<S, H>
where
    S: TcpClientStack,
    H: Handler,
{
    fn poll(&mut self, stack: &mut S) -> nb::Result<(), Error<S::Error>> {
        // send pending frame
        while !self.tx.is_empty() {
            debug!("send {} bytes", self.tx.len());
            let size = stack
                .send(&mut self.socket, &self.tx)
                .map_err(|err| err.map(Error::TcpStack))?;
            self.tx.drain(..size);
        }

        // receive a frame (arbitrary MTU)
        let mut buf = [0; 1500];
        let size = stack
            .receive(&mut self.socket, &mut buf)
            .map_err(|err| err.map(Error::TcpStack))?;
        let buf = &buf[..size];

        let (header, de) = MessageHeader::deserialize(buf).map_err(Error::Cdr)?;
        match header {
            MessageHeader::V1_0(header) => {
                debug!(
                    "received GIOP header 1.0 type {}, {} bytes",
                    header.message_type, header.message_size
                );
                let le = header.byte_order;
                if header.message_type == giop10::MsgType::Request as u8 {
                    let mut de = de.with_le(le);
                    let request =
                        giop10::RequestHeader::deserialize(&mut de).map_err(Error::Cdr)?;
                    info!(
                        "received request #{}: {}",
                        request.request_id, request.operation
                    );
                    self.tx = self
                        .handler
                        .handle(Request {
                            target: TargetAddress::ObjectKey(request.object_key),
                            operation: request.operation,
                            body: de,
                            version: Version::GIOP10,
                            byte_order: le,
                            request_id: request.request_id,
                        })
                        .map_err(Error::Cdr)?;
                } else {
                    error!("unsupported message type: {}", header.message_type);
                    let header = giop10::MessageHeader::new(giop10::MsgType::MessageError);
                    self.tx = to_bytes(header.byte_order, header).map_err(Error::Cdr)?;
                }
            }
            MessageHeader::V1_1(header)
            | MessageHeader::V1_2(header)
            | MessageHeader::V1_3(header) => {
                debug!(
                    "received GIOP header {} type {}, {} bytes",
                    header.giop_version, header.message_type, header.message_size
                );
                let byte_order = header.flags & 1 != 0;
                let mut de = de.with_le(byte_order);
                if header.message_type == giop11::MsgType::Request as u8 {
                    self.tx = if Version::GIOP11 == header.giop_version {
                        let request =
                            giop11::RequestHeader::deserialize(&mut de).map_err(Error::Cdr)?;
                        info!(
                            "received request #{}: {}",
                            request.request_id, request.operation
                        );
                        self.handler.handle(Request {
                            target: TargetAddress::ObjectKey(request.object_key),
                            operation: request.operation,
                            body: de,
                            version: header.giop_version,
                            byte_order,
                            request_id: request.request_id,
                        })
                    } else {
                        let request =
                            giop12::RequestHeader::deserialize(&mut de).map_err(Error::Cdr)?;
                        info!(
                            "received request #{}: {}",
                            request.request_id, request.operation
                        );
                        self.handler.handle(Request {
                            target: request.target,
                            operation: request.operation,
                            body: de,
                            version: header.giop_version,
                            byte_order,
                            request_id: request.request_id,
                        })
                    }
                    .map_err(Error::Cdr)?;
                } else {
                    error!("unsupported message type: {}", header.message_type);
                    let header = giop11::MessageHeader::new(
                        header.giop_version,
                        giop11::MsgType::MessageError,
                    );
                    self.tx = to_bytes(byte_order, header).map_err(Error::Cdr)?;
                }
            }
        }
        self.poll(stack)
    }
}

/// GIOP server
pub struct Server<S, F, H>
where
    S: TcpFullStack,
    F: Fn(SocketAddr) -> H,
    H: Handler,
{
    listener: S::TcpSocket,
    factory: F,
    clients: Vec<Client<S, H>>,
}

impl<S, F, H> Server<S, F, H>
where
    S: TcpFullStack,
    F: Fn(SocketAddr) -> H,
    H: Handler,
{
    /// Start listening
    pub fn listen(stack: &mut S, local_port: u16, factory: F) -> Result<Self, S::Error> {
        let mut listener = stack.socket()?;
        stack.bind(&mut listener, local_port)?;
        stack.listen(&mut listener)?;
        info!("server listening on port {}", local_port);
        Ok(Self {
            listener,
            factory,
            clients: Vec::new(),
        })
    }

    /// Process pending requests
    pub fn poll(&mut self, stack: &mut S) {
        match stack.accept(&mut self.listener) {
            Ok((socket, addr)) => {
                info!("new client: {}", addr);
                self.clients.push(Client {
                    socket,
                    addr,
                    handler: (self.factory)(addr),
                    tx: Vec::new(),
                });
            }
            Err(nb::Error::Other(..)) => panic!(),
            Err(nb::Error::WouldBlock) => {}
        }

        for mut client in core::mem::take(&mut self.clients) {
            if let Err(nb::Error::Other(..)) = client.poll(stack) {
                error!("client {} is dead", client.addr);
                if let Err(err) = stack.close(client.socket) {
                    error!("failed to close socket: {:?}", err);
                }
            } else {
                self.clients.push(client);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Handler, Request, Server};
    use crate::cdr::{self, size_of, to_bytes};
    use crate::corba::{BadOperation, BadParam, CompletionStatus};
    use crate::giop::Version;
    use crate::giop13::{
        MessageHeader, MsgType, ReplyHeader, ReplyStatusType, RequestHeader,
        SystemExceptionReplyBody, TargetAddress,
    };
    use crate::mock_nal::Stack;
    use alloc::vec::Vec;
    use embedded_nal::SocketAddr;
    use simple_logger::SimpleLogger;

    struct Test;

    impl Test {
        fn factory(_addr: SocketAddr) -> Self {
            Self
        }

        fn request(request_id: u32, object_key: &[u8], operation: &str) -> Vec<u8> {
            let request_header = RequestHeader::new(
                request_id,
                false,
                TargetAddress::ObjectKey(object_key.to_vec()),
                operation,
            );
            let message_header = MessageHeader::new(Version::GIOP13, MsgType::Request)
                .with_message_size(size_of(&request_header).unwrap() as u32);
            to_bytes(false, (message_header, request_header)).unwrap()
        }

        fn reply(request_id: u32, result: Result<(), SystemExceptionReplyBody>) -> Vec<u8> {
            let (reply_status, reply_body) = match result {
                Ok(()) => {
                    let reply_status = ReplyStatusType::NoException;
                    let reply_body = Vec::new();
                    (reply_status, reply_body)
                }
                Err(err) => {
                    let reply_status = ReplyStatusType::SystemException;
                    let reply_body = to_bytes(false, err).unwrap();
                    (reply_status, reply_body)
                }
            };
            let reply_header = ReplyHeader {
                request_id,
                reply_status,
                service_context: Vec::new(),
            };
            let message_header = MessageHeader::new(Version::GIOP13, MsgType::Reply)
                .with_message_size((size_of(&reply_header).unwrap() + reply_body.len()) as u32);
            let mut buf = to_bytes(false, (message_header, reply_header)).unwrap();
            buf.extend(reply_body);
            buf
        }
    }

    impl Handler for Test {
        fn handle<'a>(&mut self, request: Request<'a>) -> cdr::Result<Vec<u8>> {
            match &request.target {
                TargetAddress::ObjectKey(object_key) if object_key == b"TEST" => {
                    match request.operation.as_str() {
                        "test" => request.reply_ok(()),
                        _operation => request.reply_system_exception(
                            BadOperation {
                                minor: 2,
                                completed: CompletionStatus::CompletedNo,
                            }
                            .into(),
                        ),
                    }
                }
                _target => request.reply_system_exception(
                    BadParam {
                        minor: 4,
                        completed: CompletionStatus::CompletedNo,
                    }
                    .into(),
                ),
            }
        }
    }

    #[test]
    fn server() {
        SimpleLogger::new().init().ok();

        // start the server
        let mut stack = Stack::new();
        let mut server = Server::listen(
            stack.socket(Ok(0)).bind(0, 639, Ok(())).listen(0, Ok(())),
            639,
            Test::factory,
        )
        .unwrap();
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)));

        // a client arrives
        server.poll(
            stack
                .accept(0, Ok((1, SocketAddr::from(([127, 0, 0, 1], 1234)))))
                .receive(1, 1500, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).receive(
            1,
            1500,
            Err(nb::Error::WouldBlock),
        ));

        // the client send a request
        let request = Test::request(1, b"TEST", "test");
        let reply = Test::reply(1, Ok(()));
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .receive(1, 1500, Ok(&request))
                .send(1, &reply, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).send(
            1,
            &reply,
            Err(nb::Error::WouldBlock),
        ));

        // the server send the response
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .send(1, &reply, Ok(reply.len()))
                .receive(1, 1500, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).receive(
            1,
            1500,
            Err(nb::Error::WouldBlock),
        ));

        // the client send a request
        let request = Test::request(2, b"TEST", "bad_operation");
        let reply = Test::reply(
            2,
            Err(BadOperation {
                minor: 2,
                completed: CompletionStatus::CompletedNo,
            }
            .into()),
        );
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .receive(1, 1500, Ok(&request))
                .send(1, &reply, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).send(
            1,
            &reply,
            Err(nb::Error::WouldBlock),
        ));

        // the server send the response
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .send(1, &reply, Ok(reply.len()))
                .receive(1, 1500, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).receive(
            1,
            1500,
            Err(nb::Error::WouldBlock),
        ));

        // the client send a request
        let request = Test::request(3, b"BADTARGET", "test");
        let reply = Test::reply(
            3,
            Err(BadParam {
                minor: 4,
                completed: CompletionStatus::CompletedNo,
            }
            .into()),
        );
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .receive(1, 1500, Ok(&request))
                .send(1, &reply, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).send(
            1,
            &reply,
            Err(nb::Error::WouldBlock),
        ));

        // the server send the response
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .send(1, &reply, Ok(reply.len()))
                .receive(1, 1500, Err(nb::Error::WouldBlock)),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)).receive(
            1,
            1500,
            Err(nb::Error::WouldBlock),
        ));

        // the client disconnects
        server.poll(
            stack
                .accept(0, Err(nb::Error::WouldBlock))
                .receive(1, 1500, Ok(&[]))
                .close(1, Ok(())),
        );
        server.poll(stack.accept(0, Err(nb::Error::WouldBlock)));
    }
}
