//! BASS audio library FFI bindings (dynamic load)
//! 
//! These are manually written from the original bass.h header.
//! Only the functions needed by the player are included.

#![allow(non_camel_case_types, non_snake_case, dead_code, clippy::upper_case_acronyms, clippy::unreadable_literal)]

use libloading::Library;
use std::sync::OnceLock;
use std::ffi::c_void;

// ===== Constants =====

pub const BASS_OK: i32 = 0;

// BASS_Init flags
pub const BASS_DEVICE_ENABLED: u32 = 1;
pub const BASS_DEVICE_DEFAULT: u32 = 0;
pub const BASS_DEVICE_8BITS: u32 = 1;
pub const BASS_DEVICE_MONO: u32 = 2;
pub const BASS_DEVICE_3D: u32 = 4;
pub const BASS_DEVICE_LATENCY: u32 = 0x100;
pub const BASS_DEVICE_CPSPEAKERS: u32 = 0x400;
pub const BASS_DEVICE_SPEAKERS: u32 = 0x800;
pub const BASS_DEVICE_NOSPEAKER: u32 = 0x1000;
pub const BASS_DEVICE_DMIX: u32 = 0x2000;
pub const BASS_DEVICE_FREQ: u32 = 0x4000;
pub const BASS_DEVICE_STEREO: u32 = 0x8000;

// BASS_StreamCreateFile flags
pub const BASS_STREAM_DECODE: u32 = 0x0020_0000;
pub const BASS_SAMPLE_FLOAT: u32 = 0x100;
pub const BASS_SAMPLE_LOOP: u32 = 0x4;
pub const BASS_SAMPLE_3D: u32 = 0x8;
pub const BASS_SAMPLE_SOFTWARE: u32 = 0x10;
pub const BASS_SAMPLE_MUTEMAX: u32 = 0x20;
pub const BASS_SAMPLE_VAM: u32 = 0x40;
pub const BASS_SAMPLE_FX: u32 = 0x80;
pub const BASS_SAMPLE_OVER_VOL: u32 = 0x10000;
pub const BASS_SAMPLE_OVER_POS: u32 = 0x20000;
pub const BASS_SAMPLE_OVER_DIST: u32 = 0x30000;
pub const BASS_STREAM_AUTOFREE: u32 = 0x40000;
pub const BASS_STREAM_RESTRATE: u32 = 0x80000;
pub const BASS_STREAM_BLOCK: u32 = 0x0010_0000;
pub const BASS_STREAM_STATUS: u32 = 0x0080_0000;

// BASS_ChannelPlay flags
pub const BASS_SAMPLE_REPEAT: u32 = 0x2;

// BASS_ChannelSetPosition mode
pub const BASS_POS_BYTE: u32 = 0;
pub const BASS_POS_MUSIC_ORDER: u32 = 1;
pub const BASS_POS_MUSIC_ROW: u32 = 2;
pub const BASS_POS_MUSIC_RESET: u32 = 0x10000;
pub const BASS_POS_MUSIC_SEEK: u32 = 0x20000;
pub const BASS_POS_DECODE: u32 = 0x40000;
pub const BASS_POS_DECODETO: u32 = 0x0080_0000;
pub const BASS_POS_INEXACT: u32 = 0x0100_0000;
pub const BASS_POS_MUSIC_BPM: u32 = 0x0200_0000;

// BASS_ChannelGetData flags
pub const BASS_DATA_AVAILABLE: u32 = 0;
pub const BASS_DATA_FLOAT: u32 = 0x40000000;
pub const BASS_DATA_FFT256: u32 = 0x80000000;
pub const BASS_DATA_FFT512: u32 = 0x80000001;
pub const BASS_DATA_FFT1024: u32 = 0x80000002;
pub const BASS_DATA_FFT2048: u32 = 0x80000003;
pub const BASS_DATA_FFT4096: u32 = 0x80000004;
pub const BASS_DATA_FFT8192: u32 = 0x80000005;
pub const BASS_DATA_FFT_INDIVIDUAL: u32 = 0x10;
pub const BASS_DATA_FFT_NOWINDOW: u32 = 0x20;
pub const BASS_DATA_FFT_REMOVEDC: u32 = 0x40;
pub const BASS_DATA_FFT_COMPLEX: u32 = 0x80;
pub const BASS_DATA_FFT_NYQUIST: u32 = 0x100;

// BASS_ChannelSetAttribute attributes
pub const BASS_ATTRIB_FREQ: u32 = 1;
pub const BASS_ATTRIB_VOL: u32 = 2;
pub const BASS_ATTRIB_DB_GAIN: u32 = 201;
pub const BASS_ATTRIB_PAN: u32 = 3;
pub const BASS_ATTRIB_EAXMIX: u32 = 4;
pub const BASS_ATTRIB_NOBUFFER: u32 = 5;
pub const BASS_ATTRIB_VBR: u32 = 6;
pub const BASS_ATTRIB_CPU: u32 = 7;
pub const BASS_ATTRIB_SRC: u32 = 8;
pub const BASS_ATTRIB_NET_RESUME: u32 = 9;
pub const BASS_ATTRIB_SCANINFO: u32 = 10;
pub const BASS_ATTRIB_NORAMP: u32 = 11;
pub const BASS_ATTRIB_BITRATE: u32 = 12;
pub const BASS_ATTRIB_MUSIC_AMPLIFY: u32 = 0x100;
pub const BASS_ATTRIB_MUSIC_PANSEP: u32 = 0x101;
pub const BASS_ATTRIB_MUSIC_PSCALER: u32 = 0x102;
pub const BASS_ATTRIB_MUSIC_BPM: u32 = 0x103;
pub const BASS_ATTRIB_MUSIC_SPEED: u32 = 0x104;
pub const BASS_ATTRIB_MUSIC_VOL_GLOBAL: u32 = 0x105;
pub const BASS_ATTRIB_MUSIC_ACTIVE: u32 = 0x106;
pub const BASS_ATTRIB_MUSIC_VOL_CHAN: u32 = 0x200;
pub const BASS_ATTRIB_MUSIC_VOL_INST: u32 = 0x300;

