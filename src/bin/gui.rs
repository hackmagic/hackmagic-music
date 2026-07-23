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
    hm::color::enable_ansi_support();

    // Log panic to file for diagnosis
    let panic_log = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()).join("hm_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {info}\nbacktrace:\n{}\n", std::backtrace::Backtrace::force_capture());
        let _ = std::fs::write(&panic_log, &msg);
        eprintln!("{msg}");
    }));

    // Configure tracing: write to a log file in temp dir AND stderr.
    // Use env var RUST_LOG to override the default "info" level.
    let log_path = std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()).join("hm_gui.log");
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

    tracing::info!("=== HackMagic GUI starting, log file: {} ===", log_path.display());
    tracing::info!("[GUI] Starting HackMagic Music Player with GPUI Component");

    // GPUI render functions build deeply nested element trees that can overflow
    // the default 2MB main-thread stack on Windows. Run the whole app on a
    // worker thread with a larger stack (8MB) and let main wait for it.
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .name("hm-gui".into())
        .spawn(move || {
            Application::new().run(|cx: &mut App| {
                gpui_component::init(cx);
                hm::gui::run(cx);
            });
        })
        .expect("failed to spawn GUI thread");

    let _ = handle.join();
}
