use core::net::SocketAddr;

use alloc::collections::VecDeque;
use embedded_nal::{TcpClientStack, TcpError, TcpErrorKind, TcpFullStack};

pub type Socket = usize;

#[derive(Debug)]
pub struct Error;

impl TcpError for Error {
    fn kind(&self) -> TcpErrorKind {
        TcpErrorKind::Other
    }
}

#[derive(Debug)]
pub enum Expect<'a> {
    Socket {
        result: Result<Socket, Error>,
    },
    Send {
        socket: Socket,
        buf: &'a [u8],
        result: nb::Result<usize, Error>,
    },
    Receive {
        socket: Socket,
        buf: usize,
        result: nb::Result<&'a [u8], Error>,
    },
    Close {
        socket: Socket,
        result: Result<(), Error>,
    },
    Bind {
        socket: Socket,
        port: u16,
        result: Result<(), Error>,
    },
    Listen {
        socket: Socket,
        result: Result<(), Error>,
    },
    Accept {
        socket: Socket,
        result: nb::Result<(usize, SocketAddr), Error>,
    },
}

pub struct Stack<'a> {
    expects: VecDeque<Expect<'a>>,
}

impl<'a> Stack<'a> {
    pub fn new() -> Self {
        Self {
            expects: VecDeque::new(),
        }
    }

    pub fn socket(&mut self, result: Result<Socket, Error>) -> &mut Self {
        self.expects.push_back(Expect::Socket { result });
        self
    }

    pub fn send(
        &mut self,
        socket: Socket,
        buf: &'a [u8],
        result: nb::Result<usize, Error>,
    ) -> &mut Self {
        self.expects.push_back(Expect::Send {
            socket,
            buf,
            result,
        });
        self
    }

    pub fn receive(
        &mut self,
        socket: Socket,
        buf: usize,
        result: nb::Result<&'a [u8], Error>,
    ) -> &mut Self {
        self.expects.push_back(Expect::Receive {
            socket,
            buf,
            result,
        });
        self
    }

    pub fn close(&mut self, socket: Socket, result: Result<(), Error>) -> &mut Self {
        self.expects.push_back(Expect::Close { socket, result });
        self
    }

    pub fn bind(&mut self, socket: Socket, port: u16, result: Result<(), Error>) -> &mut Self {
        self.expects.push_back(Expect::Bind {
            socket,
            port,
            result,
        });
        self
    }

    pub fn listen(&mut self, socket: Socket, result: Result<(), Error>) -> &mut Self {
        self.expects.push_back(Expect::Listen { socket, result });
        self
    }

    pub fn accept(
        &mut self,
        socket: Socket,
        result: nb::Result<(usize, SocketAddr), Error>,
    ) -> &mut Self {
        self.expects.push_back(Expect::Accept { socket, result });
        self
    }

    fn pop(&mut self, func: &str) -> Expect {
        match self.expects.pop_front() {
            None => panic!("expected nothing but got a call to {}()", func),
            Some(expect) => expect,
        }
    }
}

impl TcpClientStack for Stack<'_> {
    type TcpSocket = Socket;
    type Error = Error;

    fn socket(&mut self) -> Result<Self::TcpSocket, Self::Error> {
        match self.pop("socket") {
            Expect::Socket { result } => result,
            expect => panic!("expected {:?} but got a call to socket()", expect),
        }
    }

    fn connect(
        &mut self,
        _socket: &mut Self::TcpSocket,
        _addr: SocketAddr,
    ) -> nb::Result<(), Self::Error> {
        let expect = self.pop("connect");
        panic!("expected {:?} but got a call to connect()", expect);
    }

    fn send(&mut self, socket: &mut Self::TcpSocket, buf: &[u8]) -> nb::Result<usize, Self::Error> {
        match self.pop("send") {
            Expect::Send {
                socket: expect_socket,
                buf: expect_buf,
                result,
            } => {
                assert_eq!(*socket, expect_socket);
                assert_eq!(buf, expect_buf);
                result
            }
            expect => panic!("expected {:?} but got a call to send()", expect),
        }
    }

    fn receive(
        &mut self,
        socket: &mut Self::TcpSocket,
        buf: &mut [u8],
    ) -> nb::Result<usize, Self::Error> {
        match self.pop("receive") {
            Expect::Receive {
                socket: expect_socket,
                buf: expected_buf,
                result,
            } => {
                assert_eq!(*socket, expect_socket);
                assert_eq!(buf.len(), expected_buf);
                result.map(|ok| {
                    buf[..ok.len()].copy_from_slice(ok);
                    ok.len()
                })
            }
            expect => panic!("expected {:?} but got a call to receive()", expect),
        }
    }

    fn close(&mut self, socket: Self::TcpSocket) -> Result<(), Self::Error> {
        match self.pop("close") {
            Expect::Close {
                socket: expect_socket,
                result,
            } => {
                assert_eq!(socket, expect_socket);
                result
            }
            expect => panic!("expected {:?} but got a call to close()", expect),
        }
    }
}

impl TcpFullStack for Stack<'_> {
    fn bind(&mut self, socket: &mut Self::TcpSocket, local_port: u16) -> Result<(), Self::Error> {
        match self.pop("bind") {
            Expect::Bind {
                socket: expect_socket,
                port,
                result,
            } => {
                assert_eq!(*socket, expect_socket);
                assert_eq!(local_port, port);
                result
            }
            expect => panic!("expected {:?} but got a call to bind()", expect),
        }
    }

    fn listen(&mut self, socket: &mut Self::TcpSocket) -> Result<(), Self::Error> {
        match self.pop("listen") {
            Expect::Listen {
                socket: expect_socket,
                result,
            } => {
                assert_eq!(*socket, expect_socket);
                result
            }
            expect => panic!("expected {:?} but got a call to listen()", expect),
        }
    }

    fn accept(
        &mut self,
        socket: &mut Self::TcpSocket,
    ) -> nb::Result<(Self::TcpSocket, SocketAddr), Self::Error> {
        match self.pop("accept") {
            Expect::Accept {
                socket: expect_socket,
                result,
            } => {
                assert_eq!(*socket, expect_socket);
                result
            }
            expect => panic!("expected {:?} but got a call to accept()", expect),
        }
    }
}
