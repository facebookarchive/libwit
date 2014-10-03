#![macro_escape]

// Not sure there is a much better way. We want to be able to use
// logging from anywhere, without having to carry the verbosity level
// everywhere in the code
static mut VERBOSITY: uint = 0;

#[deriving(PartialEq, PartialOrd)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug
}

impl LogLevel {
    pub fn should_show(level: LogLevel) -> bool {
        match unsafe {VERBOSITY} {
            0 => false,
            1 => level <= Error,
            2 => level <= Warn,
            3 => level <= Info,
            _ => level <= Debug
        }
    }
}

pub fn set_verbosity(verbosity: uint) {
    unsafe {VERBOSITY = verbosity};
}

macro_rules! wit_log(
    ($level: expr, $($arg:expr),+) => ({
        if log::LogLevel::should_show($level) {
            print!("[wit] ");
            println!($($arg),+);
        }
    });
)
