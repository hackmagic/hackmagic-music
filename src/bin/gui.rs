use gpui::*;

fn main() {
    hm::color::enable_ansi_support();

    let panic_log = std::env::temp_dir().join("hm_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {info}\n");
        let _ = std::fs::write(&panic_log, &msg);
        eprintln!("{msg}");
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }));

    tracing::info!("[GUI] Starting HackMagic Music Player with GPUI Component");

    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        hm::gui::run(cx);
    });
}
