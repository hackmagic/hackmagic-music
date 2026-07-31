//! Shared playback backend for Rodio/Symphonia engines.
//! Both engines follow the same pattern: decode to f32 buffer, play via rodio::Sink.
//! This module extracts the common parts to eliminate code duplication.

use crate::core::engine_trait::EngineState;
use crate::error::Result;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Decoded PCM for a gapless-preloaded track: (path, samples, sample_rate, channels).
pub type PreloadedBuffer = (String, Vec<f32>, u32, usize);

/// Interleaved f32 audio source for rodio playback.
struct SharedSource {
    buffer: Arc<Vec<f32>>,
    channels: usize,
    sample_rate: u32,
    frame_pos: u64,
    frame_pos_shared: Arc<AtomicU64>,
    finished: bool,
}

impl Iterator for SharedSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.finished {
            return None;
        }
        let idx = self.frame_pos as usize * self.channels;
        if idx >= self.buffer.len() {
            self.finished = true;
            self.frame_pos_shared.store(self.frame_pos, Ordering::Relaxed);
            return None;
        }
        let sample = self.buffer[idx];
        self.frame_pos += 1;
        if self.frame_pos.is_multiple_of(1000) {
            self.frame_pos_shared.store(self.frame_pos, Ordering::Relaxed);
        }
        Some(sample)
    }
}

impl rodio::Source for SharedSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.channels as u16
    }
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        let frames = self.buffer.len() / self.channels;
        Some(Duration::from_secs_f64(frames as f64 / self.sample_rate as f64))
    }
}

/// Shared playback state for rodio-based engines.
pub struct RodioBackend {
    pub buffer: Mutex<Arc<Vec<f32>>>,
    pub sample_rate: AtomicU32,
    pub channels: Mutex<usize>,
    pub total_frames: AtomicU64,
    /// Decoded PCM for the next track (gapless preload).
    pub preloaded: Mutex<Option<PreloadedBuffer>>,
    /// Generation counter: bumped on every preload/take, so stale in-flight
    /// decode threads never overwrite the slot with an outdated track.
    pub preload_seq: AtomicU64,
    /// True while a preload decode thread is running; bounds memory by only
    /// allowing one in-flight decode at a time.
    pub preload_in_flight: std::sync::atomic::AtomicBool,
    pub state: Mutex<EngineState>,
    pub play_pos: Arc<AtomicU64>,
    pub volume: Mutex<f32>,
    pub speed: Mutex<f32>,
    pub pitch: Mutex<i32>,
    pub eq_gains: Mutex<[i32; 10]>,
    pub output_stream: Mutex<Option<rodio::OutputStream>>,
    pub sink: Mutex<Option<rodio::Sink>>,
    pub paused: Mutex<bool>,
}

// Safety: all fields are Send+Sync (Mutex, Atomic*, Arc)
unsafe impl Send for RodioBackend {}
unsafe impl Sync for RodioBackend {}