// BASS_ChannelIsActive return values
pub const BASS_ACTIVE_STOPPED: u32 = 0;
pub const BASS_ACTIVE_PLAYING: u32 = 1;
pub const BASS_ACTIVE_STALLED: u32 = 2;
pub const BASS_ACTIVE_PAUSED: u32 = 3;
pub const BASS_ACTIVE_PAUSED_DEVICE: u32 = 4;

// BASS_ChannelGetLevelEx flags
pub const BASS_LEVEL_MONO: u32 = 1;
pub const BASS_LEVEL_STEREO: u32 = 2;
pub const BASS_LEVEL_RMS: u32 = 4;
pub const BASS_LEVEL_VOLPAN: u32 = 8;

// BASS_ErrorGetCode return values
pub const BASS_ERROR_MEM: i32 = 1;
pub const BASS_ERROR_FILEOPEN: i32 = 2;
pub const BASS_ERROR_DRIVER: i32 = 3;
pub const BASS_ERROR_BUFLOST: i32 = 4;
pub const BASS_ERROR_HANDLE: i32 = 5;
pub const BASS_ERROR_FORMAT: i32 = 6;
pub const BASS_ERROR_POSITION: i32 = 7;
pub const BASS_ERROR_INIT: i32 = 8;
pub const BASS_ERROR_START: i32 = 9;
pub const BASS_ERROR_ALREADY: i32 = 14;
pub const BASS_ERROR_NOCHAN: i32 = 18;
pub const BASS_ERROR_ILLTYPE: i32 = 19;
pub const BASS_ERROR_ILLPARAM: i32 = 20;
pub const BASS_ERROR_NO3D: i32 = 21;
pub const BASS_ERROR_NOEAX: i32 = 22;
pub const BASS_ERROR_DEVICE: i32 = 23;
pub const BASS_ERROR_NOPLAY: i32 = 24;
pub const BASS_ERROR_FREQ: i32 = 25;
pub const BASS_ERROR_NOTFILE: i32 = 27;
pub const BASS_ERROR_NOHW: i32 = 29;
pub const BASS_ERROR_EMPTY: i32 = 31;
pub const BASS_ERROR_NONET: i32 = 32;
pub const BASS_ERROR_CREATE: i32 = 33;
pub const BASS_ERROR_NOFX: i32 = 34;
pub const BASS_ERROR_NOTAVAIL: i32 = 37;
pub const BASS_ERROR_DECODE: i32 = 38;
pub const BASS_ERROR_DX: i32 = 39;
pub const BASS_ERROR_TIMEOUT: i32 = 40;
pub const BASS_ERROR_FILEFORM: i32 = 41;
pub const BASS_ERROR_SPEAKER: i32 = 42;
pub const BASS_ERROR_VERSION: i32 = 43;
pub const BASS_ERROR_CODEC: i32 = 44;
pub const BASS_ERROR_ENDED: i32 = 45;
pub const BASS_ERROR_BUSY: i32 = 46;
pub const BASS_ERROR_UNKNOWN: i32 = -1;

// BASS_FX_TempoCreate flags
pub const BASS_FX_FREESOURCE: u32 = 0x100000;
pub const BASS_FX_TEMPO_ALGO_LINEAR: u32 = 0x200;
pub const BASS_FX_TEMPO_ALGO_CUBIC: u32 = 0x400;
pub const BASS_FX_TEMPO_ALGO_SHANNON: u32 = 0x800;

// BASS_FX tempo/pitch attributes
pub const BASS_ATTRIB_TEMPO: u32 = 0x10000;
pub const BASS_ATTRIB_TEMPO_PITCH: u32 = 0x10001;
pub const BASS_ATTRIB_TEMPO_FREQ: u32 = 0x10002;
pub const BASS_ATTRIB_TEMPO_OPTION_USE_QUICKALGO: u32 = 0x20010;
pub const BASS_ATTRIB_TEMPO_OPTION_OVERLAP_MS: u32 = 0x20011;
pub const BASS_ATTRIB_TEMPO_OPTION_SEQUENCE_MS: u32 = 0x20012;
pub const BASS_ATTRIB_TEMPO_OPTION_SEEKWINDOW_MS: u32 = 0x20013;
pub const BASS_ATTRIB_TEMPO_OPTION_LATENCY: u32 = 0x20014;
pub const BASS_ATTRIB_TEMPO_OPTION_PITCH_AQUALITY: u32 = 0x20015;
pub const BASS_ATTRIB_TEMPO_OPTION_SAMPLERATE: u32 = 0x20016;
pub const BASS_ATTRIB_TEMPO_OPTION_RESAMPLER: u32 = 0x20017;
pub const BASS_ATTRIB_TEMPO_OPTION_ANTI_FILTER: u32 = 0x20018;

// BASS_FX peak equalizer
pub const BASS_DX8_PEAKEQ: u32 = 7;
pub const BASS_FX_BFX_PEAKEQ: u32 = 29;

#[repr(C)]
pub struct BASS_BFX_PEAKEQ {
    pub lBand: i32,        // band number (0-9)
    pub fBandwidth: f32,   // bandwidth in octaves (0.1-10)
    pub fGain: f32,        // gain in dB (-15 to 15)
    pub fQ: f32,           // quality factor (0.1-10)
    pub lChannel: i32,     // channel (0=both)
}

