//! Safe Rust wrapper around the BASS audio engine.
//! Implements the `PlayerEngine` trait for the BASS backend.
#![allow(dead_code)]

use crate::core::engine_trait::{EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use crate::bass::sys;
use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Global WASAPI callback stream handle. Set by engine when starting playback.
static WASAPI_STREAM: AtomicU32 = AtomicU32::new(0);

/// WASAPI callback — called from WASAPI output thread to fill audio buffer.
unsafe extern "system" fn wasapi_proc(buffer: *mut c_void, length: u32, _user: *mut c_void) -> u32 {
    let handle = WASAPI_STREAM.load(Ordering::SeqCst);
    if handle == 0 {
        return 0;
    }
    sys::BASS_ChannelGetData(handle, buffer, length)
}

/// The BASS player engine implementation
pub struct BassEngine {
    /// Current stream handle (HSTREAM)
    stream: Mutex<u32>,
    /// Tempo stream handle (for speed/pitch)
    tempo_stream: Mutex<u32>,
    /// Current state
    state: Mutex<EngineState>,
    /// Equalizer FX handles [10 bands]
    eq_handles: Mutex<[u32; 10]>,
    /// Reverb FX handle
    reverb_handle: Mutex<u32>,
    /// Current device index
    _device: Mutex<u32>,
    /// Volume (0-100)
    volume: Mutex<f32>,
    /// Speed multiplier
    speed: Mutex<f32>,
    /// Pitch shift (semitones)
    pitch: Mutex<i32>,
    /// Fade in/out effect
    fade_effect: Mutex<bool>,
    /// Fade time in milliseconds
    fade_time: Mutex<u32>,
    /// `SoundFont` handle (0 = not loaded)
    midi_font: Mutex<u32>,
    /// Whether current file is MIDI
    is_midi_file: Mutex<bool>,
    /// `ReplayGain` dB adjustment (0.0 = off)
    replaygain_db: Mutex<f32>,
    /// Whether WASAPI output is active
    wasapi_active: Mutex<bool>,
}

unsafe impl Send for BassEngine {}
unsafe impl Sync for BassEngine {}

impl BassEngine {
    pub fn new() -> Self {
        Self {
            stream: Mutex::new(0),
            tempo_stream: Mutex::new(0),
            state: Mutex::new(EngineState::Stopped),
            eq_handles: Mutex::new([0; 10]),
            reverb_handle: Mutex::new(0),
            _device: Mutex::new(0),
            volume: Mutex::new(80.0),
            speed: Mutex::new(1.0),
            pitch: Mutex::new(0),
            fade_effect: Mutex::new(true),
            fade_time: Mutex::new(500),
            midi_font: Mutex::new(0),
            is_midi_file: Mutex::new(false),
            replaygain_db: Mutex::new(0.0),
            wasapi_active: Mutex::new(false),
        }
    }
}

impl PlayerEngine for BassEngine {
    fn name(&self) -> &'static str {
        "BASS"
    }

    fn init(&self) -> Result<()> {
        // Load BASS library
        if !sys::is_bass_loaded() && !sys::load_bass(None, None) {
            return Err(PlayerError::BassNotLoaded(
                "Failed to load BASS library. Make sure bass.dll/bass.so is in PATH.".into()
            ));
        }

        let version = sys::BASS_GetVersion();
        tracing::info!("BASS version: {}.{}.{}", 
            (version >> 24) & 0xFF, (version >> 16) & 0xFF, (version >> 8) & 0xFF);

        let cfg = crate::config::Config::load();
        let use_wasapi = cfg.play.output_mode != "directsound";
        let wasapi_exclusive = cfg.play.output_mode == "wasapi_exclusive";

        if use_wasapi {
            // Load BASSWASAPI plugin
            if sys::load_bass_wasapi(None) {
                // Initialize with WASAPI
                let device = if cfg.play.wasapi_device >= 0 { cfg.play.wasapi_device } else { -1 };
                let freq = 44100u32;
                let chans = 2u32;
                let flags = if wasapi_exclusive { sys::BASS_WASAPI_EXCLUSIVE } else { 0 };
                let buffer = 0.100f32;  // 100ms buffer
                let period = 0.025f32;  // 25ms period

                match sys::BASS_WASAPI_Init(device, freq, chans, flags, buffer, period, Some(wasapi_proc as sys::WASAPIPROC), std::ptr::null_mut()) {
                    Ok(()) => {
                        tracing::info!("BASSWASAPI initialized (exclusive={})", wasapi_exclusive);
                        *self.wasapi_active.lock().unwrap() = true;
                    }
                    Err(e) => {
                        tracing::warn!("BASSWASAPI_Init failed: {}, falling back to DirectSound", e);
                        *self.wasapi_active.lock().unwrap() = false;
                    }
                }
            } else {
                tracing::warn!("BASSWASAPI not available, falling back to DirectSound");
                *self.wasapi_active.lock().unwrap() = false;
                // Fall through to standard init
            }
        }

        if !*self.wasapi_active.lock().unwrap() {
            // Standard BASS initialization
            sys::BASS_Init(-1, 44100, sys::BASS_DEVICE_DEFAULT, std::ptr::null_mut(), std::ptr::null_mut())
                .map_err(|e| PlayerError::BassError(format!("BASS_Init failed: {e}")))?;
            sys::BASS_Start()
                .map_err(|e| PlayerError::BassError(format!("BASS_Start failed: {e}")))?;
            tracing::info!("BASS initialized successfully (DirectSound)");
        }

        // Try to load bassmidi.dll for MIDI playback
        sys::load_bass_midi(None);

        // Load SoundFont from config
        if cfg.midi.enabled && !cfg.midi.soundfont.is_empty() && sys::is_bass_midi_loaded() {
            let sf_path_wide: Vec<u16> = cfg.midi.soundfont.encode_utf16().chain(std::iter::once(0)).collect();
            match sys::BASS_MIDI_FontInit(sf_path_wide.as_ptr(), sys::BASS_MIDI_FONT_GM) {
                Ok(font) => {
                    let _ = sys::BASS_MIDI_FontLoad(font, -1, -1);
                    *self.midi_font.lock().unwrap() = font;
                    tracing::info!("SoundFont loaded: {}", cfg.midi.soundfont);
                }
                Err(e) => {
                    tracing::warn!("Failed to load SoundFont '{}': {}", cfg.midi.soundfont, e);
                }
            }
        }

        Ok(())
    }

    fn uninit(&self) -> Result<()> {
        self.close()?;
        // Free SoundFont
        let font = *self.midi_font.lock().unwrap();
        if font != 0 {
            let _ = sys::BASS_MIDI_FontFree(font);
        }
        if *self.wasapi_active.lock().unwrap() {
            sys::BASS_WASAPI_Free()
                .map_err(|e| PlayerError::BassError(format!("BASS_WASAPI_Free failed: {e}")))?;
        } else {
            sys::BASS_Free()
                .map_err(|e| PlayerError::BassError(format!("BASS_Free failed: {e}")))?;
        }
        Ok(())
    }

    fn open(&self, path: &str) -> Result<()> {
        tracing::info!("[BASS] open(\"{}\")", path);
        self.close()?;

        let is_midi = path.ends_with(".mid") || path.ends_with(".midi") || path.ends_with(".rmi")
            || path.ends_with(".MID") || path.ends_with(".MIDI") || path.ends_with(".RMI");
        let is_url = path.starts_with("http://") || path.starts_with("https://") || path.starts_with("ftp://") || path.starts_with("mms://");
        tracing::info!("[BASS] is_midi={}, is_url={}", is_midi, is_url);

        *self.is_midi_file.lock().unwrap() = is_midi && !is_url && sys::is_bass_midi_loaded();

        if *self.is_midi_file.lock().unwrap() {
            let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            let stream = sys::BASS_MIDI_StreamCreateFile(
                0,
                path_wide.as_ptr().cast::<c_void>(),
                0, 0,
                sys::BASS_UNICODE, // wide (UTF-16) path，否则中文路径打不开
                0, // default freq (44100)
            ).map_err(|e| PlayerError::CannotOpen(format!("Cannot open MIDI '{path}': {e}")))?;

            *self.stream.lock().unwrap() = stream;
            *self.tempo_stream.lock().unwrap() = stream; // same handle for MIDI

            // Set SoundFont on the stream
            let font = *self.midi_font.lock().unwrap();
            if font != 0 {
                let midi_font = sys::BASS_MIDI_FONT { font, preset: -1, bank: -1, flags: 0 };
                let _ = sys::BASS_MIDI_StreamSetFonts(stream, &raw const midi_font, 1);
            } else {
                tracing::warn!("No SoundFont loaded for MIDI playback");
            }

            // Set volume
            let vol = *self.volume.lock().unwrap();
            let _ = sys::BASS_ChannelSetAttribute(stream, sys::BASS_ATTRIB_VOL, vol / 100.0);
        } else {
            let flags = sys::BASS_STREAM_DECODE;

            let stream = if is_url {
                let url_bytes: Vec<u8> = path.bytes().chain(std::iter::once(0)).collect();
                sys::BASS_StreamCreateURL(
                    url_bytes.as_ptr(),
                    0,
                    flags,
                    None,
                    std::ptr::null_mut(),
                )
            } else {
                let path_wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
                sys::BASS_StreamCreateFile(
                    0,
                    path_wide.as_ptr().cast::<c_void>(),
                    0, 0,
                    flags | sys::BASS_UNICODE, // wide (UTF-16) path，否则中文路径打不开
                )
            }.map_err(|e| PlayerError::CannotOpen(format!("Cannot open '{path}': {e}")))?;

            *self.stream.lock().unwrap() = stream;

            let tempo_flags = sys::BASS_FX_FREESOURCE | sys::BASS_FX_TEMPO_ALGO_LINEAR;
            let tempo = sys::BASS_FX_TempoCreate(stream, tempo_flags)
                .map_err(|e| PlayerError::BassError(format!("Cannot create tempo stream: {e}")))?;

            *self.tempo_stream.lock().unwrap() = tempo;

            let vol = *self.volume.lock().unwrap();
            sys::BASS_ChannelSetAttribute(tempo, sys::BASS_ATTRIB_VOL, vol / 100.0)
                .map_err(|e| PlayerError::BassError(format!("Failed to set volume: {e}")))?;

            let speed = *self.speed.lock().unwrap();
            self.apply_speed(tempo, speed)?;

            let pitch = *self.pitch.lock().unwrap();
            self.apply_pitch(tempo, pitch)?;
        }

        // Apply ReplayGain dB
        let rg_db = *self.replaygain_db.lock().unwrap();
        if rg_db != 0.0 {
            let ch = *self.tempo_stream.lock().unwrap();
            let _ = sys::BASS_ChannelSetAttribute(ch, sys::BASS_ATTRIB_DB_GAIN, rg_db);
        }

        tracing::info!("Opened: {}", path);
        Ok(())
    }

    fn close(&self) -> Result<()> {
        let mut stream = self.stream.lock().unwrap();
        let mut tempo = self.tempo_stream.lock().unwrap();

        // Reset EQ and reverb handles (they're tied to the old stream)
        *self.eq_handles.lock().unwrap() = [0; 10];
        *self.reverb_handle.lock().unwrap() = 0;

        if *self.wasapi_active.lock().unwrap() {
            let _ = sys::BASS_WASAPI_Stop(1);
            WASAPI_STREAM.store(0, Ordering::SeqCst);
        }

        if *tempo != 0 {
            // Brief fade-out before closing (crossfade effect)
            if !*self.wasapi_active.lock().unwrap() && *self.fade_effect.lock().unwrap() {
                let fade_ms = (*self.fade_time.lock().unwrap()).min(200);
                let _ = sys::BASS_ChannelSlideAttribute(*tempo, sys::BASS_ATTRIB_VOL, 0.0, fade_ms);
                std::thread::sleep(std::time::Duration::from_millis(u64::from(fade_ms / 2)));
            }
            let _ = sys::BASS_StreamFree(*tempo);
            *tempo = 0;
        }
        if *stream != 0 {
            if *stream != *tempo {
                let _ = sys::BASS_StreamFree(*stream);
            }
            *stream = 0;
        }
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn play(&self) -> Result<()> {
        tracing::info!("[BASS] play() called");
        let tempo = *self.tempo_stream.lock().unwrap();
        tracing::info!("[BASS] tempo_stream handle = {}", tempo);
        if tempo == 0 {
            tracing::warn!("[BASS] play() -> NoTrack (tempo_stream == 0, open not called?)");
            return Err(PlayerError::NoTrack);
        }
        tracing::info!("[BASS] wasapi_active={}, fade_effect={}",
            *self.wasapi_active.lock().unwrap(),
            *self.fade_effect.lock().unwrap());

        if *self.wasapi_active.lock().unwrap() {
            // WASAPI mode: set stream handle and start output
            WASAPI_STREAM.store(tempo, Ordering::SeqCst);
            sys::BASS_WASAPI_Start()
                .map_err(|e| {
                    tracing::error!("[BASS] BASS_WASAPI_Start failed: {}", e);
                    PlayerError::Playback(format!("Cannot start WASAPI: {e}"))
                })?;
            tracing::info!("[BASS] WASAPI started");
        } else if *self.fade_effect.lock().unwrap() {
            let fade_ms = *self.fade_time.lock().unwrap();
            let target_vol = *self.volume.lock().unwrap() / 100.0;
            sys::BASS_ChannelSetAttribute(tempo, sys::BASS_ATTRIB_VOL, 0.0)
                .map_err(|e| PlayerError::Playback(format!("Cannot set volume: {e}")))?;
            sys::BASS_ChannelPlay(tempo, 0)
                .map_err(|e| PlayerError::Playback(format!("Cannot play: {e}")))?;
            sys::BASS_ChannelSlideAttribute(tempo, sys::BASS_ATTRIB_VOL, target_vol, fade_ms)
                .map_err(|e| PlayerError::Playback(format!("Cannot fade in: {e}")))?;
            tracing::info!("[BASS] fade-in play started ({} ms)", fade_ms);
        } else {
            sys::BASS_ChannelPlay(tempo, 0)
                .map_err(|e| {
                    tracing::error!("[BASS] BASS_ChannelPlay failed: {}", e);
                    PlayerError::Playback(format!("Cannot play: {e}"))
                })?;
            tracing::info!("[BASS] BASS_ChannelPlay ok");
        }
        *self.state.lock().unwrap() = EngineState::Playing;
        tracing::info!("[BASS] play() OK, state=Playing");
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return Err(PlayerError::NoTrack);
        }

        match *self.state.lock().unwrap() {
            EngineState::Playing => {
                if *self.wasapi_active.lock().unwrap() {
                    sys::BASS_WASAPI_Stop(0)
                        .map_err(|e| PlayerError::Playback(format!("Cannot pause WASAPI: {e}")))?;
                } else if *self.fade_effect.lock().unwrap() {
                    let fade_ms = *self.fade_time.lock().unwrap();
                    let tempo_c = tempo;
                    sys::BASS_ChannelSlideAttribute(tempo, sys::BASS_ATTRIB_VOL, 0.0, fade_ms)
                        .map_err(|e| PlayerError::Playback(format!("Cannot fade out: {e}")))?;
                    std::thread::spawn(move || {
                        std::thread::sleep(Duration::from_millis(u64::from(fade_ms) + 50));
                        let _ = sys::BASS_ChannelPause(tempo_c);
                    });
                } else {
                    sys::BASS_ChannelPause(tempo)
                        .map_err(|e| PlayerError::Playback(format!("Cannot pause: {e}")))?;
                }
                *self.state.lock().unwrap() = EngineState::Paused;
            }
            EngineState::Paused => {
                if *self.wasapi_active.lock().unwrap() {
                    WASAPI_STREAM.store(tempo, Ordering::SeqCst);
                    sys::BASS_WASAPI_Start()
                        .map_err(|e| PlayerError::Playback(format!("Cannot resume WASAPI: {e}")))?;
                } else {
                    if *self.fade_effect.lock().unwrap() {
                        let target_vol = *self.volume.lock().unwrap() / 100.0;
                        sys::BASS_ChannelSlideAttribute(tempo, sys::BASS_ATTRIB_VOL, target_vol, *self.fade_time.lock().unwrap())
                            .ok();
                    }
                    sys::BASS_ChannelPlay(tempo, 0)
                        .map_err(|e| PlayerError::Playback(format!("Cannot resume: {e}")))?;
                }
                *self.state.lock().unwrap() = EngineState::Playing;
            }
            _ => {}
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo != 0 {
            if *self.wasapi_active.lock().unwrap() {
                let _ = sys::BASS_WASAPI_Stop(1);
                WASAPI_STREAM.store(0, Ordering::SeqCst);
            } else if *self.fade_effect.lock().unwrap() && *self.state.lock().unwrap() == EngineState::Playing {
                let fade_ms = *self.fade_time.lock().unwrap();
                let tempo_c = tempo;
                let _ = sys::BASS_ChannelSlideAttribute(tempo, sys::BASS_ATTRIB_VOL, 0.0, fade_ms);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(u64::from(fade_ms) + 50));
                    let _ = sys::BASS_ChannelStop(tempo_c);
                });
            } else {
                let _ = sys::BASS_ChannelStop(tempo);
            }
        }
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn state(&self) -> EngineState {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return EngineState::Stopped;
        }
        if *self.wasapi_active.lock().unwrap() {
            if sys::BASS_WASAPI_IsStarted() != 0 && sys::BASS_ChannelIsActive(tempo) == sys::BASS_ACTIVE_PLAYING {
                EngineState::Playing
            } else if sys::BASS_WASAPI_IsStarted() == 0 && sys::BASS_ChannelIsActive(tempo) == sys::BASS_ACTIVE_PLAYING {
                EngineState::Paused
            } else {
                EngineState::Stopped
            }
        } else {
            let active = sys::BASS_ChannelIsActive(tempo);
            match active {
                sys::BASS_ACTIVE_PLAYING => EngineState::Playing,
                sys::BASS_ACTIVE_PAUSED | sys::BASS_ACTIVE_PAUSED_DEVICE => EngineState::Paused,
                _ => EngineState::Stopped,
            }
        }
    }

    fn position(&self) -> Duration {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return Duration::ZERO;
        }
        let bytes = sys::BASS_ChannelGetPosition(tempo, sys::BASS_POS_BYTE);
        let secs = sys::BASS_ChannelBytes2Seconds(tempo, bytes);
        Duration::from_secs_f64(secs)
    }

    fn duration(&self) -> Duration {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return Duration::ZERO;
        }
        // Get length from the original stream
        let stream = *self.stream.lock().unwrap();
        if stream == 0 {
            return Duration::ZERO;
        }
        let bytes = sys::BASS_ChannelGetLength(stream, sys::BASS_POS_BYTE);
        let secs = sys::BASS_ChannelBytes2Seconds(stream, bytes);
        Duration::from_secs_f64(secs)
    }

    fn seek(&self, pos: Duration) -> Result<()> {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return Err(PlayerError::NoTrack);
        }
        let secs = pos.as_secs_f64();
        let bytes = sys::BASS_ChannelSeconds2Bytes(tempo, secs);
        sys::BASS_ChannelSetPosition(tempo, bytes, sys::BASS_POS_BYTE)
            .map_err(|e| PlayerError::Playback(format!("Seek failed: {e}")))
    }

    fn set_volume(&self, vol: u32) -> Result<()> {
        let vol = vol.clamp(0, 100) as f32 / 100.0;
        *self.volume.lock().unwrap() = vol * 100.0;
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo != 0 {
            sys::BASS_ChannelSetAttribute(tempo, sys::BASS_ATTRIB_VOL, vol)
                .map_err(|e| PlayerError::BassError(format!("Set volume failed: {e}")))?;
        }
        Ok(())
    }

    fn volume(&self) -> u32 {
        (*self.volume.lock().unwrap() as u32).min(100)
    }

    fn set_speed(&self, speed: f32) -> Result<()> {
        let speed = speed.clamp(0.1, 4.0);
        *self.speed.lock().unwrap() = speed;
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo != 0 {
            self.apply_speed(tempo, speed)?;
        }
        Ok(())
    }

    fn speed(&self) -> f32 {
        *self.speed.lock().unwrap()
    }

    fn set_pitch(&self, pitch: i32) -> Result<()> {
        let pitch = pitch.clamp(-12, 12);
        *self.pitch.lock().unwrap() = pitch;
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo != 0 {
            self.apply_pitch(tempo, pitch)?;
        }
        Ok(())
    }

    fn pitch(&self) -> i32 {
        *self.pitch.lock().unwrap()
    }

    fn set_equalizer(&self, band: usize, gain: i32) -> Result<()> {
        if band > 9 {
            return Err(PlayerError::BassError("Equalizer band out of range (0-9)".into()));
        }
        if *self.is_midi_file.lock().unwrap() {
            return Ok(()); // MIDI streams don't support DSP FX
        }
        let stream = *self.stream.lock().unwrap();
        if stream == 0 {
            return Ok(()); // Silent ignore if no stream
        }

        let gain_f32 = (gain as f32).clamp(-15.0, 15.0);

        // Get or create FX handle for this band
        let mut handles = self.eq_handles.lock().unwrap();
        if handles[band] == 0 {
            // Create a new peak EQ effect for this band
            let fx_handle = sys::BASS_ChannelSetFX(stream, sys::BASS_FX_BFX_PEAKEQ, 0)
                .map_err(|e| PlayerError::BassError(format!("Cannot create EQ FX: {e}")))?;
            handles[band] = fx_handle;
        }

        let mut params = sys::BASS_BFX_PEAKEQ {
            lBand: band as i32,
            fBandwidth: 0.5,  // 0.5 octave
            fGain: gain_f32,
            fQ: 1.0,
            lChannel: 0,  // Both channels
        };

        sys::BASS_FXSetParameters(handles[band], (&raw mut params).cast::<c_void>())
            .map_err(|e| PlayerError::BassError(format!("Cannot set EQ parameters: {e}")))?;

        Ok(())
    }

    fn equalizer(&self) -> [i32; 10] {
        let mut gains = [0i32; 10];
        if *self.is_midi_file.lock().unwrap() {
            return gains;
        }
        let stream = *self.stream.lock().unwrap();
        if stream == 0 {
            return gains;
        }
        let handles = self.eq_handles.lock().unwrap();
        for (i, handle) in handles.iter().enumerate() {
            if *handle == 0 {
                continue;
            }
            let mut params = sys::BASS_BFX_PEAKEQ {
                lBand: i as i32,
                fBandwidth: 0.5,
                fGain: 0.0,
                fQ: 1.0,
                lChannel: 0,
            };
            if sys::BASS_FXGetParameters(*handle, (&raw mut params).cast::<c_void>()).is_ok() {
                gains[i] = params.fGain.round() as i32;
            }
        }
        gains
    }

    fn set_reverb(&self, mix: u32, time: u32) -> Result<()> {
        if *self.is_midi_file.lock().unwrap() {
            return Ok(());
        }
        let stream = *self.stream.lock().unwrap();
        if stream == 0 {
            return Ok(());
        }

        let mut rev_handle = self.reverb_handle.lock().unwrap();
        if *rev_handle == 0 {
            *rev_handle = sys::BASS_ChannelSetFX(stream, sys::BASS_DX8_REVERB, 1)
                .map_err(|e| PlayerError::BassError(format!("Cannot create reverb FX: {e}")))?;
        }

        // Convert mix (0-100) to dB (-96 to 0)
        let mix_db = if mix == 0 { -96.0 } else { (mix as f32 / 100.0) * 20.0 - 30.0 };
        let time_ms = (time as f32).clamp(1.0, 3000.0);

        let params = sys::BASS_DX8_REVERB_PARAMETERS {
            fInGain: 0.0,
            fReverbMix: mix_db,
            fReverbTime: time_ms,
            fHighFreqRTRatio: 0.001,
        };

        sys::BASS_FXSetParameters(*rev_handle, (&raw const params).cast::<c_void>())
            .map_err(|e| PlayerError::BassError(format!("Cannot set reverb parameters: {e}")))?;

        Ok(())
    }

    fn clear_reverb(&self) -> Result<()> {
        if *self.is_midi_file.lock().unwrap() {
            return Ok(());
        }
        let stream = *self.stream.lock().unwrap();
        let rev_handle = *self.reverb_handle.lock().unwrap();
        if stream != 0 && rev_handle != 0 {
            let _ = sys::BASS_ChannelRemoveFX(stream, rev_handle);
        }
        *self.reverb_handle.lock().unwrap() = 0;
        Ok(())
    }

    fn fft_data(&self) -> Vec<f32> {
        self.fft_data_with_size(512)
    }

    fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32> {
        let tempo = *self.tempo_stream.lock().unwrap();
        if tempo == 0 {
            return vec![0.0; (fft_size / 2) as usize];
        }

        let flag = match fft_size {
            256 => sys::BASS_DATA_FFT256,
            1024 => sys::BASS_DATA_FFT1024,
            2048 => sys::BASS_DATA_FFT2048,
            _ => sys::BASS_DATA_FFT512,
        };
        let half = (fft_size / 2) as usize;
        let mut fft = vec![0.0f32; half];
        sys::BASS_ChannelGetData(tempo, fft.as_mut_ptr().cast::<c_void>(), flag);
        fft
    }

    fn song_is_over(&self) -> bool {
        let tempo = *self.tempo_stream.lock().unwrap();
        tempo != 0 && sys::BASS_ChannelIsActive(tempo) == sys::BASS_ACTIVE_STOPPED
    }

    fn is_midi(&self) -> bool {
        *self.is_midi_file.lock().unwrap()
    }

    fn set_replaygain(&self, gain_db: f32) {
        BassEngine::set_replaygain(self, gain_db);
    }

    fn crossfade_out(&self, time_ms: u32) {
        let ch = *self.tempo_stream.lock().unwrap();
        if ch == 0 { return; }
        let _ = sys::BASS_ChannelSlideAttribute(ch, sys::BASS_ATTRIB_VOL, 0.0, time_ms);
    }
}