impl Default for RodioBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl RodioBackend {
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(Arc::new(Vec::new())),
            sample_rate: AtomicU32::new(44100),
            channels: Mutex::new(2),
            total_frames: AtomicU64::new(0),
            preloaded: Mutex::new(None),
            preload_seq: AtomicU64::new(0),
            preload_in_flight: std::sync::atomic::AtomicBool::new(false),
            state: Mutex::new(EngineState::Stopped),
            play_pos: Arc::new(AtomicU64::new(0)),
            volume: Mutex::new(0.8),
            speed: Mutex::new(1.0),
            pitch: Mutex::new(0),
            eq_gains: Mutex::new([0; 10]),
            output_stream: Mutex::new(None),
            sink: Mutex::new(None),
            paused: Mutex::new(false),
        }
    }

    pub fn set_buffer(&self, samples: Vec<f32>, sample_rate: u32, channels: usize) {
        *self.buffer.lock().unwrap() = Arc::new(samples);
        self.sample_rate.store(sample_rate, Ordering::Relaxed);
        *self.channels.lock().unwrap() = channels;
    }

    pub fn init(&self) -> Result<()> {
        tracing::info!("Rodio backend initialized");
        Ok(())
    }

    pub fn uninit(&self) -> Result<()> {
        *self.sink.lock().unwrap() = None;
        *self.output_stream.lock().unwrap() = None;
        self.preload_seq.fetch_add(1, Ordering::SeqCst);
        self.preload_in_flight.store(false, Ordering::SeqCst);
        *self.preloaded.lock().unwrap() = None;
        Ok(())
    }

    pub fn close(&self) -> Result<()> {
        *self.sink.lock().unwrap() = None;
        *self.output_stream.lock().unwrap() = None;
        self.preload_seq.fetch_add(1, Ordering::SeqCst);
        self.preload_in_flight.store(false, Ordering::SeqCst);
        *self.preloaded.lock().unwrap() = None;
        *self.state.lock().unwrap() = EngineState::Stopped;
        self.play_pos.store(0, Ordering::SeqCst);
        Ok(())
    }

    /// Start a preload round. Returns a generation token, or `None` if a
    /// preload decode is already in flight.
    pub fn begin_preload(&self) -> Option<u64> {
        if self
            .preload_in_flight
            .swap(true, Ordering::SeqCst)
        {
            return None;
        }
        Some(self.preload_seq.fetch_add(1, Ordering::SeqCst) + 1)
    }

    /// Store a decoded buffer only if `gen` is still the current generation
    /// (i.e. no newer preload/take superseded it), then clear the in-flight flag.
    pub fn store_preloaded_if_current(
        &self,
        gen: u64,
        path: String,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: usize,
    ) {
        if self.preload_seq.load(Ordering::SeqCst) == gen {
            *self.preloaded.lock().unwrap() = Some((path, samples, sample_rate, channels));
        }
        self.preload_in_flight.store(false, Ordering::SeqCst);
    }

    /// Mark a preload as finished without storing anything (decode error).
    pub fn abort_preload(&self) {
        self.preload_in_flight.store(false, Ordering::SeqCst);
    }

    /// Take the preloaded buffer if it belongs to `path` (consumes it).
    pub fn take_preloaded(&self, path: &str) -> Option<(Vec<f32>, u32, usize)> {
        // Invalidate any in-flight preload and clear the flag: after consuming
        // the cache we never want a stale thread to refill it.
        self.preload_seq.fetch_add(1, Ordering::SeqCst);
        self.preload_in_flight.store(false, Ordering::SeqCst);
        let mut slot = self.preloaded.lock().unwrap();
        if slot.as_ref().map(|(p, _, _, _)| p == path).unwrap_or(false) {
            slot.take().map(|(_, samples, sample_rate, channels)| (samples, sample_rate, channels))
        } else {
            None
        }
    }

    pub fn play(&self) -> Result<()> {
        let buffer = self.buffer.lock().unwrap().clone();
        if buffer.is_empty() {
            return Err(crate::error::PlayerError::Playback("No audio data loaded".into()));
        }
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        let channels = *self.channels.lock().unwrap();
        let total_frames = buffer.len() / channels;
        self.total_frames.store(total_frames as u64, Ordering::Relaxed);

        let play_pos = self.play_pos.clone();
        let (_stream, stream_handle) = rodio::OutputStream::try_default()
            .map_err(|e| crate::error::PlayerError::Playback(format!("Cannot open audio output: {e}")))?;
        let sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| crate::error::PlayerError::Playback(format!("Cannot create sink: {e}")))?;

        let source = SharedSource {
            buffer,
            channels,
            sample_rate,
            frame_pos: 0,
            frame_pos_shared: play_pos,
            finished: false,
        };
        sink.append(source);
        *self.sink.lock().unwrap() = Some(sink);
        *self.output_stream.lock().unwrap() = Some(_stream);
        *self.state.lock().unwrap() = EngineState::Playing;
        *self.paused.lock().unwrap() = false;
        Ok(())
    }

    pub fn pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        let sink = self.sink.lock().unwrap();
        if *state == EngineState::Playing {
            if let Some(ref s) = *sink {
                s.pause();
            }
            *state = EngineState::Paused;
            *self.paused.lock().unwrap() = true;
        } else if *state == EngineState::Paused {
            if let Some(ref s) = *sink {
                s.play();
            }
            *state = EngineState::Playing;
            *self.paused.lock().unwrap() = false;
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        *self.sink.lock().unwrap() = None;
        *self.state.lock().unwrap() = EngineState::Stopped;
        *self.paused.lock().unwrap() = false;
        self.play_pos.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub fn state(&self) -> EngineState {
        *self.state.lock().unwrap()
    }

    pub fn position(&self) -> Duration {
        let pos = self.play_pos.load(Ordering::Relaxed);
        let sr = self.sample_rate.load(Ordering::Relaxed);
        if sr > 0 {
            Duration::from_secs_f64(pos as f64 / sr as f64)
        } else {
            Duration::ZERO
        }
    }

    pub fn duration(&self) -> Duration {
        let frames = self.total_frames.load(Ordering::Relaxed);
        let sr = self.sample_rate.load(Ordering::Relaxed);
        if sr > 0 {
            Duration::from_secs_f64(frames as f64 / sr as f64)
        } else {
            Duration::ZERO
        }
    }

    pub fn seek(&self, pos: Duration) -> Result<()> {
        let sr = self.sample_rate.load(Ordering::Relaxed);
        if sr == 0 {
            return Ok(());
        }
        let target_frame = (pos.as_secs_f64() * sr as f64) as u64;
        let total = self.total_frames.load(Ordering::Relaxed);
        let target_frame = target_frame.min(total.saturating_sub(1));

        // Recreate sink with seeked position
        let buffer = self.buffer.lock().unwrap().clone();
        if buffer.is_empty() {
            return Ok(());
        }
        let channels = *self.channels.lock().unwrap();
        let sample_rate = self.sample_rate.load(Ordering::Relaxed);
        let play_pos = self.play_pos.clone();
        let (_stream, stream_handle) = rodio::OutputStream::try_default()
            .map_err(|e| crate::error::PlayerError::Playback(format!("Cannot open audio output: {e}")))?;
        let sink = rodio::Sink::try_new(&stream_handle)
            .map_err(|e| crate::error::PlayerError::Playback(format!("Cannot create sink: {e}")))?;

        let source = SharedSource {
            buffer,
            channels,
            sample_rate,
            frame_pos: target_frame,
            frame_pos_shared: play_pos,
            finished: false,
        };
        sink.append(source);
        *self.sink.lock().unwrap() = Some(sink);
        *self.output_stream.lock().unwrap() = Some(_stream);
        // Keep current playback state
        let is_paused = *self.paused.lock().unwrap();
        if is_paused {
            if let Some(ref s) = *self.sink.lock().unwrap() {
                s.pause();
            }
        }
        Ok(())
    }

    pub fn set_volume(&self, vol: u32) -> Result<()> {
        *self.volume.lock().unwrap() = vol as f32 / 100.0;
        let sink = self.sink.lock().unwrap();
        if let Some(ref s) = *sink {
            s.set_volume(vol as f32 / 100.0);
        }
        Ok(())
    }

    pub fn volume(&self) -> u32 {
        (*self.volume.lock().unwrap() * 100.0) as u32
    }

    pub fn set_speed(&self, speed: f32) -> Result<()> {
        *self.speed.lock().unwrap() = speed.clamp(0.1, 4.0);
        Ok(())
    }

    pub fn speed(&self) -> f32 {
        *self.speed.lock().unwrap()
    }

    pub fn set_pitch(&self, pitch: i32) -> Result<()> {
        *self.pitch.lock().unwrap() = pitch.clamp(-12, 12);
        Ok(())
    }

    pub fn pitch(&self) -> i32 {
        *self.pitch.lock().unwrap()
    }

    pub fn set_equalizer(&self, band: usize, gain: i32) -> Result<()> {
        // No-op: rodio doesn't support EQ
        self.eq_gains.lock().unwrap()[band] = gain.clamp(-15, 15);
        Ok(())
    }

    pub fn equalizer(&self) -> [i32; 10] {
        *self.eq_gains.lock().unwrap()
    }

    pub fn set_reverb(&self, _mix: u32, _time: u32) -> Result<()> {
        Ok(()) // No-op
    }

    pub fn clear_reverb(&self) -> Result<()> {
        Ok(()) // No-op
    }

    pub fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32> {
        let buffer = self.buffer.lock().unwrap();
        if buffer.is_empty() {
            return vec![];
        }
        let buf = &buffer[..];
        let n = buf.len().min(fft_size as usize);
        buf[..n].to_vec()
    }

    pub fn song_is_over(&self) -> bool {
        let sink = self.sink.lock().unwrap();
        sink.as_ref().map(|s| s.empty()).unwrap_or(true)
    }

    pub fn is_midi(&self) -> bool {
        false
    }
}

