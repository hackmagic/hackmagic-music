use crate::core::engine_trait::{EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use rodio::Source;
use std::io::BufReader;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct RodioSource {
    buffer: Arc<Vec<f32>>,
    channels: usize,
    sample_rate: u32,
    frame_pos: u64,
    frame_pos_shared: Arc<AtomicU64>,
    finished: bool,
}

impl Iterator for RodioSource {
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
        if self.frame_pos % 1000 == 0 {
            self.frame_pos_shared.store(self.frame_pos, Ordering::Relaxed);
        }
        Some(sample)
    }
}

impl rodio::Source for RodioSource {
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

pub struct RodioEngine {
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: Mutex<u32>,
    channels: Mutex<usize>,
    total_frames: Mutex<u64>,
    state: Mutex<EngineState>,
    play_pos: Arc<AtomicU64>,
    volume: Mutex<f32>,
    speed: Mutex<f32>,
    pitch: Mutex<i32>,
    eq_gains: Mutex<[i32; 10]>,
    output_stream: Mutex<Option<rodio::OutputStream>>,
    sink: Mutex<Option<rodio::Sink>>,
    paused: Mutex<bool>,
}

unsafe impl Send for RodioEngine {}
unsafe impl Sync for RodioEngine {}

impl RodioEngine {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: Mutex::new(44100),
            channels: Mutex::new(2),
            total_frames: Mutex::new(0),
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

    fn init(&self) -> Result<()> {
        tracing::info!("Rodio engine initialized");
        Ok(())
    }

    fn uninit(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        *self.sink.lock().unwrap() = None;
        *self.state.lock().unwrap() = EngineState::Stopped;
        *self.buffer.lock().unwrap() = Vec::new();
        Ok(())
    }

    fn open(&self, path: &str) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        *self.sink.lock().unwrap() = None;
        *self.paused.lock().unwrap() = false;
        self.play_pos.store(0, Ordering::SeqCst);

        let (samples, sample_rate, channels) = Self::decode_to_buffer(path)?;
        let total_frames = if channels > 0 { samples.len() / channels } else { 0 };

        *self.buffer.lock().unwrap() = samples;
        *self.sample_rate.lock().unwrap() = sample_rate;
        *self.channels.lock().unwrap() = channels;
        *self.total_frames.lock().unwrap() = total_frames as u64;
        *self.state.lock().unwrap() = EngineState::Stopped;

        Ok(())
    }

