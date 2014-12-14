use std::io;
use std::io::util::copy;
use hyper::client::request::Request;
use hyper::client::response::Response;
use hyper::Url;
use hyper::header::Headers;
use hyper::header::common::{ContentType, Authorization, Accept};
//use hyper::header::common::TransferEncoding;
//use hyper::header::common::transfer_encoding::Encoding;
use hyper::status::{StatusCode, StatusClass};
use mime::{Mime, TopLevel, SubLevel, Attr, Value};
use serialize::json::{mod, Json};
use url;

use mic;
use log;
use log::LogLevel::{Error, Warn, Info, Debug};

pub enum WitCommand {
    Text(String, String, Sender<Result<Json, RequestError>>),
    Start(String, Option<Sender<Result<Json, RequestError>>>),
    Stop(Sender<Result<Json, RequestError>>),
    Cleanup
}

pub type WitHandle = Sender<WitCommand>;

#[deriving(Show,Copy)]
pub enum RequestError {
    InvalidResponseError,
    ChannelClosedError,
    ClientError,
    InternalError,
    ParserError(json::ParserError),
    StatusError(StatusCode)
}

enum State {
    Ongoing(Context),
    Idle,
    Stopped
}

struct Context {
    http: Receiver<Result<Json,RequestError>>,
    mic: Sender<bool>,
    client: Option<Sender<Result<Json,RequestError>>>
}

#[deriving(Clone)]
pub struct Options {
    pub input_device: Option<String>,
    pub verbosity: uint
}

fn read_response(response: &mut Response) -> Result<Json,RequestError> {
    let status = response.status;
    if status.class() != StatusClass::Informational && status.class() != StatusClass::Success {
        wit_log!(Error, "server responded with error: {}", status);
        return Err(RequestError::StatusError(status));
    }
    match response.read_to_string() {
        Ok(str) => {
            let obj = json::from_str(str.as_slice());
            obj.map_err(|e| {
                wit_log!(Error, "could not parse response from server: {}", str);
                RequestError::ParserError(e)
            })
        }
        Err(e) => {
            wit_log!(Error, "failed to read response body: {}", e);
            Err(RequestError::InvalidResponseError)
        }
    }
}

fn set_common_headers(h: &mut Headers, token: String) {
    h.set(Authorization(format!("Bearer {}", token)));
    h.set(Accept(vec![Mime(TopLevel::Application, SubLevel::Ext("vnd.wit.20141124+json".to_string()), vec![])]));
}

fn do_message_request(msg: String, token: String) -> Result<Json,RequestError> {
    let encoded = url::utf8_percent_encode(msg.as_slice(), url::QUERY_ENCODE_SET);
    let mut req = Request::get(Url::parse(format!("https://api.wit.ai/message?q={}", encoded).as_slice()).unwrap()).unwrap();
    set_common_headers(req.headers_mut(), token);
    let mut res = req.start().unwrap().send().unwrap();
    read_response(&mut res)
}

fn do_speech_request(stream: &mut io::ChanReader, encoding:String, rate:u32, token: String) -> Result<Json,RequestError> {
    let mut req = Request::post(Url::parse("https://api.wit.ai/speech").unwrap()).unwrap();
    let mime = Mime(
        TopLevel::Audio,
        SubLevel::Ext("raw".to_string()),
        vec![(Attr::Ext("encoding".to_string()), Value::Ext(encoding)),
             (Attr::Ext("bits".to_string()), Value::Ext("16".to_string())),
             (Attr::Ext("rate".to_string()), Value::Ext(format!("{}", rate))),
             (Attr::Ext("endian".to_string()), Value::Ext("big".to_string()))]
    );
    {
        let h = req.headers_mut();
        h.set(ContentType(mime));
        //h.set(TransferEncoding(vec![Encoding::Chunked]));
        set_common_headers(h, token);
    }
    let mut streaming_req = req.start().unwrap();
    match copy(stream, &mut streaming_req) {
        Ok(..) => (),
        Err(e) => wit_log!(Error, "failed to stream audio to server: {}", e)
    };
    match streaming_req.send() {
        Ok(mut res) => read_response(&mut res),
        Err(_) => Err(RequestError::ClientError)
    }
}

