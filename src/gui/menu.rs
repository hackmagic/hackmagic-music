use bevy::prelude::*;
use crate::gui::theme::{self, UiColors};
use crate::gui::i18n::Tr;
use crate::gui::settings;
use crate::gui::media_lib;
use crate::gui::AboutState;

#[derive(Resource, Default)]
pub struct MenuState {
    pub open_menu: Option<MenuId>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuId {
    File, Playback, Playlist, Lyric, View, Tools, Help,
}

#[derive(Component)]
pub struct MenuRoot;

#[derive(Component)]
pub struct MenuBarItem(pub MenuId);

#[derive(Component)]
pub struct MenuDropdown(pub MenuId);

#[derive(Component)]
pub struct MenuDropdownItem;

#[derive(Component)]
pub struct MenuAction(pub MenuActionKind);

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MenuActionKind {
    // File
    OpenFile, OpenFolder, OpenUrl, OpenPlaylist, Exit,
    // Playback
    PlayPause, Stop, Previous, Next, Rewind, FastForward,
    SpeedUp, SlowDown, OriginalSpeed,
    PitchUp, PitchDown, OriginalPitch,
    CycleRepeatMode, AbRepeat,
    // Playlist
    AddFile, AddFolder, AddUrl,
    DeleteFromList, DeleteFromDisk, ClearList, RemoveDuplicates, RemoveInvalid,
    ReloadPlaylist, SaveAsNew,
    SortByName, SortByPath, SortByTitle, SortByArtist, SortByAlbum, SortByTrackNo, SortByDuration, SortByModified,
    ShowModeFilename, ShowModeTitle, ShowModeArtistTitle,
    LocateCurrent,
    // Lyric
    ReloadLyric, CopyCurrentLine, CopyAllLyric, EditLyric,
    ShowTranslation, ShowDesktopLyric, DelayOffset,
    DownloadLyric, BatchDownloadLyric,
    // View
    TogglePlaylist, FloatPlaylist, ToggleStandardTitleBar, ToggleMenuBar, ToggleStatusBar,
    AlwaysOnTop, MiniMode, Fullscreen, ToggleDarkMode,
    SwitchUiDefault, SwitchUiClassic, SwitchUiModern,
    // Tools
    MediaLib, Find, Equalizer, Settings,
    ExplorePath, SongInfo, FormatConvert, ReInitBass,
    // Help
    Help, About,
}

pub fn spawn_menu_bar(parent: &mut ChildSpawnerCommands, colors: &UiColors, tr: &Tr) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(theme::MENUBAR_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(4.0)),
                column_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(colors.menubar_bg),
            MenuRoot,
        ))
        .with_children(|bar| {
            let menus = [
                (tr.menu_file, MenuId::File),
                (tr.menu_playback, MenuId::Playback),
                (tr.menu_playlist, MenuId::Playlist),
                (tr.menu_lyric, MenuId::Lyric),
                (tr.menu_view, MenuId::View),
                (tr.menu_tools, MenuId::Tools),
                (tr.menu_help, MenuId::Help),
            ];
            for (label, id) in menus {
                bar.spawn((
                    Button,
                    MenuBarItem(id),
                    Node {
                        padding: UiRect::horizontal(Val::Px(10.0)),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(label),
                        TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                        TextColor(colors.text),
                    ));
                });
            }
        });
}

