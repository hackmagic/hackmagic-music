use crate::core::engine_trait::{EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::io::Read;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct FfmpegEngine {
    buffer: Mutex<Vec<f32>>,
    sample_rate: Mutex<u32>,
    channels: Mutex<usize>,
    total_frames: Mutex<u64>,
    state: Mutex<EngineState>,
    play_pos: AtomicU64,
    volume: Mutex<f32>,
    speed: Mutex<f32>,
    pitch: Mutex<i32>,
    output_stream: Mutex<Option<cpal::Stream>>,
    paused_flag: AtomicBool,
}

unsafe impl Send for FfmpegEngine {}
unsafe impl Sync for FfmpegEngine {}

impl FfmpegEngine {
    pub fn new() -> Self {
        Self {
            buffer: Mutex::new(Vec::new()),
            sample_rate: Mutex::new(44100),
            channels: Mutex::new(2),
            total_frames: Mutex::new(0),
            state: Mutex::new(EngineState::Stopped),
            play_pos: AtomicU64::new(0),
            volume: Mutex::new(0.8),
            speed: Mutex::new(1.0),
            pitch: Mutex::new(0),
            output_stream: Mutex::new(None),
            paused_flag: AtomicBool::new(false),
        }
    }

    fn probe_audio(path: &str) -> Result<(u32, usize, f64)> {
        tracing::info!("[FFMPEG] probe_audio(\"{}\")", path);
        let out = std::process::Command::new("ffprobe")
            .args([
                "-v", "quiet",
                "-print_format", "json",
                "-show_streams",
                "-select_streams", "a:0",
                path,
            ])
            .output()
            .map_err(|e| PlayerError::Other(format!("Cannot run ffprobe: {e}")))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::error!("[FFMPEG] ffprobe failed: {}", stderr);
            return Err(PlayerError::Other("ffprobe failed - is FFmpeg installed?".into()));
        }

        let json: serde_json::Value = serde_json::from_slice(&out.stdout)
            .map_err(|e| PlayerError::Other(format!("Cannot parse ffprobe output: {e}")))?;

        let streams = json["streams"].as_array().and_then(|a| a.first())
            .ok_or_else(|| PlayerError::Other("No audio stream found".into()))?;

        let sample_rate: u32 = streams["sample_rate"].as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(44100);

        let channels: usize = streams["channels"].as_i64()
            .unwrap_or(2) as usize;

        let duration: f64 = streams["duration"].as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        Ok((sample_rate, channels, duration))
    }

    fn decode_to_buffer(path: &str) -> Result<Vec<f32>> {
        tracing::info!("[FFMPEG] decode_to_buffer(\"{}\")", path);
        let mut child = std::process::Command::new("ffmpeg")
            .args(["-i", path, "-f", "f32le", "-acodec", "pcm_f32le", "-"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                tracing::error!("[FFMPEG] cannot spawn ffmpeg: {}", e);
                PlayerError::Other(format!("Cannot spawn ffmpeg: {e}"))
            })?;

        let mut stdout = child.stdout.take()
            .ok_or_else(|| PlayerError::Other("Cannot capture ffmpeg output".into()))?;
        let mut stderr = child.stderr.take()
            .ok_or_else(|| PlayerError::Other("Cannot capture ffmpeg stderr".into()))?;

        let mut raw = Vec::new();
        stdout.read_to_end(&mut raw)
            .map_err(|e| PlayerError::Other(format!("Cannot read ffmpeg output: {e}")))?;
        let mut stderr_buf = String::new();
        let _ = stderr.read_to_string(&mut stderr_buf);

        let status = child.wait()
            .map_err(|e| PlayerError::Other(format!("Cannot wait ffmpeg: {e}")))?;

        if !status.success() {
            tracing::error!("[FFMPEG] ffmpeg decoding failed (status={}): {}", status, stderr_buf.trim());
            return Err(PlayerError::Other("ffmpeg decoding failed - is FFmpeg installed?".into()));
        }
        tracing::info!("[FFMPEG] decode done, {} raw bytes", raw.len());

        let sample_count = raw.len() / 4;
        let mut samples = Vec::with_capacity(sample_count);
        for chunk in raw.chunks_exact(4) {
            samples.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(samples)
    }
}

impl PlayerEngine for FfmpegEngine {
    fn name(&self) -> &'static str {
        "FFmpeg"
    }

    fn init(&self) -> Result<()> {
        let ffmpeg_ok = std::process::Command::new("ffmpeg")
            .arg("-version").output().map(|o| o.status.success()).unwrap_or(false);
        let ffprobe_ok = std::process::Command::new("ffprobe")
            .arg("-version").output().map(|o| o.status.success()).unwrap_or(false);

        if !ffmpeg_ok || !ffprobe_ok {
            return Err(PlayerError::Other(
                "ffmpeg/ffprobe not found in PATH. Please install FFmpeg.".into()
            ));
        }
        tracing::info!("FFmpeg engine initialized (ffmpeg subprocess + cpal)");
        Ok(())
    }

    fn uninit(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn open(&self, path: &str) -> Result<()> {
        tracing::info!("[FFMPEG] open(\"{}\")", path);
        *self.output_stream.lock().unwrap() = None;
        self.paused_flag.store(false, Ordering::SeqCst);
        self.play_pos.store(0, Ordering::SeqCst);

        let (sample_rate, channels, duration) = Self::probe_audio(path).map_err(|e| {
            tracing::error!("[FFMPEG] probe_audio failed for \"{}\": {}", path, e);
            e
        })?;
        tracing::info!("[FFMPEG] probe ok: {} Hz, {} ch, {:.2}s", sample_rate, channels, duration);
        *self.sample_rate.lock().unwrap() = sample_rate;
        *self.channels.lock().unwrap() = channels;

        let samples = Self::decode_to_buffer(path).map_err(|e| {
            tracing::error!("[FFMPEG] decode_to_buffer failed for \"{}\": {}", path, e);
            e
        })?;
        tracing::info!("[FFMPEG] decoded {} samples", samples.len());
        let total_frames = if channels > 0 { samples.len() / channels } else { 0 };
        let total_dur_frames = (duration * f64::from(sample_rate)) as u64;

        *self.total_frames.lock().unwrap() = if total_dur_frames > 0 { total_dur_frames } else { total_frames as u64 };
        *self.buffer.lock().unwrap() = samples;
        *self.state.lock().unwrap() = EngineState::Stopped;

        tracing::info!("Decoded via ffmpeg: {} ({} frames, {} Hz, {} ch)", path, total_frames, sample_rate, channels);
        Ok(())
    }

    fn close(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        self.paused_flag.store(false, Ordering::SeqCst);
        self.play_pos.store(0, Ordering::SeqCst);
        *self.buffer.lock().unwrap() = Vec::new();
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn play(&self) -> Result<()> {
        tracing::info!("[FFMPEG] play() state={:?}", *self.state.lock().unwrap());
        let state = *self.state.lock().unwrap();
        if state == EngineState::Paused {
            self.paused_flag.store(false, Ordering::SeqCst);
            *self.state.lock().unwrap() = EngineState::Playing;
            tracing::info!("[FFMPEG] resumed from paused");
            return Ok(());
        }
        if state == EngineState::Playing {
            tracing::info!("[FFMPEG] already playing, no-op");
            return Ok(());
        }

        let buf_data = self.buffer.lock().unwrap().clone();
        if buf_data.is_empty() {
            tracing::warn!("[FFMPEG] play() -> NoTrack (buffer empty, open not called?)");
            return Err(PlayerError::NoTrack);
        }
        tracing::info!("[FFMPEG] buffer ready, {} samples", buf_data.len());
        let ch = *self.channels.lock().unwrap();
        let total = *self.total_frames.lock().unwrap();
        let init_vol = *self.volume.lock().unwrap();
        let init_speed = *self.speed.lock().unwrap();
        tracing::info!("[FFMPEG] config: ch={}, total_frames={}, vol={}, speed={}", ch, total, init_vol, init_speed);

        let buf = Arc::new(Mutex::new(buf_data));
        let pos = Arc::new(AtomicU64::new(self.play_pos.load(Ordering::SeqCst)));
        let pos_c = pos.clone();
        let total_c = Arc::new(AtomicU64::new(total));
        let vol = Arc::new(AtomicU64::new((init_vol * 100.0) as u64));
        let vol_c = vol.clone();
        let spd = Arc::new(AtomicU64::new((init_speed * 1000.0) as u64));
        let spd_c = spd.clone();
        let paused = Arc::new(AtomicBool::new(false));
        let paused_c = paused.clone();
        let ch_arc = ch;

        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| PlayerError::Other("No audio output device found".into()))?;
        let config = device.default_output_config()
            .map_err(|e| PlayerError::Other(format!("Cannot get output config: {e}")))?;

        let err_fn = move |err| {
            tracing::error!("cpal stream error: {}", err);
        };

        let config_clone = config.config();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_output_stream(
                    &config_clone,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if paused_c.load(Ordering::Relaxed) {
                            data.fill(0.0);
                            return;
                        }
                        let speed_f = spd_c.load(Ordering::Relaxed) as f64 / 1000.0;
                        let volume_f = vol_c.load(Ordering::Relaxed) as f64 / 100.0;
                        let cbuf = buf.lock().unwrap();
                        if cbuf.is_empty() { return; }
                        let frames_needed = data.len() / ch_arc;
                        let mut current_f = pos_c.load(Ordering::Relaxed) as f64;
                        let total_f = total_c.load(Ordering::Relaxed) as f64;
                        for frame_out in 0..frames_needed {
                            if current_f >= total_f { break; }
                            for ch_idx in 0..ch_arc {
                                let idx_f = current_f * ch_arc as f64 + ch_idx as f64;
                                let idx = idx_f as usize;
                                let frac = idx_f - idx as f64;
                                let next_idx = (idx + ch_arc).min(cbuf.len().saturating_sub(1));
                                let curr = if idx < cbuf.len() { f64::from(cbuf[idx]) } else { 0.0 };
                                let nxt = if next_idx < cbuf.len() && next_idx != idx { f64::from(cbuf[next_idx]) } else { curr };
                                data[frame_out * ch_arc + ch_idx] = ((curr * (1.0 - frac) + nxt * frac) * volume_f) as f32;
                            }
                            current_f += speed_f;
                        }
                        let start = data.len().min(frames_needed * ch_arc);
                        data[start..].fill(0.0);
                        pos_c.store(current_f.min(total_f) as u64, Ordering::Relaxed);
                    },
                    err_fn,
                    None,
                )
            }
            _ => {
                device.build_output_stream(
                    &config_clone,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        data.fill(0.0);
                    },
                    err_fn,
                    None,
                )
            }
        }
        .map_err(|e| PlayerError::Other(format!("Cannot build output stream: {e}")))?;

        stream.play()
            .map_err(|e| PlayerError::Other(format!("Cannot start stream: {e}")))?;

        *self.output_stream.lock().unwrap() = Some(stream);
        *self.state.lock().unwrap() = EngineState::Playing;
        self.paused_flag.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        match *state {
            EngineState::Playing => {
                self.paused_flag.store(true, Ordering::SeqCst);
                *state = EngineState::Paused;
            }
            EngineState::Paused => {
                self.paused_flag.store(false, Ordering::SeqCst);
                *state = EngineState::Playing;
            }
            _ => {}
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        *self.output_stream.lock().unwrap() = None;
        self.paused_flag.store(false, Ordering::SeqCst);
        self.play_pos.store(0, Ordering::SeqCst);
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn state(&self) -> EngineState {
        *self.state.lock().unwrap()
    }

    fn position(&self) -> Duration {
        let fps = f64::from(*self.sample_rate.lock().unwrap());
        if fps <= 0.0 { return Duration::ZERO; }
        Duration::from_secs_f64(self.play_pos.load(Ordering::Relaxed) as f64 / fps)
    }

    fn duration(&self) -> Duration {
        let total = *self.total_frames.lock().unwrap() as f64;
        let fps = f64::from(*self.sample_rate.lock().unwrap());
        if fps <= 0.0 { return Duration::ZERO; }
        Duration::from_secs_f64(total / fps)
    }

    fn seek(&self, pos: Duration) -> Result<()> {
        let fps = f64::from(*self.sample_rate.lock().unwrap());
        if fps <= 0.0 { return Err(PlayerError::NoTrack); }
        let target = ((pos.as_secs_f64() * fps) as u64)
            .min(self.total_frames.lock().unwrap().saturating_sub(1));
        self.play_pos.store(target, Ordering::SeqCst);
        Ok(())
    }

    fn set_volume(&self, vol: u32) -> Result<()> {
        *self.volume.lock().unwrap() = vol.min(100) as f32 / 100.0;
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

    fn set_equalizer(&self, _band: usize, _gain: i32) -> Result<()> {
        Ok(())
    }

    fn equalizer(&self) -> [i32; 10] {
        [0; 10]
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
            let buf_idx = pos + i;
            let idx = buf_idx * channels;
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

        let mut spectrum = vec![0.0f32; half / 2];
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
        let total = *self.total_frames.lock().unwrap() as f64;
        total > 0.0 && self.play_pos.load(Ordering::Relaxed) as f64 >= total - 1.0
    }

    fn is_midi(&self) -> bool {
        false
    }
}
