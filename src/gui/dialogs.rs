//! GPUI-based dialog panels: Settings, About, Search.
//! Replaces the legacy bevy-ECS settings.rs.

use gpui::*;
use gpui_component::{h_flex, v_flex};
use gpui_component::button::{Button, ButtonVariants};

use crate::gui::theme::UiColors;
use crate::gui::layout::txt;

/// Settings dialog tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Appearance,
    Playback,
    Hotkeys,
    MediaLib,
}

/// Render the settings panel with tab navigation.
pub fn render_settings_panel(
    c: &UiColors,
    active_tab: SettingsTab,
    _window: &mut Window,
    cx: &mut Context<super::MusicPlayer>,
) -> impl IntoElement {
    let tab_btn = |label: &'static str, id: &'static str, tab: SettingsTab| {
        let is_active = active_tab == tab;
        let bg = if is_active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
        Button::new(id).label(label).compact().ghost().bg(bg)
            .on_click(cx.listener(move |this, _, _w, _cx| {
                this.settings_tab = tab;
            }))
    };

    let content: Vec<AnyElement> = match active_tab {
        SettingsTab::General => render_general_settings(c),
        SettingsTab::Appearance => render_appearance_settings(c),
        SettingsTab::Playback => render_playback_settings(c),
        SettingsTab::Hotkeys => render_hotkeys_settings(c, &cx.entity().clone()),
        SettingsTab::MediaLib => render_media_lib_settings(c),
    };

    v_flex()
        .flex_grow()
        .h_full()
        .bg(c.bg)
        // Title bar
        .child(
            h_flex()
                .items_center()
                .justify_between()
                .w_full()
                .px_4().py_2()
                .bg(c.control_bar_bg)
                .child(txt("设置", 14.0, c.text_title))
        )
        // Tab bar
        .child(
            h_flex()
                .w_full()
                .px_4().py_1()
                .gap_2()
                .bg(c.panel_alt)
                .border_b(px(1.0))
                .border_color(c.border)
                .items_center()
                .child(tab_btn("常规", "set_tab_general", SettingsTab::General))
                .child(tab_btn("外观", "set_tab_appearance", SettingsTab::Appearance))
                .child(tab_btn("播放", "set_tab_playback", SettingsTab::Playback))
                .child(tab_btn("热键", "set_tab_hotkeys", SettingsTab::Hotkeys))
                .child(tab_btn("媒体库", "set_tab_medialib", SettingsTab::MediaLib))
        )
        // Content
        .child(
            v_flex()
                .flex_1()
                .w_full()
                .p_4()
                .gap_2()
                .children(content)
        )
}

fn section_title(c: &UiColors, label: &str) -> AnyElement {
    div()
        .w_full()
        .mt_2()
        .text_size(px(13.0))
        .text_color(c.text_title)
        .font_weight(FontWeight::SEMIBOLD)
        .child(label.to_string())
        .into_any_element()
}

fn setting_row(c: &UiColors, label: &str, control: AnyElement) -> AnyElement {
    h_flex()
        .w_full()
        .h(px(28.0))
        .items_center()
        .gap_4()
        .child(
            div()
                .w(px(140.0))
                .text_size(px(12.0))
                .text_color(c.text)
                .child(label.to_string())
        )
        .child(control)
        .into_any_element()
}

