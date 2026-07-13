//! Windows system tray icon for hm daemon mode.
//!
//! Provides a tray icon with right-click context menu:
//! Play/Pause, Next, Previous, Stop, Exit.
//! Double-click opens the Web UI in browser.

use std::sync::mpsc;
use windows::Win32::Foundation::{HWND, HINSTANCE, WPARAM, LPARAM, LRESULT};
use windows::Win32::System::LibraryLoader::GetModuleHandleA;
use windows::Win32::UI::WindowsAndMessaging::{HICON, WNDCLASSW, WNDCLASS_STYLES, HCURSOR, RegisterClassW, CreateWindowExW, WINDOW_EX_STYLE, WINDOW_STYLE, HWND_MESSAGE, LoadIconW, IDI_APPLICATION, WM_APP, DestroyWindow, MSG, PeekMessageW, PM_REMOVE, TranslateMessage, DispatchMessageW, WM_RBUTTONUP, WM_LBUTTONDBLCLK, WM_COMMAND, DefWindowProcW, WM_DESTROY, PostQuitMessage, CreatePopupMenu, AppendMenuW, MF_STRING, MF_SEPARATOR, CURSORINFO, GetCursorInfo, SetForegroundWindow, TPM_RIGHTBUTTON, TPM_LEFTALIGN, TPM_BOTTOMALIGN, TrackPopupMenu, PostMessageW, WM_NULL, DestroyMenu};
use windows::Win32::UI::Shell::{NOTIFYICONDATAW, NIF_MESSAGE, NIF_ICON, NIF_TIP, Shell_NotifyIconW, NIM_ADD, NIM_MODIFY, NIM_DELETE};
use windows::Win32::Graphics::Gdi::HBRUSH;
use windows::core::{PCWSTR, w};

/// Commands emitted by the tray icon context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    TogglePlayPause,
    Next,
    Previous,
    Stop,
    Exit,
}

/// Manages a Windows system tray icon with a right-click context menu.
///
/// Uses a hidden message-only window to receive tray callbacks.
/// Commands are delivered via an internal mpsc channel — call `try_recv()`
/// periodically from the daemon loop.
pub struct TrayManager {
    hwnd: HWND,
    hicon: HICON,
    rx: mpsc::Receiver<TrayCommand>,
}

// Global sender for the window proc (single-threaded daemon, safe)
static mut TRAY_TX: Option<mpsc::Sender<TrayCommand>> = None;
static WINDOW_CLASS_REGISTERED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

impl TrayManager {
    /// Create a new tray icon. Must be called from the main daemon thread.
    ///
    /// Registers a window class, creates a message-only hidden window,
    /// loads a standard application icon, and adds the tray icon.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();

        unsafe {
            // Save global sender for window proc
            TRAY_TX = Some(tx);

            let instance = GetModuleHandleA(None)?;
            let class_name: Vec<u16> = "MusicPlayer2_TrayHidden\0".encode_utf16().collect();
            let class_name_pcwstr = PCWSTR::from_raw(class_name.as_ptr());

            // Register window class if not already done
            if !WINDOW_CLASS_REGISTERED.load(std::sync::atomic::Ordering::Relaxed) {
                let wc = WNDCLASSW {
                    style: WNDCLASS_STYLES(0),
                    lpfnWndProc: Some(Self::wnd_proc),
                    cbClsExtra: 0,
                    cbWndExtra: 0,
                    hInstance: HINSTANCE(instance.0),
                    hIcon: HICON(0),
                    hCursor: HCURSOR(0),
                    hbrBackground: HBRUSH(0),
                    lpszMenuName: PCWSTR::null(),
                    lpszClassName: class_name_pcwstr,
                };
                RegisterClassW(&raw const wc);
                WINDOW_CLASS_REGISTERED.store(true, std::sync::atomic::Ordering::Relaxed);
            }

            // Create message-only hidden window
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name_pcwstr,
                PCWSTR::null(),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                None,
                HINSTANCE(instance.0),
                None,
            );
            if hwnd.0 == 0 {
                return Err("Failed to create tray hidden window".into());
            }

            // Load standard application icon
            let hicon = LoadIconW(None, IDI_APPLICATION)?;

