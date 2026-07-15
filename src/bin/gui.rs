//! HackMagic Music Player - Bevy GUI binary entry point
//!
//! This binary provides a native GUI built with the Bevy engine.
//! It directly uses the Player API (not CLI commands).

use bevy::prelude::*;
use hm::gui::GuiPlugin;

fn main() {
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