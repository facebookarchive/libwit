#![feature(macro_rules)]

extern crate hyper;
extern crate mime;
extern crate serialize;
extern crate libc;
extern crate url;
extern crate "sox-sys" as ffi;
extern crate "fake-sys" as fakeffi;

mod log;
mod client;
mod mic;

pub mod cmd;
pub mod c;
