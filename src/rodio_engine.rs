//! Rodio audio engine — pure Rust playback (mp3/flac/vorbis).
//! Uses the shared `RodioBackend` from `rodio_common` for playback infrastructure.

use crate::error::{PlayerError, Result};
use crate::rodio_common::{impl_rodio_backend_engine, RodioBackend};
use rodio::Source;
use std::io::BufReader;

pub struct RodioEngine {
    backend: RodioBackend,
}

unsafe impl Send for RodioEngine {}
unsafe impl Sync for RodioEngine {}

impl Default for RodioEngine {
    fn default() -> Self {
        Self::new()
    }
}

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

impl_rodio_backend_engine!(RodioEngine, "Rodio");
