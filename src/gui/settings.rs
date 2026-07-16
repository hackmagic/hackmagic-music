use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::i18n::Tr;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsTab {
    General,
    Appearance,
    Playback,
    Lyrics,
    Equalizer,
    Hotkeys,
    MediaLib,
}

#[derive(Component)]
pub struct SettingsOverlay;
#[derive(Component)]
pub struct SettingsDialog;
#[derive(Component)]
pub struct SettingsTabBtn(pub SettingsTab);
#[derive(Component)]
pub struct SettingsContent;
#[derive(Component)]
pub struct SettingsCloseBtn;

#[derive(Resource)]
pub struct SettingsState {
    pub visible: bool,
    pub active_tab: SettingsTab,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self { visible: false, active_tab: SettingsTab::General }
    }
}

pub fn spawn_settings_dialog(commands: &mut Commands, colors: &UiColors, tr: &Tr) {
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
            overlay
                .spawn((
                    Node {
                        width: Val::Px(720.0),
                        height: Val::Px(520.0),
                        position_type: PositionType::Absolute,
                        left: Val::Percent(50.0),
                        top: Val::Percent(50.0),
                        margin: UiRect::axes(Val::Px(-360.0), Val::Px(-260.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(colors.panel),
                    SettingsDialog,
                ))
                .with_children(|dialog| {
                    dialog.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(32.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(colors.titlebar_bg),
                    )).with_children(|title| {
                        title.spawn((
                            Text::new(tr.settings_title),
                            TextFont { font: crate::gui::ui_font(), font_size: 14.0, ..default() },
                            TextColor(colors.text_title),
                            Node { flex_grow: 1.0, ..default() },
                        ));
                        title.spawn((
                            Button,
                            Node { width: Val::Px(30.0), height: Val::Px(30.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                            BackgroundColor(colors.btn_close),
                            SettingsCloseBtn,
                        )).with_children(|btn| { btn.spawn((Text::new("X"), TextFont { font: crate::gui::ui_font(), font_size: 16.0, ..default() }, TextColor(colors.text))); });
                    });
                    dialog.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(30.0),
                            flex_direction: FlexDirection::Row,
                            padding: UiRect::horizontal(Val::Px(4.0)),
                            column_gap: Val::Px(2.0),
                            ..default()
                        },
                        BackgroundColor(colors.menubar_bg),
                    )).with_children(|tabs| {
                        let tab_list = [
                            (tr.settings_tab_general, SettingsTab::General),
                            (tr.settings_tab_appearance, SettingsTab::Appearance),
                            (tr.settings_tab_playback, SettingsTab::Playback),
                            (tr.settings_tab_lyrics, SettingsTab::Lyrics),
                            (tr.settings_tab_equalizer, SettingsTab::Equalizer),
                            (tr.settings_tab_hotkeys, SettingsTab::Hotkeys),
                            (tr.settings_tab_media_lib, SettingsTab::MediaLib),
                        ];
                        for (label, tab) in tab_list {
                            tabs.spawn((
                                Button,
                                Node { padding: UiRect::horizontal(Val::Px(12.0)), height: Val::Px(26.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() },
                                BackgroundColor(Color::NONE),
                                SettingsTabBtn(tab),
                            )).with_children(|btn| { btn.spawn((Text::new(label), TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() }, TextColor(colors.text))); });
                        }
                    });
                    dialog.spawn((
                        Node { width: Val::Percent(100.0), flex_grow: 1.0, padding: UiRect::all(Val::Px(16.0)), flex_direction: FlexDirection::Column, row_gap: Val::Px(8.0), ..default() },
                        BackgroundColor(colors.bg),
                        SettingsContent,
                    ));
                });
        });
}

pub fn toggle_settings(mut state: ResMut<SettingsState>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyO) { state.visible = !state.visible; }
}

pub fn settings_dialog_system(
    state: Res<SettingsState>,
    mut commands: Commands,
    overlay_q: Query<Entity, With<SettingsOverlay>>,
    colors: Res<UiColors>,
    locale: Res<crate::gui::i18n::Locale>,
) {
    if state.is_changed() {
        if state.visible && overlay_q.is_empty() {
            spawn_settings_dialog(&mut commands, &colors, locale.tr);
        } else if !state.visible {
            for e in overlay_q.iter() { commands.entity(e).despawn(); }
        }
    }
}

pub fn handle_settings_interaction(
    mut state: ResMut<SettingsState>,
    mut interaction_q: Query<(&Interaction, Option<&SettingsCloseBtn>, Option<&SettingsTabBtn>), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, close, tab) in interaction_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        if close.is_some() { state.visible = false; }
        else if let Some(t) = tab { state.active_tab = t.0; }
    }
}

