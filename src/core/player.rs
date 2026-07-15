//! Core player - the central state machine that manages playback, playlist, and effects.
//! Mirrors the original `CPlayer` class.
#![allow(dead_code)]

use crate::bass::engine::BassEngine;
use crate::core::engine_trait::{EngineState, EngineType, PlayerEngine};
use crate::core::playlist::{Playlist, RepeatMode, Track};
use crate::error::{PlayerError, Result};
use crate::ffmpeg_engine::FfmpegEngine;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::Duration;
use tracing;

/// Command enum matching original
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackCommand {
    Play,
    Pause,
    Stop,
    Next,
    Prev,
    Open,
    Close,
}

/// AB Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ABRepeatMode {
    None,
    ASelected,
    ABRepeat,
}

/// A repeat point
#[derive(Debug, Clone, Copy)]
pub struct ABRepeatPoint {
    pub a: Duration,
    pub b: Duration,
    pub mode: ABRepeatMode,
}

/// Default FFT analysis data (overridden by config at runtime)
const FFT_SAMPLE_DEFAULT: usize = 512;
const SPECTRUM_COL_DEFAULT: usize = FFT_SAMPLE_DEFAULT / 4;

/// The main player controller (singleton, like original `CPlayer`)
pub struct Player {
    /// Audio engine
    engine: Box<dyn PlayerEngine>,
    /// Engine type
    engine_type: EngineType,
    /// Current playlist
    playlist: Mutex<Playlist>,
    /// Equalizer gains [10 bands]
    eq_gains: Mutex<[i32; 10]>,
    /// Equalizer enabled
    eq_enabled: Mutex<bool>,
    /// Reverb mix (0-100) and time (1-300)
    reverb_mix: Mutex<u32>,
    reverb_time: Mutex<u32>,
    /// Reverb enabled
    reverb_enabled: Mutex<bool>,
    /// AB repeat state
    ab_repeat: Mutex<ABRepeatPoint>,
    /// Spectrum config
    spectrum_columns: Mutex<usize>,
    fft_size: Mutex<usize>,
    /// FFT data
    fft_data: Mutex<[f32; FFT_SAMPLE_DEFAULT]>,
    /// Peak values for spectrum (decay)
    spectrum_peaks: Mutex<Vec<f32>>,
    /// Display format
    display_format: Mutex<String>,
    /// Loading flag
    pub loading: AtomicBool,
    /// Volume map (percentage mapping)
    volume_map: Mutex<u32>,
    /// CUE track end position (None if not a CUE track)
    cue_end_pos: Mutex<Option<Duration>>,
}

unsafe impl Send for Player {}
unsafe impl Sync for Player {}

impl Player {
    /// Create a new player with the specified engine
    pub fn new(engine_type: EngineType) -> Self {
        let engine: Box<dyn PlayerEngine> = match engine_type {
            EngineType::Bass => Box::new(BassEngine::new()),
            EngineType::Mci => Box::new(BassEngine::new()), // Placeholder
            EngineType::Ffmpeg => Box::new(FfmpegEngine::new()),
        };

        Self {
            engine,
            engine_type,
            playlist: Mutex::new(Playlist::new("default")),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        }
    }

    /// Initialize the player
    pub fn init(&self) -> Result<()> {
        self.engine.init()?;
        tracing::info!("Player initialized with {} engine", self.engine.name());
        Ok(())
    }

    // === Engine info ===

