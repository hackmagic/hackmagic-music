use crate::core::engine_trait::{EngineState, PlayerEngine};
use crate::error::{PlayerError, Result};
use std::sync::Mutex;
use std::time::Duration;

#[cfg(windows)]
extern "system" {
    fn mciSendStringW(
        lpszCommand: *const u16,
        lpszReturnString: *mut u16,
        cchReturn: u32,
        hwndCallback: *mut std::ffi::c_void,
    ) -> u32;
    fn mciGetErrorStringW(
        errorCode: u32,
        lpszErrorText: *mut u16,
        cchErrorText: u32,
    ) -> i32;
}

fn mci(command: &str) -> std::result::Result<String, String> {
    let wcmd: Vec<u16> = command.encode_utf16().chain(std::iter::once(0)).collect();
    let mut ret_buf = [0u16; 1024];
    unsafe {
        let rc = mciSendStringW(wcmd.as_ptr(), ret_buf.as_mut_ptr(), 1023, std::ptr::null_mut());
        if rc != 0 {
            let mut err_buf = [0u16; 512];
            mciGetErrorStringW(rc, err_buf.as_mut_ptr(), 512);
            let err_msg = String::from_utf16_lossy(&err_buf);
            let msg = err_msg.trim_matches('\0').to_string();
            Err(if msg.is_empty() {
                format!("MCI error {}", rc)
            } else {
                msg
            })
        } else {
            let end = ret_buf.iter().position(|&c| c == 0).unwrap_or(1023);
            Ok(String::from_utf16_lossy(&ret_buf[..end]))
        }
    }
}

fn mci_ok(command: &str) -> Result<()> {
    mci(command).map_err(|e| PlayerError::Playback(format!("MCI '{command}': {e}")))?;
    Ok(())
}

fn mci_value(command: &str) -> Result<u64> {
    let s = mci(command)
        .map_err(|e| PlayerError::Playback(format!("MCI '{command}': {e}")))?;
    s.trim().parse::<u64>()
        .map_err(|e| PlayerError::Playback(format!("MCI parse '{s}': {e}")))
}

const ALIAS: &str = "hm";

pub struct MciEngine {
    state: Mutex<EngineState>,
    duration: Mutex<Duration>,
    volume: Mutex<f32>,
    opened: Mutex<bool>,
}

unsafe impl Send for MciEngine {}
unsafe impl Sync for MciEngine {}

impl MciEngine {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(EngineState::Stopped),
            duration: Mutex::new(Duration::ZERO),
            volume: Mutex::new(0.8),
            opened: Mutex::new(false),
        }
    }
}