pub fn spawn_dropdown(commands: &mut Commands, menu_id: MenuId, colors: &UiColors, tr: &Tr) -> Entity {
    let items: Vec<(&str, Option<MenuActionKind>)> = match menu_id {
        MenuId::File => vec![
            (tr.menu_open_file, Some(MenuActionKind::OpenFile)),
            (tr.menu_open_folder, Some(MenuActionKind::OpenFolder)),
            (tr.menu_open_url, Some(MenuActionKind::OpenUrl)),
            (tr.menu_open_playlist, Some(MenuActionKind::OpenPlaylist)),
            ("-", None),
            (tr.menu_exit, Some(MenuActionKind::Exit)),
        ],
        MenuId::Playback => vec![
            (tr.ctrl_play, Some(MenuActionKind::PlayPause)),
            (tr.ctrl_stop, Some(MenuActionKind::Stop)),
            (tr.ctrl_prev, Some(MenuActionKind::Previous)),
            (tr.ctrl_next, Some(MenuActionKind::Next)),
            ("-", None),
            (tr.ctrl_rew, Some(MenuActionKind::Rewind)),
            (tr.ctrl_ff, Some(MenuActionKind::FastForward)),
            ("-", None),
            (tr.menu_speed_up, Some(MenuActionKind::SpeedUp)),
            (tr.menu_slow_down, Some(MenuActionKind::SlowDown)),
            (tr.menu_original_speed, Some(MenuActionKind::OriginalSpeed)),
            ("-", None),
            (tr.menu_cycle_repeat, Some(MenuActionKind::CycleRepeatMode)),
            (tr.menu_ab_repeat, Some(MenuActionKind::AbRepeat)),
        ],
        MenuId::Playlist => vec![
            (tr.menu_add_file, Some(MenuActionKind::AddFile)),
            (tr.menu_add_folder, Some(MenuActionKind::AddFolder)),
            (tr.menu_add_url, Some(MenuActionKind::AddUrl)),
            ("-", None),
            (tr.menu_clear_list, Some(MenuActionKind::ClearList)),
            (tr.menu_remove_duplicates, Some(MenuActionKind::RemoveDuplicates)),
            (tr.menu_remove_invalid, Some(MenuActionKind::RemoveInvalid)),
            ("-", None),
            (tr.menu_reload_playlist, Some(MenuActionKind::ReloadPlaylist)),
            (tr.menu_save_as_new, Some(MenuActionKind::SaveAsNew)),
            ("-", None),
            (tr.menu_locate_current, Some(MenuActionKind::LocateCurrent)),
        ],
        MenuId::Lyric => vec![
            (tr.menu_reload_lyric, Some(MenuActionKind::ReloadLyric)),
            (tr.menu_copy_current_line, Some(MenuActionKind::CopyCurrentLine)),
            (tr.menu_copy_all_lyric, Some(MenuActionKind::CopyAllLyric)),
            ("-", None),
            (tr.menu_edit_lyric, Some(MenuActionKind::EditLyric)),
            ("-", None),
            (tr.menu_show_translation, Some(MenuActionKind::ShowTranslation)),
            (tr.menu_show_desktop_lyric, Some(MenuActionKind::ShowDesktopLyric)),
            ("-", None),
            (tr.menu_download_lyric, Some(MenuActionKind::DownloadLyric)),
            (tr.menu_batch_download_lyric, Some(MenuActionKind::BatchDownloadLyric)),
        ],
        MenuId::View => vec![
            (tr.menu_toggle_playlist, Some(MenuActionKind::TogglePlaylist)),
            (tr.menu_float_playlist, Some(MenuActionKind::FloatPlaylist)),
            ("-", None),
            (tr.menu_toggle_menubar, Some(MenuActionKind::ToggleMenuBar)),
            (tr.menu_toggle_statusbar, Some(MenuActionKind::ToggleStatusBar)),
            ("-", None),
            (tr.menu_always_on_top, Some(MenuActionKind::AlwaysOnTop)),
            (tr.menu_mini_mode, Some(MenuActionKind::MiniMode)),
            (tr.menu_fullscreen, Some(MenuActionKind::Fullscreen)),
            (tr.menu_toggle_dark_mode, Some(MenuActionKind::ToggleDarkMode)),
        ],
        MenuId::Tools => vec![
            (tr.nav_media_lib, Some(MenuActionKind::MediaLib)),
            (tr.menu_find, Some(MenuActionKind::Find)),
            ("-", None),
            (tr.menu_equalizer, Some(MenuActionKind::Equalizer)),
            ("-", None),
            (tr.menu_settings, Some(MenuActionKind::Settings)),
        ],
        MenuId::Help => vec![
            (tr.menu_help_content, Some(MenuActionKind::Help)),
            ("-", None),
            (tr.menu_about, Some(MenuActionKind::About)),
        ],
    };

    let dropdown_entity = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(theme::TITLEBAR_HEIGHT + theme::MENUBAR_HEIGHT),
                left: Val::Px(4.0),
                flex_direction: FlexDirection::Column,
                min_width: Val::Px(200.0),
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(colors.panel),
            Visibility::Hidden,
            MenuDropdown(menu_id),
        ))
        .with_children(|dd| {
            for (label, action) in &items {
                if *label == "-" {
                    dd.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(1.0),
                            margin: UiRect::vertical(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(colors.divider),
                    ));
                } else {
                    let mut item_entity = dd.spawn((
                        Button,
                        MenuDropdownItem,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(24.0),
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(8.0)),
                            column_gap: Val::Px(6.0),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ));
                    if let Some(action_kind) = action {
                        item_entity.insert(MenuAction(action_kind.clone()));
                    }
                    item_entity.with_children(|item| {
                        item.spawn((
                            Text::new(*label),
                            TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                            TextColor(colors.text),
                        ));
                    });
                }
            }
        })
        .id();

    dropdown_entity
}

