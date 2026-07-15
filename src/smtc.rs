//! Windows SMTC integration via `ISystemMediaTransportControlsInterop`.
//! Works in Win32 desktop (console) apps without a `CoreWindow`.
//! Handles media button presses (Play/Pause/Next/Prev/Stop).
//! Also monitors Bluetooth audio device connections for auto-pause.

#![allow(non_snake_case)]

use std::collections::HashSet;
use std::sync::mpsc;

use windows::core::{GUID, Result, HSTRING, Error, HRESULT, Interface, PCSTR, PCWSTR};
use windows::Foundation::EventRegistrationToken;
use windows::Foundation::TypedEventHandler;
use windows::Media::{SystemMediaTransportControls, SystemMediaTransportControlsButtonPressedEventArgs, SystemMediaTransportControlsButton, MediaPlaybackStatus, MediaPlaybackType, ISystemMediaTransportControls};
use windows::Win32::Foundation::{HWND, WPARAM, LPARAM, LRESULT, HINSTANCE};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::Win32::System::LibraryLoader::{LoadLibraryW, GetProcAddress, GetModuleHandleA};
use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, WNDCLASSW, WNDCLASS_STYLES, HICON, HCURSOR, RegisterClassW, CreateWindowExW, WINDOW_EX_STYLE, WINDOW_STYLE, HWND_MESSAGE};

use crate::core::player::Player;

/// Button press events received from SMTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtcButtonEvent {
    Play,
    Pause,
    Stop,
    Next,
    Previous,
}

/// Bluetooth device connection events.
#[derive(Debug, Clone)]
pub enum BluetoothEvent {
    Connected { name: String, device_id: u32 },
    Disconnected { name: String, device_id: u32 },
}

/// Information about a Bluetooth audio device.
#[derive(Debug, Clone)]
pub struct BluetoothDevice {
    pub device_id: u32,
    pub name: String,
    pub enabled: bool,
    pub is_default: bool,
}

// ISystemMediaTransportControlsInterop
const IID_INTEROP: GUID = GUID::from_u128(0xdd7c0918_5f86_4c09_a0f3_e0d9b6a0e0b8);

unsafe extern "system" fn smtc_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

pub struct SmtcManager {
    smtc: SystemMediaTransportControls,
    _hwnd: HWND,
    _token: EventRegistrationToken,
    rx: mpsc::Receiver<SmtcButtonEvent>,
    bt_monitor: BluetoothMonitor,
}

unsafe impl Send for SmtcManager {}
unsafe impl Sync for SmtcManager {}

impl SmtcManager {
    pub fn new() -> Result<Self> {
        let hwnd = create_hidden_window()?;
        let smtc = get_smtc_for_window(hwnd)?;

        smtc.SetIsPlayEnabled(true)?;
        smtc.SetIsPauseEnabled(true)?;
        smtc.SetIsNextEnabled(true)?;
        smtc.SetIsPreviousEnabled(true)?;
        smtc.SetIsStopEnabled(true)?;
        smtc.SetIsEnabled(true)?;

        // Subscribe to ButtonPressed events
        let (tx, rx) = mpsc::channel::<SmtcButtonEvent>();
        let handler = TypedEventHandler::new(
            move |_sender: &Option<SystemMediaTransportControls>,
                  args: &Option<SystemMediaTransportControlsButtonPressedEventArgs>| {
                if let Some(args) = args {
                    let button = args.Button()?;
                    let event = match button {
                        SystemMediaTransportControlsButton::Play => SmtcButtonEvent::Play,
                        SystemMediaTransportControlsButton::Pause => SmtcButtonEvent::Pause,
                        SystemMediaTransportControlsButton::Stop => SmtcButtonEvent::Stop,
                        SystemMediaTransportControlsButton::Next => SmtcButtonEvent::Next,
                        SystemMediaTransportControlsButton::Previous => SmtcButtonEvent::Previous,
                        _ => return Ok(()),
                    };
                    let _ = tx.send(event);
                }
                Ok(())
            },
        );
        let token = smtc.ButtonPressed(&handler)?;

        tracing::info!("SMTC initialized with ButtonPressed handler");

        let bt_monitor = BluetoothMonitor::new();
        tracing::info!("Bluetooth monitor initialized");
        Ok(Self { smtc, _hwnd: hwnd, _token: token, rx, bt_monitor })
    }