/// Generate the `PlayerEngine` impl shared by rodio-based engines (Rodio,
/// Symphonia). Only `name()` and `decode_to_buffer()` differ between them:
/// everything else delegates to the common `RodioBackend`.
macro_rules! impl_rodio_backend_engine {
    ($engine:ident, $engine_name:literal) => {
        impl crate::core::engine_trait::PlayerEngine for $engine {
            fn name(&self) -> &'static str {
                $engine_name
            }

            fn capabilities(&self) -> crate::core::engine_trait::EngineCapabilities {
                crate::core::engine_trait::EngineCapabilities::decode_only()
            }

            fn init(&self) -> crate::error::Result<()> {
                self.backend.init()
            }

            fn uninit(&self) -> crate::error::Result<()> {
                self.backend.uninit()
            }

            fn open(&self, path: &str) -> crate::error::Result<()> {
                if let Some((samples, sample_rate, channels)) = self.backend.take_preloaded(path) {
                    self.backend.set_buffer(samples, sample_rate, channels);
                    tracing::info!("[{}] using preloaded buffer for {}", $engine_name, path);
                    return Ok(());
                }
                let (samples, sample_rate, channels) = Self::decode_to_buffer(path)?;
                self.backend.set_buffer(samples, sample_rate, channels);
                Ok(())
            }

            fn preload_next(&self, path: &str) {
                let Some(gen) = self.backend.begin_preload() else {
                    return; // a preload decode is already in flight
                };
                let backend = self.backend.clone();
                let path = path.to_string();
                std::thread::spawn(move || {
                    match Self::decode_to_buffer(&path) {
                        Ok((samples, sample_rate, channels)) => {
                            backend.store_preloaded_if_current(
                                gen,
                                path.clone(),
                                samples,
                                sample_rate,
                                channels,
                            );
                            tracing::info!("[{}] preloaded {}", $engine_name, path);
                        }
                        Err(e) => {
                            backend.abort_preload();
                            tracing::warn!("[{}] preload failed for {}: {}", $engine_name, path, e);
                        }
                    }
                });
            }

            fn close(&self) -> crate::error::Result<()> {
                self.backend.close()
            }

            fn play(&self) -> crate::error::Result<()> {
                self.backend.play()
            }

            fn pause(&self) -> crate::error::Result<()> {
                self.backend.pause()
            }

            fn stop(&self) -> crate::error::Result<()> {
                self.backend.stop()
            }

            fn state(&self) -> crate::core::engine_trait::EngineState {
                self.backend.state()
            }

            fn position(&self) -> std::time::Duration {
                self.backend.position()
            }

            fn duration(&self) -> std::time::Duration {
                self.backend.duration()
            }

            fn seek(&self, pos: std::time::Duration) -> crate::error::Result<()> {
                self.backend.seek(pos)
            }

            fn set_volume(&self, vol: u32) -> crate::error::Result<()> {
                self.backend.set_volume(vol)
            }

            fn volume(&self) -> u32 {
                self.backend.volume()
            }

            fn set_speed(&self, speed: f32) -> crate::error::Result<()> {
                self.backend.set_speed(speed)
            }

            fn speed(&self) -> f32 {
                self.backend.speed()
            }

            fn set_pitch(&self, pitch: i32) -> crate::error::Result<()> {
                self.backend.set_pitch(pitch)
            }

            fn pitch(&self) -> i32 {
                self.backend.pitch()
            }

            fn set_equalizer(&self, band: usize, gain: i32) -> crate::error::Result<()> {
                self.backend.set_equalizer(band, gain)
            }

            fn equalizer(&self) -> [i32; 10] {
                self.backend.equalizer()
            }

            fn set_reverb(&self, mix: u32, time: u32) -> crate::error::Result<()> {
                self.backend.set_reverb(mix, time)
            }

            fn clear_reverb(&self) -> crate::error::Result<()> {
                self.backend.clear_reverb()
            }

            fn fft_data(&self) -> Vec<f32> {
                self.backend.fft_data_with_size(512)
            }

            fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32> {
                self.backend.fft_data_with_size(fft_size)
            }

            fn song_is_over(&self) -> bool {
                self.backend.song_is_over()
            }

            fn is_midi(&self) -> bool {
                self.backend.is_midi()
            }
        }
    };
}

