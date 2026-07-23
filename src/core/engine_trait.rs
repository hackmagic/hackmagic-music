//! Player engine trait - abstraction for different backends (BASS, MCI, FFMPEG)
//! Mirrors the original `IPlayerCore` interface.
#![allow(dead_code)]

use crate::error::Result;
use std::time::Duration;

/// Engine types, matching original `PlayerCoreType`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Bass,
    Mci,
    Ffmpeg,
    Rodio,
    Symphonia,
}

impl EngineType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mci" => EngineType::Mci,
            "ffmpeg" => EngineType::Ffmpeg,
            "rodio" => EngineType::Rodio,
            "symphonia" => EngineType::Symphonia,
            _ => EngineType::Bass,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_str(&self) -> &'static str {
        match self {
            EngineType::Bass => "bass",
            EngineType::Mci => "mci",
            EngineType::Ffmpeg => "ffmpeg",
            EngineType::Rodio => "rodio",
            EngineType::Symphonia => "symphonia",
        }
    }
}

/// Playback state, matching original `PlayingState`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    Stopped,
    Playing,
    Paused,
}

/// Lock-free snapshot of player state published by `ArcSwap<EngineStatus>`.
/// GUI reads this in `render()` without any Mutex contention.
#[derive(Clone)]
pub struct EngineStatus {
    pub state: EngineState,
    pub position_secs: f64,
    pub duration_secs: f64,
    pub volume: u32,
    pub speed: f32,
    pub song_is_over: bool,
    pub loading: bool,
    pub engine_name: &'static str,
    pub current_track_index: Option<usize>,
    pub current_track_path: String,
    pub current_track_title: String,
    pub current_track_artist: String,
    pub current_track_album: String,
    pub current_track_is_favourite: bool,
    pub spectrum: Vec<f32>,
    pub spectrum_peaks: Vec<f32>,
    pub fft: Vec<f32>,
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self {
            state: EngineState::Stopped,
            position_secs: 0.0,
            duration_secs: 0.0,
            volume: 80,
            speed: 1.0,
            song_is_over: false,
            loading: false,
            engine_name: "",
            current_track_index: None,
            current_track_path: String::new(),
            current_track_title: String::new(),
            current_track_artist: String::new(),
            current_track_album: String::new(),
            current_track_is_favourite: false,
            spectrum: Vec::new(),
            spectrum_peaks: Vec::new(),
            fft: Vec::new(),
        }
    }
}

/// Engine capabilities — tells the UI which features are actually functional.
/// Some engines (MCI, FFmpeg) don't support speed/pitch/EQ/reverb/FFT.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCapabilities {
    /// Variable-speed playback
    pub speed: bool,
    /// Pitch shifting
    pub pitch: bool,
    /// 10-band equalizer
    pub equalizer: bool,
    /// Reverb effect
    pub reverb: bool,
    /// FFT spectrum data for visualization
    pub fft: bool,
    /// Gapless playback (preload next track)
    pub gapless: bool,
}

impl EngineCapabilities {
    /// Full capabilities (BASS engine).
    pub const fn all() -> Self {
        Self { speed: true, pitch: true, equalizer: true, reverb: true, fft: true, gapless: true }
    }
    /// Minimal capabilities (MCI, FFmpeg).
    pub const fn minimal() -> Self {
        Self { speed: false, pitch: false, equalizer: false, reverb: false, fft: false, gapless: false }
    }
    /// Decode-only engines (Rodio, Symphonia) — support FFT and speed, but not EQ/reverb/pitch/gapless.
    pub const fn decode_only() -> Self {
        Self { speed: true, pitch: false, equalizer: false, reverb: false, fft: true, gapless: false }
    }
}

/// The abstract player engine interface (mirrors `IPlayerCore`)
pub trait PlayerEngine: Send + Sync {
    /// Engine display name
    fn name(&self) -> &'static str;

    /// Return the capabilities this engine supports.
    fn capabilities(&self) -> EngineCapabilities { EngineCapabilities::minimal() }

    /// Initialize the engine
    fn init(&self) -> Result<()>;

    /// Uninitialize the engine
    fn uninit(&self) -> Result<()>;

    /// Open an audio file
    fn open(&self, path: &str) -> Result<()>;

    /// Close current stream
    fn close(&self) -> Result<()>;

    /// Start playback
    fn play(&self) -> Result<()>;

    /// Toggle pause
    fn pause(&self) -> Result<()>;

    /// Stop playback
    fn stop(&self) -> Result<()>;

    /// Get current state
    fn state(&self) -> EngineState;

    /// Get current playback position
    fn position(&self) -> Duration;

    /// Get total duration of current track
    fn duration(&self) -> Duration;

    /// Seek to position
    fn seek(&self, pos: Duration) -> Result<()>;

    /// Set volume (0-100)
    fn set_volume(&self, vol: u32) -> Result<()>;

    /// Get volume (0-100)
    fn volume(&self) -> u32;

    /// Set playback speed (0.1 - 4.0)
    fn set_speed(&self, speed: f32) -> Result<()>;

    /// Get playback speed
    fn speed(&self) -> f32;

    /// Set pitch shift (-12 to 12 semitones)
    fn set_pitch(&self, pitch: i32) -> Result<()>;

    /// Get pitch shift
    fn pitch(&self) -> i32;

    /// Set equalizer band gain
    fn set_equalizer(&self, band: usize, gain: i32) -> Result<()>;

    /// Get all equalizer gains
    fn equalizer(&self) -> [i32; 10];

    /// Set reverb (mix 0-100, time 1-300)
    fn set_reverb(&self, mix: u32, time: u32) -> Result<()>;

    /// Clear reverb
    fn clear_reverb(&self) -> Result<()>;

    /// Get FFT spectrum data
    fn fft_data(&self) -> Vec<f32>;
    fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32>;

    /// Check if current song has ended
    fn song_is_over(&self) -> bool;

    /// Check if current file is MIDI
    fn is_midi(&self) -> bool;

    /// Set `ReplayGain` dB adjustment (default no-op)
    fn set_replaygain(&self, _gain_db: f32) {}

    /// Crossfade: slide current track volume to 0 over `time_ms`, then stop (default no-op)
    fn crossfade_out(&self, _time_ms: u32) {}

    /// Preload the next track (mmap + stream) for gapless transition (default no-op).
    fn preload_next(&self, _path: &str) {}
}
