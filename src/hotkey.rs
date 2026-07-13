#![allow(clippy::upper_case_acronyms)]

use std::sync::OnceLock;

use clap::Parser;
use crate::core::player::Player;

static PLAYER: OnceLock<std::sync::Arc<Player>> = OnceLock::new();

pub fn init(player: std::sync::Arc<Player>) {
    let _ = PLAYER.set(player);
    #[cfg(target_os = "windows")]
    start_message_thread();
}

#[cfg(not(target_os = "windows"))]
fn start_message_thread() {
    tracing::info!("Global hotkeys not supported on this platform");
}

/// Try to forward a CLI command to an already-running instance.
/// Returns true if the command was successfully forwarded.
pub fn try_forward_command(args: &[String]) -> bool {
    #[cfg(target_os = "windows")]
    {
        forward_via_wm_copydata(args)
    }
    #[cfg(not(target_os = "windows"))]
    {
        forward_via_http(args)
    }
}

/// Forward a command to the daemon via its HTTP API endpoint.
/// Used on non-Windows platforms where WM_COPYDATA is unavailable.
#[cfg(not(target_os = "windows"))]
fn forward_via_http(args: &[String]) -> bool {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    // Reconstruct the command string from args
    let cmd_str = args.join(" ");
    let payload = serde_json::json!({ "command": cmd_str }).to_string();

    let body = payload.as_bytes();
    let content_length = body.len();

    // Build raw HTTP/1.1 POST request
    let request = format!(
        "POST /api/command HTTP/1.1\r\n\
         Host: 127.0.0.1:10280\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
         \r\n",
        content_length
    );

    // Connect and send with a short timeout
    let mut stream = match TcpStream::connect_timeout(
        &"127.0.0.1:10280".parse().unwrap(),
        Duration::from_secs(2),
    ) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));

    // Send request headers + body
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }
    if stream.write_all(body).is_err() {
        return false;
    }

    // Read response — we only care about the status line
    let mut buf = [0u8; 4096];
    let n = match stream.read(&mut buf) {
        Ok(n) if n > 0 => n,
        _ => return false,
    };
    let response = String::from_utf8_lossy(&buf[..n]);

    // Check for HTTP 200
    response.starts_with("HTTP/1.1 200")
}

#[cfg(target_os = "windows")]
fn forward_via_wm_copydata(args: &[String]) -> bool {
    use std::ffi::c_void;

    extern "system" {
        fn FindWindowW(class_name: *const u16, window_name: *const u16) -> isize;
        fn SendMessageTimeoutW(
            h_wnd: isize, msg: u32, w_param: usize, l_param: isize,
            flags: u32, timeout: u32, result: *mut usize,
        ) -> isize;
    }

    const WM_COPYDATA: u32 = 0x004A;
    const SMTO_ABORTIFHUNG: u32 = 0x0002;
    const SMTO_TIMEOUT: u32 = 0x0001;

    #[repr(C)]
    struct COPYDATASTRUCT {
        dw_data: usize,
        cb_data: u32,
        lp_data: *const c_void,
    }

    let class_name: Vec<u16> = "MusicPlayer2_hm\0".encode_utf16().collect();

    unsafe {
        let hwnd = FindWindowW(class_name.as_ptr(), std::ptr::null());
        if hwnd == 0 {
            return false;
        }

        let payload = serde_json::to_string(args).unwrap_or_default();
        let payload_bytes = payload.as_bytes();
        let cds = COPYDATASTRUCT {
            dw_data: 1,
            cb_data: payload_bytes.len() as u32,
            lp_data: payload_bytes.as_ptr().cast::<c_void>(),
        };

        let mut result: usize = 0;
        let sent = SendMessageTimeoutW(
            hwnd, WM_COPYDATA, 0, &raw const cds as isize,
            SMTO_TIMEOUT | SMTO_ABORTIFHUNG, 3000, &raw mut result,
        );
        sent != 0
    }
}

