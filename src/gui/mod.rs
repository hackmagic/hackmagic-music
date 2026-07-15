//! HackMagic Music Player - Bevy GUI
//!
//! Native GUI built with the Bevy engine, directly using the Player API.
//! Layout inspired by MusicPlayer2 (C++ MFC).

pub mod controls;
pub mod layout;
pub mod lyrics;
pub mod media_lib;
pub mod player_info;
pub mod playlist;
pub mod responsive;
pub mod settings;
pub mod styles;

use bevy::prelude::*;
use crate::config::Config;
use crate::core::engine_trait::EngineType;
use crate::core::player::Player;
use std::sync::Arc;
use std::sync::OnceLock;

/// Global UI font handle, loaded once at startup.
static UI_FONT: OnceLock<Handle<Font>> = OnceLock::new();

/// Get the global UI font handle (Segoe UI or fallback default).
pub(crate) fn ui_font() -> Handle<Font> {
    UI_FONT.get().cloned().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        tracing::info!("[GUI] GuiPlugin::build starting");

        let player = Arc::new(Player::new(EngineType::Bass));
        tracing::info!("[GUI] Player created");

        // Initialize engine
        if let Err(e) = player.init() {
            tracing::warn!("[GUI] BASS init failed ({}), trying FFmpeg engine", e);
            let fallback = Arc::new(Player::new(EngineType::Ffmpeg));
            if let Err(e2) = fallback.init() {
                eprintln!("Fatal: failed to init any audio engine: {e2}");
                std::process::exit(1);
            }
            app.insert_resource(PlayerResource(fallback));
            tracing::info!("[GUI] FFmpeg engine inserted");
        } else {
            app.insert_resource(PlayerResource(player));
            tracing::info!("[GUI] BASS engine inserted");
        }

        // Load config
        let cfg = Config::load();
        tracing::info!("[GUI] Config loaded, default_volume={}", cfg.play.default_volume);
        let player = app.world_mut().resource::<PlayerResource>().0.clone();
        player.set_volume(cfg.play.default_volume).ok();

        // Auto-scan media library at startup
        if cfg.media_lib.auto_scan && !cfg.media_lib.media_dirs.is_empty() {
            let dirs = cfg.media_lib.media_dirs.clone();
            tracing::info!("[GUI] Spawning auto-scan thread for {} dirs", dirs.len());
            std::thread::spawn(move || {
                for dir in &dirs {
                    tracing::info!("Auto-scanning media directory: {}", dir);
                    match crate::media::scan_directory(dir, true, None) {
                        Ok(entries) => {
                            let mut lib = crate::media::MediaLib::load();
                            let before = lib.entries.len();
                            for e in entries {
                                lib.upsert(e);
                            }
                            let added = lib.entries.len() - before;
                            if added > 0 || before > 0 {
                                let _ = lib.save();
                                tracing::info!(
                                    "Media library updated: {} tracks ({} new)",
                                    lib.entries.len(),
                                    added
                                );
                            }
                        }
                        Err(e) => tracing::warn!("Auto-scan failed for {}: {}", dir, e),
                    }
                }
            });
        }

        app.insert_resource(PlayerState::default());
        app.insert_resource(styles::UiColors::default());
        app.insert_resource(settings::SettingsState::default());
        app.insert_resource(lyrics::DesktopLyricsState::default());
        app.insert_resource(responsive::LayoutMode::default());
        app.insert_resource(media_lib::MediaLibState::default());
        tracing::info!("[GUI] Resources inserted");

        app.add_systems(Startup, setup_ui);
        app.add_systems(Update, responsive::update_layout_mode);
        app.add_systems(Update, (
            player_info::update_track_info,
            player_info::update_progress,
            player_info::update_spectrum,
        ));
        app.add_systems(Update, (
            playlist::update_playlist,
            playlist::update_playlist_count,
        ));
        app.add_systems(Update, (
            controls::handle_controls,
            controls::update_play_button,
            controls::update_status_bar,
        ));
        app.add_systems(Update, (
            settings::toggle_settings,
            settings::settings_dialog_system,
            settings::handle_settings_interaction,
        ));
        app.add_systems(Update, (
            lyrics::toggle_desktop_lyrics,
            lyrics::desktop_lyrics_system,
            lyrics::update_lyrics,
        ));
        app.add_systems(Update, (
            media_lib::toggle_media_lib,
            media_lib::media_lib_system,
            media_lib::handle_media_lib_interaction,
        ));
        app.add_systems(Update, (
            update_titlebar_buttons,
            update_menu_hover,
            handle_menu_clicks,
            handle_keyboard,
        ));
        tracing::info!("[GUI] Systems registered, build complete");
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Wraps the shared player instance.
#[derive(Resource)]
pub struct PlayerResource(pub Arc<Player>);

/// Cached state snapshot for UI rendering.
#[derive(Resource, Default)]
pub struct PlayerState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub position: f64,
    pub duration: f64,
    pub volume: u32,
    pub is_playing: bool,
    pub is_paused: bool,
    pub spectrum: Vec<f32>,
    pub playlist_names: Vec<String>,
    pub playlist_tracks: Vec<String>,
    pub active_playlist: String,
    pub repeat_mode: String,
}