            // Build NOTIFYICONDATA for icon addition
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: hwnd,
                uID: 1,
                uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
                uCallbackMessage: WM_APP,
                hIcon: hicon,
                ..Default::default()
            };
            // Copy tooltip string
            let tip: Vec<u16> = "hm - Stopped\0".encode_utf16().collect();
            let tip_len = tip.len().min(127);
            nid.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);

            if !Shell_NotifyIconW(NIM_ADD, &raw const nid).as_bool() {
                let _ = DestroyWindow(hwnd);
                return Err("Failed to create tray icon".into());
            }

            Ok(TrayManager { hwnd, hicon, rx })
        }
    }

    /// Update tooltip to reflect current playback state.
    pub fn set_playing(&self, is_playing: bool) {
        unsafe {
            let tip: Vec<u16> = if is_playing {
                "hm - Playing\0".encode_utf16().collect()
            } else {
                "hm - Paused\0".encode_utf16().collect()
            };
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                uFlags: NIF_TIP,
                ..Default::default()
            };
            let tip_len = tip.len().min(127);
            nid.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);
            let _ = Shell_NotifyIconW(NIM_MODIFY, &raw const nid);
        }
    }

    /// Set the tray icon to show "stopped" state.
    pub fn set_stopped(&self) {
        unsafe {
            let tip: Vec<u16> = "hm - Stopped\0".encode_utf16().collect();
            let mut nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                uFlags: NIF_TIP,
                ..Default::default()
            };
            let tip_len = tip.len().min(127);
            nid.szTip[..tip_len].copy_from_slice(&tip[..tip_len]);
            let _ = Shell_NotifyIconW(NIM_MODIFY, &raw const nid);
        }
    }

    /// Try to receive a command from the tray context menu (non-blocking).
    pub fn try_recv(&self) -> Result<TrayCommand, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    /// Pump Windows messages for the hidden window (must be called periodically).
    pub fn poll_messages(&mut self) {
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&raw mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                TranslateMessage(&raw const msg);
                DispatchMessageW(&raw const msg);
            }
        }
    }

    // ── Window procedure ────────────────────────────────────────────

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_APP => {
                // Tray icon callback — lparam holds the mouse event
                match lparam.0 as u32 {
                    WM_RBUTTONUP => {
                        Self::show_context_menu(hwnd);
                    }
                    WM_LBUTTONDBLCLK => {
                        // Double-click → open Web UI in browser (cross-platform)
                        let url = "http://127.0.0.1:10280";
                        #[cfg(windows)]
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "", url])
                            .spawn();
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open")
                            .arg(url)
                            .spawn();
                        #[cfg(not(any(windows, target_os = "macos")))]
                        let _ = std::process::Command::new("xdg-open")
                            .arg(url)
                            .spawn();
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_COMMAND => {
                let id = wparam.0 as u16;
                if let Some(ref tx) = TRAY_TX {
                    let cmd = match id {
                        1001 => TrayCommand::TogglePlayPause,
                        1002 => TrayCommand::Next,
                        1003 => TrayCommand::Previous,
                        1004 => TrayCommand::Stop,
                        1005 => TrayCommand::Exit,
                        _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
                    };
                    let _ = tx.send(cmd);
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    // ── Context menu ────────────────────────────────────────────────

    unsafe fn show_context_menu(hwnd: HWND) {
        let hmenu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };

        let _ = AppendMenuW(hmenu, MF_STRING, 1001, w!("Play/Pause\tSpace"));
        let _ = AppendMenuW(hmenu, MF_STRING, 1002, w!("Next\tCtrl+N"));
        let _ = AppendMenuW(hmenu, MF_STRING, 1003, w!("Previous\tCtrl+B"));
        let _ = AppendMenuW(hmenu, MF_STRING, 1004, w!("Stop\tCtrl+S"));
        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(hmenu, MF_STRING, 1005, w!("Exit"));

        // Show menu at cursor position
        let mut cursor = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        if GetCursorInfo(&raw mut cursor).is_ok() {
            SetForegroundWindow(hwnd);
            let flags = TPM_RIGHTBUTTON | TPM_LEFTALIGN | TPM_BOTTOMALIGN;
            let _ = TrackPopupMenu(hmenu, flags, cursor.ptScreenPos.x, cursor.ptScreenPos.y, 0, hwnd, None);
            // Force taskbar to re-evaluate foreground window
            let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        }

        let _ = DestroyMenu(hmenu);
    }
}

unsafe impl Send for TrayManager {}
unsafe impl Sync for TrayManager {}

impl Drop for TrayManager {
    fn drop(&mut self) {
        unsafe {
            let nid = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: 1,
                ..Default::default()
            };
            Shell_NotifyIconW(NIM_DELETE, &raw const nid);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}