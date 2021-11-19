#![cfg(not(feature = "std"))]
#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod cdr;
#[cfg(feature = "std")]
pub mod client;
pub mod corba;
pub mod giop;
pub mod giop10;
pub mod giop11;
pub mod giop12;
pub mod giop13;
pub mod iop;
#[cfg(feature = "std")]
pub mod server;