// BASS_FX reverb
pub const BASS_DX8_REVERB: u32 = 9;

#[repr(C)]
pub struct BASS_DX8_REVERB_PARAMETERS {
    pub fInGain: f32,          // input gain in dB (-96 to 0)
    pub fReverbMix: f32,       // reverb mix in dB (-96 to 0)
    pub fReverbTime: f32,      // reverb time in ms (0.001 to 3000)
    pub fHighFreqRTRatio: f32, // high frequency ratio (0.001 to 0.999)
}

// BASS_RecordInit / Device info
#[repr(C)]
pub struct BASS_DEVICEINFO {
    pub name: *const u16,     // device description
    pub driver: *const u16,   // driver
    pub flags: u32,
}

// ===== Library handle =====
static BASS_LIB: OnceLock<Library> = OnceLock::new();
static BASS_FX_LIB: OnceLock<Library> = OnceLock::new();
static BASS_MIDI_LIB: OnceLock<Library> = OnceLock::new();

pub fn is_bass_loaded() -> bool {
    BASS_LIB.get().is_some()
}

pub fn is_bass_midi_loaded() -> bool {
    BASS_MIDI_LIB.get().is_some()
}

/// Load BASS libraries. Returns true if successful.
pub fn load_bass(bass_path: Option<&str>, bass_fx_path: Option<&str>) -> bool {
    // Try loading from given paths or default names
    let bass_name = bass_path.unwrap_or("bass");
    let fx_name = bass_fx_path.unwrap_or("bass_fx");

    let bass_lib = unsafe {
        Library::new(bass_name)
            .or_else(|_| Library::new("libbass"))
            .or_else(|_| Library::new("libbass.so"))
            .or_else(|_| Library::new("libbass.dylib"))
    };

    match bass_lib {
        Ok(lib) => {
            let _ = BASS_LIB.set(lib);
        }
        Err(e) => {
            tracing::error!("Failed to load BASS library: {}", e);
            return false;
        }
    }

    // BASS_FX is optional
    let fx_lib = unsafe {
        Library::new(fx_name)
            .or_else(|_| Library::new("libbass_fx"))
            .or_else(|_| Library::new("libbass_fx.so"))
            .or_else(|_| Library::new("libbass_fx.dylib"))
    };
    if let Ok(lib) = fx_lib {
        let _ = BASS_FX_LIB.set(lib);
    }

    true
}

/// Load `BASS_MIDI` library separately. Returns true if successful.
pub fn load_bass_midi(path: Option<&str>) -> bool {
    let name = path.unwrap_or("bassmidi");
    match unsafe { Library::new(name) } {
        Ok(lib) => {
            let _ = BASS_MIDI_LIB.set(lib);
            true
        }
        Err(e) => {
            tracing::warn!("Failed to load bassmidi: {}", e);
            false
        }
    }
}

// ===== Private loader helpers =====

type BOOL = i32;
type DWORD = u32;
type HSYNC = u32;
type HSTREAM = u32;
type HMUSIC = u32;
type HRECORD = u32;
type HCHANNEL = u32;

macro_rules! load_fn {
    ($lib:expr, $name:ident, ($($aty:ty),*) -> $ret:ty) => {
        (|| -> std::result::Result<libloading::Symbol<unsafe extern "system" fn($($aty),*) -> $ret>, String> {
            let lib = $lib.ok_or_else(|| format!("BASS library not loaded"))?;
            unsafe {
                let sym = lib.get(stringify!($name).as_bytes())
                    .map_err(|e| format!("Cannot find {}: {}", stringify!($name), e))?;
                Ok(sym)
            }
        })()
    };
}

/// Get the last BASS error code
pub fn get_bass_error_code() -> i32 {
    BASS_LIB.get().map_or(BASS_ERROR_UNKNOWN, |lib| unsafe {
            let sym: libloading::Symbol<unsafe extern "system" fn() -> i32> =
                lib.get(b"BASS_ErrorGetCode").unwrap();
            sym()
        })
}

fn get_bass_error() -> String {
    let code = get_bass_error_code();
    format!("BASS error {}: {}", code, bass_error_description(code))
}

pub const fn bass_error_description(code: i32) -> &'static str {
    match code {
        BASS_ERROR_MEM => "memory error",
        BASS_ERROR_FILEOPEN => "can't open the file",
        BASS_ERROR_DRIVER => "can't find a free/valid driver",
        BASS_ERROR_BUFLOST => "the sample buffer was lost",
        BASS_ERROR_HANDLE => "invalid handle",
        BASS_ERROR_FORMAT => "unsupported sample format",
        BASS_ERROR_POSITION => "invalid position",
        BASS_ERROR_INIT => "BASS_Init has not been successfully called",
        BASS_ERROR_START => "BASS_Start has not been successfully called",
        BASS_ERROR_ALREADY => "already initialized/paused/whatever",
        BASS_ERROR_NOCHAN => "can't get a free channel",
        BASS_ERROR_ILLTYPE => "an illegal type was specified",
        BASS_ERROR_ILLPARAM => "an illegal parameter was specified",
        BASS_ERROR_NO3D => "no 3D support",
        BASS_ERROR_NOEAX => "no EAX support",
        BASS_ERROR_DEVICE => "illegal device number",
        BASS_ERROR_NOPLAY => "not playing",
        BASS_ERROR_FREQ => "illegal sample rate",
        BASS_ERROR_NOTFILE => "the stream is not a file stream",
        BASS_ERROR_NOHW => "no hardware voices available",
        BASS_ERROR_EMPTY => "no data",
        BASS_ERROR_NONET => "internet connection error",
        BASS_ERROR_CREATE => "couldn't create the file",
        BASS_ERROR_NOFX => "no FX available",
        BASS_ERROR_NOTAVAIL => "requested data is not available",
        BASS_ERROR_DECODE => "the channel is a 'decoding channel'",
        BASS_ERROR_DX => "a sufficient DirectX version is not installed",
        BASS_ERROR_TIMEOUT => "connection timed out",
        BASS_ERROR_FILEFORM => "unsupported file format",
        BASS_ERROR_SPEAKER => "unavailable speaker",
        BASS_ERROR_VERSION => "invalid BASS version",
        BASS_ERROR_CODEC => "codec is not available/supported",
        BASS_ERROR_ENDED => "the channel/file has ended",
        BASS_ERROR_BUSY => "the device is busy",
        BASS_ERROR_UNKNOWN => "unknown error",
        _ => "unknown error code",
    }
}

