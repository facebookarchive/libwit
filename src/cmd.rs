use serialize::json::Json;
use client;
use log;
use log::LogLevel::Info;

pub use client::RequestError;
pub use client::RequestError::ChannelClosedError;
pub use client::WitHandle;

/**
 * Initialize the resources for audio recording and Wit API requests.
 * This function returns a handle used by all the other functions
 * in the library.
 * The resources can be released using the cleanup method.
 */
pub fn init(device_opt: Option<String>, verbosity: uint) -> WitHandle {
    let handle = client::init(client::Options{
        input_device: device_opt.clone(),
        verbosity: verbosity
    });
    wit_log!(Info, "initialized with device: {}", device_opt.unwrap_or("default".to_string()));
    handle
}

/**
 * Release the resources allocated by the init method.
 * The context object should not be used for any other purpose after this function
 * has been called.
 */
pub fn cleanup(handle: &WitHandle) {
    client::cleanup(handle)
}

/**
 * Send a text query to the Wit instance identified by the access_token.
 * This function is blocking, and returns the response from the Wit instance.
 */
pub fn text_query(handle: &WitHandle, text: String, access_token: String) -> Result<Json, RequestError> {
    text_query_async(handle, text, access_token).recv_opt().unwrap_or(Err(ChannelClosedError))
}

/**
 * Send a text query to the Wit instance identified by the access_token.
 * This function is non-blocking. It returns a Receiver that can be used
 * to get the response from Wit.
 */
pub fn text_query_async(handle: &WitHandle, text: String, access_token: String) -> Receiver<Result<Json, RequestError>> {
    client::interpret_string(handle, access_token, text)
}

/**
 * Send a voice query to the Wit instance identified by the access_token.
 * This function is blocking, and returns the response from the Wit instance.
 *
 * The function attempts to automatically detect when the user stops speaking. If this
 * fails, the voice_query_stop or voice_query_stop_async functions below can
 * be used to trigger the end of the request and receive the response.
 */
pub fn voice_query_auto(handle: &WitHandle, access_token: String) -> Result<Json, RequestError> {
    voice_query_auto_async(handle, access_token).recv_opt().unwrap_or(Err(ChannelClosedError))
}

/**
 * Send a voice query to the Wit instance identified by the access_token.
 * This function is non-blocking. It returns a Receiver that can be used
 * to get the response from Wit.
 *
 * The function attempts to automatically detect when the user stops speaking. If this
 * fails, the voice_query_stop or voice_query_stop_async functions below can
 * be used to trigger the end of the request and receive the response.
 */
pub fn voice_query_auto_async(handle: &WitHandle, access_token: String) -> Receiver<Result<Json, RequestError>> {
    client::start_autoend_recording(handle, access_token)
}

/**
 * Send a voice query to the Wit instance identified by the access_token.
 * This function returns immediately. The recording session stops only when either
 * voice_query_stop or voice_query_stop_async is called. No end-of-speech detection
 * is performed.
 */
pub fn voice_query_start(handle: &WitHandle, access_token: String) {
    client::start_recording(handle, access_token);
}

/**
 * Stop the ongoing recording session and receive the response.
 * This function is blocking, and returns the response from the Wit instance.
 * This function has no effect if there is no ongoing recording session.
 */
pub fn voice_query_stop(handle: &WitHandle) -> Result<Json, RequestError> {
    voice_query_stop_async(handle).recv_opt().unwrap_or(Err(ChannelClosedError))
}

/**
 * Stop the ongoing recording session and receive the response.
 * This function is non-blocking. It returns a Receiver that can be used
 * to get the response from Wit.
 * This function has no effect if there is no ongoing recording session.
 */
pub fn voice_query_stop_async(handle: &WitHandle) -> Receiver<Result<Json, RequestError>> {
    client::stop_recording(handle)
}
