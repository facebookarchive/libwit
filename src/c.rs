#![allow(non_camel_case_types)]

use std::c_str::CString;
use libc::{c_char, c_uint};
use cmd;
use cmd::WitHandle;
use std::{mem, ptr, rt, io};
use std::sync::atomic::{AtomicBool, SeqCst, INIT_ATOMIC_BOOL};
use client;
use serialize::json;
use std::io::MemWriter;
use log;
use log::LogLevel::{Error, Warn, Debug};
use rustrt::task::Task;
use rustrt::local::Local;

static mut RUNTIME_INITIALIZED: AtomicBool = INIT_ATOMIC_BOOL;

fn run<T>(f: || -> T) -> T {
    if unsafe {!RUNTIME_INITIALIZED.load(SeqCst)} {
        // Force runtime initialization
        rt::init(0, ptr::null());
        unsafe {RUNTIME_INITIALIZED.swap(true, SeqCst)};
    }
    if Local::exists(None::<Task>) {
        // We're already inside a task
        f()
    } else {
        // Run the closure inside a task
        let task = box Task::new(None, None);
        let mut result: Option<T> = None;
        task.run(|| {
            result = Some(f());
        }).destroy();
        result.unwrap()
    }
}

macro_rules! c_fn(
    ($fname:ident($($arg_name:ident: $arg_type:ty),*) -> $return_type:ty $body:block) => (
        #[no_mangle]
        pub unsafe extern "C" fn $fname($($arg_name: $arg_type),*) -> $return_type {
            run(|| $body)
        }
    );
)

struct WitContext {
    handle: WitHandle
}

pub type wit_context_ptr = *const ();

fn to_c_str_opt(json_result: Result<json::Json, client::RequestError>) -> Option<*const c_char> {
    let opt_str = json_result.ok().and_then(|json| {
        wit_log!(Debug, "received response: {}", json);
        let mut s = MemWriter::new();
        json.to_writer(&mut s as &mut io::Writer).unwrap();
        String::from_utf8(s.into_inner()).ok()
    });
    opt_str.map(|string| {
        let c_string = string.to_c_str();
        // Very important, otherwise the C code doesn't have the ownership of the string
        unsafe {c_string.into_inner()}
    })
}

fn c_str_result(json_result: Result<json::Json, client::RequestError>) -> *const c_char {
    to_c_str_opt(json_result).unwrap_or(ptr::null())
}

fn from_c_string(string: *const c_char) -> Option<String> {
    let str_opt = unsafe {CString::new(string, false)};
    str_opt.as_str().map(|string| {string.to_string()})
}

fn receive_with_callback(receiver: Receiver<Result<json::Json, client::RequestError>>, cb: Option<extern "C" fn(*const c_char)>) {
    match cb {
        Some(f) => spawn(proc() {
            match to_c_str_opt(receiver.recv()) {
                Some(c_str) => {
                    wit_log!(Debug, "calling provided callback function");
                    f(c_str);
                }
                None => wit_log!(Warn, "null string pointer, doing nothing")
            };
        }),
        None => wit_log!(Warn, "no callback, discarding result")
    }
}

c_fn!(wit_init(device_opt: *const c_char, verbosity: c_uint) -> wit_context_ptr {
    let device = if device_opt.is_null() {
        None
    } else {
        let device = from_c_string(device_opt);
        if device.is_none() {
            wit_log!(Warn, "failed to read device name. Using default instead");
        }
        device
    };
    let handle = cmd::init(device, verbosity as uint);

    let boxed = box WitContext {
        handle: handle
    };
    let res: wit_context_ptr = mem::transmute(boxed);
    res
})

c_fn!(wit_close(context: wit_context_ptr) -> () {
    let context: &WitContext = mem::transmute(context);
    cmd::cleanup(&context.handle)
})

c_fn!(wit_text_query(context: wit_context_ptr, text: *const c_char, access_token: *const c_char) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => {
            match from_c_string(text) {
                Some(text) => {
                    let result = cmd::text_query(&context.handle, text, access_token);
                    return c_str_result(result)
                },
                None => wit_log!(Error, "failed to read query text")
            }
        }
        None => wit_log!(Error, "failed to read access token")
    };
    ptr::null()
})

c_fn!(wit_text_query_async(context: wit_context_ptr, text: *const c_char, access_token: *const c_char, cb: Option<extern "C" fn(*const c_char)>) -> () {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => {
            match from_c_string(text) {
                Some(text) => {
                    let receiver = cmd::text_query_async(&context.handle, text, access_token);
                    receive_with_callback(receiver, cb);
                },
                None => wit_log!(Error, "failed to read query text")
            }
        }
        None => wit_log!(Error, "failed to read access token")
    };
})

c_fn!(wit_voice_query_auto(context: wit_context_ptr, access_token: *const c_char) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => {
            let result = cmd::voice_query_auto(&context.handle, access_token);
            return c_str_result(result)
        }
        None => wit_log!(Error, "failed to read access token")
    }
    ptr::null()
})

c_fn!(wit_voice_query_auto_async(context: wit_context_ptr, access_token: *const c_char, cb: Option<extern "C" fn(*const c_char)>) -> () {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => {
            let receiver = cmd::voice_query_auto_async(&context.handle, access_token);
            receive_with_callback(receiver, cb);
        }
        None => wit_log!(Error, "failed to read access token")
    };
})

c_fn!(wit_voice_query_start(context: wit_context_ptr, access_token: *const c_char) -> () {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => cmd::voice_query_start(&context.handle, access_token),
        None => wit_log!(Error, "failed to read access token")
    };
})

c_fn!(wit_voice_query_stop(context: wit_context_ptr) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    let result = cmd::voice_query_stop(&context.handle);
    c_str_result(result)
})

c_fn!(wit_voice_query_stop_async(context: wit_context_ptr, cb: Option<extern "C" fn(*const c_char)>) -> () {
    let context: &WitContext = mem::transmute(context);
    let receiver = cmd::voice_query_stop_async(&context.handle);
    receive_with_callback(receiver, cb);
})
