//! Root-level window layout - MusicPlayer2-style layout.

use bevy::prelude::*;
use crate::gui::styles::*;
use crate::gui::player_info;
use crate::gui::playlist;
use crate::gui::controls;

// ---------------------------------------------------------------------------
// Marker components
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct RootContainer;
#[derive(Component)]
pub struct TitleBar;
#[derive(Component)]
pub struct MenuBar;
#[derive(Component)]
pub struct MainContent;
#[derive(Component)]
pub struct InfoPanel;
#[derive(Component)]
pub struct PlaylistPanel;
#[derive(Component)]
pub struct ControlBar;
#[derive(Component)]
pub struct StatusBar;

// Title-bar buttons
#[derive(Component)]
pub struct TitleBtnMinimize;
#[derive(Component)]
pub struct TitleBtnMaximize;
#[derive(Component)]
pub struct TitleBtnClose;

// Menu item marker
#[derive(Component)]
pub struct MenuBtn;

// Menu item labels
#[derive(Component)]
pub struct MenuLabelPlay;
#[derive(Component)]
pub struct MenuLabelList;
#[derive(Component)]
pub struct MenuLabelTool;
#[derive(Component)]
pub struct MenuLabelOption;
#[derive(Component)]
pub struct MenuLabelHelp;

// ---------------------------------------------------------------------------
// Layout builder - entry point
// ---------------------------------------------------------------------------

pub fn spawn_layout(commands: &mut Commands, colors: &UiColors) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(colors.bg),
            RootContainer,
        ))
        .with_children(|root| {
            spawn_titlebar(root, colors);
            spawn_menubar(root, colors);
            spawn_main_content(root, colors);
            spawn_controlbar(root, colors);
            spawn_statusbar(root, colors);
        });
}

// ---------------------------------------------------------------------------
// Title bar
// ---------------------------------------------------------------------------

fn spawn_titlebar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(TITLEBAR_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(colors.titlebar_bg),
            TitleBar,
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new("\u{266B}"),
                TextFont { font: crate::gui::ui_font(), font_size: 16.0, ..default() },
                TextColor(colors.text_dim),
                Node { width: Val::Px(24.0), ..default() },
            ));
            bar.spawn((
                Text::new("HackMagic Music Player"),
                TextFont { font: crate::gui::ui_font(), font_size: 12.0, ..default() },
                TextColor(colors.text_dim),
                Node { flex_grow: 1.0, ..default() },
            ));
            title_btn(bar, "\u{2014}", TitleBtnMinimize, colors.btn_minmax, colors);
            title_btn(bar, "\u{25A1}", TitleBtnMaximize, colors.btn_minmax, colors);
            title_btn(bar, "\u{00D7}", TitleBtnClose, colors.btn_close_hover, colors);
        });
}

fn title_btn(parent: &mut ChildSpawnerCommands, label: &str, _marker: impl Component, bg: Color, _colors: &UiColors) {
    parent.spawn((
        Button,
        Node {
            width: Val::Px(46.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(bg),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font: crate::gui::ui_font(), font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.85, 0.85, 0.88)),
        ));
    });
}

// ---------------------------------------------------------------------------
// Menu bar
// ---------------------------------------------------------------------------

fn spawn_menubar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(MENUBAR_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(4.0)),
                column_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(colors.menubar_bg),
            MenuBar,
        ))
        .with_children(|menu| {
            menu_item(menu, "Play", MenuLabelPlay, colors);
            menu_item(menu, "List", MenuLabelList, colors);
            menu_item(menu, "Tools", MenuLabelTool, colors);
            menu_item(menu, "Options", MenuLabelOption, colors);
            menu_item(menu, "Help", MenuLabelHelp, colors);
        });
}