fn next_state(state: State, cmd: WitCommand, opts: Options) -> State {
    match cmd {
        WitCommand::Text(token, text, result_tx) => {
            let r = do_message_request(text, token);
            result_tx.send(r);
            state
        }
        WitCommand::Start(token, autoend_result_tx) => {
            match state {
                State::Ongoing(context) => State::Ongoing(context),
                _ => {
                    let mic_context_opt = mic::start(opts.input_device.clone(), autoend_result_tx.is_some());

                    let (http_tx, http_rx) = channel();
                    let mic::MicContext {
                        mut reader,
                        sender: mic_tx,
                        rate,
                        encoding
                    } = mic_context_opt.unwrap();

                    spawn(proc() {
                        let reader_ref = &mut *reader;
                        let foo = do_speech_request(reader_ref, encoding, rate, token);
                        http_tx.send(foo);
                    });

                    State::Ongoing(Context {
                        http: http_rx,
                        mic: mic_tx,
                        client: autoend_result_tx
                    })
                }
            }
        }
        WitCommand::Stop(result_tx) => {
            match state {
                State::Ongoing(context) => {
                    let Context { http: http_rx, mic: mic_tx, client: _ } = context;

                    mic::stop(&mic_tx);
                    let foo = http_rx.recv();
                    result_tx.send(foo);

                    State::Idle
                },
                s => {
                    wit_log!(Warn, "trying to stop but no request started");
                    s
                }
            }
        }
        WitCommand::Cleanup => {
            match state {
                State::Ongoing(context) => {
                    let Context { http: _, mic: mic_tx, client: _ } = context;
                    mic::stop(&mic_tx)
                },
                _ => ()
            };
            State::Stopped
        }
    }
}

pub fn interpret_string(ctl: &WitHandle,
                        token: String,
                        text: String) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Text(token, text, result_tx));
    return result_rx
}

pub fn start_recording(ctl: &WitHandle, token: String) {
    ctl.send(WitCommand::Start(token, None));
}

pub fn start_autoend_recording(ctl: &WitHandle, token: String) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Start(token, Some(result_tx)));
    result_rx
}

pub fn stop_recording(ctl: &WitHandle) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Stop(result_tx));
    result_rx
}

pub fn cleanup(ctl: &WitHandle) {
    ctl.send(WitCommand::Cleanup);
    // TODO: have the mic call sox_quit()
}

pub fn init(opts: Options) -> WitHandle{
    log::set_verbosity(opts.verbosity);

    mic::init();

    let (cmd_tx, cmd_rx): (WitHandle, Receiver<WitCommand>) = channel();

    wit_log!(Debug, "init state machine");

    spawn(proc() {
        let mut ongoing: State = State::Idle;
        loop {
            wit_log!(Info, "ready. state={}", match ongoing {
                State::Ongoing(_) => "recording",
                State::Idle => "idle",
                State::Stopped => "stopped"
            });

            match ongoing {
                State::Stopped => break,
                _ => ()
            }

            ongoing = match ongoing {
                State::Ongoing(context) => {
                    match context.client {
                        Some(client) => {
                            let http = context.http;
                            let mic = context.mic;
                            let cmd_opt = select! (
                                cmd = cmd_rx.recv() => Some(cmd),
                                foo = http.recv() => {
                                    client.send(foo);
                                    None
                                }
                            );
                            match cmd_opt {
                                Some(cmd) => {
                                    let context = Context {
                                        http: http,
                                        mic: mic,
                                        client: Some(client.clone())
                                    };
                                    next_state(State::Ongoing(context), cmd, opts.clone())
                                }
                                None => State::Idle
                            }
                        },
                        None => {
                            let cmd = cmd_rx.recv();
                            next_state(State::Ongoing(context), cmd, opts.clone())
                        }
                    }
                },
                s => {
                    let cmd = cmd_rx.recv();
                    next_state(s, cmd, opts.clone())
                }
            };
        }
    });
    return cmd_tx
}
