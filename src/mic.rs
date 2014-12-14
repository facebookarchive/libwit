use std::ptr::null;
use std::io;
use std::c_str::CString;
use libc::{c_void, size_t};
use std::comm::{Empty, Disconnected};
use std::vec::Vec;
use std::num::Int;
use log;
use log::LogLevel::{Error, Debug, Info};
use ffi::{mod, SoxEncodingT, SoxErrorT, SoxBool, SoxFormatT};
use vad;

const BUF_SIZE: uint = 100;

pub struct MicContext {
    pub reader: Box<io::ChanReader>,
    pub sender: Sender<bool>,
    pub rate: u32,
    pub encoding: String
}

fn is_big_endian() -> bool {
    1u16.to_be() == 1u16
}

fn cleanup_recording_session(input_ptr: *const SoxFormatT, vad_state: *const c_void) {
    wit_log!(Info, "stopping mic");
    unsafe {ffi::sox_close(input_ptr)};
    if !vad_state.is_null() {
        unsafe {vad::wvs_clean(vad_state)};
    }
}

pub fn start(input_device: Option<String>, vad_enabled: bool) -> Option<MicContext> {

    let (tx, rx) = channel();
    let reader = io::ChanReader::new(rx);

    let (ctl_tx, ctl_rx) = channel();

    let path = input_device.unwrap_or("default".to_string()).to_c_str();
    let alsa = "alsa".to_c_str();
    let coreaudio = "coreaudio".to_c_str();

    let mut input_ptr = unsafe {ffi::sox_open_read(path.as_ptr(), null(), null(), alsa.as_ptr())};
    if input_ptr.is_null() {
        wit_log!(Info, "couldn't open input device using alsa. Trying with coreaudio...");
        input_ptr = unsafe {ffi::sox_open_read(path.as_ptr(), null(), null(), coreaudio.as_ptr())};
    }
    if input_ptr.is_null() {
        wit_log!(Error, "Failed to open input device");
        return None;
    }

    let input = unsafe {&*input_ptr};
    wit_log!(Info, "initialized recording device");
    wit_log!(Debug, "rate: {}, channels: {}, encoding: {}, bits_per_sample: {}, opposite_endian: {}",
        input.signal.rate,
        input.signal.channels,
        input.encoding.encoding,
        input.encoding.bits_per_sample,
        input.encoding.opposite_endian);

    let is_big_endian = match input.encoding.opposite_endian {
        SoxBool::SoxFalse => is_big_endian(),
        SoxBool::SoxTrue => !is_big_endian()
    };

    // initialize VAD
    let vad_state = if vad_enabled {
        unsafe {vad::wvs_init(8f64, input.signal.rate as i32)}
    } else {
        null()
    };

    let cloned_input = input.clone();
    spawn(proc() {
        loop {
            match ctl_rx.try_recv() {
                Ok(x) => {
                    wit_log!(Debug, "received {}", x);
                    match x {
                        true => (),
                        false => {
                            cleanup_recording_session(input_ptr, vad_state);
                            break;
                        }
                    }
                }
                Err(Empty) => {
                    let num_channels = cloned_input.signal.channels as uint;
                    let total_bytes = 4 * (BUF_SIZE - BUF_SIZE % num_channels);
                    let buf = Vec::from_elem(total_bytes, 0u8);
                    unsafe {ffi::sox_read(input_ptr, (&buf).as_ptr() as *const i32, BUF_SIZE as size_t)};
                    //println!("Read: {}", buf);
                    let total_mono_bytes = total_bytes / (2 * num_channels); // 32bit -> 16bit
                    let num_samples = total_mono_bytes / 2;
                    let monobuf = Vec::from_fn(total_mono_bytes, |idx| {
                        let byte_offset = if is_big_endian {
                            idx % 2
                        } else {
                            3 - idx % 2
                        };
                        buf[(idx / 4) * 4 * num_channels * 2 + byte_offset]
                    });

                    if vad_enabled {
                        // TODO we shouldn't need to change the format twice
                        let monobuf_platform_endianness = Vec::from_fn(total_mono_bytes, |idx| {
                            let byte_offset = match cloned_input.encoding.opposite_endian {
                                SoxBool::SoxFalse => idx % 2,
                                SoxBool::SoxTrue => 1 - idx % 2
                            };
                            buf[(idx / 2) * 2 + byte_offset]
                        });

                        let still_talking = unsafe {
                            vad::wvs_still_talking(
                                vad_state,
                                monobuf_platform_endianness.as_ptr() as *const i16,
                                num_samples as i32)
                            };
                        if still_talking == 0 {
                            wit_log!(Info, "detected end of speech");
                            cleanup_recording_session(input_ptr, vad_state);
                            break;
                        }
                    }

                    let result = tx.send_opt(monobuf);
                    if result.is_err() {
                        wit_log!(Error, "error while sending: {}", result.err());
                    }
                }
                Err(Disconnected) => {
                    wit_log!(Info, "done");
                    cleanup_recording_session(input_ptr, vad_state);
                    break;
                }
            }
        }
    });

    ctl_tx.send(true);

    let ref sox_encoding = input.encoding.encoding;
    let encoding_opt = match sox_encoding {
        &SoxEncodingT::SOX_ENCODING_SIGN2 => Some("signed-integer"),
        &SoxEncodingT::SOX_ENCODING_UNSIGNED => Some("unsigned-integer"),
        &SoxEncodingT::SOX_ENCODING_FLOAT => Some("floating-point"),
        &SoxEncodingT::SOX_ENCODING_ULAW => Some("ulaw"),
        &SoxEncodingT::SOX_ENCODING_ALAW => Some("alaw"),
        _ => None
    };
    if encoding_opt.is_none() {
        wit_log!(Error, "unsupported encoding: {}", sox_encoding);
        return None
    }
    Some(MicContext {
        reader: box reader,
        sender: ctl_tx,
        rate: input.signal.rate as u32,
        encoding: encoding_opt.unwrap().to_string()
    })
}

pub fn stop(tx: &Sender<bool>) {
    tx.send(false);
}

pub fn init (/*args: &[String]*/) {
    match unsafe {ffi::sox_format_init()} {
        SoxErrorT::SOX_SUCCESS => wit_log!(Info, "initialized sox: {}", unsafe {CString::new(ffi::sox_version(), false)}),
        err => {
            wit_log!(Error, "failed to initialize sox: {}", err);
            return;
        }
    };
}
