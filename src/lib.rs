#![feature(macro_rules)]

extern crate curl;
extern crate serialize;
extern crate libc;
extern crate url;

#[cfg(c_target)]
extern crate native;

mod log;
mod client;
mod mic;

#[cfg(c_target)]
mod cmd;

#[cfg(c_target)]
pub mod c;

#[cfg(not(c_target))]
pub mod cmd;
