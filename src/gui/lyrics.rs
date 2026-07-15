//! Desktop lyrics — a floating overlay window that displays synced lyrics.
//!
//! In Bevy 0.18, this is rendered as a semi-transparent overlay
//! within the main window (a separate window requires OS-level window management).

use bevy::prelude::*;
use crate::gui::styles::*;

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct DesktopLyricsContainer;

#[derive(Component)]
pub struct DesktopLyricsText;

#[derive(Component)]
pub struct DesktopLyricsTranslation;

#[derive(Component)]
pub struct DesktopLyricsProgress;

// ---------------------------------------------------------------------------
// Resource
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct DesktopLyricsState {
    pub visible: bool,
    pub locked: bool,       // 锁定窗口位置
    pub font_size: f32,
    pub opacity: f32,
}

impl Default for DesktopLyricsState {
    fn default() -> Self {
        Self {
            visible: false,
            locked: false,
            font_size: 24.0,
            opacity: 0.85,
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn
// ---------------------------------------------------------------------------

pub fn spawn_desktop_lyrics(commands: &mut Commands, colors: &UiColors) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(120.0),
                position_type: PositionType::Absolute,
                bottom: Val::Px(80.0), // above control bar
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)), // fully transparent bg
            DesktopLyricsContainer,
        ))
        .with_children(|parent| {
            // Lyric text
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgba(0.9, 0.9, 0.95, 0.9)),
                DesktopLyricsText,
            ));
            // Translation text
            parent.spawn((
                Text::new(""),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgba(0.65, 0.65, 0.70, 0.7)),
                DesktopLyricsTranslation,
            ));
            // Progress bar
            parent.spawn((
                Node {
                    width: Val::Px(400.0),
                    height: Val::Px(3.0),
                    margin: UiRect::top(Val::Px(4.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.35, 0.5)),
                DesktopLyricsProgress,
            ));
        });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Toggle desktop lyrics with Ctrl+L
pub fn toggle_desktop_lyrics(
    mut state: ResMut<DesktopLyricsState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyL) && keys.pressed(KeyCode::ControlLeft) {
        state.visible = !state.visible;
    }
}

/// Spawn/despawn desktop lyrics based on visibility.
pub fn desktop_lyrics_system(
    state: Res<DesktopLyricsState>,
    mut commands: Commands,
    container_q: Query<Entity, With<DesktopLyricsContainer>>,
    colors: Res<UiColors>,
) {
    if state.is_changed() {
        if state.visible && container_q.is_empty() {
            spawn_desktop_lyrics(&mut commands, &colors);
        } else if !state.visible {
            for entity in container_q.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Update lyrics text from player state.
pub fn update_lyrics(
    player: Res<crate::gui::PlayerResource>,
    state: Res<DesktopLyricsState>,
    mut text_q: Query<&mut Text, (With<DesktopLyricsText>, Without<DesktopLyricsTranslation>)>,
    mut trans_q: Query<&mut Text, (With<DesktopLyricsTranslation>, Without<DesktopLyricsText>)>,
    mut prog_q: Query<&mut Node, With<DesktopLyricsProgress>>,
) {
    if !state.visible { return; }

    let pos = player.0.position();
    let pos_secs = pos.as_secs_f64();

    if let Ok(mut text) = text_q.single_mut() {
        let dur = player.0.duration();
        let total = dur.as_secs_f64();
        if total > 0.0 {
            let pct = (pos_secs / total * 100.0) as f32;
            if let Ok(mut prog) = prog_q.single_mut() {
                prog.width = Val::Px(400.0 * pct / 100.0);
            }
        }
    }

    if let Ok(mut trans) = trans_q.single_mut() {
        trans.0 = String::new();
    }
}