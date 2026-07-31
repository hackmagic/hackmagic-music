//! Symphonia audio engine — pure Rust decoding (all codecs via symphonia),
//! playback via rodio Sink (shared RodioBackend).
//! Only the `decode_to_buffer` method differs from `RodioEngine`.

use crate::error::{PlayerError, Result};
use crate::rodio_common::{impl_rodio_backend_engine, RodioBackend};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct SymphoniaEngine {
    backend: std::sync::Arc<RodioBackend>,
}

unsafe impl Send for SymphoniaEngine {}
unsafe impl Sync for SymphoniaEngine {}

impl Default for SymphoniaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SymphoniaEngine {
    pub fn new() -> Self {
        Self { backend: std::sync::Arc::new(RodioBackend::new()) }
    }

    fn decode_to_buffer(path: &str) -> Result<(Vec<f32>, u32, usize)> {
        let file = std::fs::File::open(path)
            .map_err(|e| PlayerError::CannotOpen(format!("{path}: {e}")))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = std::path::Path::new(path).extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let format_opts = symphonia::core::formats::FormatOptions::default();
        let meta_opts = MetadataOptions::default();
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &meta_opts)
            .map_err(|e| PlayerError::CannotOpen(format!("symphonia probe failed for {path}: {e}")))?;
        let mut reader = probed.format;
        let track = reader
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| PlayerError::CannotOpen("No audio track found".into()))?;
        let codec_params = track.codec_params.clone();
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let num_channels = codec_params
            .channels
            .map(|c| c.count())
            .unwrap_or(2);
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| PlayerError::CannotOpen(format!("Cannot create decoder: {e}")))?;
        let mut all_samples = Vec::new();
        loop {
            let packet = match reader.next_packet() {
                Ok(pkt) => pkt,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break
                }
                Err(symphonia::core::errors::Error::ResetRequired) => continue,
                Err(_) => break,
            };
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    let num_frames = decoded.frames();
                    let spec = *decoded.spec();
                    let num_ch = spec.channels.count();
                    match &decoded {
                        AudioBufferRef::F32(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f]);
                                }
                            }
                        }
                        AudioBufferRef::S16(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f] as f32 / 32768.0);
                                }
                            }
                        }
                        AudioBufferRef::S32(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f] as f32 / 2147483648.0);
                                }
                            }
                        }
                        AudioBufferRef::F64(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f] as f32);
                                }
                            }
                        }
                        AudioBufferRef::U8(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push((buf.chan(c)[f] as f32 / 128.0) - 1.0);
                                }
                            }
                        }
                        AudioBufferRef::S24(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f].inner() as f32 / 8388608.0);
                                }
                            }
                        }
                        AudioBufferRef::U16(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push((buf.chan(c)[f] as f32 / 32768.0) - 1.0);
                                }
                            }
                        }
                        AudioBufferRef::S8(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f] as f32 / 128.0);
                                }
                            }
                        }
                        AudioBufferRef::U24(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f].inner() as f32 / 8388608.0);
                                }
                            }
                        }
                        AudioBufferRef::U32(buf) => {
                            for f in 0..num_frames {
                                for c in 0..num_ch {
                                    all_samples.push(buf.chan(c)[f] as f32 / 2147483648.0);
                                }
                            }
                        }
                    }
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
                Err(_) => break,
            }
        }
        tracing::info!(
            "[SYMPHONIA] decoded {}: {} samples, {} Hz, {} ch",
            path,
            all_samples.len(),
            sample_rate,
            num_channels
        );
        Ok((all_samples, sample_rate, num_channels))
    }
}

impl_rodio_backend_engine!(SymphoniaEngine, "Symphonia");

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine_trait::{EngineState, PlayerEngine};
    use std::time::Duration;

    #[test]
    fn symphonia_new() {
        let engine = SymphoniaEngine::new();
        assert_eq!(engine.name(), "Symphonia");
    }

    #[test]
    fn symphonia_state_initial() {
        let engine = SymphoniaEngine::new();
        assert!(matches!(engine.state(), EngineState::Stopped));
    }

    #[test]
    fn symphonia_position_initial() {
        let engine = SymphoniaEngine::new();
        assert_eq!(engine.position(), Duration::ZERO);
    }

    #[test]
    fn symphonia_volume_initial() {
        let engine = SymphoniaEngine::new();
        // Default volume from RodioBackend
        let vol = engine.volume();
        assert!(vol <= 100);
    }

    #[test]
    fn symphonia_is_midi() {
        let engine = SymphoniaEngine::new();
        assert!(!engine.is_midi());
    }
}