pub(crate) use impl_rodio_backend_engine;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preloaded_buffer_roundtrip() {
        let backend = RodioBackend::new();
        let gen = backend.begin_preload().unwrap();
        backend.store_preloaded_if_current(gen, "/a.flac".into(), vec![0.1, 0.2, 0.3], 44100, 2);

        let taken = backend.take_preloaded("/a.flac");
        assert!(taken.is_some());
        let (samples, sample_rate, channels) = taken.unwrap();
        assert_eq!(samples, vec![0.1, 0.2, 0.3]);
        assert_eq!(sample_rate, 44100);
        assert_eq!(channels, 2);

        // The cache is consumed by `take_preloaded`.
        assert!(backend.take_preloaded("/a.flac").is_none());
    }

    #[test]
    fn preloaded_buffer_path_mismatch_keeps_cache() {
        let backend = RodioBackend::new();
        let gen = backend.begin_preload().unwrap();
        backend.store_preloaded_if_current(gen, "/a.flac".into(), vec![1.0], 48000, 1);

        // Different path: nothing is taken and the cache survives.
        assert!(backend.take_preloaded("/b.flac").is_none());
        let taken = backend.take_preloaded("/a.flac");
        assert!(taken.is_some());
        let (samples, _, _) = taken.unwrap();
        assert_eq!(samples, vec![1.0]);
    }

    #[test]
    fn close_clears_preloaded_buffer() {
        let backend = RodioBackend::new();
        let gen = backend.begin_preload().unwrap();
        backend.store_preloaded_if_current(gen, "/a.flac".into(), vec![1.0], 48000, 1);
        backend.close().unwrap();
        assert!(backend.take_preloaded("/a.flac").is_none());
    }

    #[test]
    fn preload_single_in_flight() {
        let backend = RodioBackend::new();
        assert!(backend.begin_preload().is_some());
        // A second preload while the first is in flight is rejected.
        assert!(backend.begin_preload().is_none());
        backend.abort_preload();
        assert!(backend.begin_preload().is_some());
    }

    #[test]
    fn stale_generation_does_not_overwrite() {
        let backend = RodioBackend::new();
        let gen = backend.begin_preload().unwrap();
        backend.store_preloaded_if_current(gen, "/a.flac".into(), vec![1.0], 48000, 1);

        // Take bumps the generation; a stale store from an old thread is dropped.
        let taken = backend.take_preloaded("/a.flac");
        assert!(taken.is_some());
        let stale = backend.begin_preload().unwrap();
        backend.store_preloaded_if_current(stale - 1, "/b.flac".into(), vec![9.0], 48000, 1);
        assert!(backend.take_preloaded("/b.flac").is_none());
    }
}