// ===== Individual typed BASS API wrappers =====

pub fn BASS_Init(device: i32, freq: u32, flags: u32, win: *mut c_void, ds: *mut c_void) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_Init, (i32, u32, u32, *mut c_void, *mut c_void) -> i32)?;
    let ok = unsafe { f(device, freq, flags, win, ds) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_Free() -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_Free, () -> i32)?;
    let ok = unsafe { f() };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_Start() -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_Start, () -> i32)?;
    let ok = unsafe { f() };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_Stop() -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_Stop, () -> i32)?;
    let ok = unsafe { f() };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_Pause() -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_Pause, () -> i32)?;
    let ok = unsafe { f() };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_StreamCreateFile(mem: i32, file: *const c_void, offset: u64, length: u64, flags: u32) -> Result<u32, String> {
    let f = load_fn!(BASS_LIB.get(), BASS_StreamCreateFile, (i32, *const c_void, u64, u64, u32) -> u32)?;
    let handle = unsafe { f(mem, file, offset, length, flags) };
    if handle == 0 { Err(get_bass_error()) } else { Ok(handle) }
}

/// DOWNLOADPROC callback type for `BASS_StreamCreateURL`
pub type DOWNLOADPROC = Option<unsafe extern "system" fn(buffer: *const c_void, length: u32, user: *mut c_void)>;

pub fn BASS_StreamCreateURL(url: *const u8, offset: u32, flags: u32, proc: DOWNLOADPROC, user: *mut c_void) -> Result<u32, String> {
    let f = load_fn!(BASS_LIB.get(), BASS_StreamCreateURL, (*const u8, u32, u32, DOWNLOADPROC, *mut c_void) -> u32)?;
    let handle = unsafe { f(url, offset, flags, proc, user) };
    if handle == 0 { Err(get_bass_error()) } else { Ok(handle) }
}

pub fn BASS_StreamFree(handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_StreamFree, (u32) -> i32)?;
    let ok = unsafe { f(handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelPlay(handle: u32, restart: i32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelPlay, (u32, i32) -> i32)?;
    let ok = unsafe { f(handle, restart) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelPause(handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelPause, (u32) -> i32)?;
    let ok = unsafe { f(handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelStop(handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelStop, (u32) -> i32)?;
    let ok = unsafe { f(handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelSetPosition(handle: u32, pos: u64, mode: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelSetPosition, (u32, u64, u32) -> i32)?;
    let ok = unsafe { f(handle, pos, mode) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelGetPosition(handle: u32, mode: u32) -> u64 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelGetPosition, (u32, u32) -> u64).unwrap_or_else(|_| panic!("BASS_ChannelGetPosition not found"));
    unsafe { f(handle, mode) }
}

pub fn BASS_ChannelGetLength(handle: u32, mode: u32) -> u64 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelGetLength, (u32, u32) -> u64).unwrap_or_else(|_| panic!("BASS_ChannelGetLength not found"));
    unsafe { f(handle, mode) }
}

pub fn BASS_ChannelIsActive(handle: u32) -> u32 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelIsActive, (u32) -> u32).unwrap_or_else(|_| panic!("BASS_ChannelIsActive not found"));
    unsafe { f(handle) }
}

pub fn BASS_ChannelSetAttribute(handle: u32, attrib: u32, value: f32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelSetAttribute, (u32, u32, f32) -> i32)?;
    let ok = unsafe { f(handle, attrib, value) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelSlideAttribute(handle: u32, attrib: u32, value: f32, time: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelSlideAttribute, (u32, u32, f32, u32) -> i32)?;
    let ok = unsafe { f(handle, attrib, value, time) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_ChannelGetAttribute(handle: u32, attrib: u32) -> Result<f32, String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelGetAttribute, (u32, u32, *mut f32) -> i32)?;
    let mut value: f32 = 0.0;
    let ok = unsafe { f(handle, attrib, &raw mut value) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(value) }
}

pub fn BASS_ChannelGetData(handle: u32, buffer: *mut c_void, length: u32) -> u32 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelGetData, (u32, *mut c_void, u32) -> u32).unwrap_or_else(|_| panic!("BASS_ChannelGetData not found"));
    unsafe { f(handle, buffer, length) }
}

pub fn BASS_ChannelGetLevelEx(handle: u32, levels: *mut f32, length: f32, flags: u32) -> bool {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelGetLevelEx, (u32, *mut f32, f32, u32) -> i32).unwrap_or_else(|_| panic!("BASS_ChannelGetLevelEx not found"));
    unsafe { f(handle, levels, length, flags) != 0 }
}

pub fn BASS_ChannelBytes2Seconds(handle: u32, pos: u64) -> f64 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelBytes2Seconds, (u32, u64) -> f64).unwrap_or_else(|_| panic!("BASS_ChannelBytes2Seconds not found"));
    unsafe { f(handle, pos) }
}

