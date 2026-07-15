//! HackMagic Music Player - Bevy GUI binary entry point
//!
//! This binary provides a native GUI built with the Bevy engine.
//! It directly uses the Player API (not CLI commands).

use bevy::prelude::*;
use hm::gui::GuiPlugin;

fn main() {
    // Enable ANSI colour support early (Windows needs VT processing)
    hm::color::enable_ansi_support();

    // Log panic to file AND print to stderr for diagnosis
    let panic_log = std::env::temp_dir().join("hm_panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!("[PANIC] {info}\n");
        // Write to log file
        let _ = std::fs::write(&panic_log, &msg);
        // Print to stderr immediately
        eprintln!("{msg}");
        // Also flush to ensure it's visible
        use std::io::Write;
        let _ = std::io::stderr().flush();
    }));

    tracing::info!("[GUI] Starting HackMagic Music Player GUI");

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "HackMagic Music Player".into(),
                        resolution: (1200u32, 800u32).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(GuiPlugin)
        .run();
}