    /// Poll for the latest button event (non-blocking).
    /// Returns `None` if no event is pending.
    pub fn poll_event(&self) -> Option<SmtcButtonEvent> {
        self.rx.try_recv().ok()
    }

    pub fn update(&self, player: &Player) -> Result<()> {
        use crate::core::engine_trait::EngineState;
        let status = match player.state() {
            EngineState::Playing => MediaPlaybackStatus::Playing,
            EngineState::Paused => MediaPlaybackStatus::Paused,
            EngineState::Stopped => MediaPlaybackStatus::Stopped,
        };
        self.smtc.SetPlaybackStatus(status)?;

        let playlist = player.playlist_mut();
        if let Some(track) = playlist.current_track() {
            let updater = self.smtc.DisplayUpdater()?;
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            if !track.title.is_empty() {
                music.SetTitle(&HSTRING::from(&track.title))?;
            }
            if !track.artist.is_empty() {
                music.SetArtist(&HSTRING::from(&track.artist))?;
            }
            if !track.album.is_empty() {
                music.SetAlbumTitle(&HSTRING::from(&track.album))?;
            }
            updater.Update()?;
        }
        Ok(())
    }

    /// Poll for the latest Bluetooth connection event (non-blocking).
    /// Returns `None` if no event is pending.
    pub fn poll_bluetooth_event(&mut self) -> Option<BluetoothEvent> {
        self.bt_monitor.poll_event()
    }

    /// Get the list of currently connected Bluetooth audio devices.
    pub fn bluetooth_devices(&self) -> Vec<BluetoothDevice> {
        self.bt_monitor.devices()
    }

    /// Returns true if any Bluetooth audio device is currently connected.
    pub fn has_bluetooth_device(&self) -> bool {
        self.bt_monitor.has_connected_device()
    }
}

/// Monitors Bluetooth audio devices by scanning BASS output devices.
/// Bluetooth devices typically show up in BASS device list with names
/// containing "Bluetooth" or "A2DP".
pub struct BluetoothMonitor {
    connected: HashSet<u32>,
    events: mpsc::Receiver<BluetoothEvent>,
    event_tx: mpsc::Sender<BluetoothEvent>,
}

impl BluetoothMonitor {
    /// Create a new `BluetoothMonitor`. Starts with an initial scan.
    pub fn new() -> Self {
        let (event_tx, events) = mpsc::channel();
        let connected = Self::scan_bluetooth_devices();
        Self { connected, events, event_tx }
    }

    /// Poll for the latest Bluetooth connection event (non-blocking).
    pub fn poll_event(&mut self) -> Option<BluetoothEvent> {
        self.events.try_recv().ok()
    }

    /// Get currently connected Bluetooth device IDs.
    pub fn devices(&self) -> Vec<BluetoothDevice> {
        let mut devices = Vec::new();
        let count = crate::bass::sys::BASS_GetDeviceCount();
        let current_dev = crate::bass::sys::BASS_GetDevice();
        for i in 0..count {
            let name = match crate::bass::sys::get_device_name(i) {
                Some(n) => n,
                None => continue,
            };
            if Self::is_bluetooth_device(&name) {
                let mut info = crate::bass::sys::BASS_DEVICEINFO {
                    name: std::ptr::null(),
                    driver: std::ptr::null(),
                    flags: 0,
                };
                let enabled = crate::bass::sys::BASS_GetDeviceInfo(i, &raw mut info)
                    && (info.flags & crate::bass::sys::BASS_DEVICE_ENABLED) != 0;
                devices.push(BluetoothDevice {
                    device_id: i,
                    name,
                    enabled,
                    is_default: i == current_dev,
                });
            }
        }
        devices
    }

    /// Returns true if any Bluetooth audio device is currently connected.
    pub fn has_connected_device(&self) -> bool {
        !self.connected.is_empty()
    }

