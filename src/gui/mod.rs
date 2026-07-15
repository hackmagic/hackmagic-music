//! HackMagic Music Player - Bevy GUI
//!
//! Native GUI built with the Bevy engine, directly using the Player API.
//! Features: playback controls, playlist, spectrum visualizer, now-playing info.

use bevy::prelude::*;
use crate::config::Config;
use crate::core::engine_trait::EngineType;
use crate::core::player::Player;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct GuiPlugin;

impl Plugin for GuiPlugin {
    fn build(&self, app: &mut App) {
        let player = Arc::new(Player::new(EngineType::Bass));

        // Initialize engine
        if let Err(e) = player.init() {
            tracing::warn!("BASS init failed ({}), trying FFmpeg engine", e);
            let fallback = Arc::new(Player::new(EngineType::Ffmpeg));
            if let Err(e2) = fallback.init() {
                eprintln!("Fatal: failed to init any audio engine: {e2}");
                std::process::exit(1);
            }
            app.insert_resource(PlayerResource(fallback));
        } else {
            app.insert_resource(PlayerResource(player));
        }

        // Load config
        let cfg = Config::load();
        let player = app.world_mut().resource::<PlayerResource>().0.clone();
        player.set_volume(cfg.play.default_volume).ok();

        // Auto-scan media library at startup
        if cfg.media_lib.auto_scan && !cfg.media_lib.media_dirs.is_empty() {
            let dirs = cfg.media_lib.media_dirs.clone();
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
        app.insert_resource(UiColors::default());

        app.add_systems(Startup, setup_ui);
        app.add_systems(
            Update,
            (
                update_now_playing,
                update_progress,
                update_playlist_view,
                update_spectrum,
                handle_button_interaction,
                handle_keyboard,
            ),
        );
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
}

/// Colour palette for the UI.
#[derive(Resource)]
pub struct UiColors {
    pub bg: Color,
    pub panel: Color,
    pub accent: Color,
    pub text: Color,
    pub text_dim: Color,
    pub button: Color,
    pub button_hover: Color,
    pub button_press: Color,
    pub progress_bar: Color,
    pub progress_fill: Color,
    pub spectrum_bar: Color,
}

impl Default for UiColors {
    fn default() -> Self {
        Self {
            bg: Color::srgb(0.08, 0.08, 0.10),
            panel: Color::srgb(0.12, 0.12, 0.15),
            accent: Color::srgb(0.20, 0.50, 1.0),
            text: Color::srgb(0.92, 0.92, 0.95),
            text_dim: Color::srgb(0.55, 0.55, 0.60),
            button: Color::srgb(0.18, 0.18, 0.22),
            button_hover: Color::srgb(0.25, 0.25, 0.30),
            button_press: Color::srgb(0.35, 0.35, 0.40),
            progress_bar: Color::srgb(0.20, 0.20, 0.25),
            progress_fill: Color::srgb(0.20, 0.50, 1.0),
            spectrum_bar: Color::srgb(0.20, 0.50, 1.0),
        }
    }
}

// ---------------------------------------------------------------------------
// Marker Components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct NowPlayingText;

#[derive(Component)]
pub struct ArtistText;

#[derive(Component)]
pub struct AlbumText;

#[derive(Component)]
pub struct TimeText;

#[derive(Component)]
pub struct ProgressFill;

#[derive(Component)]
pub struct VolumeFill;

#[derive(Component)]
pub struct SpectrumContainer;

#[derive(Component)]
pub struct SpectrumBar(pub usize);

#[derive(Component)]
pub struct PlayButton;

#[derive(Component)]
pub struct StopButton;

#[derive(Component)]
pub struct NextButton;

#[derive(Component)]
pub struct PrevButton;

#[derive(Component)]
pub struct VolumeUpButton;

#[derive(Component)]
pub struct VolumeDownButton;

// ---------------------------------------------------------------------------
// UI Setup
// ---------------------------------------------------------------------------

const SPECTRUM_BARS: usize = 32;

#[allow(clippy::too_many_lines)]
fn setup_ui(mut commands: Commands, colors: Res<UiColors>) {
    // Root container
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(colors.bg),
        ))
        .with_children(|root| {
            // ── Top section: now playing + spectrum ──
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(220.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    ..default()
                },
                BackgroundColor(colors.panel),
            ))
            .with_children(|top| {
                // Track info
                top.spawn((
                    Text::new("No track playing"),
                    TextFont {
                        font_size: 22.0,
                        ..default()
                    },
                    TextColor(colors.text),
                    Node {
                        margin: UiRect::bottom(Val::Px(4.0)),
                        ..default()
                    },
                    NowPlayingText,
                ));

                // Artist
                top.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(colors.text_dim),
                    ArtistText,
                ));

