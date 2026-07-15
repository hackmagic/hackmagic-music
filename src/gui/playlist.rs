//! Playlist panel: toolbar + song list + status bar.
//! Displayed in the right panel of the main content area.

use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::PlayerState;

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct PlaylistToolbar;

#[derive(Component)]
pub struct PlaylistSearchInput;

#[derive(Component)]
pub struct PlaylistItem;

#[derive(Component)]
pub struct PlaylistItemIndex(pub usize);

#[derive(Component)]
pub struct PlaylistItemPlaying;

#[derive(Component)]
pub struct PlaylistItemSelected;

#[derive(Component)]
pub struct PlaylistList;

#[derive(Component)]
pub struct PlaylistCount;

// ---------------------------------------------------------------------------
// Spawn the playlist panel (called from layout.rs)
// ---------------------------------------------------------------------------

pub fn spawn_playlist_panel(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    // ── Toolbar ──
    spawn_toolbar(parent, colors);
    // ── Song list ──
    spawn_list(parent, colors);
    // ── Count bar ──
    spawn_count_bar(parent, colors);
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

fn spawn_toolbar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(28.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                column_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(colors.panel_alt),
            PlaylistToolbar,
        ))
        .with_children(|bar| {
            // Search input placeholder
            bar.spawn((
                Text::new("\u{1F50D} Search songs..."),
                TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                TextColor(colors.text_dim),
                Node {
                    flex_grow: 1.0,
                    ..default()
                },
                PlaylistSearchInput,
            ));
            // Sort button
            tool_btn(bar, "\u{2195}", colors); // ↕
            // Clear button
            tool_btn(bar, "\u{2716}", colors); // ✖
        });
}

fn tool_btn(parent: &mut ChildSpawnerCommands, label: &str, colors: &UiColors) {
    parent.spawn((
        Button,
        Node {
            width: Val::Px(24.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 14.0, ..default() },
            TextColor(colors.text_dim),
        ));
    });
}

// ---------------------------------------------------------------------------
// Song list (scrollable)
// ---------------------------------------------------------------------------

fn spawn_list(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip_y(),
                ..default()
            },
            BackgroundColor(colors.bg),
            PlaylistList,
        ))
        .with_children(|list| {
            // Placeholder text when playlist is empty
            list.spawn((
                Text::new("No songs, drag files here"), // 没有歌曲，拖拽文件到此处
                TextFont { font_size: 13.0, ..default() },
                TextColor(colors.text_dim),
                Node {
                    align_self: AlignSelf::Center,
                    margin: UiRect::top(Val::Px(40.0)),
                    ..default()
                },
                PlaylistItem,
                PlaylistItemIndex(0),
            ));
        });
}

// ---------------------------------------------------------------------------
// Count bar at bottom of playlist
// ---------------------------------------------------------------------------

fn spawn_count_bar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(22.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(colors.panel_alt),
    ))
    .with_children(|bar| {
        bar.spawn((
            Text::new("0 songs"), // 共 0 首
            TextFont { font_size: 11.0, ..default() },
            TextColor(colors.text_dim),
            PlaylistCount,
        ));
    });
}

// ---------------------------------------------------------------------------
// Update systems
// ---------------------------------------------------------------------------

/// Rebuild playlist items when the track list changes.
pub fn update_playlist(
    player: Res<crate::gui::PlayerResource>,
    mut state: ResMut<PlayerState>,
    mut list_q: Query<Entity, With<PlaylistList>>,
    mut commands: Commands,
    colors: Res<UiColors>,
) {
    let pl = player.0.playlist();
    let track_count = pl.len();
    let prev = state.playlist_tracks.len();
    state.active_playlist = player.0.active_playlist_name();

    if track_count != prev {
        state.playlist_tracks = (0..track_count)
            .map(|i| {
                if let Some(t) = pl.get(i) {
                    let title = if t.title.is_empty() { "Unknown".into() } else { t.title.clone() };
                    let artist = if t.artist.is_empty() { String::new() } else { t.artist.clone() };
                    if artist.is_empty() { title } else { format!("{artist} - {title}") }
                } else { "".into() }
            })
            .collect();

        // Rebuild the list items
        if let Ok(list_entity) = list_q.single_mut() {
            // Remove old children
            commands.entity(list_entity).remove::<Children>();

            // Spawn new items
            if track_count == 0 {
                commands.entity(list_entity).with_children(|list| {
                    list.spawn((
                        Text::new("No songs, drag files here"),
                        TextFont { font_size: 13.0, ..default() },
                        TextColor(colors.text_dim),
                        Node {
                            align_self: AlignSelf::Center,
                            margin: UiRect::top(Val::Px(40.0)),
                            ..default()
                        },
                        PlaylistItem,
                        PlaylistItemIndex(0),
                    ));
                });
            } else {
                commands.entity(list_entity).with_children(|list| {
                    for (i, track_str) in state.playlist_tracks.iter().enumerate() {
                        list.spawn((
                            Button,
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(24.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(if i == 0 { colors.playlist_playing } else { colors.playlist_item }),
                            PlaylistItem,
                            PlaylistItemIndex(i),
                        ))
                        .with_children(|item| {
                            item.spawn((
                                Text::new(format!("{}", i + 1)),
                                TextFont { font_size: 11.0, ..default() },
                                TextColor(colors.text_dim),
                                Node { width: Val::Px(24.0), ..default() },
                            ));
                            item.spawn((
                                Text::new(track_str),
                                TextFont { font_size: 12.0, ..default() },
                                TextColor(if i == 0 { colors.text_title } else { colors.text }),
                                Node { flex_grow: 1.0, ..default() },
                            ));
                        });
                    }
                });
            }
        }
    }
}

/// Update the playlist count display.
pub fn update_playlist_count(
    mut count_q: Query<&mut Text, With<PlaylistCount>>,
    state: Res<PlayerState>,
) {
    if let Ok(mut text) = count_q.single_mut() {
        text.0 = format!("{} songs", state.playlist_tracks.len());
    }
}