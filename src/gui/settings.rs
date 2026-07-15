//! Settings dialog system — appearance, playback, lyrics, equalizer.
//!
//! In Bevy 0.18, dialogs are implemented as full-screen overlays
//! with a semi-transparent backdrop.

use bevy::prelude::*;
use crate::gui::styles::*;

// ---------------------------------------------------------------------------
// Settings panel identifiers
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Appearance,
    Playback,
    Lyrics,
    Equalizer,
}

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct SettingsOverlay;

#[derive(Component)]
pub struct SettingsDialog;

#[derive(Component)]
pub struct SettingsTabBtn(pub SettingsTab);

#[derive(Component)]
pub struct SettingsTabActive;

#[derive(Component)]
pub struct SettingsContent;

#[derive(Component)]
pub struct SettingsCloseBtn;

// ---------------------------------------------------------------------------
// Resource: current settings state
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct SettingsState {
    pub visible: bool,
    pub active_tab: SettingsTab,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            visible: false,
            active_tab: SettingsTab::Appearance,
        }
    }
}

// ---------------------------------------------------------------------------
// Spawn the settings dialog
// ---------------------------------------------------------------------------

pub fn spawn_settings_dialog(commands: &mut Commands, colors: &UiColors) {
    // Semi-transparent backdrop
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            SettingsOverlay,
        ))
        .with_children(|overlay| {
            // Dialog box
            overlay
                .spawn((
                    Node {
                        width: Val::Px(640.0),
                        height: Val::Px(480.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        margin: UiRect::axes(Val::Px(-320.0), Val::Px(-240.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(colors.panel),
                    SettingsDialog,
                ))
                .with_children(|dialog| {
                    // ── Title bar ──
                    dialog
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(32.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(12.0)),
                                ..default()
                            },
                            BackgroundColor(colors.titlebar_bg),
                        ))
                        .with_children(|title| {
                            title.spawn((
                                Text::new("\u{8BBE}\u{7F6E}"), // 设置
                                TextFont { font_size: 14.0, ..default() },
                                TextColor(colors.text_title),
                                Node { flex_grow: 1.0, ..default() },
                            ));
                            // Close button
                            title.spawn((
                                Button,
                                Node {
                                    width: Val::Px(30.0),
                                    height: Val::Px(30.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..default()
                                },
                                BackgroundColor(colors.btn_close),
                                SettingsCloseBtn,
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new("\u{00D7}"),
                                    TextFont { font_size: 16.0, ..default() },
                                    TextColor(colors.text),
                                ));
                            });
                        });

                    // ── Tab bar ──
                    dialog
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(30.0),
                                flex_direction: FlexDirection::Row,
                                padding: UiRect::horizontal(Val::Px(4.0)),
                                column_gap: Val::Px(2.0),
                                ..default()
                            },
                            BackgroundColor(colors.menubar_bg),
                        ))
                        .with_children(|tabs| {
                            settings_tab(tabs, "\u{5916}\u{89C2}", SettingsTab::Appearance, colors); // 外观
                            settings_tab(tabs, "\u{64AD}\u{653E}", SettingsTab::Playback, colors);   // 播放
                            settings_tab(tabs, "\u{6B4C}\u{8BCD}", SettingsTab::Lyrics, colors);     // 歌词
                            settings_tab(tabs, "\u{5747}\u{8861}\u{5668}", SettingsTab::Equalizer, colors); // 均衡器
                        });

                    // ── Content area ──
                    dialog.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            padding: UiRect::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(colors.bg),
                        SettingsContent,
                    ));
                });
        });
}

fn settings_tab(parent: &mut ChildSpawnerCommands, label: &str, tab: SettingsTab, colors: &UiColors) {
    parent.spawn((
        Button,
        Node {
            padding: UiRect::horizontal(Val::Px(12.0)),
            height: Val::Px(26.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        SettingsTabBtn(tab),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 12.0, ..default() },
            TextColor(colors.text),
        ));
    });
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Toggle settings dialog visibility.
pub fn toggle_settings(
    mut settings_state: ResMut<SettingsState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyO) {
        settings_state.visible = !settings_state.visible;
    }
}

/// Spawn or despawn the settings dialog based on visibility.
pub fn settings_dialog_system(
    settings_state: Res<SettingsState>,
    mut commands: Commands,
    overlay_q: Query<Entity, With<SettingsOverlay>>,
    colors: Res<UiColors>,
) {
    if settings_state.is_changed() {
        if settings_state.visible && overlay_q.is_empty() {
            spawn_settings_dialog(&mut commands, &colors);
        } else if !settings_state.visible {
            for entity in overlay_q.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Handle tab switching and close button.
pub fn handle_settings_interaction(
    mut settings_state: ResMut<SettingsState>,
    mut interaction_q: Query<
        (&Interaction, &mut BackgroundColor, Entity),
        (Changed<Interaction>, With<Button>),
    >,
    tab_q: Query<&SettingsTabBtn>,
    close_q: Query<(), With<SettingsCloseBtn>>,
    colors: Res<UiColors>,
) {
    for (interaction, mut bg, entity) in interaction_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        bg.0 = colors.button_press;

        // Check close button
        if close_q.contains(entity) {
            settings_state.visible = false;
            continue;
        }

        // Check tab buttons
        if let Ok(tab) = tab_q.get(entity) {
            settings_state.active_tab = tab.0;
        }
    }
}