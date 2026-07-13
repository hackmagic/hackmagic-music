#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpStream;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::Duration;
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

struct DaemonProcess(Mutex<Option<Child>>);

fn find_player_exe() -> String {
    let exe = std::env::current_exe().ok();
    if let Some(path) = exe {
        let dir = path.parent().unwrap();
        let candidate = dir.join("hm.exe");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
        let candidate = dir.join("hm");
        if candidate.exists() {
            return candidate.to_string_lossy().to_string();
        }
    }
    if let Ok(path) = which::which("hm") {
        return path.to_string_lossy().to_string();
    }
    "hm".to_string()
}

fn send_command(cmd: &str) {
    let exe = find_player_exe();
    let _ = Command::new(&exe)
        .args(cmd.split_whitespace())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

#[cfg(target_os = "windows")]
fn toggle_window_passthrough(window: &tauri::Window, enabled: bool) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use std::ffi::c_void;
    const GWL_EXSTYLE: i32 = -20;
    const WS_EX_TRANSPARENT: isize = 0x00000020;
    extern "system" {
        fn GetWindowLongPtrA(hWnd: *mut c_void, nIndex: i32) -> isize;
        fn SetWindowLongPtrA(hWnd: *mut c_void, nIndex: i32, dwNewLong: isize) -> isize;
    }
    let Ok(handle) = window.window_handle() else { return };
    let RawWindowHandle::Win32(w) = handle.as_raw() else { return };
    let hwnd = w.hwnd.get() as *mut c_void;
    unsafe {
        let current = GetWindowLongPtrA(hwnd, GWL_EXSTYLE);
        if enabled {
            SetWindowLongPtrA(hwnd, GWL_EXSTYLE, current | WS_EX_TRANSPARENT);
        } else {
            SetWindowLongPtrA(hwnd, GWL_EXSTYLE, current & !WS_EX_TRANSPARENT);
        }
    }
}

#[tauri::command]
fn toggle_overlay_passthrough(window: tauri::Window, enabled: bool) {
    #[cfg(target_os = "windows")]
    toggle_window_passthrough(&window, enabled);
    #[cfg(not(target_os = "windows"))]
    let _ = (window, enabled);
}

fn main() {
    tauri::Builder::default()
        .manage(DaemonProcess(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![toggle_overlay_passthrough])
        .setup(|app| {
            let play = MenuItemBuilder::with_id("play", "Play/Pause")
                .accelerator("Ctrl+Alt+P")
                .build(app)?;
            let next = MenuItemBuilder::with_id("next", "Next")
                .accelerator("Ctrl+Alt+N")
                .build(app)?;
            let prev = MenuItemBuilder::with_id("prev", "Previous")
                .accelerator("Ctrl+Alt+B")
                .build(app)?;
            let stop = MenuItemBuilder::with_id("stop", "Stop").build(app)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let minimode = MenuItemBuilder::with_id("minimode", "Mini Mode")
                .accelerator("Ctrl+Alt+M")
                .build(app)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "Quit")
                .accelerator("Ctrl+Alt+Q")
                .build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&play, &next, &prev, &stop, &sep1, &minimode, &sep2, &quit])
                .build()?;

            TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "play" => send_command("pause"),
                    "next" => send_command("next"),
                    "prev" => send_command("prev"),
                    "stop" => send_command("stop"),
                    "minimode" => {
                        if let Some(window) = app.get_webview_window("overlay") {
                            let _ = window.close();
                        } else {
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "overlay",
                                WebviewUrl::App("mini.html".into()),
                            )
                            .title("")
                            .inner_size(380.0, 230.0)
                            .min_inner_size(300.0, 160.0)
                            .transparent(true)
                            .decorations(false)
                            .always_on_top(true)
                            .resizable(false)
                            .center()
                            .build();
                        }
                    }
                    "quit" => {
                        send_command("daemon stop");
                        std::process::exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            let exe = find_player_exe();
            let handle = app.handle().clone();

            std::thread::spawn(move || {
                match Command::new(&exe)
                    .args(["daemon", "start"])
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(child) => {
                        let state = handle.state::<DaemonProcess>();
                        *state.0.lock().unwrap() = Some(child);

                        // Wait for backend to be ready (poll TCP port 10280)
                        let wait_start = std::time::Instant::now();
                        let max_wait = Duration::from_secs(15);
                        let mut delay_ms = 200u64;
                        loop {
                            std::thread::sleep(Duration::from_millis(delay_ms));
                            if wait_start.elapsed() > max_wait {
                                eprintln!("Backend did not become ready within 15s");
                                break;
                            }
                            let addr = "127.0.0.1:10280";
                            if let Ok(_stream) = TcpStream::connect_timeout(
                                &addr.parse().unwrap(),
                                Duration::from_secs(2),
                            ) {
                                eprintln!("Backend is ready on {}", addr);
                                break;
                            }
                            delay_ms = (delay_ms * 3).min(3000);
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to start daemon: {}", e);
                    }
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                window.hide().ok();
            }
            #[cfg(target_os = "windows")]
            if let tauri::WindowEvent::DragDrop(drag) = event {
                use tauri::DragDropEvent;
                match drag {
                    DragDropEvent::Drop { paths, .. } => {
                        for path in paths {
                            if let Some(p) = path.to_str() {
                                let ext = p.rsplit('.').next().unwrap_or("").to_lowercase();
                                let is_audio = matches!(ext.as_str(), "mp3"|"flac"|"wav"|"ogg"|"opus"|"m4a"|"aac"|"wma"|"cue");
                                if is_audio {
                                    send_command(&format!("playlist add \"{}\"", p.replace('"', "\\\"")));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            #[cfg(not(target_os = "windows"))]
            if let tauri::WindowEvent::DragDrop(drag) = event {
                use tauri::DragDropEvent;
                if let DragDropEvent::Drop { paths, .. } = drag {
                    for path in paths {
                        if let Some(p) = path.to_str() {
                            let ext = p.rsplit('.').next().unwrap_or("").to_lowercase();
                            let is_audio = matches!(ext.as_str(), "mp3"|"flac"|"wav"|"ogg"|"opus"|"m4a"|"aac"|"wma"|"cue");
                            if is_audio {
                                send_command(&format!("playlist add \"{}\"", p.replace('"', "\\\"")));
                            }
                        }
                    }
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("Failed to run Tauri application");
}