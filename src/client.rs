use std::{str, io};
use curl::http::{mod, Request};
use curl::ErrCode;
use serialize::json::{mod, Json};
use curl::http::body::{Body, ToBody, ChunkedBody};
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

#[deriving(Show)]
pub enum RequestError {
    InvalidResponseError,
    ChannelClosedError,
    ParserError(json::ParserError),
    NetworkError(ErrCode),
    StatusError(uint)
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

fn exec_request(request: Request, token: String) -> Result<Json,RequestError> {
    request
        .header("Authorization", format!("Bearer {}", token).as_slice())
        .header("Accept", "application/vnd.wit.20141124+json")
        .exec()
        .map_err(|e| {
            wit_log!(Error, "network error: {}", e);
            RequestError::NetworkError(e)
        })
        .and_then(|x| {
            let status = x.get_code();
            if status >= 400 {
                wit_log!(Error, "server responded with error: {}", status);
                return Err(RequestError::StatusError(status));
            }
            let body = x.get_body();
            match str::from_utf8(body.as_slice()) {
                Some(str) => {
                    let obj = json::from_str(str);
                    obj.map_err(|e| {
                        wit_log!(Error, "could not parse response from server: {}", str);
                        RequestError::ParserError(e)
                    })
                }
                None => {
                    wit_log!(Error, "response was not valid UTF-8");
                    Err(RequestError::InvalidResponseError)
                }
            }
        })
}

fn do_message_request(msg: String, token: String) -> Result<Json,RequestError> {
    let mut init_req = http::handle();
    let encoded = url::utf8_percent_encode(msg.as_slice(), url::QUERY_ENCODE_SET);
    let req = init_req
        .get(format!("https://api.wit.ai/message?q={}", encoded));
    exec_request(req, token)
}

pub struct WrapReader<'a>(pub &'a mut Reader+'static);

impl<'a> ToBody<'a> for WrapReader<'a> {
    fn to_body(self) -> Body<'a> {
        let WrapReader(x) = self;
        ChunkedBody(x)
    }
}

fn do_speech_request(stream: &mut io::ChanReader, content_type:String, token: String) -> Result<Json,RequestError> {
    let mut init_req = http::handle();
    let req = init_req.post("https://api.wit.ai/speech", WrapReader(stream))
        .content_type(content_type.as_slice())
        .chunked();
    exec_request(req, token)
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

                    let content_type =
                        format!("audio/raw;encoding={};bits=16;rate={};endian=big", encoding, rate);
                    wit_log!(Debug, "Sending speech request with content type: {}", content_type);
                    spawn(proc() {
                        let reader_ref = &mut *reader;
                        let foo = do_speech_request(reader_ref, content_type, token);
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

pub fn interpret_string(ctl: &Sender<WitCommand>,
                        token: String,
                        text: String) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Text(token, text, result_tx));
    return result_rx
}

pub fn start_recording(ctl: &Sender<WitCommand>, token: String) {
    ctl.send(WitCommand::Start(token, None));
}

pub fn start_autoend_recording(ctl: &Sender<WitCommand>, token: String) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Start(token, Some(result_tx)));
    result_rx
}

pub fn stop_recording(ctl: &Sender<WitCommand>) -> Receiver<Result<Json,RequestError>> {
    let (result_tx, result_rx) = channel();
    ctl.send(WitCommand::Stop(result_tx));
    result_rx
}

pub fn cleanup(ctl: &Sender<WitCommand>) {
    ctl.send(WitCommand::Cleanup);
    // TODO: have the mic call sox_quit()
}

pub fn init(opts: Options) -> Sender<WitCommand>{
    log::set_verbosity(opts.verbosity);

    mic::init();

    let (cmd_tx, cmd_rx): (Sender<WitCommand>, Receiver<WitCommand>) = channel();

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