    /// Update the Bluetooth device set by re-scanning.
    /// Detects new connections and disconnections.
    pub fn update(&mut self) {
        let current = Self::scan_bluetooth_devices();
        let prev = &self.connected;

        // Detect disconnections
        for &id in prev.difference(&current) {
            if let Some(name) = crate::bass::sys::get_device_name(id) {
                tracing::info!("Bluetooth device disconnected: {} (id={})", name, id);
                let _ = self.event_tx.send(BluetoothEvent::Disconnected {
                    name: name.clone(),
                    device_id: id,
                });
            }
        }

        // Detect new connections
        for &id in current.difference(prev) {
            if let Some(name) = crate::bass::sys::get_device_name(id) {
                tracing::info!("Bluetooth device connected: {} (id={})", name, id);
                let _ = self.event_tx.send(BluetoothEvent::Connected {
                    name: name.clone(),
                    device_id: id,
                });
            }
        }

        self.connected = current;
    }

    /// Scan all BASS audio devices and return IDs of Bluetooth devices.
    fn scan_bluetooth_devices() -> HashSet<u32> {
        let mut bt_devices = HashSet::new();
        let count = crate::bass::sys::BASS_GetDeviceCount();
        for i in 0..count {
            if let Some(name) = crate::bass::sys::get_device_name(i) {
                if Self::is_bluetooth_device(&name) {
                    bt_devices.insert(i);
                }
            }
        }
        bt_devices
    }

    /// Check if a device name indicates a Bluetooth audio device.
    /// Heuristic: names containing "bluetooth" or "a2dp" (case-insensitive).
    fn is_bluetooth_device(name: &str) -> bool {
        let lower = name.to_lowercase();
        Self::contains_word(&lower, "bluetooth") || Self::contains_word(&lower, "a2dp")
    }

    /// Check if `text` contains `word` at word boundaries.
    /// A word boundary is the start/end of string or a non-alphabetic character.
    fn contains_word(text: &str, word: &str) -> bool {
        if word.is_empty() || text.len() < word.len() {
            return false;
        }
        let wlen = word.len();
        let max_start = text.len().saturating_sub(wlen);
        for i in 0..=max_start {
            if text[i..].starts_with(word) {
                let prev_ok = i == 0 || !text.as_bytes()[i - 1].is_ascii_alphabetic();
                let next_ok = i + wlen >= text.len()
                    || !text.as_bytes()[i + wlen].is_ascii_alphabetic();
                if prev_ok && next_ok {
                    return true;
                }
            }
        }
        false
    }
}

fn get_smtc_for_window(hwnd: HWND) -> Result<SystemMediaTransportControls> {
    unsafe {
        let class_id = HSTRING::from("Windows.Media.SystemMediaTransportControls");
        let mut factory: *mut std::ffi::c_void = std::ptr::null_mut();
        get_activation_factory(&class_id, &IID_INTEROP, &mut factory)?;
        if factory.is_null() {
            return Err(Error::from(HRESULT(-1i32)));
        }

        // ISystemMediaTransportControlsInterop: IUnknown(3) + IInspectable(3) + GetForWindow(1)
        let vtable = *factory.cast::<*mut usize>();
        let get_for_window: unsafe extern "system" fn(
            *mut std::ffi::c_void, HWND, *const GUID, *mut *mut std::ffi::c_void,
        ) -> HRESULT = std::mem::transmute(vtable.add(6));

        let mut smtc_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        (get_for_window)(factory, hwnd, &ISystemMediaTransportControls::IID, &raw mut smtc_ptr)
            .ok()?;
        if smtc_ptr.is_null() {
            return Err(Error::from(HRESULT(-1i32)));
        }

        Ok(std::mem::transmute::<*mut std::ffi::c_void, SystemMediaTransportControls>(smtc_ptr))
    }
}