pub fn render_settings_content(
    state: Res<SettingsState>,
    content_q: Query<Entity, With<SettingsContent>>,
    mut commands: Commands,
    tr: Res<crate::gui::i18n::Locale>,
    colors: Res<UiColors>,
) {
    if !state.is_changed() { return; }
    if let Ok(entity) = content_q.single() {
        if let Ok(mut e) = commands.get_entity(entity) {
            e.despawn_children();
        }
        spawn_tab_content(&mut commands, entity, state.active_tab, &colors, tr.tr);
    }
}

fn spawn_tab_content(commands: &mut Commands, parent: Entity, tab: SettingsTab, colors: &UiColors, tr: &Tr) {
    commands.entity(parent).with_children(|c| {
        match tab {
            SettingsTab::General => render_general(c, tr, colors),
            SettingsTab::Appearance => render_appearance(c, tr, colors),
            SettingsTab::Playback => render_playback(c, tr, colors),
            SettingsTab::Lyrics => render_lyrics(c, tr, colors),
            SettingsTab::Equalizer => render_equalizer(c, tr, colors),
            SettingsTab::Hotkeys => render_hotkeys(c, tr, colors),
            SettingsTab::MediaLib => render_media_lib(c, tr, colors),
        }
    });
}

fn opt_row(parent: &mut ChildSpawnerCommands, label: &str, colors: &UiColors) {
    parent.spawn((
        Node { width: Val::Percent(100.0), height: Val::Px(28.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() },
    )).with_children(|r| {
        r.spawn((Text::new(label), TextFont { font: crate::gui::ui_font(), font_size: 13.0, ..default() }, TextColor(colors.text)));
    });
}

fn section_title(parent: &mut ChildSpawnerCommands, label: &str, colors: &UiColors) {
    parent.spawn((
        Node { width: Val::Percent(100.0), height: Val::Px(24.0), margin: UiRect::top(Val::Px(8.0)), ..default() },
    )).with_children(|r| {
        r.spawn((Text::new(label), TextFont { font: crate::gui::ui_font(), font_size: 14.0, ..default() }, TextColor(colors.text_title)));
    });
}

fn render_general(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_general, colors);
    opt_row(parent, tr.settings_lang_label, colors);
    opt_row(parent, tr.settings_auto_download, colors);
    opt_row(parent, tr.settings_check_update, colors);
    opt_row(parent, tr.settings_minimize_tray, colors);
}

fn render_appearance(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_appearance, colors);
    opt_row(parent, tr.settings_theme_label, colors);
    opt_row(parent, tr.settings_dark_mode, colors);
    opt_row(parent, tr.settings_show_spectrum, colors);
    opt_row(parent, tr.settings_window_opacity, colors);
    opt_row(parent, tr.settings_always_status, colors);
}

fn render_playback(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_playback, colors);
    opt_row(parent, tr.settings_engine_label, colors);
    opt_row(parent, tr.settings_auto_play, colors);
    opt_row(parent, tr.settings_fade, colors);
    opt_row(parent, tr.settings_remember_pos, colors);
}

fn render_lyrics(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_lyrics, colors);
    opt_row(parent, tr.settings_lyric_download, colors);
    opt_row(parent, tr.settings_lyric_font, colors);
    opt_row(parent, tr.settings_desktop_lyric, colors);
    opt_row(parent, tr.settings_lyric_dual, colors);
}

fn render_equalizer(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_equalizer, colors);
    opt_row(parent, "31 Hz", colors);
    opt_row(parent, "62 Hz", colors);
    opt_row(parent, "125 Hz", colors);
    opt_row(parent, "250 Hz", colors);
    opt_row(parent, "500 Hz", colors);
    opt_row(parent, "1K Hz", colors);
    opt_row(parent, "2K Hz", colors);
    opt_row(parent, "4K Hz", colors);
    opt_row(parent, "8K Hz", colors);
    opt_row(parent, "16K Hz", colors);
}

fn render_hotkeys(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_hotkeys, colors);
    opt_row(parent, tr.settings_hk_enable, colors);
    opt_row(parent, tr.settings_hk_play_pause, colors);
    opt_row(parent, tr.settings_hk_next, colors);
    opt_row(parent, tr.settings_hk_prev, colors);
    opt_row(parent, tr.settings_hk_vol_up, colors);
    opt_row(parent, tr.settings_hk_vol_down, colors);
}

fn render_media_lib(parent: &mut ChildSpawnerCommands, tr: &Tr, colors: &UiColors) {
    section_title(parent, tr.settings_tab_media_lib, colors);
    opt_row(parent, tr.settings_ml_folders, colors);
    opt_row(parent, tr.settings_auto_scan, colors);
    opt_row(parent, tr.settings_ml_ignore_short, colors);
}
