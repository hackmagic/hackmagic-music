//! GPUI-based dialog panels: Settings, About, Search.
//! Replaces the legacy bevy-ECS settings.rs.

use gpui::*;
use gpui_component::{h_flex, v_flex};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::input::{Input, InputState};

use crate::gui::theme::UiColors;
use crate::gui::i18n::Tr;
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
    let entity = cx.entity().downgrade();

    let build_tab_btn = |label: &'static str, id: &'static str, tab: SettingsTab| {
        let is_active = active_tab == tab;
        let bg = if is_active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
        let entity_clone = entity.clone();
        Button::new(id).label(label).compact().ghost().bg(bg)
            .on_click(move |_, _, cx| {
                let _ = entity_clone.update(cx, |this, _cx| {
                    this.settings_tab = tab;
                });
            })
    };

    let content: Vec<AnyElement> = match active_tab {
        SettingsTab::General => render_general_settings(c, cx),
        SettingsTab::Appearance => render_appearance_settings(c, cx),
        SettingsTab::Playback => render_playback_settings(c, cx),
        SettingsTab::Hotkeys => render_hotkeys_settings(c, cx),
        SettingsTab::MediaLib => render_media_lib_settings(c, cx),
        SettingsTab::Lyrics => render_lyrics_settings(c, cx),
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
                .child(build_tab_btn("常规", "set_tab_general", SettingsTab::General))
                .child(build_tab_btn("外观", "set_tab_appearance", SettingsTab::Appearance))
                .child(build_tab_btn("播放", "set_tab_playback", SettingsTab::Playback))
                .child(build_tab_btn("歌词", "set_tab_lyrics", SettingsTab::Lyrics))
                .child(build_tab_btn("热键", "set_tab_hotkeys", SettingsTab::Hotkeys))
                .child(build_tab_btn("媒体库", "set_tab_medialib", SettingsTab::MediaLib))
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

/// Create a toggle button that reads/writes a boolean config key.
fn toggle_config(
    c: &UiColors,
    id: &'static str,
    enabled: bool,
    config_key: &'static str,
    on_toggled: Option<Box<dyn Fn(bool, &mut super::MusicPlayer, &mut Window, &mut Context<super::MusicPlayer>) + 'static>>,
    cx: &mut Context<super::MusicPlayer>,
) -> AnyElement {
    let label = if enabled { "已启用" } else { "已禁用" };
    let bg = if enabled { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
    let entity = cx.entity().downgrade();
    Button::new(id)
        .label(label)
        .compact()
        .ghost()
        .bg(bg)
        .on_click(move |_, _window, cx| {
            let new_val = !enabled;
            let _ = crate::config::Config::set(config_key, if new_val { "true" } else { "false" });
            if let Some(ref callback) = on_toggled {
                let _ = entity.update(cx, |this, cx| {
                    (callback)(new_val, this, _window, cx);
                });
            } else {
                let _ = entity.update(cx, |_, _cx| {});
            }
        })
        .into_any_element()
}

// ---------------------------------------------------------------------------
// General settings
// ---------------------------------------------------------------------------
fn render_general_settings(
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    vec![
        section_title(c, "常规设置"),
        setting_row(c, "语言", txt(
            match cfg.general.language.as_str() {
                "zh-CN" | "zh" => "简体中文",
                _ => "English",
            },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "自动下载歌词", toggle_config(c, "gen_auto_lyric",
            cfg.general.auto_download_lyric, "general.auto_download_lyric", None, cx)),
        setting_row(c, "自动下载封面", toggle_config(c, "gen_auto_cover",
            cfg.general.auto_download_album_cover, "general.auto_download_album_cover", None, cx)),
        setting_row(c, "ID3v2优先", toggle_config(c, "gen_id3v2",
            cfg.general.id3v2_first, "general.id3v2_first", None, cx)),
        setting_row(c, "启动时检查更新", toggle_config(c, "gen_check_update",
            cfg.general.check_update_when_start, "general.check_update_when_start", None, cx)),
        setting_row(c, "最小化到托盘", toggle_config(c, "gen_min_tray",
            cfg.general.minimize_to_notify_icon, "general.minimize_to_notify_icon", None, cx)),
    ]
}

// ---------------------------------------------------------------------------
// Appearance settings
// ---------------------------------------------------------------------------
fn render_appearance_settings(
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    let opacity_val = cfg.appearance.window_transparency;
    let opacity_label = if opacity_val >= 100 {
        "不透明".to_string()
    } else {
        format!("{:.0}%", opacity_val as f32)
    };
    let spectrum_height_label = format!("{}px", cfg.appearance.spectrum_height);

    vec![
        section_title(c, "外观设置"),
        setting_row(c, "深色模式", toggle_config(c, "app_dark",
            cfg.appearance.dark_mode, "appearance.dark_mode",
            Some(Box::new(move |new_val, this, _window, cx| {
                let mut new_cfg = crate::config::Config::load();
                new_cfg.appearance.dark_mode = new_val;
                let theme_name = crate::gui::theme::ThemeName::from_config(&new_cfg.appearance.theme);
                this.colours = crate::gui::theme::UiColors::build(new_val, &theme_name);
                // Force repaint of everything
                cx.notify();
            })),
            cx)),
        setting_row(c, "显示频谱分析", toggle_config(c, "app_spectrum",
            cfg.appearance.show_spectrum, "appearance.show_spectrum", None, cx)),
        setting_row(c, "频谱高度", txt(&spectrum_height_label, 12.0, c.accent).into_any_element()),
        setting_row(c, "显示专辑封面", txt("是", 12.0, c.accent).into_any_element()),
        setting_row(c, "背景高斯模糊", toggle_config(c, "app_blur", false, "appearance.blur", None, cx)),
        setting_row(c, "窗口透明度", txt(&opacity_label, 12.0, c.accent).into_any_element()),
        setting_row(c, "显示状态栏",
            if cfg.appearance.dark_mode { txt("已启用", 12.0, c.accent).into_any_element() }
            else { txt("已启用", 12.0, c.accent).into_any_element() }),
    ]
}

// ---------------------------------------------------------------------------
// Playback settings
// ---------------------------------------------------------------------------
fn render_playback_settings(
    c: &UiColors,
    _cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
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
                if len > 0 {
                    String::from_utf16_lossy(std::slice::from_raw_parts(info.name, len as usize))
                } else {
                    "默认".to_string()
                }
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
        setting_row(c, "自动播放", txt(
            if cfg.play.auto_play_when_start { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "淡入淡出", txt(
            if cfg.play.fade_effect { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "淡入淡出时间", txt(&format!("{} ms", cfg.play.fade_time), 12.0, c.accent).into_any_element()),
        setting_row(c, "默认音量", txt(&format!("{}%", cfg.play.default_volume), 12.0, c.accent).into_any_element()),
        setting_row(c, "音量步进", txt(&cfg.play.volume_step.to_string(), 12.0, c.accent).into_any_element()),
        setting_row(c, "ReplayGain", txt(&cfg.play.replaygain, 12.0, c.accent).into_any_element()),
        setting_row(c, "记忆播放位置", txt("已启用", 12.0, c.accent).into_any_element()),
        setting_row(c, "合并多版本", toggle_config(c, "play_merge",
            cfg.play.merge_song_different_versions, "play.merge_song_different_versions", None, _cx)),
        setting_row(c, "FFmpeg缓存", txt(&format!("{}秒", cfg.play.ffmpeg_cache_len), 12.0, c.accent).into_any_element()),
        section_title(c, "MIDI设置"),
        setting_row(c, "启用MIDI", txt(
            if cfg.midi.enabled { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "SF2音色库", txt(
            if cfg.midi.soundfont.is_empty() { "未设置" } else { &cfg.midi.soundfont },
            12.0, c.accent,
        ).into_any_element()),
    ]
}

// ---------------------------------------------------------------------------
// Hotkey settings
// ---------------------------------------------------------------------------
fn render_hotkeys_settings(
    c: &UiColors,
    _cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    vec![
        section_title(c, "全局热键"),
        setting_row(c, "启用全局热键", txt("已启用", 12.0, c.accent).into_any_element()),
        setting_row(c, "播放/暂停", txt("Space", 12.0, c.accent).into_any_element()),
        setting_row(c, "停止", txt("Ctrl+S", 12.0, c.accent).into_any_element()),
        setting_row(c, "下一曲", txt("Ctrl+Right / Media Next", 12.0, c.accent).into_any_element()),
        setting_row(c, "上一曲", txt("Ctrl+Left / Media Prev", 12.0, c.accent).into_any_element()),
        setting_row(c, "快进5秒", txt("Ctrl+Shift+Right", 12.0, c.accent).into_any_element()),
        setting_row(c, "快退5秒", txt("Ctrl+Shift+Left", 12.0, c.accent).into_any_element()),
        setting_row(c, "音量+", txt("Ctrl+Up / Media Up", 12.0, c.accent).into_any_element()),
        setting_row(c, "音量-", txt("Ctrl+Down / Media Down", 12.0, c.accent).into_any_element()),
        setting_row(c, "静音", txt("M", 12.0, c.accent).into_any_element()),
        setting_row(c, "显示/隐藏", txt("Ctrl+H", 12.0, c.accent).into_any_element()),
        setting_row(c, "显示桌面歌词", txt("Ctrl+L", 12.0, c.accent).into_any_element()),
    ]
}

// ---------------------------------------------------------------------------
// Media library settings
// ---------------------------------------------------------------------------
fn render_media_lib_settings(
    c: &UiColors,
    _cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
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

    let mut children = vec![
        section_title(c, "媒体库设置"),
        setting_row(c, "启动时自动扫描", txt(
            if cfg.media_lib.auto_scan { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "忽略短曲目", txt(
            &(if cfg.media_lib.min_duration_secs > 0 { format!("{}秒", cfg.media_lib.min_duration_secs) } else { "不过滤".to_string() }),
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "显示格式", txt(&cfg.media_lib.display_format, 12.0, c.accent).into_any_element()),
        setting_row(c, "播放列表行高", txt(&format!("{}px", cfg.appearance.playlist_row_height), 12.0, c.accent).into_any_element()),
        section_title(c, "扫描文件夹"),
    ];

    // List media directories
    for (i, dir) in cfg.media_lib.media_dirs.iter().enumerate() {
        children.push(
            setting_row(c, &format!("  目录 {}", i + 1), txt(dir, 11.0, c.text_dim).into_any_element())
        );
    }

    children.extend(vec![
        section_title(c, "数据管理"),
        setting_row(c, "媒体库条目数", txt(&format!("{} 首", lib.entries.len()), 12.0, c.accent).into_any_element()),
        setting_row(c, "数据文件大小", txt(&data_size, 12.0, c.accent).into_any_element()),
    ].into_iter());

    children
}

// ---------------------------------------------------------------------------
// Lyrics settings
// ---------------------------------------------------------------------------
fn render_lyrics_settings(
    c: &UiColors,
    _cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    vec![
        section_title(c, "歌词显示"),
        setting_row(c, "卡拉OK风格", txt("已启用", 12.0, c.accent).into_any_element()),
        setting_row(c, "双行显示", txt("已启用", 12.0, c.accent).into_any_element()),
        setting_row(c, "内嵌歌词优先", txt(
            if cfg.lyric.use_inner_lyric_first { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "歌词模糊匹配", txt(
            if cfg.lyric.fuzzy_match { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "显示翻译", txt(
            if cfg.lyric.show_translate { "已启用" } else { "已禁用" },
            12.0, c.accent,
        ).into_any_element()),
        setting_row(c, "歌词对齐", txt("居中", 12.0, c.accent).into_any_element()),
        setting_row(c, "行间距", txt("正常", 12.0, c.accent).into_any_element()),
        setting_row(c, "字体大小", txt("13px", 12.0, c.accent).into_any_element()),
        section_title(c, "桌面歌词"),
        setting_row(c, "字体颜色", txt("默认", 12.0, c.accent).into_any_element()),
        setting_row(c, "高亮颜色", txt("默认", 12.0, c.accent).into_any_element()),
        setting_row(c, "背景透明度", txt("80%", 12.0, c.accent).into_any_element()),
        section_title(c, "自动歌词处理"),
        setting_row(c, "自动保存歌词", txt("已启用", 12.0, c.accent).into_any_element()),
        setting_row(c, "无歌词时隐藏", txt("已禁用", 12.0, c.accent).into_any_element()),
        setting_row(c, "暂停时隐藏", txt("已禁用", 12.0, c.accent).into_any_element()),
    ]
}

/// Render the about dialog.
pub fn render_about_dialog(tr: &Tr, c: &UiColors) -> impl IntoElement {
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
                .child(tr.about_original)
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child(tr.about_platforms)
        )
        .child(
            div()
                .mt_4()
                .text_size(px(10.0))
                .text_color(c.text_dim)
                .child(tr.about_copyright)
        )
}

/// Render the search panel with an editable search input field.
pub fn render_search_panel(
    c: &UiColors,
    query: &str,
    search_input: &Entity<InputState>,
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
        // Header with search input
        .child(
            h_flex()
                .items_center()
                .w_full()
                .px_4().py_2()
                .bg(c.control_bar_bg)
                .gap_2()
                .child(txt("搜索", 14.0, c.text_title))
                .child(
                    h_flex().flex_grow().h(px(24.0))
                        .bg(c.panel).rounded(px(4.0)).px_2()
                        .child(
                        Input::new(&search_input)

                                .w_full()
                        )
                )
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
