#![feature(macro_rules)]

extern crate hyper;
extern crate mime;
extern crate serialize;
extern crate libc;
extern crate url;

mod log;
mod client;
mod mic;

#[cfg(c_target)]
mod cmd;

#[cfg(c_target)]
pub mod c;

#[cfg(not(c_target))]
pub mod cmd;