impl BassEngine {
    fn apply_speed(&self, tempo: Dword, speed: f32) -> Result<()> {
        let tempo_attr = (speed - 1.0) * 100.0; // BASS tempo: 0 = normal, -95 = 5% speed, +5000 = 50x
        sys::BASS_ChannelSetAttribute(tempo, sys::BASS_ATTRIB_TEMPO, tempo_attr)
            .map_err(|e| PlayerError::BassError(format!("Set speed failed: {e}")))
    }

    fn apply_pitch(&self, tempo: Dword, pitch: i32) -> Result<()> {
        let pitch_attr = pitch as f32;
        sys::BASS_ChannelSetAttribute(tempo, sys::BASS_ATTRIB_TEMPO_PITCH, pitch_attr)
            .map_err(|e| PlayerError::BassError(format!("Set pitch failed: {e}")))
    }

    pub fn set_fade(&self, enabled: bool, time_ms: u32) {
        *self.fade_effect.lock().unwrap() = enabled;
        *self.fade_time.lock().unwrap() = time_ms;
    }

    /// Set `ReplayGain` adjustment (dB). Call before or during playback.
    pub fn set_replaygain(&self, gain_db: f32) {
        *self.replaygain_db.lock().unwrap() = gain_db;
        let ch = *self.tempo_stream.lock().unwrap();
        if ch != 0 {
            let _ = sys::BASS_ChannelSetAttribute(ch, sys::BASS_ATTRIB_DB_GAIN, gain_db);
        }
    }
}

impl BassEngine {
    /// List available audio output devices
    pub fn list_devices() -> Vec<(i32, String)> {
        let mut devices = Vec::new();
        let mut info = sys::BASS_DEVICEINFO {
            name: std::ptr::null(),
            driver: std::ptr::null(),
            flags: 0,
        };
        let mut idx: u32 = 0;
        while sys::BASS_GetDeviceInfo(idx, &raw mut info) {
            if !info.name.is_null() {
                let name = String::from_utf16_lossy(
                    unsafe { std::slice::from_raw_parts(info.name, 256) }
                ).trim_matches('\0').to_string();
                devices.push((idx as i32, name));
            }
            idx += 1;
        }
        devices
    }
}

type Dword = u32;

impl Drop for BassEngine {
    fn drop(&mut self) {
        let _ = self.uninit();
    }
}