pub fn BASS_ChannelSeconds2Bytes(handle: u32, pos: f64) -> u64 {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelSeconds2Bytes, (u32, f64) -> u64).unwrap_or_else(|_| panic!("BASS_ChannelSeconds2Bytes not found"));
    unsafe { f(handle, pos) }
}

pub fn BASS_GetDeviceInfo(device: u32, info: *mut BASS_DEVICEINFO) -> bool {
    let f = load_fn!(BASS_LIB.get(), BASS_GetDeviceInfo, (u32, *mut BASS_DEVICEINFO) -> i32).unwrap_or_else(|_| panic!("BASS_GetDeviceInfo not found"));
    unsafe { f(device, info) != 0 }
}

pub fn BASS_SetDevice(device: u32) -> bool {
    let f = load_fn!(BASS_LIB.get(), BASS_SetDevice, (u32) -> i32).unwrap_or_else(|_| panic!("BASS_SetDevice not found"));
    unsafe { f(device) != 0 }
}

pub fn BASS_GetDevice() -> u32 {
    let f = load_fn!(BASS_LIB.get(), BASS_GetDevice, () -> u32).unwrap_or_else(|_| panic!("BASS_GetDevice not found"));
    unsafe { f() }
}

pub fn BASS_GetVersion() -> u32 {
    let f = load_fn!(BASS_LIB.get(), BASS_GetVersion, () -> u32).unwrap_or_else(|_| panic!("BASS_GetVersion not found"));
    unsafe { f() }
}

pub fn BASS_ChannelSetFX(handle: u32, fx_type: u32, priority: i32) -> Result<u32, String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelSetFX, (u32, u32, i32) -> u32)?;
    let result = unsafe { f(handle, fx_type, priority) };
    if result == 0 { Err(get_bass_error()) } else { Ok(result) }
}

