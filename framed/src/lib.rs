extern crate bytes;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate log;

pub mod codec;

pub mod framed;
mod framed_read;
mod framed_write;
