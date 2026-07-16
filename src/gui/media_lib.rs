use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::i18n::Tr;

#[derive(Component)]
pub struct MediaLibOverlay;

#[derive(Component)]
pub struct MediaLibDialog;

#[derive(Component)]
pub struct MediaLibSearchInput;

#[derive(Component)]
pub struct MediaLibItem;

#[derive(Component)]
pub struct MediaLibItemIndex(pub usize);

#[derive(Component)]
pub struct MediaLibCloseBtn;

#[derive(Component)]
pub struct MediaLibScanBtn;

#[derive(Resource)]
pub struct MediaLibState {
    pub visible: bool,
    pub search_query: String,
    pub items: Vec<String>,
}

impl Default for MediaLibState {
    fn default() -> Self {
        Self { visible: false, search_query: String::new(), items: Vec::new() }
    }
}

pub fn spawn_media_lib(commands: &mut Commands, colors: &UiColors, tr: &Tr) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            MediaLibOverlay,
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: Val::Px(700.0),
                        height: Val::Px(500.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        margin: UiRect::axes(Val::Px(-350.0), Val::Px(-250.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(colors.panel),
                    MediaLibDialog,
                ))
                .with_children(|dialog| {
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
                                Text::new(tr.media_lib_title),
                                TextFont { font: crate::gui::ui_font(), font_size: 14.0, ..default() },
                                TextColor(colors.text_title),
                                Node { flex_grow: 1.0, ..default() },
                            ));
                            title.spawn((
                                Button,
                                Node {
                                    padding: UiRect::horizontal(Val::Px(8.0)),
                                    height: Val::Px(24.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    margin: UiRect::right(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(colors.button_accent),
                                MediaLibScanBtn,
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new(tr.media_lib_scan),
                                    TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                                    TextColor(Color::WHITE),
                                ));
                            });
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
                                MediaLibCloseBtn,
                            )).with_children(|btn| {
                                btn.spawn((
                                    Text::new("X"),
                                    TextFont { font: crate::gui::ui_font(), font_size: 16.0, ..default() },
                                    TextColor(colors.text),
                                ));
                            });
                        });

                    dialog
                        .spawn((
                            Node {
                                width: Val::Percent(100.0),
                                height: Val::Px(30.0),
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                padding: UiRect::horizontal(Val::Px(8.0)),
                                ..default()
                            },
                            BackgroundColor(colors.panel_alt),
                        ))
                        .with_children(|search| {
                            search.spawn((
                                Text::new(tr.media_lib_search),
                                TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                                TextColor(colors.text_dim),
                                MediaLibSearchInput,
                            ));
                        });

                    dialog.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            flex_grow: 1.0,
                            flex_direction: FlexDirection::Column,
                            overflow: Overflow::clip_y(),
                            padding: UiRect::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(colors.bg),
                    ));
                });
        });
}

pub fn toggle_media_lib(
    mut state: ResMut<MediaLibState>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    if keys.just_pressed(KeyCode::KeyM) && keys.pressed(KeyCode::ControlLeft) {
        state.visible = !state.visible;
    }
}

pub fn media_lib_system(
    state: Res<MediaLibState>,
    mut commands: Commands,
    overlay_q: Query<Entity, With<MediaLibOverlay>>,
    colors: Res<UiColors>,
    locale: Res<crate::gui::i18n::Locale>,
) {
    if state.is_changed() {
        if state.visible && overlay_q.is_empty() {
            spawn_media_lib(&mut commands, &colors, locale.tr);
        } else if !state.visible {
            for entity in overlay_q.iter() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn handle_media_lib_interaction(
    mut state: ResMut<MediaLibState>,
    colors: Res<UiColors>,
    mut interaction_q: Query<(
        &Interaction,
        &mut BackgroundColor,
        Entity,
        Option<&MediaLibCloseBtn>,
        Option<&MediaLibScanBtn>,
    ), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut bg, _entity, close, scan) in interaction_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        bg.0 = colors.button_press;

        if close.is_some() {
            state.visible = false;
        } else if scan.is_some() {
            std::thread::spawn(|| {
                tracing::info!("[MediaLib] Manual scan triggered");
                let cfg = crate::config::Config::load();
                for dir in &cfg.media_lib.media_dirs {
                    if let Ok(entries) = crate::media::scan_directory(dir, true, None) {
                        let mut lib = crate::media::MediaLib::load();
                        let before = lib.entries.len();
                        for e in entries {
                            lib.upsert(e);
                        }
                        if lib.entries.len() != before {
                            let _ = lib.save();
                            tracing::info!("[MediaLib] Scan updated: {} tracks", lib.entries.len());
                        }
                    }
                }
            });
        }
    }
}