fn menu_item(parent: &mut ChildSpawnerCommands, label: &str, _marker: impl Component, colors: &UiColors) {
    parent.spawn((
        Button,
        MenuBtn,
        _marker,
        Node {
            padding: UiRect::horizontal(Val::Px(10.0)),
            height: Val::Px(20.0),
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

// ---------------------------------------------------------------------------
// Main content: InfoPanel (left) + PlaylistPanel (right)
// ---------------------------------------------------------------------------

fn spawn_main_content(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                ..default()
            },
            MainContent,
        ))
        .with_children(|content| {
            // Left: info panel
            content
                .spawn((
                    Node {
                        width: Val::Percent(50.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        padding: UiRect::all(Val::Px(12.0)),
                        ..default()
                    },
                    BackgroundColor(colors.panel),
                    InfoPanel,
                ))
                .with_children(|info| {
                    player_info::spawn_info_panel(info, colors);
                });
            // Right: playlist panel
            content
                .spawn((
                    Node {
                        width: Val::Percent(50.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(colors.bg),
                    PlaylistPanel,
                ))
                .with_children(|pl| {
                    playlist::spawn_playlist_panel(pl, colors);
                });
        });
}

// ---------------------------------------------------------------------------
// Control bar
// ---------------------------------------------------------------------------

fn spawn_controlbar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(CONTROL_BAR_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(Val::Px(16.0)),
                column_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(colors.panel),
            ControlBar,
        ))
        .with_children(|bar| {
            ctrl_btn(bar, "\u{23EE}", colors.button, colors, controls::BtnPrev);
            ctrl_btn(bar, "\u{25B6}", colors.button_accent, colors, controls::BtnPlayPause);
            ctrl_btn(bar, "\u{23F9}", colors.button, colors, controls::BtnStop);
            ctrl_btn(bar, "\u{23ED}", colors.button, colors, controls::BtnNext);
            bar.spawn(Node { flex_grow: 1.0, ..default() });
            ctrl_btn(bar, "\u{2212}", colors.button, colors, controls::BtnVolDown);
            bar.spawn((
                Node { width: Val::Px(80.0), height: Val::Px(6.0), ..default() },
                BackgroundColor(colors.progress_track),
                controls::VolumeSlider,
            ));
            ctrl_btn(bar, "+", colors.button, colors, controls::BtnVolUp);
        });
}

fn ctrl_btn(parent: &mut ChildSpawnerCommands, label: &str, bg: Color, colors: &UiColors, _marker: impl Component) {
    parent.spawn((
        Button,
        _marker,
        Node {
            width: Val::Px(BUTTON_SIZE),
            height: Val::Px(BUTTON_SIZE),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(bg),
    )).with_children(|btn| {
        btn.spawn((
            Text::new(label),
            TextFont { font: crate::gui::ui_font(), font_size: BUTTON_ICON_SIZE, ..default() },
            TextColor(colors.text),
        ));
    });
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn spawn_statusbar(parent: &mut ChildSpawnerCommands, colors: &UiColors) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(STATUSBAR_HEIGHT),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(8.0)),
                column_gap: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(colors.statusbar_bg),
            StatusBar,
        ))
        .with_children(|bar| {
            bar.spawn((
                Text::new("FPS: --"),
                TextFont { font: crate::gui::ui_font(), font_size: 11.0, ..default() },
                TextColor(colors.text_dim),
                controls::StatusFps,
            ));
            bar.spawn((
                Text::new("|"),
                TextFont { font: crate::gui::ui_font(), font_size: 11.0, ..default() },
                TextColor(colors.divider),
            ));
            bar.spawn((
                Text::new("Next: --"),
                TextFont { font: crate::gui::ui_font(), font_size: 11.0, ..default() },
                TextColor(colors.text_dim),
                Node { flex_grow: 1.0, ..default() },
                controls::StatusNextTrack,
            ));
            bar.spawn((
                Text::new("\u{1F503} Loop"),
                TextFont { font: crate::gui::ui_font(), font_size: 11.0, ..default() },
                TextColor(colors.text_dim),
                controls::StatusRepeatMode,
            ));
        });
}