pub fn BASS_ChannelRemoveFX(handle: u32, fx_handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_ChannelRemoveFX, (u32, u32) -> i32)?;
    let ok = unsafe { f(handle, fx_handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

// ===== Plugin loading =====

pub fn BASS_PluginLoad(filename: *const u16, flags: u32) -> Result<u32, String> {
    let f = load_fn!(BASS_LIB.get(), BASS_PluginLoad, (*const u16, u32) -> u32)?;
    let handle = unsafe { f(filename, flags) };
    if handle == 0 { Err(get_bass_error()) } else { Ok(handle) }
}

pub fn BASS_PluginFree(handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_LIB.get(), BASS_PluginFree, (u32) -> i32)?;
    let ok = unsafe { f(handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

// ===== BASS_FX API =====

pub fn BASS_FX_TempoCreate(channel: u32, flags: u32) -> Result<u32, String> {
    let fx_lib = BASS_FX_LIB.get().ok_or_else(|| "BASS_FX not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(u32, u32) -> u32> =
            fx_lib.get(b"BASS_FX_TempoCreate")
                .map_err(|e| format!("Cannot find BASS_FX_TempoCreate: {e}"))?;
        let result = sym(channel, flags);
        if result == 0 { Err(get_bass_error()) } else { Ok(result) }
    }
}

pub fn BASS_FXSetParameters(handle: u32, params: *const c_void) -> Result<(), String> {
    let fx_lib = BASS_FX_LIB.get().ok_or_else(|| "BASS_FX not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(u32, *const c_void) -> i32> =
            fx_lib.get(b"BASS_FXSetParameters")
                .map_err(|e| format!("Cannot find BASS_FXSetParameters: {e}"))?;
        let ok = sym(handle, params);
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

pub fn BASS_FXGetParameters(handle: u32, params: *mut c_void) -> Result<(), String> {
    let fx_lib = BASS_FX_LIB.get().ok_or_else(|| "BASS_FX not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(u32, *mut c_void) -> i32> =
            fx_lib.get(b"BASS_FXGetParameters")
                .map_err(|e| format!("Cannot find BASS_FXGetParameters: {e}"))?;
        let ok = sym(handle, params);
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

// ===== BASS_MIDI types =====

pub type HSOUNDFONT = u32;

pub const BASS_MIDI_FONT_GM: u32 = 0x10000;

pub const BASS_ATTRIB_MIDI_PPQ: u32 = 0x11000;
pub const BASS_ATTRIB_MIDI_CPU: u32 = 0x11001;
pub const BASS_ATTRIB_MIDI_CHANS: u32 = 0x11002;
pub const BASS_ATTRIB_MIDI_VOICES: u32 = 0x11003;
pub const BASS_ATTRIB_MIDI_TRACK_VOL: u32 = 0x11006;

#[repr(C)]
pub struct BASS_MIDI_FONT {
    pub font: HSOUNDFONT,
    pub preset: i32,
    pub bank: i32,
    pub flags: u32,
}

pub fn BASS_MIDI_FontInit(file: *const u16, flags: u32) -> Result<u32, String> {
    let f = load_fn!(BASS_MIDI_LIB.get(), BASS_MIDI_FontInit, (*const u16, u32) -> u32)?;
    let handle = unsafe { f(file, flags) };
    if handle == 0 { Err(get_bass_error()) } else { Ok(handle) }
}

pub fn BASS_MIDI_FontFree(handle: u32) -> Result<(), String> {
    let f = load_fn!(BASS_MIDI_LIB.get(), BASS_MIDI_FontFree, (u32) -> i32)?;
    let ok = unsafe { f(handle) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_MIDI_FontLoad(font: u32, preset: i32, bank: i32) -> Result<(), String> {
    let f = load_fn!(BASS_MIDI_LIB.get(), BASS_MIDI_FontLoad, (u32, i32, i32) -> i32)?;
    let ok = unsafe { f(font, preset, bank) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_MIDI_StreamSetFonts(stream: u32, fonts: *const BASS_MIDI_FONT, count: u32) -> Result<(), String> {
    let f = load_fn!(BASS_MIDI_LIB.get(), BASS_MIDI_StreamSetFonts, (u32, *const BASS_MIDI_FONT, u32) -> i32)?;
    let ok = unsafe { f(stream, fonts, count) };
    if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
}

pub fn BASS_MIDI_StreamCreateFile(mem: i32, file: *const c_void, offset: u64, length: u64, flags: u32, freq: u32) -> Result<u32, String> {
    let f = load_fn!(BASS_MIDI_LIB.get(), BASS_MIDI_StreamCreateFile, (i32, *const c_void, u64, u64, u32, u32) -> u32)?;
    let handle = unsafe { f(mem, file, offset, length, flags, freq) };
    if handle == 0 { Err(get_bass_error()) } else { Ok(handle) }
}

// ===== Utility =====

pub fn get_device_name(device: u32) -> Option<String> {
    let mut info = BASS_DEVICEINFO {
        name: std::ptr::null(),
        driver: std::ptr::null(),
        flags: 0,
    };
    if BASS_GetDeviceInfo(device, &raw mut info) && !info.name.is_null() {
        #[allow(clippy::maybe_infinite_iter)]
        unsafe {
            let len = (0..).find(|&i| *info.name.offset(i) == 0).unwrap_or(0);
            let slice = std::slice::from_raw_parts(info.name, len.try_into().unwrap());
            Some(String::from_utf16_lossy(slice))
        }
    } else {
        None
    }
}

pub fn BASS_GetDeviceCount() -> u32 {
    let mut count = 0u32;
    loop {
        let mut info = BASS_DEVICEINFO {
            name: std::ptr::null(),
            driver: std::ptr::null(),
            flags: 0,
        };
        if !BASS_GetDeviceInfo(count, &raw mut info) {
            break;
        }
        count += 1;
    }
    count
}

// ===== BASSWASAPI =====

pub const BASS_WASAPI_EXCLUSIVE: u32 = 0x1000000;
pub const BASS_WASAPI_AUTOFORMAT: u32 = 0x2000000;
pub const BASS_WASAPI_BUFFER: u32 = 0x4000000;
pub const BASS_WASAPI_EVENT: u32 = 0x8000000;

pub const BASS_WASAPI_DEVICE_ENABLED: u32 = 1;
pub const BASS_WASAPI_DEVICE_INPUT: u32 = 2;
pub const BASS_WASAPI_DEVICE_LOOPBACK: u32 = 4;
pub const BASS_WASAPI_DEVICE_INITIALIZED: u32 = 8;

pub const BASS_WASAPI_CURVE_DB: i32 = 1;
pub const BASS_WASAPI_CURVE_LINEAR: i32 = 2;
pub const BASS_WASAPI_CURVE_WINDOWS: i32 = 3;

#[repr(C)]
pub struct BASS_WASAPI_DEVICEINFO {
    pub name: *const u16,
    pub id: *const u16,
    pub driver: *const u16,
    pub flags: u32,
    pub minperiod: f32,
    pub maxperiod: f32,
    pub mixfreq: u32,
}

#[repr(C)]
pub struct BASS_WASAPI_INFO {
    pub initflags: u32,
    pub freq: u32,
    pub chans: u32,
    pub format: u32,
    pub buflen: u32,
    pub volflags: u32,
    pub period: u32,
    pub latency: u32,
}

static BASS_WASAPI_LIB: OnceLock<Library> = OnceLock::new();

pub fn is_wasapi_loaded() -> bool {
    BASS_WASAPI_LIB.get().is_some()
}

/// Load BASSWASAPI library. Returns true if successful.
pub fn load_bass_wasapi(path: Option<&str>) -> bool {
    let name = path.unwrap_or("basswasapi");
    match unsafe { Library::new(name) } {
        Ok(lib) => {
            let _ = BASS_WASAPI_LIB.set(lib);
            true
        }
        Err(e) => {
            tracing::warn!("Failed to load basswasapi: {}", e);
            false
        }
    }
}

pub type WASAPIPROC = unsafe extern "system" fn(buffer: *mut c_void, length: u32, user: *mut c_void) -> u32;
pub type WASAPINOTIFYPROC = unsafe extern "system" fn(notify: u32, device: u32, user: *mut c_void);

#[allow(clippy::too_many_arguments)]
pub fn BASS_WASAPI_Init(device: i32, freq: u32, chans: u32, flags: u32, buffer: f32, period: f32, proc: Option<WASAPIPROC>, user: *mut c_void) -> Result<(), String> {
    let lib = BASS_WASAPI_LIB.get().ok_or_else(|| "BASSWASAPI not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(i32, u32, u32, u32, f32, f32, Option<WASAPIPROC>, *mut c_void) -> i32> =
            lib.get(b"BASS_WASAPI_Init")
                .map_err(|e| format!("Cannot find BASS_WASAPI_Init: {e}"))?;
        let ok = sym(device, freq, chans, flags, buffer, period, proc, user);
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

pub fn BASS_WASAPI_Free() -> Result<(), String> {
    let lib = BASS_WASAPI_LIB.get().ok_or_else(|| "BASSWASAPI not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn() -> i32> =
            lib.get(b"BASS_WASAPI_Free")
                .map_err(|e| format!("Cannot find BASS_WASAPI_Free: {e}"))?;
        let ok = sym();
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

pub fn BASS_WASAPI_Start() -> Result<(), String> {
    let lib = BASS_WASAPI_LIB.get().ok_or_else(|| "BASSWASAPI not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn() -> i32> =
            lib.get(b"BASS_WASAPI_Start")
                .map_err(|e| format!("Cannot find BASS_WASAPI_Start: {e}"))?;
        let ok = sym();
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

pub fn BASS_WASAPI_Stop(reset: i32) -> Result<(), String> {
    let lib = BASS_WASAPI_LIB.get().ok_or_else(|| "BASSWASAPI not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(i32) -> i32> =
            lib.get(b"BASS_WASAPI_Stop")
                .map_err(|e| format!("Cannot find BASS_WASAPI_Stop: {e}"))?;
        let ok = sym(reset);
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

#[allow(clippy::manual_let_else)]
pub fn BASS_WASAPI_IsStarted() -> u32 {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return 0 };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn() -> u32> =
            lib.get(b"BASS_WASAPI_IsStarted").unwrap_or_else(|_| panic!("BASS_WASAPI_IsStarted not found"));
        sym()
    }
}

#[allow(clippy::manual_let_else)]
pub fn BASS_WASAPI_GetDevice() -> u32 {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return 0 };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn() -> u32> =
            lib.get(b"BASS_WASAPI_GetDevice").unwrap_or_else(|_| panic!("BASS_WASAPI_GetDevice not found"));
        sym()
    }
}

#[allow(clippy::manual_let_else)]
pub fn BASS_WASAPI_GetDeviceInfo(device: u32, info: *mut BASS_WASAPI_DEVICEINFO) -> bool {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return false };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(u32, *mut BASS_WASAPI_DEVICEINFO) -> i32> =
            lib.get(b"BASS_WASAPI_GetDeviceInfo").unwrap_or_else(|_| panic!("BASS_WASAPI_GetDeviceInfo not found"));
        sym(device, info) != 0
    }
}

#[allow(clippy::manual_let_else)]
pub fn BASS_WASAPI_GetInfo(info: *mut BASS_WASAPI_INFO) -> bool {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return false };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(*mut BASS_WASAPI_INFO) -> i32> =
            lib.get(b"BASS_WASAPI_GetInfo").unwrap_or_else(|_| panic!("BASS_WASAPI_GetInfo not found"));
        sym(info) != 0
    }
}

pub fn BASS_WASAPI_SetVolume(mode: i32, volume: f32) -> Result<(), String> {
    let lib = BASS_WASAPI_LIB.get().ok_or_else(|| "BASSWASAPI not loaded".to_string())?;
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(i32, f32) -> i32> =
            lib.get(b"BASS_WASAPI_SetVolume")
                .map_err(|e| format!("Cannot find BASS_WASAPI_SetVolume: {e}"))?;
        let ok = sym(mode, volume);
        if ok == 0 { Err(get_bass_error()) } else { Ok(()) }
    }
}

pub fn BASS_WASAPI_GetVolume(mode: i32) -> f32 {
    let Some(lib) = BASS_WASAPI_LIB.get() else { return 0.0 };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(i32) -> f32> =
            lib.get(b"BASS_WASAPI_GetVolume").unwrap_or_else(|_| panic!("BASS_WASAPI_GetVolume not found"));
        sym(mode)
    }
}

pub fn BASS_WASAPI_GetData(buffer: *mut c_void, length: u32) -> u32 {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return 0 };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn(*mut c_void, u32) -> u32> =
            lib.get(b"BASS_WASAPI_GetData").unwrap_or_else(|_| panic!("BASS_WASAPI_GetData not found"));
        sym(buffer, length)
    }
}

pub fn BASS_WASAPI_GetCPU() -> f32 {
    let lib = match BASS_WASAPI_LIB.get() { Some(l) => l, None => return 0.0 };
    unsafe {
        let sym: libloading::Symbol<unsafe extern "system" fn() -> f32> =
            lib.get(b"BASS_WASAPI_GetCPU").unwrap_or_else(|_| panic!("BASS_WASAPI_GetCPU not found"));
        sym()
    }
}

pub fn wasapi_list_devices() -> Vec<(u32, String)> {
    let mut devices = Vec::new();
    let mut idx: u32 = 0;
    loop {
        let mut info = BASS_WASAPI_DEVICEINFO {
            name: std::ptr::null(),
            id: std::ptr::null(),
            driver: std::ptr::null(),
            flags: 0,
            minperiod: 0.0,
            maxperiod: 0.0,
            mixfreq: 0,
        };
        if !BASS_WASAPI_GetDeviceInfo(idx, &raw mut info) {
            break;
        }
        if info.flags & BASS_WASAPI_DEVICE_ENABLED != 0 && info.flags & BASS_WASAPI_DEVICE_INPUT == 0 {
            let name = if info.name.is_null() {
                format!("Device {idx}")
            } else {
                unsafe {
                    let len = (0..).find(|&i| *info.name.offset(i) == 0).unwrap_or(0) as usize;
                    let slice = std::slice::from_raw_parts(info.name, len);
                    String::from_utf16_lossy(slice)
                }
            };
            devices.push((idx, name));
        }
        idx += 1;
    }
    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bass_error_description_mem() {
        assert_eq!(bass_error_description(BASS_ERROR_MEM), "memory error");
    }

    #[test]
    fn test_bass_error_description_file_open() {
        assert_eq!(bass_error_description(BASS_ERROR_FILEOPEN), "can't open the file");
    }

    #[test]
    fn test_bass_error_description_driver() {
        assert_eq!(bass_error_description(BASS_ERROR_DRIVER), "can't find a free/valid driver");
    }

    #[test]
    fn test_bass_error_description_buf_lost() {
        assert_eq!(bass_error_description(BASS_ERROR_BUFLOST), "the sample buffer was lost");
    }

    #[test]
    fn test_bass_error_description_handle() {
        assert_eq!(bass_error_description(BASS_ERROR_HANDLE), "invalid handle");
    }

    #[test]
    fn test_bass_error_description_format() {
        assert_eq!(bass_error_description(BASS_ERROR_FORMAT), "unsupported sample format");
    }

    #[test]
    fn test_bass_error_description_position() {
        assert_eq!(bass_error_description(BASS_ERROR_POSITION), "invalid position");
    }

    #[test]
    fn test_bass_error_description_init() {
        assert_eq!(bass_error_description(BASS_ERROR_INIT), "BASS_Init has not been successfully called");
    }

    #[test]
    fn test_bass_error_description_start() {
        assert_eq!(bass_error_description(BASS_ERROR_START), "BASS_Start has not been successfully called");
    }

    #[test]
    fn test_bass_error_description_already() {
        assert_eq!(bass_error_description(BASS_ERROR_ALREADY), "already initialized/paused/whatever");
    }

    #[test]
    fn test_bass_error_description_no_chan() {
        assert_eq!(bass_error_description(BASS_ERROR_NOCHAN), "can't get a free channel");
    }

    #[test]
    fn test_bass_error_description_ill_type() {
        assert_eq!(bass_error_description(BASS_ERROR_ILLTYPE), "an illegal type was specified");
    }

    #[test]
    fn test_bass_error_description_ill_param() {
        assert_eq!(bass_error_description(BASS_ERROR_ILLPARAM), "an illegal parameter was specified");
    }

    #[test]
    fn test_bass_error_description_no_3d() {
        assert_eq!(bass_error_description(BASS_ERROR_NO3D), "no 3D support");
    }

    #[test]
    fn test_bass_error_description_no_eax() {
        assert_eq!(bass_error_description(BASS_ERROR_NOEAX), "no EAX support");
    }

    #[test]
    fn test_bass_error_description_device() {
        assert_eq!(bass_error_description(BASS_ERROR_DEVICE), "illegal device number");
    }

    #[test]
    fn test_bass_error_description_no_play() {
        assert_eq!(bass_error_description(BASS_ERROR_NOPLAY), "not playing");
    }

    #[test]
    fn test_bass_error_description_freq() {
        assert_eq!(bass_error_description(BASS_ERROR_FREQ), "illegal sample rate");
    }

    #[test]
    fn test_bass_error_description_not_file() {
        assert_eq!(bass_error_description(BASS_ERROR_NOTFILE), "the stream is not a file stream");
    }

    #[test]
    fn test_bass_error_description_no_hw() {
        assert_eq!(bass_error_description(BASS_ERROR_NOHW), "no hardware voices available");
    }

    #[test]
    fn test_bass_error_description_empty() {
        assert_eq!(bass_error_description(BASS_ERROR_EMPTY), "no data");
    }

    #[test]
    fn test_bass_error_description_no_net() {
        assert_eq!(bass_error_description(BASS_ERROR_NONET), "internet connection error");
    }

    #[test]
    fn test_bass_error_description_create() {
        assert_eq!(bass_error_description(BASS_ERROR_CREATE), "couldn't create the file");
    }

    #[test]
    fn test_bass_error_description_no_fx() {
        assert_eq!(bass_error_description(BASS_ERROR_NOFX), "no FX available");
    }

    #[test]
    fn test_bass_error_description_not_avail() {
        assert_eq!(bass_error_description(BASS_ERROR_NOTAVAIL), "requested data is not available");
    }

    #[test]
    fn test_bass_error_description_decode() {
        assert_eq!(bass_error_description(BASS_ERROR_DECODE), "the channel is a 'decoding channel'");
    }

    #[test]
    fn test_bass_error_description_dx() {
        assert_eq!(bass_error_description(BASS_ERROR_DX), "a sufficient DirectX version is not installed");
    }

    #[test]
    fn test_bass_error_description_timeout() {
        assert_eq!(bass_error_description(BASS_ERROR_TIMEOUT), "connection timed out");
    }

    #[test]
    fn test_bass_error_description_file_form() {
        assert_eq!(bass_error_description(BASS_ERROR_FILEFORM), "unsupported file format");
    }

    #[test]
    fn test_bass_error_description_speaker() {
        assert_eq!(bass_error_description(BASS_ERROR_SPEAKER), "unavailable speaker");
    }

    #[test]
    fn test_bass_error_description_version() {
        assert_eq!(bass_error_description(BASS_ERROR_VERSION), "invalid BASS version");
    }

    #[test]
    fn test_bass_error_description_codec() {
        assert_eq!(bass_error_description(BASS_ERROR_CODEC), "codec is not available/supported");
    }

    #[test]
    fn test_bass_error_description_ended() {
        assert_eq!(bass_error_description(BASS_ERROR_ENDED), "the channel/file has ended");
    }

    #[test]
    fn test_bass_error_description_busy() {
        assert_eq!(bass_error_description(BASS_ERROR_BUSY), "the device is busy");
    }

    #[test]
    fn test_bass_error_description_unknown() {
        assert_eq!(bass_error_description(BASS_ERROR_UNKNOWN), "unknown error");
    }

    #[test]
    fn test_bass_error_description_unknown_code() {
        // Codes that don't have a specific mapping should return "unknown error code"
        assert_eq!(bass_error_description(999), "unknown error code");
        assert_eq!(bass_error_description(-100), "unknown error code");
        assert_eq!(bass_error_description(0), "unknown error code");
    }

    #[test]
    fn test_bass_error_description_ok() {
        assert_eq!(bass_error_description(BASS_OK), "unknown error code");
    }
}