fn get_activation_factory(
    class_id: &HSTRING,
    iid: &GUID,
    factory: &mut *mut std::ffi::c_void,
) -> Result<()> {
    unsafe {
        type RoGetActivationFactoryFn = unsafe extern "system" fn(
            std::mem::MaybeUninit<HSTRING>,
            *const GUID,
            *mut *mut std::ffi::c_void,
        ) -> HRESULT;

        let lib = LoadLibraryW(&HSTRING::from("combase.dll"))?;
        let proc_name: Vec<u8> = "RoGetActivationFactory\0".bytes().collect();
        let func: RoGetActivationFactoryFn = std::mem::transmute(
            GetProcAddress(lib, PCSTR(proc_name.as_ptr()))
                .ok_or_else(Error::from_win32)?,
        );
        (func)(std::mem::MaybeUninit::new(class_id.clone()), iid, factory).ok()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::BluetoothMonitor;

    #[test]
    fn test_is_bluetooth_device_bluetooth_exact() {
        assert!(BluetoothMonitor::is_bluetooth_device("Bluetooth"));
    }

    #[test]
    fn test_is_bluetooth_device_bluetooth_lowercase() {
        assert!(BluetoothMonitor::is_bluetooth_device("bluetooth"));
    }

    #[test]
    fn test_is_bluetooth_device_bluetooth_mixed_case() {
        assert!(BluetoothMonitor::is_bluetooth_device("BLUETOOTH Audio Device"));
    }

    #[test]
    fn test_is_bluetooth_device_bluetooth_in_long_name() {
        assert!(BluetoothMonitor::is_bluetooth_device("Speakers (Bluetooth Audio)"));
    }

    #[test]
    fn test_is_bluetooth_device_a2dp_exact() {
        assert!(BluetoothMonitor::is_bluetooth_device("A2DP"));
    }

    #[test]
    fn test_is_bluetooth_device_a2dp_lowercase() {
        assert!(BluetoothMonitor::is_bluetooth_device("a2dp"));
    }

    #[test]
    fn test_is_bluetooth_device_a2dp_in_long_name() {
        assert!(BluetoothMonitor::is_bluetooth_device("Bluetooth A2DP Stereo Audio"));
    }

    #[test]
    fn test_is_bluetooth_device_a2dp_mixed_case() {
        assert!(BluetoothMonitor::is_bluetooth_device("A2DP Sink"));
    }

    #[test]
    fn test_is_bluetooth_device_speakers_not_bluetooth() {
        assert!(!BluetoothMonitor::is_bluetooth_device("Speakers (Realtek Audio)"));
    }

    #[test]
    fn test_is_bluetooth_device_headphones_wired() {
        assert!(!BluetoothMonitor::is_bluetooth_device("Headphones (3.5mm Jack)"));
    }

    #[test]
    fn test_is_bluetooth_device_system_device() {
        assert!(!BluetoothMonitor::is_bluetooth_device("Primary Sound Driver"));
    }

    #[test]
    fn test_is_bluetooth_device_empty_string() {
        assert!(!BluetoothMonitor::is_bluetooth_device(""));
    }

    #[test]
    fn test_is_bluetooth_device_contains_bluetooth_substring() {
        // "bluetooth" appears as part of a larger compound word
        assert!(!BluetoothMonitor::is_bluetooth_device("EarBluetoothBuddy"));
    }

    #[test]
    fn test_is_bluetooth_device_numeric_name() {
        assert!(!BluetoothMonitor::is_bluetooth_device("12345"));
    }
}

fn create_hidden_window() -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleA(None)?;
        let class_name: Vec<u16> = "HackMagicMusic_SmtcHidden\0".encode_utf16().collect();
        let class_name_pcwstr = PCWSTR::from_raw(class_name.as_ptr());

        let wc = WNDCLASSW {
            style: WNDCLASS_STYLES(0),
            lpfnWndProc: Some(smtc_wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(instance.0),
            hIcon: HICON(std::ptr::null_mut()),
            hCursor: HCURSOR(std::ptr::null_mut()),
            hbrBackground: HBRUSH(std::ptr::null_mut()),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name_pcwstr,
        };
        RegisterClassW(&raw const wc);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name_pcwstr,
            PCWSTR::null(),
            WINDOW_STYLE(0),
            0, 0, 0, 0,
            HWND_MESSAGE,
            None,
            HINSTANCE(instance.0),
            None,
        )?;
        Ok(hwnd)
    }
}
