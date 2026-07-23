//! Rodio audio engine — pure Rust playback (mp3/flac/vorbis).
//! Uses the shared `RodioBackend` from `rodio_common` for playback infrastructure.

use crate::core::engine_trait::{EngineCapabilities, EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use crate::rodio_common::RodioBackend;
use rodio::Source;
use std::io::BufReader;
use std::time::Duration;

pub struct RodioEngine {
    backend: RodioBackend,
}

unsafe impl Send for RodioEngine {}
unsafe impl Sync for RodioEngine {}

impl RodioEngine {
    pub fn new() -> Self {
        Self { backend: RodioBackend::new() }
    }

    fn decode_to_buffer(path: &str) -> Result<(Vec<f32>, u32, usize)> {
        let file = std::fs::File::open(path)
            .map_err(|e| PlayerError::CannotOpen(format!("{path}: {e}")))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .map_err(|e| PlayerError::CannotOpen(format!("rodio decode failed for {path}: {e}")))?;
        let sample_rate = source.sample_rate();
        let channels = source.channels() as usize;
        let samples: Vec<f32> = source.convert_samples().collect();
        let total_samples = samples.len();
        tracing::info!(
            "[RODIO] decoded {}: {} samples, {} Hz, {} ch",
            path,
            total_samples,
            sample_rate,
            channels
        );
        Ok((samples, sample_rate, channels))
    }
}

impl PlayerEngine for RodioEngine {
    fn name(&self) -> &'static str {
        "Rodio"
    }

    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities::decode_only()
    }

    fn init(&self) -> Result<()> {
        self.backend.init()
    }

    fn uninit(&self) -> Result<()> {
        self.backend.uninit()
    }

    fn open(&self, path: &str) -> Result<()> {
        let (samples, sample_rate, channels) = Self::decode_to_buffer(path)?;
        self.backend.set_buffer(samples, sample_rate, channels);
        Ok(())
    }

    fn close(&self) -> Result<()> {
        self.backend.close()
    }

    fn play(&self) -> Result<()> {
        self.backend.play()
    }

    fn pause(&self) -> Result<()> {
        self.backend.pause()
    }

    fn stop(&self) -> Result<()> {
        self.backend.stop()
    }

    fn state(&self) -> EngineState {
        self.backend.state()
    }

    fn position(&self) -> Duration {
        self.backend.position()
    }

    fn duration(&self) -> Duration {
        self.backend.duration()
    }

    fn seek(&self, pos: Duration) -> Result<()> {
        self.backend.seek(pos)
    }

    fn set_volume(&self, vol: u32) -> Result<()> {
        self.backend.set_volume(vol)
    }

    fn volume(&self) -> u32 {
        self.backend.volume()
    }

    fn set_speed(&self, speed: f32) -> Result<()> {
        self.backend.set_speed(speed)
    }

    fn speed(&self) -> f32 {
        self.backend.speed()
    }

    fn set_pitch(&self, pitch: i32) -> Result<()> {
        self.backend.set_pitch(pitch)
    }

    fn pitch(&self) -> i32 {
        self.backend.pitch()
    }

    fn set_equalizer(&self, band: usize, gain: i32) -> Result<()> {
        self.backend.set_equalizer(band, gain)
    }

    fn equalizer(&self) -> [i32; 10] {
        self.backend.equalizer()
    }

    fn set_reverb(&self, mix: u32, time: u32) -> Result<()> {
        self.backend.set_reverb(mix, time)
    }

    fn clear_reverb(&self) -> Result<()> {
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