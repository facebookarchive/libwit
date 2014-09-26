#![allow(non_camel_case_types)]

use std::c_str::CString;
use libc::c_char;
use cmd;
use cmd::WitHandle;
use native;
use std;
use std::{mem, ptr, rt};
use std::sync::atomic::{AtomicBool, SeqCst, INIT_ATOMIC_BOOL};

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


fn c_str_result(result: Option<String>) -> *const c_char {
    match result {
        Some(r) => r.to_c_str().as_ptr(),
        None => ptr::null()
    }
}

c_fn!(wit_init(device_opt: *const c_char) -> wit_context_ptr {
    let device = if device_opt.is_null() {
        None
    } else {
        let device_str = CString::new(device_opt, false);
        match device_str.as_str() {
            Some(s) => Some(s.to_string()),
            None => {
                println!("[wit] warning: failed to read device name. Using default instead");
                None
            }
        }
    };
    let handle = cmd::init(device);

    let boxed = box WitContext {
        handle: handle
    };
    let res: wit_context_ptr = mem::transmute(boxed);
    res
})

c_fn!(wit_start_recording(context: wit_context_ptr, access_token: *const c_char) -> () {
    let context: &WitContext = mem::transmute(context);
    let access_token_opt = CString::new(access_token, false);
    match access_token_opt.as_str() {
        Some(access_token_str) => {
            let access_token = access_token_str.to_string();
            cmd::start_recording(&context.handle, access_token)
        }
        None => {
            println!("[wit] error: failed to read access token");
        }
    }
})

c_fn!(wit_stop_recording(context: wit_context_ptr) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    let result = cmd::stop_recording(&context.handle);
    c_str_result(result)
})

c_fn!(wit_text_query(context: wit_context_ptr, text: *const c_char, access_token: *const c_char) -> *const c_char {
    let context: &WitContext = mem::transmute(context);
    let access_token_opt = CString::new(access_token, false);
    let result = match access_token_opt.as_str() {
        Some(access_token_str) => {
            let access_token = access_token_str.to_string();
            let text_opt = CString::new(text, false);
            match text_opt.as_str() {
                Some(text_str) => {
                    let text = text_str.to_string();
                    cmd::text_query(&context.handle, text, access_token)
                },
                None => {
                    println!("[wit] error: failed to read query text");
                    None
                }
            }
        }
        None => {
            println!("[wit] error: failed to read access token");
            None
        }
    };
    c_str_result(result)
})