fn toggle_control(c: &UiColors, id: &'static str, enabled: bool) -> AnyElement {
    let label = if enabled { "已启用" } else { "已禁用" };
    let bg = if enabled { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
    Button::new(id).label(label).compact().ghost().bg(bg)
        .into_any_element()
}

fn render_general_settings(c: &UiColors) -> Vec<AnyElement> {
    vec![
        section_title(c, "常规设置"),
        setting_row(c, "语言", toggle_control(c, "gen_lang", true)),
        setting_row(c, "自动下载歌词", toggle_control(c, "gen_auto_lyric", false)),
        setting_row(c, "启动时检查更新", toggle_control(c, "gen_check_update", true)),
        setting_row(c, "最小化到托盘", toggle_control(c, "gen_min_tray", false)),
        setting_row(c, "开机自启动", toggle_control(c, "gen_autostart", false)),
    ]
}

fn render_appearance_settings(c: &UiColors) -> Vec<AnyElement> {
    vec![
        section_title(c, "外观设置"),
        setting_row(c, "深色模式", toggle_control(c, "app_dark", true)),
        setting_row(c, "显示频谱分析", toggle_control(c, "app_spectrum", true)),
        setting_row(c, "窗口透明度", toggle_control(c, "app_opacity", false)),
        setting_row(c, "显示状态栏", toggle_control(c, "app_statusbar", true)),
    ]
}

fn render_playback_settings(c: &UiColors) -> Vec<AnyElement> {
    vec![
        section_title(c, "播放设置"),
        setting_row(c, "音频引擎", toggle_control(c, "play_engine", true)),
        setting_row(c, "自动播放", toggle_control(c, "play_auto", false)),
        setting_row(c, "淡入淡出", toggle_control(c, "play_fade", true)),
        setting_row(c, "记住播放位置", toggle_control(c, "play_remember", true)),
    ]
}

fn render_hotkeys_settings(c: &UiColors, _entity: &Entity<super::MusicPlayer>) -> Vec<AnyElement> {
    vec![
        section_title(c, "全局热键"),
        setting_row(c, "启用全局热键", toggle_control(c, "hk_enable", true)),
        setting_row(c, "播放/暂停", txt("Space", 12.0, c.accent).into_any_element()),
        setting_row(c, "下一曲", txt("Ctrl+Right", 12.0, c.accent).into_any_element()),
        setting_row(c, "上一曲", txt("Ctrl+Left", 12.0, c.accent).into_any_element()),
        setting_row(c, "音量+", txt("Ctrl+Up", 12.0, c.accent).into_any_element()),
        setting_row(c, "音量-", txt("Ctrl+Down", 12.0, c.accent).into_any_element()),
        setting_row(c, "静音", txt("M", 12.0, c.accent).into_any_element()),
        setting_row(c, "显示/隐藏", txt("Ctrl+H", 12.0, c.accent).into_any_element()),
    ]
}

fn render_media_lib_settings(c: &UiColors) -> Vec<AnyElement> {
    vec![
        section_title(c, "媒体库设置"),
        setting_row(c, "媒体库文件夹", toggle_control(c, "ml_folders", true)),
        setting_row(c, "启动时自动扫描", toggle_control(c, "ml_autoscan", true)),
        setting_row(c, "忽略短曲目", toggle_control(c, "ml_ignore_short", true)),
        setting_row(c, "最短时长(秒)", txt("5", 12.0, c.accent).into_any_element()),
    ]
}

/// Render the about dialog.
pub fn render_about_dialog(c: &UiColors) -> impl IntoElement {
    v_flex()
        .items_center()
        .justify_center()
        .flex_grow()
        .h_full()
        .bg(c.bg)
        .gap_3()
        .child(
            div()
                .w(px(64.0)).h(px(64.0))
                .rounded(px(16.0))
                .bg(c.accent)
                .flex()
                .items_center()
                .justify_center()
                .child(txt("♪", 28.0, gpui::white()))
        )
        .child(txt("HackMagic Music Player", 18.0, c.text_title))
        .child(txt("版本 1.0.0", 12.0, c.text_dim))
        .child(txt("基于 Rust + GPUI 重写", 11.0, c.text_dim))
        .child(
            div()
                .mt_4()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child("原始项目: MusicPlayer2 by zhongyang219")
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child("支持: Windows / macOS / Linux")
        )
        .child(
            div()
                .mt_4()
                .text_size(px(10.0))
                .text_color(c.text_dim)
                .child("© 2026 HackMagic Team")
        )
}

/// Render the search panel.
pub fn render_search_panel(
    c: &UiColors,
    query: &str,
    _window: &mut Window,
    _cx: &mut Context<super::MusicPlayer>,
) -> impl IntoElement {
    let results = if query.trim().is_empty() {
        vec![]
    } else {
        let lib = crate::media::MediaLib::load();
        lib.search(query).iter().map(|e| SearchResult {
            title: e.title.clone(),
            artist: e.artist.clone(),
            album: e.album.clone(),
            path: e.file_path.clone(),
        }).collect()
    };

    v_flex()
        .flex_grow()
        .h_full()
        .bg(c.bg)
        .child(
            h_flex()
                .items_center()
                .w_full()
                .px_4().py_2()
                .bg(c.control_bar_bg)
                .gap_2()
                .child(txt("搜索", 14.0, c.text_title))
                .child(div().flex_grow())
                .child(txt(&format!("{} 条结果", results.len()), 11.0, c.text_dim))
        )
        .child(
            v_flex()
                .flex_1()
                .w_full()
                .children(results.iter().enumerate().map(|(i, r)| {
                    let is_even = i % 2 == 0;
                    let bg = if is_even { c.playlist_item } else { c.playlist_item_hover };
                    let title = if r.title.is_empty() { r.path.clone() } else { r.title.clone() };
                    let path = r.path.clone();
                    h_flex()
                        .w_full()
                        .h(px(28.0))
                        .px_4().gap_3()
                        .bg(bg)
                        .hover(|s| s.bg(c.playlist_item_selected))
                        .items_center()
                        .cursor(gpui::CursorStyle::PointingHand)
                        .child(txt(&title, 11.0, c.text))
                        .child(div().flex_grow())
                        .child(txt(&r.artist, 10.0, c.text_dim))
                        .on_mouse_up(gpui::MouseButton::Left, move |_, _, _| {
                            tracing::info!("[Search] 播放: {}", path);
                        })
                }))
        )
}

struct SearchResult {
    title: String,
    artist: String,
    album: String,
    path: String,
}
