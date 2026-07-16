use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::{PlayerResource, PlayerState};
use crate::gui::i18n::Tr;

#[derive(Component)]
pub struct PlaylistDeleteBtn(pub usize);

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

#[derive(Component)]
pub struct PlaylistClear;

pub fn spawn_playlist_panel(parent: &mut ChildSpawnerCommands, colors: &UiColors, tr: &Tr) {
    parent.spawn((
        Text::new(tr.pq_title),
        TextFont { font: crate::gui::ui_font(), font_size: 24.0, ..default() },
        TextColor(colors.text_title),
        Node { margin: UiRect::all(Val::Px(16.0)), ..default() },
    ));
    spawn_toolbar(parent, colors, tr);
    spawn_list(parent, colors, tr);
    spawn_count_bar(parent, colors, tr);
}

fn spawn_toolbar(parent: &mut ChildSpawnerCommands, colors: &UiColors, tr: &Tr) {
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
            bar.spawn((
                Text::new(tr.pq_search),
                TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                TextColor(colors.text_dim),
                Node { flex_grow: 1.0, ..default() },
                PlaylistSearchInput,
            ));
            tool_btn(bar, tr.pq_sort, colors, false);
            tool_btn(bar, tr.pq_clear, colors, true);
        });
}

fn tool_btn(parent: &mut ChildSpawnerCommands, label: &str, colors: &UiColors, clear: bool) {
    let mut button = parent.spawn((
        Button,
        Node {
            width: Val::Px(24.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));
    if clear { button.insert(PlaylistClear); }
    button.with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font_size: 14.0, ..default() },
            TextColor(colors.text_dim),
        ));
    });
}

pub fn handle_playlist_interaction(
    player: Res<PlayerResource>,
    mouse: Res<ButtonInput<MouseButton>>,
    colors: Res<UiColors>,
    mut items: Query<(&Interaction, &PlaylistItemIndex, &mut BackgroundColor), (Changed<Interaction>, With<Button>, Without<PlaylistClear>, Without<PlaylistDeleteBtn>)>,
    clear_q: Query<&Interaction, (Changed<Interaction>, With<PlaylistClear>)>,
    mut del_q: Query<(&Interaction, &PlaylistDeleteBtn), (Changed<Interaction>, With<Button>)>,
) {
    // Handle right-click on playlist (tracing only until context menu UI)
    if mouse.just_pressed(MouseButton::Right) {
        tracing::info!("[Playlist] Right-click detected (context menu TBD)");
    }
    // Handle clear button
    for interaction in &clear_q {
        if *interaction == Interaction::Pressed {
            tracing::info!("[Playlist] Clear all tracks");
            let _ = player.0.stop();
            let count = player.0.playlist().len();
            player.0.playlist_mut().clear();
            tracing::info!("[Playlist] Cleared {} tracks", count);
        }
    }
    // Handle delete buttons
    for (interaction, del) in del_q.iter_mut() {
        if *interaction == Interaction::Pressed {
            tracing::info!("[Playlist] Delete track #{}", del.0);
            player.0.playlist_mut().remove(del.0);
        }
    }
    // Handle row clicks
    let current = player.0.playlist().current_index();
    for (interaction, index, mut background) in &mut items {
        match *interaction {
            Interaction::Pressed => {
                tracing::info!("[Playlist] Play track #{}", index.0);
                let _ = player.0.play_at_index(index.0);
                background.0 = colors.playlist_playing;
            }
            Interaction::Hovered => background.0 = colors.playlist_item_hover,
            Interaction::None => background.0 = if current == Some(index.0) { colors.playlist_playing } else { colors.playlist_item },
        }
    }
}

pub fn update_current_track(
    player: Res<PlayerResource>,
    colors: Res<UiColors>,
    mut items: Query<(&PlaylistItemIndex, &mut BackgroundColor), (With<PlaylistItem>, With<Button>)>,
) {
    let current = player.0.playlist().current_index();
    for (index, mut background) in &mut items {
        background.0 = if current == Some(index.0) { colors.playlist_playing } else { colors.playlist_item };
    }
}

fn spawn_list(parent: &mut ChildSpawnerCommands, colors: &UiColors, tr: &Tr) {
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
            list.spawn((
                Text::new(tr.pq_empty),
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

fn spawn_count_bar(parent: &mut ChildSpawnerCommands, colors: &UiColors, tr: &Tr) {
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
            Text::new(tr.pq_count.replace("{}", "0")),
            TextFont { font_size: 11.0, ..default() },
            TextColor(colors.text_dim),
            PlaylistCount,
        ));
    });
}

pub fn update_playlist(
    player: Res<PlayerResource>,
    mut state: ResMut<PlayerState>,
    mut list_q: Query<Entity, With<PlaylistList>>,
    mut commands: Commands,
    colors: Res<UiColors>,
    locale: Res<crate::gui::i18n::Locale>,
) {
    let tr = locale.tr;
    let pl = player.0.playlist();
    let track_count = pl.len();
    let prev = state.playlist_tracks.len();
    state.active_playlist = player.0.active_playlist_name();

    if track_count != prev {
        tracing::info!("[Playlist] Track count changed: {} -> {}", prev, track_count);
        state.playlist_tracks = (0..track_count)
            .map(|i| {
                if let Some(t) = pl.get(i) {
                    let title = if t.title.is_empty() { tr.pq_unknown.into() } else { t.title.clone() };
                    let artist = if t.artist.is_empty() { String::new() } else { t.artist.clone() };
                    if artist.is_empty() { title } else { format!("{artist} - {title}") }
                } else { "".into() }
            })
            .collect();

        if let Ok(list_entity) = list_q.single_mut() {
            commands.entity(list_entity).remove::<Children>();

            if track_count == 0 {
                commands.entity(list_entity).with_children(|list| {
                    list.spawn((
                        Text::new(tr.pq_empty),
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
                            BackgroundColor(colors.playlist_item),
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
                                TextColor(colors.text),
                                Node { flex_grow: 1.0, ..default() },
                            ));
                            // ponytail: per-row delete, full context menu later
                            item.spawn((
                                Button,
                                Node { width: Val::Px(18.0), height: Val::Px(18.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                BackgroundColor(Color::NONE),
                                PlaylistDeleteBtn(i),
                            )).with_children(|btn| {
                                btn.spawn((Text::new("X"), TextFont { font_size: 10.0, ..default() }, TextColor(colors.text_dim)));
                            });
                        });
                    }
                });
            }
        }
    }
}

pub fn update_playlist_count(
    mut count_q: Query<&mut Text, With<PlaylistCount>>,
    state: Res<PlayerState>,
    locale: Res<crate::gui::i18n::Locale>,
) {
    if let Ok(mut text) = count_q.single_mut() {
        text.0 = locale.tr.pq_count.replace("{}", &state.playlist_tracks.len().to_string());
    }
}