    fn close(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        *self.sink.lock().unwrap() = None;
        *self.paused.lock().unwrap() = false;
        self.play_pos.store(0, Ordering::SeqCst);
        *self.buffer.lock().unwrap() = Vec::new();
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn play(&self) -> Result<()> {
        let state = *self.state.lock().unwrap();
        if state == EngineState::Paused {
            if let Some(sink) = self.sink.lock().unwrap().as_ref() {
                sink.play();
            }
            *self.paused.lock().unwrap() = false;
            *self.state.lock().unwrap() = EngineState::Playing;
            return Ok(());
        }
        if state == EngineState::Playing {
            return Ok(());
        }

        let buf = self.buffer.lock().unwrap().clone();
        if buf.is_empty() {
            return Err(PlayerError::NoTrack);
        }
        let sample_rate = *self.sample_rate.lock().unwrap();
        let channels = *self.channels.lock().unwrap();
        let init_vol = *self.volume.lock().unwrap();
        let init_speed = *self.speed.lock().unwrap();
        let start_frame = self.play_pos.load(Ordering::SeqCst);

        let (stream, handle) = rodio::OutputStream::try_default()
            .map_err(|e| PlayerError::Playback(format!("Cannot open audio output: {e}")))?;

        let sink = rodio::Sink::try_new(&handle)
            .map_err(|e| PlayerError::Playback(format!("Cannot create sink: {e}")))?;
        sink.set_volume(init_vol);

        let buf_arc = Arc::new(buf);
        let pos_shared = self.play_pos.clone();

        let source = RodioSource {
            buffer: buf_arc,
            channels,
            sample_rate,
            frame_pos: start_frame,
            frame_pos_shared: pos_shared,
            finished: false,
        };
        let source = source.speed(init_speed);
        sink.append(source);

        *self.output_stream.lock().unwrap() = Some(stream);
        *self.sink.lock().unwrap() = Some(sink);
        *self.paused.lock().unwrap() = false;
        *self.state.lock().unwrap() = EngineState::Playing;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        match *state {
            EngineState::Playing => {
                if let Some(sink) = self.sink.lock().unwrap().as_ref() {
                    sink.pause();
                }
                *self.paused.lock().unwrap() = true;
                *state = EngineState::Paused;
            }
            EngineState::Paused => {
                if let Some(sink) = self.sink.lock().unwrap().as_ref() {
                    sink.play();
                }
                *self.paused.lock().unwrap() = false;
                *state = EngineState::Playing;
            }
            _ => {}
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        *self.sink.lock().unwrap() = None;
        *self.paused.lock().unwrap() = false;
        self.play_pos.store(0, Ordering::SeqCst);
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn state(&self) -> EngineState {
        *self.state.lock().unwrap()
    }

    fn position(&self) -> Duration {
        let frames = self.play_pos.load(Ordering::Relaxed);
        let sr = *self.sample_rate.lock().unwrap();
        if sr == 0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(frames as f64 / sr as f64)
    }

    fn duration(&self) -> Duration {
        let total = *self.total_frames.lock().unwrap() as f64;
        let sr = *self.sample_rate.lock().unwrap() as f64;
        if sr <= 0.0 {
            return Duration::ZERO;
        }
        Duration::from_secs_f64(total / sr)
    }

    fn seek(&self, pos: Duration) -> Result<()> {
        let sr = *self.sample_rate.lock().unwrap();
        if sr == 0 {
            return Err(PlayerError::NoTrack);
        }
        let target = (pos.as_secs_f64() * sr as f64) as u64;
        let total = *self.total_frames.lock().unwrap();
        let target = target.min(total.saturating_sub(1));
        self.play_pos.store(target, Ordering::SeqCst);

        let was_playing = *self.state.lock().unwrap() == EngineState::Playing;
        if was_playing {
            *self.output_stream.lock().unwrap() = None;
            *self.sink.lock().unwrap() = None;
            self.play()?;
        }
        Ok(())
    }

    fn set_volume(&self, vol: u32) -> Result<()> {
        let v = vol.min(100) as f32 / 100.0;
        *self.volume.lock().unwrap() = v;
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(v);
        }
        Ok(())
    }

    fn volume(&self) -> u32 {
        (*self.volume.lock().unwrap() * 100.0) as u32
    }

    fn set_speed(&self, speed: f32) -> Result<()> {
        *self.speed.lock().unwrap() = speed.clamp(0.1, 4.0);
        Ok(())
    }

    fn speed(&self) -> f32 {
        *self.speed.lock().unwrap()
    }

    fn set_pitch(&self, pitch: i32) -> Result<()> {
        *self.pitch.lock().unwrap() = pitch.clamp(-12, 12);
        Ok(())
    }

    fn pitch(&self) -> i32 {
        *self.pitch.lock().unwrap()
    }

    fn set_equalizer(&self, band: usize, gain: i32) -> Result<()> {
        let mut eq = self.eq_gains.lock().unwrap();
        if band < 10 {
            eq[band] = gain;
        }
        Ok(())
    }

    fn equalizer(&self) -> [i32; 10] {
        *self.eq_gains.lock().unwrap()
    }

    fn set_reverb(&self, _mix: u32, _time: u32) -> Result<()> {
        Ok(())
    }

    fn clear_reverb(&self) -> Result<()> {
        Ok(())
    }

    fn fft_data(&self) -> Vec<f32> {
        self.fft_data_with_size(512)
    }

    fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32> {
        let buffer = self.buffer.lock().unwrap();
        let channels = *self.channels.lock().unwrap();
        if buffer.is_empty() || channels == 0 {
            return vec![0.0; (fft_size / 2) as usize];
        }
        let pos = self.play_pos.load(Ordering::Relaxed) as usize;
        let half = fft_size as usize / 2;
        let mut fft = vec![0.0f32; half];

        for (i, fft_val) in fft.iter_mut().enumerate() {
            let idx = (pos + i) * channels;
            let sample = if idx + channels.min(2) <= buffer.len() {
                let mut s = 0.0f32;
                for ch in 0..channels.min(2) {
                    s += buffer[idx + ch];
                }
                s / channels.min(2) as f32
            } else {
                0.0
            };
            let window = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / half as f32).cos());
            *fft_val = sample * window;
        }

        let half_out = half / 2;
        let mut spectrum = vec![0.0f32; half_out];
        for (k, spectrum_val) in spectrum.iter_mut().enumerate() {
            let mut real = 0.0f32;
            let mut imag = 0.0f32;
            for (n, fft_val) in fft.iter().enumerate() {
                let angle = 2.0 * std::f32::consts::PI * k as f32 * n as f32 / half as f32;
                real += fft_val * angle.cos();
                imag -= fft_val * angle.sin();
            }
            *spectrum_val = (real * real + imag * imag).sqrt() / half as f32;
        }
        spectrum
    }

    fn song_is_over(&self) -> bool {
        let total = *self.total_frames.lock().unwrap();
        total > 0 && self.play_pos.load(Ordering::Relaxed) >= total.saturating_sub(1)
    }

    fn is_midi(&self) -> bool {
        false
    }
}