#[cfg(target_os = "windows")]
fn start_message_thread() {
    std::thread::spawn(|| {
        const WH_KEYBOARD_LL: i32 = 13;
        const WM_KEYDOWN: u32 = 0x0100;
        const WM_SYSKEYDOWN: u32 = 0x0104;
        const WM_COPYDATA: u32 = 0x004A;
        const HWND_MESSAGE: isize = -3;

        const VK_MEDIA_NEXT_TRACK: u32 = 0xB0;
        const VK_MEDIA_PREV_TRACK: u32 = 0xB1;
        const VK_MEDIA_STOP: u32 = 0xB2;
        const VK_MEDIA_PLAY_PAUSE: u32 = 0xB3;
        const VK_VOLUME_UP: u32 = 0xAF;
        const VK_VOLUME_DOWN: u32 = 0xAE;

        #[repr(C)]
        struct KBDLLHOOKSTRUCT {
            vk_code: u32,
            scan_code: u32,
            flags: u32,
            time: u32,
            extra_info: usize,
        }

        #[repr(C)]
        struct COPYDATASTRUCT {
            dw_data: usize,
            cb_data: u32,
            lp_data: *const std::ffi::c_void,
        }

        type HOOKPROC = unsafe extern "system" fn(i32, usize, *const std::ffi::c_void) -> isize;
        type WNDPROC = unsafe extern "system" fn(isize, u32, usize, isize) -> isize;
        type HHOOK = isize;
        type HINSTANCE = isize;

        extern "system" {
            fn SetWindowsHookExW(
                id_hook: i32, lpfn: HOOKPROC, hmod: HINSTANCE, dw_thread_id: u32,
            ) -> HHOOK;
            fn UnhookWindowsHookEx(hhk: HHOOK) -> i32;
            fn CallNextHookEx(
                hhk: HHOOK, n_code: i32, w_param: usize, l_param: *const std::ffi::c_void,
            ) -> isize;
            fn GetMessageW(
                msg: *mut std::ffi::c_void, h_wnd: isize,
                msg_filter_min: u32, msg_filter_max: u32,
            ) -> i32;
            fn GetAsyncKeyState(v_key: i32) -> i16;
            fn RegisterClassW(class: *const std::ffi::c_void) -> u16;
            fn CreateWindowExW(
                ex_style: u32, class_name: *const u16, window_name: *const u16,
                style: u32, x: i32, y: i32, w: i32, h: i32,
                parent: isize, menu: isize, instance: HINSTANCE, param: *const std::ffi::c_void,
            ) -> isize;
            fn DefWindowProcW(h_wnd: isize, msg: u32, w_param: usize, l_param: isize) -> isize;
            fn DestroyWindow(h_wnd: isize) -> i32;
        }

        const VK_CONTROL: i32 = 0x11;
        const VK_MENU: i32 = 0x12;
        const VK_SHIFT: i32 = 0x10;

        unsafe extern "system" fn hook_proc(
            code: i32, w_param: usize, l_param: *const std::ffi::c_void,
        ) -> isize {
            if code >= 0 && (w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize) {
                let kbd = &*l_param.cast::<KBDLLHOOKSTRUCT>();

                match kbd.vk_code {
                    VK_MEDIA_PLAY_PAUSE => {
                        if let Some(p) = PLAYER.get() { let _ = p.toggle_pause(); }
                        return 1;
                    }
                    VK_MEDIA_NEXT_TRACK => {
                        if let Some(p) = PLAYER.get() {
                            let _ = p.next();
                            crate::play_stats::track_started(
                                p.playlist_mut().current_track().map(|t| &*t.file_path),
                            );
                            p.save_playback_state();
                        }
                        return 1;
                    }
                    VK_MEDIA_PREV_TRACK => {
                        if let Some(p) = PLAYER.get() {
                            let _ = p.prev();
                            crate::play_stats::track_started(
                                p.playlist_mut().current_track().map(|t| &*t.file_path),
                            );
                            p.save_playback_state();
                        }
                        return 1;
                    }
                    VK_MEDIA_STOP => {
                        if let Some(p) = PLAYER.get() {
                            p.save_playback_state();
                            let _ = p.stop();
                        }
                        return 1;
                    }
                    _ => {
                        let ctrl = (GetAsyncKeyState(VK_CONTROL) as u16 & 0x8000) != 0;
                        let alt = (GetAsyncKeyState(VK_MENU) as u16 & 0x8000) != 0;
                        let shift = (GetAsyncKeyState(VK_SHIFT) as u16 & 0x8000) != 0;

                        if let Some(p) = PLAYER.get() {
                            match kbd.vk_code {
                                0x50 if ctrl && alt && !shift => { let _ = p.toggle_pause(); return 1; }
                                0x4E if ctrl && alt && !shift => {
                                    let _ = p.next();
                                    crate::play_stats::track_started(
                                        p.playlist_mut().current_track().map(|t| &*t.file_path),
                                    );
                                    p.save_playback_state();
                                    return 1;
                                }
                                0x42 if ctrl && alt && !shift => {
                                    let _ = p.prev();
                                    crate::play_stats::track_started(
                                        p.playlist_mut().current_track().map(|t| &*t.file_path),
                                    );
                                    p.save_playback_state();
                                    return 1;
                                }
                                0x53 if ctrl && alt && !shift => {
                                    p.save_playback_state();
                                    let _ = p.stop();
                                    return 1;
                                }
                                0x51 if ctrl && alt && !shift => { std::process::exit(0); }
                                VK_VOLUME_UP if ctrl && alt => {
                                    let v = p.volume().saturating_add(5).min(100);
                                    let _ = p.set_volume(v);
                                    return 1;
                                }
                                VK_VOLUME_DOWN if ctrl && alt => {
                                    let v = p.volume().saturating_sub(5);
                                    let _ = p.set_volume(v);
                                    return 1;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            unsafe { CallNextHookEx(0, code, w_param, l_param) }
        }

        unsafe extern "system" fn wnd_proc(
            h_wnd: isize, msg: u32, w_param: usize, l_param: isize,
        ) -> isize {
            if msg == WM_COPYDATA {
                let cds = &*(l_param as *const COPYDATASTRUCT);
                if cds.dw_data == 1 && !cds.lp_data.is_null() {
                    let slice = std::slice::from_raw_parts(
                        cds.lp_data.cast::<u8>(), cds.cb_data as usize,
                    );
                    if let Ok(json_str) = std::str::from_utf8(slice) {
                        if let Ok(args) = serde_json::from_str::<Vec<String>>(json_str) {
                            let full_args: Vec<String> =
                                std::iter::once("hm".to_string()).chain(args).collect();
                            if let Ok(cli) = crate::cli::Cli::try_parse_from(&full_args) {
                                if let Some(cmd) = cli.command {
                                    if let Err(e) = crate::commands::dispatch(&cmd) {
                                        tracing::warn!("Forwarded command failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
                return 1;
            }
            unsafe { DefWindowProcW(h_wnd, msg, w_param, l_param) }
        }

        unsafe {
            let class_name: Vec<u16> = "MusicPlayer2_hm\0".encode_utf16().collect();

            #[repr(C)]
            struct WNDCLASSW {
                style: u32,
                lpfn_wnd_proc: WNDPROC,
                cb_cls_extra: i32,
                cb_wnd_extra: i32,
                h_instance: HINSTANCE,
                h_icon: isize,
                h_cursor: isize,
                h_brush: isize,
                lpsz_menu_name: *const u16,
                lpsz_class_name: *const u16,
            }

            let wc = WNDCLASSW {
                style: 0,
                lpfn_wnd_proc: wnd_proc,
                cb_cls_extra: 0,
                cb_wnd_extra: 0,
                h_instance: 0,
                h_icon: 0,
                h_cursor: 0,
                h_brush: 0,
                lpsz_menu_name: std::ptr::null(),
                lpsz_class_name: class_name.as_ptr(),
            };

            let class_atom = RegisterClassW((&raw const wc).cast::<std::ffi::c_void>());
            if class_atom == 0 {
                tracing::warn!("Failed to register message window class");
                return;
            }

            let hwnd = CreateWindowExW(
                0, class_name.as_ptr(), std::ptr::null(),
                0, 0, 0, 0, 0,
                HWND_MESSAGE, 0, 0, std::ptr::null(),
            );
            if hwnd == 0 {
                tracing::warn!("Failed to create message window");
                return;
            }
            tracing::info!("Message window created for IPC");

            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL, hook_proc as HOOKPROC, 0, 0,
            );
            if hook == 0 {
                tracing::warn!("Failed to set keyboard hook for global hotkeys");
            } else {
                tracing::info!("Global hotkey hook installed");
            }

            let mut msg: [u8; 48] = std::mem::zeroed();
            while GetMessageW(msg.as_mut_ptr().cast::<std::ffi::c_void>(), 0, 0, 0) > 0 {}

            if hook != 0 { let _ = UnhookWindowsHookEx(hook); }
            let _ = DestroyWindow(hwnd);
        }
    });
}