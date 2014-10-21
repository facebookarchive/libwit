#![macro_escape]

use std::sync::atomic::{AtomicUint, Relaxed, INIT_ATOMIC_UINT};

// Not sure there is a much better way. We want to be able to use
// logging from anywhere, without having to carry the verbosity level
// everywhere in the code
static mut VERBOSITY: AtomicUint = INIT_ATOMIC_UINT;

#[deriving(PartialEq, PartialOrd)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug
}

impl LogLevel {
    pub fn should_show(level: LogLevel) -> bool {
        match unsafe { VERBOSITY.load(Relaxed) } {
            0 => false,
            1 => level <= Error,
            2 => level <= Warn,
            3 => level <= Info,
            _ => level <= Debug
        }
    }
}

pub fn set_verbosity(verbosity: uint) {
    unsafe { VERBOSITY.store(verbosity, Relaxed) }
}

macro_rules! wit_log(
    ($level: expr, $($arg:expr),+) => ({
        if log::LogLevel::should_show($level) {
            print!("[wit] ");
            println!($($arg),+);
        }
    });
)