impl PlayerEngine for MciEngine {
    fn name(&self) -> &'static str {
        "MCI"
    }

    fn init(&self) -> Result<()> {
        tracing::info!("MCI engine initialized");
        Ok(())
    }

    fn uninit(&self) -> Result<()> {
        let _ = mci(&format!("close {ALIAS}"));
        *self.state.lock().unwrap() = EngineState::Stopped;
        *self.opened.lock().unwrap() = false;
        Ok(())
    }

    fn open(&self, path: &str) -> Result<()> {
        let _ = mci(&format!("close {ALIAS}"));
        let wide_path: Vec<u16> = path.encode_utf16().collect();
        let alias: Vec<u16> = ALIAS.encode_utf16().collect();
        let mut cmd = Vec::new();
        cmd.extend_from_slice(b"open \"".iter().map(|&b| b as u16).collect::<Vec<_>>().as_slice());
        cmd.extend_from_slice(&wide_path);
        cmd.extend_from_slice(b"\" alias ".iter().map(|&b| b as u16).collect::<Vec<_>>().as_slice());
        cmd.extend_from_slice(&alias);
        cmd.push(0);
        unsafe {
            let rc = mciSendStringW(cmd.as_ptr(), std::ptr::null_mut(), 0, std::ptr::null_mut());
            if rc != 0 {
                let mut err_buf = [0u16; 512];
                mciGetErrorStringW(rc, err_buf.as_mut_ptr(), 512);
                let err_msg = String::from_utf16_lossy(&err_buf).trim_matches('\0').to_string();
                return Err(PlayerError::CannotOpen(format!("{path}: MCI {err_msg}")));
            }
        }
        mci_ok(&format!("set {ALIAS} time format milliseconds"))?;
        let len_ms = mci_value(&format!("status {ALIAS} length")).unwrap_or(0);
        *self.duration.lock().unwrap() = Duration::from_millis(len_ms);
        *self.state.lock().unwrap() = EngineState::Stopped;
        *self.opened.lock().unwrap() = true;
        let vol = (*self.volume.lock().unwrap() * 1000.0) as u32;
        let _ = mci(&format!("setaudio {ALIAS} volume to {vol}"));
        tracing::info!("[MCI] opened {path} ({} ms)", len_ms);
        Ok(())
    }

    fn close(&self) -> Result<()> {
        let _ = mci(&format!("close {ALIAS}"));
        *self.state.lock().unwrap() = EngineState::Stopped;
        *self.opened.lock().unwrap() = false;
        Ok(())
    }

    fn play(&self) -> Result<()> {
        if !*self.opened.lock().unwrap() {
            return Err(PlayerError::NoTrack);
        }
        let state = *self.state.lock().unwrap();
        if state == EngineState::Playing {
            return Ok(());
        }
        if state == EngineState::Paused {
            mci_ok(&format!("resume {ALIAS}"))?;
            *self.state.lock().unwrap() = EngineState::Playing;
            return Ok(());
        }
        mci_ok(&format!("play {ALIAS}"))?;
        *self.state.lock().unwrap() = EngineState::Playing;
        Ok(())
    }

    fn pause(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        match *state {
            EngineState::Playing => {
                mci_ok(&format!("pause {ALIAS}"))?;
                *state = EngineState::Paused;
            }
            EngineState::Paused => {
                mci_ok(&format!("resume {ALIAS}"))?;
                *state = EngineState::Playing;
            }
            _ => {}
        }
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        mci_ok(&format!("stop {ALIAS}"))?;
        mci_ok(&format!("seek {ALIAS} to 0"))?;
        *self.state.lock().unwrap() = EngineState::Stopped;
        Ok(())
    }

    fn state(&self) -> EngineState {
        let state = *self.state.lock().unwrap();
        if state == EngineState::Playing {
            if let Ok(mode) = mci(&format!("status {ALIAS} mode")) {
                let mode = mode.trim();
                if mode == "stopped" || mode == "not ready" {
                    return EngineState::Stopped;
                }
                if mode == "paused" {
                    return EngineState::Paused;
                }
            }
        }
        state
    }

    fn position(&self) -> Duration {
        if !*self.opened.lock().unwrap() {
            return Duration::ZERO;
        }
        let ms = mci_value(&format!("status {ALIAS} position")).unwrap_or(0);
        Duration::from_millis(ms)
    }

    fn duration(&self) -> Duration {
        *self.duration.lock().unwrap()
    }

    fn seek(&self, pos: Duration) -> Result<()> {
        let ms = pos.as_millis();
        mci_ok(&format!("seek {ALIAS} to {ms}"))?;
        let was_playing = *self.state.lock().unwrap() == EngineState::Playing;
        if was_playing {
            self.play()?;
        }
        Ok(())
    }

    fn set_volume(&self, vol: u32) -> Result<()> {
        let v = vol.min(100) as f32 / 100.0;
        *self.volume.lock().unwrap() = v;
        if *self.opened.lock().unwrap() {
            let mci_vol = (v * 1000.0) as u32;
            let _ = mci(&format!("setaudio {ALIAS} volume to {mci_vol}"));
        }
        Ok(())
    }

    fn volume(&self) -> u32 {
        (*self.volume.lock().unwrap() * 100.0) as u32
    }

    fn set_speed(&self, _speed: f32) -> Result<()> {
        Ok(())
    }

    fn speed(&self) -> f32 {
        1.0
    }

    fn set_pitch(&self, _pitch: i32) -> Result<()> {
        Ok(())
    }

    fn pitch(&self) -> i32 {
        0
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
        vec![0.0; 256]
    }

    fn fft_data_with_size(&self, fft_size: u32) -> Vec<f32> {
        vec![0.0; (fft_size / 2) as usize]
    }

    fn song_is_over(&self) -> bool {
        let state = *self.state.lock().unwrap();
        if state != EngineState::Playing {
            return state == EngineState::Stopped;
        }
        if let Ok(mode) = mci(&format!("status {ALIAS} mode")) {
            mode.trim() == "stopped"
        } else {
            false
        }
    }

    fn is_midi(&self) -> bool {
        false
    }
}
