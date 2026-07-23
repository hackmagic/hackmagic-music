//! Symphonia audio engine — pure Rust decoding (all codecs via symphonia),
//! playback via rodio Sink (shared RodioBackend).
//! Only the `decode_to_buffer` method differs from `RodioEngine`.

use crate::core::engine_trait::{EngineCapabilities, EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use crate::rodio_common::RodioBackend;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct SymphoniaEngine {
    backend: RodioBackend,
}

unsafe impl Send for SymphoniaEngine {}
unsafe impl Sync for SymphoniaEngine {}

impl SymphoniaEngine {
    pub fn new() -> Self {
        Self { backend: RodioBackend::new() }
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
            .map(|c| c.count() as usize)
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

impl PlayerEngine for SymphoniaEngine {
    fn name(&self) -> &'static str {
        "Symphonia"
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