pub fn handle_menu_bar(
    mut menu_state: ResMut<MenuState>,
    mut bar_q: Query<(&Interaction, &MenuBarItem), (Changed<Interaction>, With<Button>)>,
    mut dd_q: Query<(Entity, &MenuDropdown, &mut Visibility)>,
) {
    for (interaction, bar_item) in bar_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        let currently_open = menu_state.open_menu;

        // Close all
        for (_, _, mut vis) in dd_q.iter_mut() {
            *vis = Visibility::Hidden;
        }

        if currently_open == Some(bar_item.0) {
            menu_state.open_menu = None;
        } else {
            menu_state.open_menu = Some(bar_item.0);
            for (_, dd, mut vis) in dd_q.iter_mut() {
                if dd.0 == bar_item.0 {
                    *vis = Visibility::Visible;
                }
            }
        }
    }
}

pub fn handle_menu_dropdown_hover(
    mut item_q: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<MenuDropdownItem>)>,
    colors: Res<UiColors>,
) {
    for (interaction, mut bg) in item_q.iter_mut() {
        match *interaction {
            Interaction::Hovered => { bg.0 = colors.button_hover; }
            Interaction::Pressed => { bg.0 = colors.button_press; }
            Interaction::None => { bg.0 = Color::NONE; }
        }
    }
}

pub fn handle_menu_actions(
    mut menu_state: ResMut<MenuState>,
    mut item_q: Query<(&Interaction, &MenuAction), (Changed<Interaction>, With<Button>, With<MenuDropdownItem>)>,
    player: Res<crate::gui::PlayerResource>,
    mut settings_state: ResMut<settings::SettingsState>,
    mut media_lib_state: ResMut<media_lib::MediaLibState>,
    _keys: Res<ButtonInput<KeyCode>>,
    locale: Res<crate::gui::i18n::Locale>,
    mut about_state: ResMut<AboutState>,
    mut dd_q: Query<(Entity, &MenuDropdown, &mut Visibility)>,
) {
    for (interaction, action) in item_q.iter_mut() {
        if *interaction != Interaction::Pressed { continue; }
        tracing::info!("[Menu] Action: {:?}", action.0);

        match &action.0 {
            MenuActionKind::OpenFile => {
                crate::gui::open_file_dialog(&player, locale.tr);
            }
            MenuActionKind::OpenFolder => {
                crate::gui::open_folder_dialog(&player, locale.tr);
            }
            MenuActionKind::OpenUrl => {
                tracing::info!("[Menu] Open URL dialog");
            }
            MenuActionKind::OpenPlaylist => {
                tracing::info!("[Menu] Open playlist file");
            }
            MenuActionKind::Exit => std::process::exit(0),
            MenuActionKind::PlayPause => { player.0.toggle_pause().ok(); }
            MenuActionKind::Stop => { player.0.stop().ok(); }
            MenuActionKind::Previous => { player.0.prev().ok(); }
            MenuActionKind::Next => { player.0.next().ok(); }
            MenuActionKind::CycleRepeatMode => {
                use crate::core::playlist::RepeatMode;
                let current = player.0.repeat_mode();
                let next = match current {
                    RepeatMode::PlayOrder | RepeatMode::LoopPlaylist => RepeatMode::LoopTrack,
                    RepeatMode::LoopTrack | RepeatMode::PlayTrack => RepeatMode::PlayRandom,
                    RepeatMode::PlayRandom | RepeatMode::PlayShuffle => RepeatMode::LoopPlaylist,
                };
                player.0.set_repeat_mode(next);
                tracing::info!("[Menu] Repeat mode changed");
            }
            MenuActionKind::ClearList => {
                player.0.stop().ok();
                player.0.playlist_mut().clear();
                tracing::info!("[Menu] Playlist cleared");
            }
            MenuActionKind::MediaLib => {
                media_lib_state.visible = true;
            }
            MenuActionKind::Equalizer => {
                tracing::info!("[Menu] Equalizer dialog");
            }
            MenuActionKind::Settings => {
                settings_state.visible = true;
            }
            MenuActionKind::ToggleDarkMode => {
                tracing::info!("[Menu] Toggle dark mode");
                // ponytail: just toggles via key press, full theme rebuild later
            }
            MenuActionKind::Fullscreen => {
                tracing::info!("[Menu] Fullscreen toggle");
            }
            MenuActionKind::MiniMode => {
                tracing::info!("[Menu] Mini mode toggle");
            }
            MenuActionKind::ShowDesktopLyric => {
                // handled separately via lyrics system
            }
            MenuActionKind::About => {
                tracing::info!("[Menu] About dialog");
                about_state.visible = true;
            }
            _ => tracing::info!("[Menu] Unhandled action: {:?}", action.0),
        }

        // Close menu after action
        menu_state.open_menu = None;
        for (_, _, mut vis) in dd_q.iter_mut() {
            *vis = Visibility::Hidden;
        }
    }
}
