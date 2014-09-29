#![allow(non_camel_case_types)]

use std::c_str::CString;
use libc::c_char;
use cmd;
use cmd::WitHandle;
use native;
use std;
use std::{mem, ptr, rt, io};
use std::sync::atomic::{AtomicBool, SeqCst, INIT_ATOMIC_BOOL};
use client;
use serialize::json;
use std::io::MemWriter;

static mut RUNTIME_INITIALIZED: AtomicBool = INIT_ATOMIC_BOOL;

fn run<T>(f: || -> T) -> T {
    if unsafe {!RUNTIME_INITIALIZED.load(SeqCst)} {
        // Force runtime initialization
        rt::init(0, ptr::null());
        unsafe {RUNTIME_INITIALIZED.swap(true, SeqCst)};
    }
    let task = native::task::new((0, std::uint::MAX));
    let mut result: Option<T> = None;
    task.run(|| {
        result = Some(f());
    }).destroy();
    result.unwrap()
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

fn c_str_result(json_result: Result<json::Json, client::RequestError>) -> *const c_char {
    let opt_str = json_result.ok().and_then(|json| {
        println!("[wit] received response: {}", json);
        let mut s = MemWriter::new();
        json.to_writer(&mut s as &mut io::Writer).unwrap();
        String::from_utf8(s.unwrap()).ok()
    });
    match opt_str {
        Some(s) => s.to_c_str().as_ptr(),
        None => ptr::null()
    }
}

fn receive_c_str_result(receiver: Receiver<Result<json::Json, client::RequestError>>) -> *const c_char {
    c_str_result(receiver.recv())
}

fn from_c_string(string: *const c_char) -> Option<String> {
    let str_opt = unsafe {CString::new(string, false)};
    str_opt.as_str().map(|string| {string.to_string()})
}

fn receive_with_callback(receiver: Receiver<Result<json::Json, client::RequestError>>, cb: Option<extern "C" fn(*const c_char)>) {
    match cb {
        Some(f) => spawn(proc() {
            let result = receive_c_str_result(receiver);
            f(result)
        }),
        None => println!("[wit] warning: no callback, discarding result")
    }
}

c_fn!(wit_init(device_opt: *const c_char) -> wit_context_ptr {
    let device = if device_opt.is_null() {
        None
    } else {
        let device = from_c_string(device_opt);
        if device.is_none() {
            println!("[wit] warning: failed to read device name. Using default instead");
        }
        device
    };
    let handle = cmd::init(device);

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
                None => println!("[wit] error: failed to read query text")
            }
        }
        None => println!("[wit] error: failed to read access token")
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
                None => println!("[wit] error: failed to read query text")
            }
        }
        None => println!("[wit] error: failed to read access token")
    };
})

c_fn!(wit_voice_query_auto(context: wit_context_ptr, access_token: *const c_char) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => {
            let result = cmd::voice_query_auto(&context.handle, access_token);
            return c_str_result(result)
        }
        None => println!("[wit] error: failed to read access token")
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
        None => println!("[wit] error: failed to read access token")
    };
})

c_fn!(wit_voice_query_start(context: wit_context_ptr, access_token: *const c_char) -> () {
    let context: &WitContext = mem::transmute(context);
    match from_c_string(access_token) {
        Some(access_token) => cmd::voice_query_start(&context.handle, access_token),
        None => println!("[wit] error: failed to read access token")
    };
})

c_fn!(voice_query_stop(context: wit_context_ptr) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    let result = cmd::voice_query_stop(&context.handle);
    c_str_result(result)
})

c_fn!(voice_query_stop_async(context: wit_context_ptr, cb: Option<extern "C" fn(*const c_char)>) -> () {
    let context: &WitContext = mem::transmute(context);
    let receiver = cmd::voice_query_stop_async(&context.handle);
    receive_with_callback(receiver, cb);
})
