use gpui::*;
use tracing_subscriber::{EnvFilter, fmt::writer::MakeWriter, prelude::*};

/// A writer that duplicates tracing output to a log file in CWD (project dir),
/// so we can inspect GUI playback issues even without a console.
struct FileAndStdoutWriter {
    file: std::sync::Mutex<std::fs::File>,
}

impl<'a> MakeWriter<'a> for FileAndStdoutWriter {
    type Writer = Box<dyn std::io::Write + Send + 'a>;
    fn make_writer(&'a self) -> Self::Writer {
        // We can't easily combine stderr + file in a zero-allocation way,
        // so write only to file here; stdout/stderr is handled by fmt layer.
        let f = self.file.lock().unwrap().try_clone().expect("clone log file");
        Box::new(f)
    }
}

fn main() {
    eprintln!("[BOOT] main() entered");
    hm::color::enable_ansi_support();
    eprintln!("[BOOT] ANSI support enabled");

    // Log panic to file for diagnosis — use CWD (project dir when run via cargo run)
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir());
    let panic_log = cwd.join("hm_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {info}\nbacktrace:\n{}\n", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write(&panic_log, &msg);
        eprintln!("{msg}");
        // Surface the panic to the user in a dialog so a silent GUI crash is
        // not mistaken for a freeze. Best-effort: ignore if the dialog fails.
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                MessageBoxW, MB_ICONERROR, MB_OK,
            };
            let wide: Vec<u16> = format!("HackMagic 遇到错误已停止运行。\n\n{info}\n\n详细信息已写入:\n{}", panic_log.display())
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let _ = unsafe {
                MessageBoxW(
                    None,
                    windows::core::PCWSTR(wide.as_ptr()),
                    windows::core::w!("HackMagic Music Player"),
                    MB_OK | MB_ICONERROR,
                )
            };
        }
    }));
    eprintln!("[BOOT] Panic hook set, log dir: {}", cwd.display());

    // Configure tracing: write to a log file in CWD AND stderr.
    // Use env var RUST_LOG to override the default "trace" level.
    let log_path = cwd.join("hm_gui.log");
    eprintln!("[BOOT] Opening log file: {}", log_path.display());
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .expect("cannot open hm_gui.log");
    let writer = FileAndStdoutWriter { file: std::sync::Mutex::new(file) };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("trace"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false),
        )
        .init();
    eprintln!("[BOOT] Tracing initialized");

    tracing::info!("=== HackMagic GUI starting, log file: {} ===", log_path.display());
    tracing::info!("[GUI] Starting HackMagic Music Player with GPUI Component");

    // 初始化全链路事件日志系统
    hm::ui_test::init_event_log(10000);
    tracing::info!("[GUI] UI事件日志系统已初始化");

    // GPUI render functions build deeply nested element trees that can overflow
    // the default 2MB main-thread stack on Windows. Run the whole app on a
    // worker thread with a larger stack (8MB) and let main wait for it.
    tracing::info!("[GUI] Spawning GUI thread with 8MB stack...");
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .name("hm-gui".into())
        .spawn(move || {
            tracing::info!("[GUI] GUI thread entered, creating Application...");
            Application::new().run(|cx: &mut App| {
                tracing::info!("[GUI] Application::run callback entered");
                gpui_component::init(cx);
                tracing::info!("[GUI] gpui_component::init done, calling hm::gui::run...");
                hm::gui::run(cx);
                tracing::info!("[GUI] hm::gui::run returned (should not happen — event loop)");
            });
            tracing::info!("[GUI] Application::run exited");
        })
        .expect("failed to spawn GUI thread");

    tracing::info!("[GUI] Waiting for GUI thread to finish...");
    let _ = handle.join();
    tracing::info!("[GUI] GUI thread finished");
}
