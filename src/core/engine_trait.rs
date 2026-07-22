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
}

impl EngineType {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "mci" => EngineType::Mci,
            "ffmpeg" => EngineType::Ffmpeg,
            _ => EngineType::Bass,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_str(&self) -> &'static str {
        match self {
            EngineType::Bass => "bass",
            EngineType::Mci => "mci",
            EngineType::Ffmpeg => "ffmpeg",
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

/// The abstract player engine interface (mirrors `IPlayerCore`)
pub trait PlayerEngine: Send + Sync {
    /// Engine display name
    fn name(&self) -> &'static str;

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
