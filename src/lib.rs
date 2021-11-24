#![cfg(not(feature = "std"))]
#![no_std]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod cdr;
pub mod client;
pub mod corba;
pub mod giop;
pub mod giop10;
pub mod giop11;
pub mod giop12;
pub mod giop13;
pub mod iop;
pub mod server;

#[cfg(test)]
mod mock_nal;