    pub fn engine_name(&self) -> &'static str {
        self.engine.name()
    }

    pub fn engine_type(&self) -> EngineType {
        self.engine_type
    }

    // === Playback control ===

    /// Open and play a file
    pub fn play_file(&self, path: &str) -> Result<()> {
        // Add to playlist so frontend status/playlist API can see it
        {
            let mut pl = self.playlist.lock().unwrap();
            pl.add_track(Track::new(path));
        }
        let idx = {
            let pl = self.playlist.lock().unwrap();
            pl.len() - 1
        };
        // play_at_index opens the engine and plays
        self.play_at_index(idx)
    }

    /// Toggle pause
    pub fn toggle_pause(&self) -> Result<()> {
        self.engine.pause()
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        self.engine.stop()
    }

    /// Seek to position
    pub fn seek(&self, pos: Duration) -> Result<()> {
        self.engine.seek(pos)
    }

    /// Seek to percentage (0.0 - 1.0)
    pub fn seek_percent(&self, pct: f64) -> Result<()> {
        let dur = self.engine.duration();
        if dur == Duration::ZERO {
            return Ok(());
        }
        let pos = Duration::from_secs_f64(dur.as_secs_f64() * pct.clamp(0.0, 1.0));
        self.engine.seek(pos)
    }

    /// Play next track
    pub fn next(&self) -> Result<()> {
        let mut playlist = self.playlist.lock().unwrap();
        if playlist.is_empty() {
            return Ok(());
        }

        // Check "play next" queue
        if let Some(idx) = playlist.drain_next_track() {
            let path = playlist.get(idx).map(|t| t.file_path.clone());
            drop(playlist);
            if let Some(p) = path {
                self.play_file(&p)?;
            }
            return Ok(());
        }

        let next_idx = playlist.next_index();
        if let Some(idx) = next_idx {
            let path = playlist.get(idx).map(|t| t.file_path.clone());
            playlist.set_current(idx).ok();
            drop(playlist);
            if let Some(p) = path {
                self.play_file(&p)?;
            }
        }
        Ok(())
    }

    /// Add track index to "play next" queue
    pub fn push_next_track(&self, index: usize) {
        let mut playlist = self.playlist.lock().unwrap();
        playlist.push_next_track(index);
    }

    /// Play previous track
    pub fn prev(&self) -> Result<()> {
        let mut playlist = self.playlist.lock().unwrap();
        if playlist.is_empty() {
            return Ok(());
        }
        let prev_idx = playlist.prev_index();
        if let Some(idx) = prev_idx {
            let path = playlist.get(idx).map(|t| t.file_path.clone());
            playlist.set_current(idx).ok();
            drop(playlist);
            if let Some(p) = path {
                self.play_file(&p)?;
            }
        }
        Ok(())
    }

    /// Play track at index
    pub fn play_at_index(&self, index: usize) -> Result<()> {
        let playlist = self.playlist.lock().unwrap();
        let (path, is_cue, start_pos, end_pos, track_gain, album_gain) = playlist.get(index).map(|t| {
            (t.file_path.clone(), t.is_cue, t.start_pos, t.end_pos, t.track_gain, t.album_gain)
        }).unwrap_or_default();
        drop(playlist);
        if path.is_empty() {
            return Err(PlayerError::NoTrack);
        }
        self.engine.open(&path)?;
        // Apply ReplayGain
        let cfg = crate::config::Config::load();
        let gain_db = match cfg.play.replaygain.as_str() {
            "track" if track_gain != 0.0 => track_gain,
            "album" if album_gain != 0.0 => album_gain,
            _ => 0.0,
        };
        self.engine.set_replaygain(gain_db);
        if is_cue && start_pos != Duration::ZERO {
            self.engine.seek(start_pos)?;
        }
        *self.cue_end_pos.lock().unwrap() = if is_cue && end_pos != Duration::ZERO { Some(end_pos) } else { None };
        self.engine.play()?;
        let mut playlist = self.playlist.lock().unwrap();
        playlist.set_current(index).ok();
        Ok(())
    }

    // === Volume ===

    pub fn set_volume(&self, vol: u32) -> Result<()> {
        let vol = vol.min(100);
        let mapped = self.apply_volume_map(vol);
        self.engine.set_volume(mapped)
    }

    pub fn volume(&self) -> u32 {
        self.engine.volume()
    }

    pub fn volume_up(&self, step: u32) -> Result<()> {
        let vol = (self.engine.volume() + step).min(100);
        self.set_volume(vol)
    }

    pub fn volume_down(&self, step: u32) -> Result<()> {
        let vol = self.engine.volume().saturating_sub(step);
        self.set_volume(vol)
    }

    pub fn set_volume_map(&self, map: u32) {
        *self.volume_map.lock().unwrap() = map.min(100);
    }

    fn apply_volume_map(&self, vol: u32) -> u32 {
        let map = *self.volume_map.lock().unwrap();
        if map >= 100 {
            vol
        } else {
            (vol * map) / 100
        }
    }

    // === Speed ===

    pub fn speed(&self) -> f32 {
        self.engine.speed()
    }

    pub fn set_speed(&self, speed: f32) -> Result<()> {
        self.engine.set_speed(speed)
    }

    pub fn speed_up(&self) -> Result<()> {
        let s = self.engine.speed();
        let new_s = (s * 1.1).min(4.0);
        self.engine.set_speed(new_s)
    }

    pub fn speed_down(&self) -> Result<()> {
        let s = self.engine.speed();
        let new_s = (s / 1.1).max(0.1);
        self.engine.set_speed(new_s)
    }

    pub fn reset_speed(&self) -> Result<()> {
        self.engine.set_speed(1.0)
    }

    // === Pitch ===

    pub fn pitch(&self) -> i32 {
        self.engine.pitch()
    }

    pub fn set_pitch(&self, pitch: i32) -> Result<()> {
        self.engine.set_pitch(pitch)
    }

    pub fn pitch_up(&self) -> Result<()> {
        let p = self.engine.pitch();
        self.engine.set_pitch((p + 1).min(12))
    }

    pub fn pitch_down(&self) -> Result<()> {
        let p = self.engine.pitch();
        self.engine.set_pitch((p - 1).max(-12))
    }

    pub fn reset_pitch(&self) -> Result<()> {
        self.engine.set_pitch(0)
    }

    // === Equalizer ===

    pub fn eq_set(&self, band: usize, gain: i32) -> Result<()> {
        let gain = gain.clamp(-15, 15);
        self.eq_gains.lock().unwrap()[band] = gain;
        self.engine.set_equalizer(band, gain)
    }

    pub fn eq_get(&self) -> [i32; 10] {
        *self.eq_gains.lock().unwrap()
    }

    pub fn eq_get_band(&self, band: usize) -> i32 {
        self.eq_gains.lock().unwrap()[band]
    }

    pub fn eq_set_preset(&self, preset: &str) -> Result<()> {
        let gains = match preset.to_lowercase().as_str() {
            "none" | "无" => [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            "classical" | "古典" => [4, 3, 3, 2, 2, 1, 0, -1, -2, -2],
            "pop" | "流行" => [3, 2, 0, -1, -2, -2, -1, 0, 2, 3],
            "jazz" | "爵士" => [2, 1, 0, -1, -1, 1, 3, 5, 5, 3],
            "rock" | "摇滚" => [-2, 0, 2, 4, -1, -1, 0, 0, 2, 3],
            "soft" | "柔和" => [1, 0, 0, 1, 2, 1, -1, -2, -2, -2],
            "bass" | "重低音" => [4, 6, 6, -2, -1, 0, 0, 0, 0, 0],
            "nobass" | "消除低音" => [-5, -5, -3, -2, -2, 0, 0, 0, 0, 0],
            "nohigh" | "弱化高音" => [0, 0, 0, 0, 0, -1, -3, -5, -5, -4],
            _ => return Ok(()),
        };
        *self.eq_gains.lock().unwrap() = gains;
        for (band, &gain) in gains.iter().enumerate() {
            self.engine.set_equalizer(band, gain)?;
        }
        Ok(())
    }

    pub fn eq_reset(&self) -> Result<()> {
        *self.eq_gains.lock().unwrap() = [0; 10];
        for band in 0..10 {
            self.engine.set_equalizer(band, 0)?;
        }
        Ok(())
    }

    pub fn eq_enable(&self, enable: bool) {
        *self.eq_enabled.lock().unwrap() = enable;
    }

    pub fn eq_is_enabled(&self) -> bool {
        *self.eq_enabled.lock().unwrap()
    }

    // === Reverb ===

    pub fn reverb_set(&self, mix: u32, time: u32) -> Result<()> {
        *self.reverb_mix.lock().unwrap() = mix;
        *self.reverb_time.lock().unwrap() = time;
        *self.reverb_enabled.lock().unwrap() = true;
        self.engine.set_reverb(mix, time)
    }

    pub fn reverb_get(&self) -> (u32, u32) {
        (*self.reverb_mix.lock().unwrap(), *self.reverb_time.lock().unwrap())
    }

    pub fn reverb_is_enabled(&self) -> bool {
        *self.reverb_enabled.lock().unwrap()
    }

    pub fn reverb_clear(&self) -> Result<()> {
        *self.reverb_enabled.lock().unwrap() = false;
        self.engine.clear_reverb()
    }

    // === Audio Output Device ===

    /// List available audio output devices
    #[cfg(windows)]
    pub fn list_audio_devices(&self) -> Vec<(i32, String)> {
        crate::bass::engine::BassEngine::list_devices()
    }

    /// List available audio output devices (non-Windows stub)
    #[cfg(not(windows))]
    pub fn list_audio_devices(&self) -> Vec<(i32, String)> {
        Vec::new()
    }

    // === AB Repeat ===

    pub fn ab_set_a(&self) -> Result<()> {
        let pos = self.engine.position();
        let mut ab = self.ab_repeat.lock().unwrap();
        ab.a = pos;
        ab.mode = ABRepeatMode::ASelected;
        tracing::info!("AB repeat A point set: {:.1}s", pos.as_secs_f64());
        Ok(())
    }

    pub fn ab_set_b(&self) -> Result<()> {
        let pos = self.engine.position();
        let mut ab = self.ab_repeat.lock().unwrap();
        ab.b = pos;
        if ab.a < ab.b {
            ab.mode = ABRepeatMode::ABRepeat;
            tracing::info!("AB repeat: {:.1}s -> {:.1}s", ab.a.as_secs_f64(), ab.b.as_secs_f64());
        } else {
            tracing::warn!("B point must be after A point");
        }
        Ok(())
    }

    pub fn ab_reset(&self) {
        let mut ab = self.ab_repeat.lock().unwrap();
        ab.mode = ABRepeatMode::None;
        tracing::info!("AB repeat cleared");
    }

    pub fn ab_continue(&self) -> Result<()> {
        // Set next AB repeat start to current B point
        let mut ab = self.ab_repeat.lock().unwrap();
        ab.a = ab.b;
        ab.mode = ABRepeatMode::ASelected;
        Ok(())
    }

    pub fn ab_status(&self) -> ABRepeatPoint {
        *self.ab_repeat.lock().unwrap()
    }

    // === Playlist ===

    pub fn playlist_mut(&self) -> std::sync::MutexGuard<'_, Playlist> {
        self.playlist.lock().unwrap()
    }

    pub fn playlist(&self) -> Playlist {
        self.playlist.lock().unwrap().clone()
    }

    // === Repeat mode ===

    pub fn repeat_mode(&self) -> RepeatMode {
        self.playlist.lock().unwrap().repeat_mode()
    }

    pub fn set_repeat_mode(&self, mode: RepeatMode) {
        self.playlist.lock().unwrap().set_repeat_mode(mode);
    }

    // === State ===

    pub fn state(&self) -> EngineState {
        self.engine.state()
    }

    pub fn position(&self) -> Duration {
        self.engine.position()
    }

    pub fn duration(&self) -> Duration {
        self.engine.duration()
    }

    pub fn is_playing(&self) -> bool {
        self.engine.state() == EngineState::Playing
    }

    pub fn is_paused(&self) -> bool {
        self.engine.state() == EngineState::Paused
    }

    pub fn is_stopped(&self) -> bool {
        self.engine.state() == EngineState::Stopped
    }

    pub fn song_is_over(&self) -> bool {
        // Check end of CUE track boundary
        if let Some(end_pos) = *self.cue_end_pos.lock().unwrap() {
            let pos = self.engine.position();
            if pos >= end_pos {
                return true;
            }
        }
        self.engine.song_is_over()
    }

    // === Spectrum Config ===

    pub fn set_spectrum_config(&self, columns: usize, fft_size: usize) {
        *self.spectrum_columns.lock().unwrap() = columns.clamp(4, 128);
        *self.fft_size.lock().unwrap() = fft_size.clamp(256, 2048);
    }

    pub fn spectrum_config(&self) -> (usize, usize) {
        (*self.spectrum_columns.lock().unwrap(), *self.fft_size.lock().unwrap())
    }

    // === FFT / Spectrum ===

    pub fn fft_data(&self) -> Vec<f32> {
        let fft_size = *self.fft_size.lock().unwrap() as u32;
        self.engine.fft_data_with_size(fft_size)
    }

    /// Calculate spectrum column data for visualization (with peak tracking)
    pub fn calculate_spectrum(&self) -> Vec<f32> {
        let fft_size = *self.fft_size.lock().unwrap();
        let col = *self.spectrum_columns.lock().unwrap();
        let fft = self.engine.fft_data_with_size(fft_size as u32);
        let mut spectrum = vec![0.0f32; col];

        if fft.len() < 2 {
            return spectrum;
        }

        // Map FFT bins to spectrum columns (logarithmic or linear)
        let cfg = crate::config::Config::load();
        let use_log = cfg.appearance.spectrum_style != "linear";
        #[allow(clippy::needless_range_loop)]
        for i in 0..col {
            let (low_freq, high_freq) = if use_log {
                ((i as f32 / col as f32).powf(2.0) * fft.len() as f32,
                 ((i + 1) as f32 / col as f32).powf(2.0) * fft.len() as f32)
            } else {
                (i as f32 / col as f32 * fft.len() as f32,
                 (i + 1) as f32 / col as f32 * fft.len() as f32)
            };
            let low = low_freq as usize;
            let high = (high_freq as usize).min(fft.len());

            if high > low {
                let sum: f32 = fft[low..high].iter().sum();
                spectrum[i] = (sum / (high - low) as f32).sqrt();
            }
        }

        // Peak tracking with decay
        let mut peaks = self.spectrum_peaks.lock().unwrap();
        if peaks.len() != col {
            *peaks = vec![0.0f32; col];
        }
        const PEAK_DECAY: f32 = 0.92;
        for i in 0..col {
            if spectrum[i] > peaks[i] {
                peaks[i] = spectrum[i];
            } else {
                peaks[i] *= PEAK_DECAY;
            }
        }

        spectrum
    }

    /// Get peak values from spectrum (for visualization)
    pub fn spectrum_peak_data(&self) -> Vec<f32> {
        self.spectrum_peaks.lock().unwrap().clone()
    }

    // === Display format ===

    pub fn display_format(&self) -> String {
        self.display_format.lock().unwrap().clone()
    }

    pub fn set_display_format(&self, fmt: String) {
        *self.display_format.lock().unwrap() = fmt;
    }

    // ========== Multi-playlist management ==========

    /// Save current playlist to disk as `.playlist` file
    pub fn save_current_playlist(&self) -> Result<()> {
        let pl = self.playlist.lock().unwrap();
        let pl_dir = crate::config::get_config_dir().join("playlists");
        std::fs::create_dir_all(&pl_dir)?;
        let path = pl_dir.join(format!("{}.playlist", pl.name()));
        drop(pl); // release lock before write
        crate::playlist_format::write_playlist(
            &path.to_string_lossy(), self.playlist.lock().unwrap().tracks(), None)
    }

    /// Create a new empty playlist and switch to it (saves current first)
    pub fn create_playlist(&self, name: &str) -> Result<()> {
        let pl_dir = crate::config::get_config_dir().join("playlists");
        std::fs::create_dir_all(&pl_dir)?;
        // Save current playlist first
        self.save_current_playlist()?;
        // Create new empty playlist
        let mut pl = self.playlist.lock().unwrap();
        *pl = Playlist::new(name);
        Ok(())
    }

    /// Switch to an existing playlist by name (saves current first)
    pub fn switch_playlist(&self, name: &str) -> Result<()> {
        let pl_dir = crate::config::get_config_dir().join("playlists");
        let path = pl_dir.join(format!("{name}.playlist"));
        if !path.exists() {
            return Err(PlayerError::Other(format!("Playlist '{name}' not found")));
        }
        // Save current
        self.save_current_playlist()?;
        // Load new
        let tracks = crate::playlist_format::read_playlist(&path.to_string_lossy())?;
        let mut pl = self.playlist.lock().unwrap();
        *pl = Playlist::new(name);
        for track_path in tracks {
            let track = crate::tag::reader::read_tags(&track_path)
                .unwrap_or_else(|_| Track::new(&track_path));
            pl.add_track(track);
        }
        Ok(())
    }

    /// List all available playlists with track counts
    pub fn list_playlists(&self) -> Vec<(String, usize)> {
        let pl_dir = crate::config::get_config_dir().join("playlists");
        let mut result: Vec<(String, usize)> = Vec::new();

        if pl_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&pl_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension()
                        .is_some_and(|e| e == "playlist" || e == "m3u" || e == "m3u8")
                    {
                        let name = path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let count = crate::playlist_format::read_playlist(
                            &path.to_string_lossy()
                        ).map(|t| t.len()).unwrap_or(0);
                        result.push((name, count));
}
                }
            }
        }

        // Ensure active playlist is always present
        let active_name = self.playlist.lock().unwrap().name().to_string();
        if !result.iter().any(|(n, _)| n == &active_name) {
            result.push((active_name, self.playlist.lock().unwrap().len()));
        }

        result.sort_by(|a, b| a.0.cmp(&b.0));
        result
    }

    /// Rename a playlist on disk
    pub fn rename_playlist(&self, old_name: &str, new_name: &str) -> Result<()> {
        if old_name == "default" {
            return Err(PlayerError::Other("Cannot rename default playlist".into()));
        }
        let pl_dir = crate::config::get_config_dir().join("playlists");
        let old_path = pl_dir.join(format!("{old_name}.playlist"));
        let new_path = pl_dir.join(format!("{new_name}.playlist"));
        if !old_path.exists() {
            return Err(PlayerError::Other(format!("Playlist '{old_name}' not found")));
        }
        if new_path.exists() {
            return Err(PlayerError::Other(format!("Playlist '{new_name}' already exists")));
        }
        std::fs::rename(&old_path, &new_path)?;
        // Update active playlist name if it's the one being renamed
        let mut pl = self.playlist.lock().unwrap();
        if pl.name() == old_name {
            pl.set_name(new_name.to_string());
        }
        Ok(())
    }

    /// Delete a playlist from disk
    pub fn delete_playlist(&self, name: &str) -> Result<()> {
        if name == "default" {
            return Err(PlayerError::Other("Cannot delete default playlist".into()));
        }
        let pl_dir = crate::config::get_config_dir().join("playlists");
        let path = pl_dir.join(format!("{name}.playlist"));
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Get the name of the active playlist
    pub fn active_playlist_name(&self) -> String {
        self.playlist.lock().unwrap().name().to_string()
    }

    // ========== Playback state persistence ==========

    /// Save current playback state to disk
    pub fn save_playback_state(&self) {
        use crate::config::PlaybackState;
        let pl = self.playlist.lock().unwrap();
        let state = PlaybackState {
            last_playlist: pl.name().to_string(),
            track_index: pl.current_index().unwrap_or(0),
            position_secs: self.engine.position().as_secs_f64(),
            volume: self.volume(),
            repeat_mode: pl.repeat_mode().to_str().to_string(),
        };
        drop(pl);
        state.save();
    }

    /// Restore playback state: switch to the saved playlist and seek to saved position.
    /// Returns `true` if state was restored, `false` if state was empty/default.
    pub fn restore_playback_state(&self) -> bool {
        use crate::config::PlaybackState;
        let state = PlaybackState::load();
        if state.volume == 80 && state.track_index == 0 && state.position_secs == 0.0
            && state.last_playlist == "default"
        {
            return false; // no meaningful state
        }

        // Switch to the saved playlist if it exists
        let pl_dir = crate::config::get_config_dir().join("playlists");
        let pl_path = pl_dir.join(format!("{}.playlist", state.last_playlist));
        if pl_path.exists() {
            if let Err(e) = self.switch_playlist(&state.last_playlist) {
                tracing::warn!("Failed to switch to saved playlist '{}': {}", state.last_playlist, e);
            }
        }

        // Restore volume
        self.set_volume(state.volume).ok();

        // Restore repeat mode
        let mode = crate::core::playlist::RepeatMode::from_str(&state.repeat_mode);
        self.set_repeat_mode(mode);

        // Play at saved index and seek to saved position
        let idx = state.track_index;
        {
            let pl = self.playlist.lock().unwrap();
            if idx >= pl.len() {
                return false; // index out of range
            }
        }
        if let Err(e) = self.play_at_index(idx) {
            tracing::warn!("Failed to play saved track #{}: {}", idx, e);
            return false;
        }
        if state.position_secs > 1.0 {
            let seek_pos = Duration::from_secs_f64(state.position_secs);
            if let Err(e) = self.seek(seek_pos) {
                tracing::warn!("Failed to seek to {:.1}s: {}", state.position_secs, e);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A mock engine that tracks calls without real audio hardware.
    struct MockEngine {
        opened_paths: Arc<Mutex<Vec<String>>>,
        play_called: Arc<AtomicBool>,
    }

    impl MockEngine {
        fn new() -> Self {
            MockEngine {
                opened_paths: Arc::new(Mutex::new(Vec::new())),
                play_called: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    impl PlayerEngine for MockEngine {
        fn name(&self) -> &'static str { "mock" }
        fn init(&self) -> Result<()> { Ok(()) }
        fn uninit(&self) -> Result<()> { Ok(()) }
        fn open(&self, path: &str) -> Result<()> {
            self.opened_paths.lock().unwrap().push(path.to_string());
            Ok(())
        }
        fn close(&self) -> Result<()> { Ok(()) }
        fn play(&self) -> Result<()> {
            self.play_called.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn pause(&self) -> Result<()> { Ok(()) }
        fn stop(&self) -> Result<()> { Ok(()) }
        fn state(&self) -> EngineState { EngineState::Stopped }
        fn position(&self) -> Duration { Duration::ZERO }
        fn duration(&self) -> Duration { Duration::ZERO }
        fn seek(&self, _pos: Duration) -> Result<()> { Ok(()) }
        fn set_volume(&self, _vol: u32) -> Result<()> { Ok(()) }
        fn volume(&self) -> u32 { 80 }
        fn set_speed(&self, _speed: f32) -> Result<()> { Ok(()) }
        fn speed(&self) -> f32 { 1.0 }
        fn set_pitch(&self, _pitch: i32) -> Result<()> { Ok(()) }
        fn pitch(&self) -> i32 { 0 }
        fn set_equalizer(&self, _band: usize, _gain: i32) -> Result<()> { Ok(()) }
        fn equalizer(&self) -> [i32; 10] { [0; 10] }
        fn set_reverb(&self, _mix: u32, _time: u32) -> Result<()> { Ok(()) }
        fn clear_reverb(&self) -> Result<()> { Ok(()) }
        fn fft_data(&self) -> Vec<f32> { vec![] }
        fn fft_data_with_size(&self, _fft_size: u32) -> Vec<f32> { vec![] }
        fn song_is_over(&self) -> bool { false }
        fn is_midi(&self) -> bool { false }
    }

    /// Helper: create a Player with a MockEngine, returning the Player and shared state.
    fn mock_player(tracks: &[&str]) -> (Player, Arc<Mutex<Vec<String>>>, Arc<AtomicBool>) {
        let engine = MockEngine::new();
        let opened_paths = engine.opened_paths.clone();
        let play_called = engine.play_called.clone();
        let mut playlist = Playlist::new("test");
        for (i, t) in tracks.iter().enumerate() {
            playlist.add_track(Track::new(t));
            if i == 0 {
                playlist.set_current(0).ok();
            }
        }
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        (player, opened_paths, play_called)
    }

    #[test]
    fn test_next_empty_playlist() {
        let (player, _, _) = mock_player(&[]);
        assert!(player.next().is_ok());
    }

    #[test]
    fn test_next_normal_advances_to_next_track() {
        let (player, opened, play_called) = mock_player(&["/music/track1.mp3", "/music/track2.mp3"]);
        // Current index is 0, next should advance to 1
        player.next().unwrap();
        assert!(play_called.load(Ordering::SeqCst));
        let opened_paths = opened.lock().unwrap();
        assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/track2.mp3"));
    }

    #[test]
    fn test_next_uses_queue_before_normal_next() {
        let (player, opened, play_called) = mock_player(&["/music/a.mp3", "/music/b.mp3", "/music/c.mp3"]);
        // Push track #2 into the "play next" queue
        player.push_next_track(2);
        // next() should play #2 (from queue) instead of #1 (normal next)
        player.next().unwrap();
        assert!(play_called.load(Ordering::SeqCst));
        let opened_paths = opened.lock().unwrap();
        assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/c.mp3"));
    }

    #[test]
    fn test_next_loop_playlist_wraps_around() {
        let (player, opened, _play_called) = mock_player(&["/music/x.mp3", "/music/y.mp3"]);
        // Set repeat mode to LoopPlaylist
        player.set_repeat_mode(RepeatMode::LoopPlaylist);
        // Current index = 0, next should go to 1
        player.next().unwrap();
        {
            let opened_paths = opened.lock().unwrap();
            assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/y.mp3"));
        }
        // From index 1, next should wrap back to 0
        player.next().unwrap();
        {
            let opened_paths = opened.lock().unwrap();
            assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/x.mp3"));
        }
    }

    #[test]
    fn test_push_next_track_and_drain() {
        let (player, opened, _) = mock_player(&["/music/p1.mp3", "/music/p2.mp3", "/music/p3.mp3"]);
        player.push_next_track(2);
        player.push_next_track(0);
        // First next() should drain index 2
        player.next().unwrap();
        {
            let opened_paths = opened.lock().unwrap();
            assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/p3.mp3"));
        }
    }

    #[test]
    fn test_prev_goes_back() {
        let (player, opened, play_called) = mock_player(&["/music/first.mp3", "/music/second.mp3"]);
        // Advance to track 1 first
        player.next().unwrap();
        play_called.store(false, Ordering::SeqCst);
        // Now go back
        player.prev().unwrap();
        assert!(play_called.load(Ordering::SeqCst));
        let opened_paths = opened.lock().unwrap();
        assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/first.mp3"));
    }

    #[test]
    fn test_prev_empty_playlist() {
        let (player, _, _) = mock_player(&[]);
        assert!(player.prev().is_ok());
    }

    #[test]
    fn test_next_no_track_available_stops() {
        let (player, opened, _) = mock_player(&["/music/single.mp3"]);
        // PlayOrder mode: only one track, next should stay
        player.set_repeat_mode(RepeatMode::PlayOrder);
        player.next().unwrap();
        let opened_paths = opened.lock().unwrap();
        // next_index() returns None in PlayOrder at end of list, so no play_file call
        assert!(opened_paths.is_empty());
    }

    #[test]
    fn test_play_at_index_opens_correct_file() {
        let (player, opened, _) = mock_player(&["/music/alpha.mp3", "/music/beta.mp3"]);
        player.play_at_index(1).unwrap();
        let opened_paths = opened.lock().unwrap();
        assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/beta.mp3"));
    }

    #[test]
    fn test_play_at_index_out_of_bounds() {
        let (player, _, _) = mock_player(&["/music/one.mp3"]);
        let result = player.play_at_index(5);
        assert!(result.is_err());
    }

    #[test]
    fn test_play_file_calls_open_and_play() {
        let (player, opened, play_called) = mock_player(&[]);
        player.play_file("/music/test.mp3").unwrap();
        assert!(play_called.load(Ordering::SeqCst));
        let opened_paths = opened.lock().unwrap();
        assert_eq!(opened_paths.last().map(|s| s.as_str()), Some("/music/test.mp3"));
    }

    #[test]
    fn test_eq_set_normal() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Set band 3 to gain 7
        player.eq_set(3, 7).unwrap();
        assert_eq!(player.eq_get_band(3), 7);
        // Other bands should remain 0
        assert_eq!(player.eq_get_band(0), 0);
        assert_eq!(player.eq_get_band(9), 0);
    }

    #[test]
    fn test_eq_set_clamp_high() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Set band 5 to gain 100 — should clamp to 15
        player.eq_set(5, 100).unwrap();
        assert_eq!(player.eq_get_band(5), 15);
    }

    #[test]
    fn test_eq_set_clamp_low() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Set band 1 to gain -100 — should clamp to -15
        player.eq_set(1, -100).unwrap();
        assert_eq!(player.eq_get_band(1), -15);
    }

    #[test]
    fn test_eq_set_multiple_bands() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Set multiple bands and verify each independently
        player.eq_set(0, 3).unwrap();
        player.eq_set(4, -8).unwrap();
        player.eq_set(9, 12).unwrap();
        let gains = player.eq_get();
        assert_eq!(gains[0], 3);
        assert_eq!(gains[4], -8);
        assert_eq!(gains[9], 12);
        // Ensure unset bands are still 0
        assert_eq!(gains[1], 0);
        assert_eq!(gains[5], 0);
    }

    #[test]
    fn test_eq_reset_clears_all_bands() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Set some bands
        player.eq_set(2, 10).unwrap();
        player.eq_set(5, -5).unwrap();
        assert_eq!(player.eq_get_band(2), 10);
        // Reset
        player.eq_reset().unwrap();
        // All bands should be 0
        let gains = player.eq_get();
        assert!(gains.iter().all(|&g| g == 0));
    }

    #[test]
    fn test_eq_enable_toggle() {
        let engine = MockEngine::new();
        let playlist = Playlist::new("test");
        let player = Player {
            engine: Box::new(engine),
            engine_type: EngineType::Bass,
            playlist: Mutex::new(playlist),
            eq_gains: Mutex::new([0; 10]),
            eq_enabled: Mutex::new(false),
            reverb_mix: Mutex::new(0),
            reverb_time: Mutex::new(1),
            reverb_enabled: Mutex::new(false),
            ab_repeat: Mutex::new(ABRepeatPoint {
                a: Duration::ZERO,
                b: Duration::ZERO,
                mode: ABRepeatMode::None,
            }),
            spectrum_columns: Mutex::new(SPECTRUM_COL_DEFAULT),
            fft_size: Mutex::new(FFT_SAMPLE_DEFAULT),
            fft_data: Mutex::new([0.0; FFT_SAMPLE_DEFAULT]),
            spectrum_peaks: Mutex::new(Vec::new()),
            display_format: Mutex::new("title".to_string()),
            loading: AtomicBool::new(false),
            volume_map: Mutex::new(100),
            cue_end_pos: Mutex::new(None),
        };
        // Default is disabled
        assert!(!player.eq_is_enabled());
        // Enable
        player.eq_enable(true);
        assert!(player.eq_is_enabled());
        // Disable
        player.eq_enable(false);
        assert!(!player.eq_is_enabled());
    }
}