                // Album
                top.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(colors.text_dim),
                    AlbumText,
                ));

                // Time display
                top.spawn((
                    Text::new("00:00 / 00:00"),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                    TextColor(colors.text_dim),
                    Node {
                        margin: UiRect::vertical(Val::Px(6.0)),
                        ..default()
                    },
                    TimeText,
                ));

                // Progress bar
                top.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(colors.progress_bar),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(0.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(colors.progress_fill),
                        ProgressFill,
                    ));
                });

                // Spectrum visualizer
                top.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(60.0),
                        margin: UiRect::top(Val::Px(8.0)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::End,
                        ..default()
                    },
                    SpectrumContainer,
                ))
                .with_children(|spec| {
                    for i in 0..SPECTRUM_BARS {
                        spec.spawn((
                            Node {
                                width: Val::Px(8.0),
                                height: Val::Px(2.0),
                                margin: UiRect::horizontal(Val::Px(1.0)),
                                ..default()
                            },
                            BackgroundColor(colors.spectrum_bar),
                            SpectrumBar(i),
                        ));
                    }
                });
            });

            // ── Middle section: playlist (placeholder) ──
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    flex_grow: 1.0,
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundColor(colors.bg),
            ))
            .with_children(|mid| {
                mid.spawn((
                    Text::new("Playlist: drag & drop files or use 'hm play <file>'"),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(colors.text_dim),
                ));
            });

            // ── Bottom section: controls ──
            root.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(60.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    padding: UiRect::horizontal(Val::Px(16.0)),
                    column_gap: Val::Px(8.0),
                    ..default()
                },
                BackgroundColor(colors.panel),
            ))
            .with_children(|bottom| {
                // Prev button
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.button),
                    PrevButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("⏮"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(colors.text),
                    ));
                });

                // Play/Pause button
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(50.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.accent),
                    PlayButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("▶"),
                        TextFont {
                            font_size: 20.0,
                            ..default()
                        },
                        TextColor(Color::srgb(1.0, 1.0, 1.0)),
                    ));
                });

                // Stop button
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.button),
                    StopButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("⏹"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(colors.text),
                    ));
                });

                // Next button
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(40.0),
                        height: Val::Px(36.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.button),
                    NextButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("⏭"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(colors.text),
                    ));
                });

                // Spacer
                bottom.spawn((
                    Node {
                        width: Val::Px(40.0),
                        ..default()
                    },
                ));

                // Volume down
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.button),
                    VolumeDownButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("−"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(colors.text),
                    ));
                });

                // Volume bar
                bottom.spawn((
                    Node {
                        width: Val::Px(100.0),
                        height: Val::Px(6.0),
                        ..default()
                    },
                    BackgroundColor(colors.progress_bar),
                ))
                .with_children(|bar| {
                    bar.spawn((
                        Node {
                            width: Val::Percent(80.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(colors.progress_fill),
                        VolumeFill,
                    ));
                });

                // Volume up
                bottom.spawn((
                    Button,
                    Node {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(colors.button),
                    VolumeUpButton,
                ))
                .with_children(|btn| {
                    btn.spawn((
                        Text::new("+"),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                        TextColor(colors.text),
                    ));
                });
            });
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn update_now_playing(
    player: Res<PlayerResource>,
    mut state: ResMut<PlayerState>,
    mut title_q: Query<&mut Text, (With<NowPlayingText>, Without<ArtistText>, Without<AlbumText>)>,
    mut artist_q: Query<&mut Text, (With<ArtistText>, Without<NowPlayingText>, Without<AlbumText>)>,
    mut album_q: Query<&mut Text, (With<AlbumText>, Without<NowPlayingText>, Without<ArtistText>)>,
) {
    let pl = player.0.playlist();
    let idx = pl.current_index();

    let (title, artist, album) = if let Some(idx) = idx {
        if let Some(track) = pl.get(idx) {
            let title = if track.title.is_empty() {
                "Unknown".into()
            } else {
                track.title.clone()
            };
            let artist = if track.artist.is_empty() {
                "Unknown Artist".into()
            } else {
                track.artist.clone()
            };
            let album = track.album.clone();
            (title, artist, album)
        } else {
            ("No track playing".into(), String::new(), String::new())
        }
    } else {
        ("No track playing".into(), String::new(), String::new())
    };

    state.title = title.clone();
    state.artist = artist.clone();
    state.album = album.clone();

    if let Ok(mut t) = title_q.single_mut() {
        t.0 = title;
    }
    if let Ok(mut a) = artist_q.single_mut() {
        a.0 = if artist.is_empty() { String::new() } else { artist };
    }
    if let Ok(mut a) = album_q.single_mut() {
        a.0 = if album.is_empty() { String::new() } else { album };
    }
}

fn update_progress(
    player: Res<PlayerResource>,
    mut state: ResMut<PlayerState>,
    mut time_q: Query<&mut Text, With<TimeText>>,
    mut fill_q: Query<&mut Node, With<ProgressFill>>,
    mut vol_fill_q: Query<&mut Node, (With<VolumeFill>, Without<ProgressFill>)>,
) {
    let p = &player.0;
    let pos = p.position();
    let dur = p.duration();
    let pos_secs = pos.as_secs_f64();
    let dur_secs = dur.as_secs_f64();
    let vol = p.volume();

    state.position = pos_secs;
    state.duration = dur_secs;
    state.volume = vol;
    state.is_playing = p.is_playing();
    state.is_paused = p.is_paused();

    // Time text
    let pos_str = format!("{:02}:{:02}", pos_secs as u64 / 60, pos_secs as u64 % 60);
    let dur_str = if dur_secs > 0.0 {
        format!("{:02}:{:02}", dur_secs as u64 / 60, dur_secs as u64 % 60)
    } else {
        "00:00".into()
    };
    if let Ok(mut t) = time_q.single_mut() {
        t.0 = format!("{pos_str} / {dur_str}");
    }

    // Progress fill
    let pct = if dur_secs > 0.0 {
        (pos_secs / dur_secs * 100.0) as f32
    } else {
        0.0
    };
    if let Ok(mut s) = fill_q.single_mut() {
        s.width = Val::Percent(pct.clamp(0.0, 100.0));
    }

    // Volume fill
    if let Ok(mut s) = vol_fill_q.single_mut() {
        s.width = Val::Percent(vol as f32);
    }
}

fn update_playlist_view(
    player: Res<PlayerResource>,
    mut state: ResMut<PlayerState>,
) {
    let pl_name = player.0.active_playlist_name();
    let pl = player.0.playlist();
    let track_count = pl.len();
    state.active_playlist = pl_name;

    let prev = state.playlist_tracks.len();
    if track_count != prev {
        state.playlist_tracks = (0..track_count)
            .map(|i| {
                if let Some(t) = pl.get(i) {
                    let title = if t.title.is_empty() {
                        "Unknown".into()
                    } else {
                        t.title.clone()
                    };
                    let artist = if t.artist.is_empty() {
                        String::new()
                    } else {
                        t.artist.clone()
                    };
                    if artist.is_empty() {
                        title
                    } else {
                        format!("{artist} - {title}")
                    }
                } else {
                    "".into()
                }
            })
            .collect();
    }
}

fn update_spectrum(
    player: Res<PlayerResource>,
    mut state: ResMut<PlayerState>,
    mut bars: Query<(&mut Node, &SpectrumBar)>,
) {
    let p = &player.0;
    if !p.is_playing() {
        for (mut node, _) in bars.iter_mut() {
            node.height = Val::Px(2.0);
        }
        return;
    }

    let spectrum = p.calculate_spectrum();
    state.spectrum = spectrum.clone();

    let max_bars = SPECTRUM_BARS.min(spectrum.len());
    let max_height = 58.0;

    for (mut node, bar) in bars.iter_mut() {
        let idx = bar.0;
        if idx < max_bars {
            let val = spectrum[idx].clamp(0.0, 1.0);
            node.height = Val::Px(2.0 + val * max_height);
        } else {
            node.height = Val::Px(2.0);
        }
    }
}

fn handle_button_interaction(
    player: Res<PlayerResource>,
    mut interaction_q: Query<(&Interaction, &mut BackgroundColor, Entity), With<Button>>,
    colors: Res<UiColors>,
) {
    for (interaction, mut bg, _entity) in interaction_q.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = colors.button_press;
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

fn handle_keyboard(
    player: Res<PlayerResource>,
    keys: Res<ButtonInput<KeyCode>>,
) {
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
}