//! GPUI-based dialog panels: Settings, About, Search.
//! Replaces the legacy bevy-ECS settings.rs.

use gpui::*;
use gpui_component::{h_flex, v_flex};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::switch::Switch;

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
    Lyrics,
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
        SettingsTab::Lyrics => render_lyrics_settings(c),
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
                .child(tab_btn("歌词", "set_tab_lyrics", SettingsTab::Lyrics))
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
    let cfg = crate::config::Config::load();
    let opacity = cfg.appearance.window_transparency;
    let opacity_label = if opacity == 0 { "不透明".to_string() } else { format!("{}%", 100 - opacity) };
    let height_label = format!("{}px", cfg.appearance.spectrum_height);
    vec![
        section_title(c, "外观设置"),
        setting_row(c, "深色模式", toggle_control(c, "app_dark", cfg.appearance.dark_mode)),
        setting_row(c, "显示频谱分析", toggle_control(c, "app_spectrum", cfg.appearance.show_spectrum)),
        setting_row(c, "频谱高度", txt(&height_label, 12.0, c.accent).into_any_element()),
        setting_row(c, "显示专辑封面", toggle_control(c, "app_cover", true)),
        setting_row(c, "背景高斯模糊", toggle_control(c, "app_blur", false)),
        setting_row(c, "窗口透明度", txt(&opacity_label, 12.0, c.accent).into_any_element()),
        setting_row(c, "显示状态栏", toggle_control(c, "app_statusbar", true)),
    ]
}

fn render_playback_settings(c: &UiColors) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    let device_count = crate::bass::sys::BASS_GetDeviceCount();
    let current_device = cfg.play.wasapi_device;
    let device_str = if current_device >= 0 && (current_device as u32) < device_count {
        let mut info = crate::bass::sys::BASS_DEVICEINFO {
            name: std::ptr::null(),
            driver: std::ptr::null(),
            flags: 0,
        };
        if crate::bass::sys::BASS_GetDeviceInfo(current_device as u32, &raw mut info) && !info.name.is_null() {
            unsafe {
                let len = (0..).find(|&i| *info.name.offset(i) == 0).unwrap_or(0);
                String::from_utf16_lossy(std::slice::from_raw_parts(info.name, len as usize))
            }
        } else {
            "默认".to_string()
        }
    } else {
        "默认".to_string()
    };
    vec![
        section_title(c, "播放设置"),
        setting_row(c, "音频引擎", txt(&cfg.play.engine, 12.0, c.accent).into_any_element()),
        setting_row(c, "输出设备", txt(&device_str, 12.0, c.accent).into_any_element()),
        setting_row(c, "自动播放", toggle_control(c, "play_auto", cfg.play.auto_play_when_start)),
        setting_row(c, "淡入淡出", toggle_control(c, "play_fade", cfg.play.fade_effect)),
        setting_row(c, "记住播放位置", toggle_control(c, "play_remember", true)),
        section_title(c, "MIDI设置"),
        setting_row(c, "启用MIDI", toggle_control(c, "midi_enabled", cfg.midi.enabled)),
        setting_row(c, "SF2音色库", txt(&cfg.midi.soundfont, 12.0, c.accent).into_any_element()),
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
    let lib = crate::media::MediaLib::load();
    let data_path = crate::config::get_config_dir().join("media_lib.json");
    let data_size = if data_path.exists() {
        let bytes = std::fs::metadata(&data_path).map(|m| m.len()).unwrap_or(0);
        if bytes > 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes > 1024 {
            format!("{:.0} KB", bytes as f64 / 1024.0)
        } else {
            format!("{} B", bytes)
        }
    } else {
        "无数据".to_string()
    };
    vec![
        section_title(c, "媒体库设置"),
        setting_row(c, "媒体库文件夹", toggle_control(c, "ml_folders", true)),
        setting_row(c, "启动时自动扫描", toggle_control(c, "ml_autoscan", true)),
        setting_row(c, "忽略短曲目", toggle_control(c, "ml_ignore_short", true)),
        setting_row(c, "最短时长(秒)", txt("5", 12.0, c.accent).into_any_element()),
        section_title(c, "数据管理"),
        setting_row(c, "数据文件大小", txt(&data_size, 12.0, c.accent).into_any_element()),
        setting_row(c, "清理数据文件", toggle_control(c, "ml_clean", false)),
    ]
}

fn render_lyrics_settings(c: &UiColors) -> Vec<AnyElement> {
    vec![
        section_title(c, "歌词显示"),
        setting_row(c, "卡拉OK风格", toggle_control(c, "lyr_karaoke", true)),
        setting_row(c, "双行显示", toggle_control(c, "lyr_two_line", true)),
        setting_row(c, "内嵌歌词优先", toggle_control(c, "lyr_embedded", true)),
        setting_row(c, "歌词模糊匹配", toggle_control(c, "lyr_fuzzy", true)),
        setting_row(c, "色彩方案", txt("默认", 12.0, c.accent).into_any_element()),
        setting_row(c, "歌词对齐", txt("居中", 12.0, c.accent).into_any_element()),
        setting_row(c, "行间距", txt("正常", 12.0, c.accent).into_any_element()),
        setting_row(c, "阴影效果", toggle_control(c, "lyr_shadow", false)),
        setting_row(c, "字体大小", txt("13px", 12.0, c.accent).into_any_element()),
        section_title(c, "自动歌词处理"),
        setting_row(c, "自动保存歌词", toggle_control(c, "lyr_auto_save", true)),
        setting_row(c, "无歌词时隐藏", toggle_control(c, "lyr_hide_empty", false)),
        setting_row(c, "暂停时隐藏", toggle_control(c, "lyr_hide_paused", false)),
        section_title(c, "歌词下载"),
        setting_row(c, "自动下载歌词", toggle_control(c, "lyr_auto_download", true)),
        setting_row(c, "下载翻译", toggle_control(c, "lyr_download_trans", false)),
        setting_row(c, "下载编码", txt("UTF-8", 12.0, c.accent).into_any_element()),
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
                        .on_mouse_up(gpui::MouseButton::Left, {
                            let path2 = path.clone();
                            move |_, _, _| {
                                tracing::info!("[Search] 播放: {}", path2);
                            }
                        })
                        .context_menu(move |menu, _w, _cx| {
                            let p_play = path.clone();
                            let p_open = path.clone();
                            let p_add = path.clone();
                            let p_copy = path.clone();
                            menu.item(PopupMenuItem::new("播放").on_click(move |_, _, _| {
                                tracing::info!("[Search] 播放: {}", p_play);
                            }))
                            .item(PopupMenuItem::new("添加到播放列表").on_click(move |_, _, _| {
                                tracing::info!("[Search] 添加到播放列表: {}", p_add);
                            }))
                            .separator()
                            .item(PopupMenuItem::new("打开文件位置").on_click(move |_, _, _| {
                                let path = std::path::Path::new(&p_open);
                                if let Some(_parent) = path.parent() {
                                    #[cfg(windows)]
                                    { let _ = std::process::Command::new("explorer").arg("/select,").arg(&p_open).spawn(); }
                                    #[cfg(not(windows))]
                                    { let _ = std::process::Command::new("xdg-open").arg(&p_open).spawn(); }
                                }
                                tracing::info!("[Search] 打开文件位置: {}", p_open);
                            }))
                            .item(PopupMenuItem::new("复制路径").on_click(move |_, _, _| {
                                tracing::info!("[Search] 复制路径: {}", p_copy);
                            }))
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