// ---------------------------------------------------------------------------
// UI Setup
// ---------------------------------------------------------------------------

fn setup_ui(mut commands: Commands, colors: Res<styles::UiColors>, asset_server: Res<AssetServer>) {
    tracing::info!("[GUI] setup_ui called");

    // Load a proper UI font (Segoe UI on Windows)
    let font: Handle<Font> = asset_server.load("fonts/segoeui.ttf");
    UI_FONT.set(font).ok();
    tracing::info!("[GUI] Font loaded");

    // Ensure a 2D camera exists for UI rendering
    commands.spawn(Camera2d);

    // Build the entire layout
    layout::spawn_layout(&mut commands, &colors);
    tracing::info!("[GUI] Layout spawned");
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Update title-bar button hover/press states.
fn update_titlebar_buttons(
    mut interaction_q: Query<
        (&Interaction, &mut BackgroundColor, Entity),
        (
            Or<(
                With<layout::TitleBtnMinimize>,
                With<layout::TitleBtnMaximize>,
                With<layout::TitleBtnClose>,
            )>,
        ),
    >,
    close_q: Query<(), With<layout::TitleBtnClose>>,
    colors: Res<styles::UiColors>,
) {
    for (interaction, mut bg, entity) in interaction_q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = colors.button_press;
                if close_q.contains(entity) {
                    std::process::exit(0);
                }
            }
            Interaction::Hovered => {
                bg.0 = colors.button_hover;
            }
            Interaction::None => {
                bg.0 = colors.button;
            }
        }
    }
}

/// Update menu-item hover state.
fn update_menu_hover(
    mut interaction_q: Query<(&Interaction, &mut BackgroundColor), With<layout::MenuBtn>>,
    colors: Res<styles::UiColors>,
) {
    for (interaction, mut bg) in interaction_q.iter_mut() {
        match *interaction {
            Interaction::Hovered => {
                bg.0 = colors.button_hover;
            }
            Interaction::Pressed => {
                bg.0 = colors.button_press;
            }
            Interaction::None => {
                bg.0 = Color::NONE;
            }
        }
    }
}

/// Handle menu item clicks.
fn handle_menu_clicks(
    mut interaction_q: Query<(
        &Interaction,
        Entity,
        Option<&layout::MenuLabelPlay>,
        Option<&layout::MenuLabelList>,
        Option<&layout::MenuLabelTool>,
        Option<&layout::MenuLabelOption>,
        Option<&layout::MenuLabelHelp>,
    ), (Changed<Interaction>, With<layout::MenuBtn>)>,
    mut settings_state: ResMut<settings::SettingsState>,
    mut media_lib_state: ResMut<media_lib::MediaLibState>,
    player: Res<PlayerResource>,
) {
    for (interaction, _entity, play, _list, tool, option, _help) in interaction_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        if play.is_some() {
            tracing::info!("[GUI] Menu: Play clicked");
            open_file_dialog(&player);
        } else if tool.is_some() {
            tracing::info!("[GUI] Menu: Tools clicked");
            media_lib_state.visible = true;
        } else if option.is_some() {
            tracing::info!("[GUI] Menu: Options clicked");
            settings_state.visible = true;
        } else {
            tracing::info!("[GUI] Menu item clicked");
        }
    }
}

