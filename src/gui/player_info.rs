//! Info panel: album art, track info, progress bar, time, spectrum.
//! Displayed in the left panel of the main content area.

use bevy::prelude::*;
use crate::gui::styles::*;

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct AlbumArt;

#[derive(Component)]
pub struct TrackTitle;

#[derive(Component)]
pub struct TrackArtist;

#[derive(Component)]
pub struct TrackAlbum;

#[derive(Component)]
pub struct TimeDisplay;

#[derive(Component)]
pub struct ProgressTrack;

#[derive(Component)]
pub struct ProgressFill;

#[derive(Component)]
pub struct VolumeFill;

#[derive(Component)]
pub struct SpectrumContainer;

#[derive(Component)]
pub struct SpectrumBar(pub usize);

pub const SPECTRUM_BARS: usize = 32;

// ---------------------------------------------------------------------------
// Spawn the info panel (called from layout.rs)
// ---------------------------------------------------------------------------

pub fn spawn_info_panel(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    // Album art area
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(ALBUM_COVER_SIZE),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            margin: UiRect::bottom(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(colors.panel_alt),
        AlbumArt,
    ));

    // Track title
    parent.spawn((
        Text::new("No track playing"),
        TextFont { font: crate::gui::ui_font(), font_size: 18.0, ..default() },
        TextColor(colors.text_title),
        Node { margin: UiRect::bottom(Val::Px(2.0)), ..default() },
        TrackTitle,
    ));

    // Artist
    parent.spawn((
        Text::new(""),
        TextFont { font: crate::gui::ui_font(), font_size: 13.0, ..default() },
        TextColor(colors.text_dim),
        TrackArtist,
    ));

    // Album
    parent.spawn((
        Text::new(""),
        TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
        TextColor(colors.text_dim),
        Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() },
        TrackAlbum,
    ));

    // Progress bar
    progress_bar(parent, colors);

    // Time display
    parent.spawn((
        Text::new("00:00 / 00:00"),
        TextFont { font: crate::gui::ui_font(), font_size: 11.0, ..default() },
        TextColor(colors.text_dim),
        Node { margin: UiRect::vertical(Val::Px(4.0)), ..default() },
        TimeDisplay,
    ));

    // Spectrum visualizer
    spectrum(parent, colors);
}

// ---------------------------------------------------------------------------
// Progress bar
// ---------------------------------------------------------------------------

fn progress_bar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(PROGRESS_BAR_HEIGHT),
                ..default()
            },
            BackgroundColor(colors.progress_track),
            ProgressTrack,
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
}

// ---------------------------------------------------------------------------
// Spectrum visualizer
// ---------------------------------------------------------------------------

fn spectrum(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(SPECTRUM_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::End,
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            SpectrumContainer,
        ))
        .with_children(|spec| {
            for i in 0..SPECTRUM_BARS {
                spec.spawn((
                    Node {
                        width: Val::Px(6.0),
                        height: Val::Px(2.0),
                        flex_grow: 1.0,
                        margin: UiRect::horizontal(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(colors.spectrum_bar),
                    SpectrumBar(i),
                ));
            }
        });
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

pub fn update_track_info(
    player: Res<crate::gui::PlayerResource>,
    mut state: ResMut<crate::gui::PlayerState>,
    mut title_q: Query<&mut Text, (With<TrackTitle>, Without<TrackArtist>, Without<TrackAlbum>)>,
    mut artist_q: Query<&mut Text, (With<TrackArtist>, Without<TrackTitle>, Without<TrackAlbum>)>,
    mut album_q: Query<&mut Text, (With<TrackAlbum>, Without<TrackTitle>, Without<TrackArtist>)>,
) {
    let pl = player.0.playlist();
    let idx = pl.current_index();

    let (title, artist, album) = if let Some(idx) = idx {
        if let Some(track) = pl.get(idx) {
            let t = if track.title.is_empty() { "Unknown".into() } else { track.title.clone() };
            let a = if track.artist.is_empty() { "Unknown Artist".into() } else { track.artist.clone() };
            let al = track.album.clone();
            (t, a, al)
        } else {
            ("No track playing".into(), String::new(), String::new())
        }
    } else {
        ("No track playing".into(), String::new(), String::new())
    };

    state.title = title.clone();
    state.artist = artist.clone();
    state.album = album.clone();

    if let Ok(mut t) = title_q.single_mut() { t.0 = title; }
    if let Ok(mut a) = artist_q.single_mut() { a.0 = if artist.is_empty() { String::new() } else { artist }; }
    if let Ok(mut a) = album_q.single_mut() { a.0 = if album.is_empty() { String::new() } else { album }; }
}

pub fn update_progress(
    player: Res<crate::gui::PlayerResource>,
    mut state: ResMut<crate::gui::PlayerState>,
    mut time_q: Query<&mut Text, With<TimeDisplay>>,
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

    let pos_str = format!("{:02}:{:02}", pos_secs as u64 / 60, pos_secs as u64 % 60);
    let dur_str = if dur_secs > 0.0 {
        format!("{:02}:{:02}", dur_secs as u64 / 60, dur_secs as u64 % 60)
    } else {
        "00:00".into()
    };
    if let Ok(mut t) = time_q.single_mut() {
        t.0 = format!("{pos_str} / {dur_str}");
    }

    let pct = if dur_secs > 0.0 {
        (pos_secs / dur_secs * 100.0) as f32
    } else {
        0.0
    };
    if let Ok(mut s) = fill_q.single_mut() {
        s.width = Val::Percent(pct.clamp(0.0, 100.0));
    }
    if let Ok(mut s) = vol_fill_q.single_mut() {
        s.width = Val::Percent(vol as f32);
    }
}

pub fn update_spectrum(
    player: Res<crate::gui::PlayerResource>,
    mut state: ResMut<crate::gui::PlayerState>,
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
    let max_height = SPECTRUM_HEIGHT - 2.0;

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