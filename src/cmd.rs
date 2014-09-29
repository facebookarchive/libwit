use serialize::json::Json;
use client;

pub use client::{RequestError, ChannelClosedError};

pub type WitHandle = Sender<client::WitCommand>;

pub fn init(device_opt: Option<String>) -> WitHandle {
    let handle = client::init(client::Options{input_device: device_opt.clone()});
    println!("[wit] initialized with device: {}", device_opt.unwrap_or("default".to_string()));
    handle
}

pub fn cleanup(handle: &WitHandle) {
    client::cleanup(handle)
}

pub fn text_query(handle: &WitHandle, text: String, access_token: String) -> Result<Json, RequestError> {
    text_query_async(handle, text, access_token).recv_opt().unwrap_or(Err(ChannelClosedError))
}

pub fn text_query_async(handle: &WitHandle, text: String, access_token: String) -> Receiver<Result<Json, RequestError>> {
    client::interpret_string(handle, access_token, text)
}

pub fn voice_query_auto(handle: &WitHandle, access_token: String) -> Result<Json, RequestError> {
    voice_query_auto_async(handle, access_token).recv_opt().unwrap_or(Err(ChannelClosedError))
}

pub fn voice_query_auto_async(handle: &WitHandle, access_token: String) -> Receiver<Result<Json, RequestError>> {
    client::start_autoend_recording(handle, access_token)
}

pub fn voice_query_start(handle: &WitHandle, access_token: String) {
    client::start_recording(handle, access_token);
}

pub fn voice_query_stop(handle: &WitHandle) -> Result<Json, RequestError> {
    voice_query_stop_async(handle).recv_opt().unwrap_or(Err(ChannelClosedError))
}

pub fn voice_query_stop_async(handle: &WitHandle) -> Receiver<Result<Json, RequestError>> {
    client::stop_recording(handle)
}