/// Global keyboard shortcuts.
fn handle_keyboard(
    player: Res<PlayerResource>,
    keys: Res<ButtonInput<KeyCode>>,
    mut settings_state: ResMut<settings::SettingsState>,
    mut lyrics_state: ResMut<lyrics::DesktopLyricsState>,
) {
    // Playback control
    if keys.just_pressed(KeyCode::Space) {
        player.0.toggle_pause().ok();
    }
    if keys.just_pressed(KeyCode::ArrowRight) {
        player.0.next().ok();
    }
    if keys.just_pressed(KeyCode::ArrowLeft) {
        player.0.prev().ok();
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        player.0.volume_up(5).ok();
    }
    if keys.just_pressed(KeyCode::ArrowDown) {
        player.0.volume_down(5).ok();
    }
    if keys.just_pressed(KeyCode::KeyS) {
        player.0.stop().ok();
    }
    // Ctrl+O: Open file(s)
    if keys.just_pressed(KeyCode::KeyO) && keys.pressed(KeyCode::ControlLeft) {
        open_file_dialog(&player);
    }
    // Ctrl+F: Open folder
    if keys.just_pressed(KeyCode::KeyF) && keys.pressed(KeyCode::ControlLeft) {
        open_folder_dialog(&player);
    }
    // Ctrl+O: Settings (Shift+O to avoid conflict with Open File)
    if keys.just_pressed(KeyCode::KeyO) && keys.pressed(KeyCode::ShiftLeft) {
        settings_state.visible = !settings_state.visible;
    }
    // Ctrl+L: Desktop lyrics
    if keys.just_pressed(KeyCode::KeyL) && keys.pressed(KeyCode::ControlLeft) {
        lyrics_state.visible = !lyrics_state.visible;
    }
    // Escape: Close settings
    if keys.just_pressed(KeyCode::Escape) {
        settings_state.visible = false;
        lyrics_state.visible = false;
    }
    // M: Mute toggle
    if keys.just_pressed(KeyCode::KeyM) {
        if player.0.volume() > 0 {
            let _ = player.0.set_volume(0);
        } else {
            let _ = player.0.set_volume(80);
        }
    }
    // R: Repeat mode cycle
    if keys.just_pressed(KeyCode::KeyR) {
        use crate::core::playlist::RepeatMode;
        let current = player.0.repeat_mode();
        let next = match current {
            RepeatMode::PlayOrder | RepeatMode::LoopPlaylist => RepeatMode::LoopTrack,
            RepeatMode::LoopTrack | RepeatMode::PlayTrack => RepeatMode::PlayRandom,
            RepeatMode::PlayRandom | RepeatMode::PlayShuffle => RepeatMode::LoopPlaylist,
        };
        player.0.set_repeat_mode(next);
    }
}

/// Open a file dialog to select audio files.
fn open_file_dialog(player: &PlayerResource) {
    tracing::info!("[GUI] Opening file dialog...");
    let path = rfd::FileDialog::new()
        .add_filter("Audio", &["mp3", "flac", "wav", "ogg", "m4a", "wma", "ape", "aac"])
        .set_title("Select Audio File")
        .pick_file();
    if let Some(path) = path {
        let path_str = path.to_string_lossy().to_string();
        tracing::info!("[GUI] Playing file: {}", path_str);
        player.0.play_file(&path_str).ok();
    }
}

/// Open a folder dialog to select a music folder.
fn open_folder_dialog(player: &PlayerResource) {
    tracing::info!("[GUI] Opening folder dialog...");
    let path = rfd::FileDialog::new()
        .set_title("Select Music Folder")
        .pick_folder();
    if let Some(path) = path {
        let path_str = path.to_string_lossy().to_string();
        tracing::info!("[GUI] Scanning folder: {}", path_str);
        // Scan the directory and add to playlist
        let dir = path_str.clone();
        let player_clone = player.0.clone();
        std::thread::spawn(move || {
            if let Ok(entries) = crate::media::scan_directory(&dir, true, None) {
                let count = entries.len();
                for entry in &entries {
                    player_clone.play_file(&entry.file_path).ok();
                }
                tracing::info!("[GUI] Added {} files from folder", count);
            }
        });
    }
}