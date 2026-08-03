//! GPUI-based dialog panels: Settings, About, Search.
//! Replaces the legacy bevy-ECS settings.rs.

use gpui::*;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::{h_flex, v_flex};

use crate::gui::i18n::Tr;
use crate::gui::layout::txt;
use crate::gui::theme::UiColors;
use gpui_component::scroll::ScrollableElement;

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
    tr: &Tr,
    c: &UiColors,
    active_tab: SettingsTab,
    _window: &mut Window,
    cx: &mut Context<super::MusicPlayer>,
) -> impl IntoElement {
    let entity = cx.entity().downgrade();

    let build_tab_btn = |label: &'static str, id: &'static str, tab: SettingsTab| {
        let is_active = active_tab == tab;
        let bg = if is_active {
            c.accent
        } else {
            Hsla {
                h: 0.0,
                s: 0.0,
                l: 0.0,
                a: 0.0,
            }
        };
        let text_color = if is_active { c.bg } else { c.text };
        let entity_clone = entity.clone();
        Button::new(id)
            .label(label)
            .ghost()
            .px_3()
            .py_1()
            .text_size(px(11.0))
            .text_color(text_color)
            .bg(bg)
            .on_click(move |_, _, cx| {
                let _ = entity_clone.update(cx, |this, _cx| {
                    this.settings_tab = tab;
                });
            })
    };

    let content: Vec<AnyElement> = match active_tab {
        SettingsTab::Lyrics => render_lyrics_settings(tr, c, cx),
        SettingsTab::Appearance => render_appearance_settings(tr, c, cx),
        SettingsTab::General => render_general_settings(tr, c, cx),
        SettingsTab::Playback => render_playback_settings(tr, c, cx),
        SettingsTab::MediaLib => render_media_lib_settings(tr, c, cx),
        SettingsTab::Hotkeys => render_hotkeys_settings(tr, c, cx),
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
                .px_4()
                .py_2()
                .bg(c.control_bar_bg)
                .child(txt(tr.settings_title, 14.0, c.text_title)),
        )
        // Tab bar
        .child(
            h_flex()
                .w_full()
                .px_4()
                .py_1()
                .gap_1()
                .bg(c.panel_alt)
                .border_b(px(1.0))
                .border_color(c.border)
                .items_center()
                .child(build_tab_btn(
                    tr.settings_tab_lyrics,
                    "set_tab_lyrics",
                    SettingsTab::Lyrics,
                ))
                .child(build_tab_btn(
                    tr.settings_tab_appearance,
                    "set_tab_appearance",
                    SettingsTab::Appearance,
                ))
                .child(build_tab_btn(
                    tr.settings_tab_general,
                    "set_tab_general",
                    SettingsTab::General,
                ))
                .child(build_tab_btn(
                    tr.settings_tab_playback,
                    "set_tab_playback",
                    SettingsTab::Playback,
                ))
                .child(build_tab_btn(
                    tr.settings_tab_media_lib,
                    "set_tab_medialib",
                    SettingsTab::MediaLib,
                ))
                .child(build_tab_btn(
                    tr.settings_tab_hotkeys,
                    "set_tab_hotkeys",
                    SettingsTab::Hotkeys,
                )),
        )
        // Content (scrollable)
        .child(
            v_flex()
                .flex_1()
                .w_full()
                .p_4()
                .gap_2()
                .overflow_y_scrollbar()
                .children(content),
        )
        // Bottom button bar
        .child(
            h_flex()
                .w_full()
                .justify_end()
                .px_4()
                .py_2()
                .gap_2()
                .border_t(px(1.0))
                .border_color(c.border)
                .child({
                    let e = entity.clone();
                    Button::new("btn_ok")
                        .label(tr.settings_btn_ok)
                        .compact()
                        .on_click(move |_, _, cx| {
                            let _ = e.update(cx, |this, cx| {
                                this.modal = None;
                                super::ACTIVE_PANEL.store(0, std::sync::atomic::Ordering::Relaxed);
                                cx.notify();
                            });
                        })
                })
                .child({
                    let e = entity.clone();
                    Button::new("btn_cancel")
                        .label(tr.settings_btn_cancel)
                        .compact()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            let _ = e.update(cx, |this, cx| {
                                this.modal = None;
                                super::ACTIVE_PANEL.store(0, std::sync::atomic::Ordering::Relaxed);
                                cx.notify();
                            });
                        })
                })
                .child({
                    let e = entity.clone();
                    Button::new("btn_apply")
                        .label(tr.settings_btn_apply)
                        .compact()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            // Settings already take effect immediately; just refresh the view.
                            let _ = e.update(cx, |_, cx| cx.notify());
                        })
                }),
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
        .min_h(px(28.0))
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(160.0))
                .flex_shrink_0()
                .text_size(px(11.0))
                .text_color(c.text)
                .child(label.to_string()),
        )
        .child(control)
        .into_any_element()
}

/// Callback invoked when a settings toggle changes.
type ToggleCallback = Box<
    dyn Fn(bool, &mut super::MusicPlayer, &mut Window, &mut Context<super::MusicPlayer>) + 'static,
>;

/// Callback that formats a numeric setting value for display.
type LabelFn = Box<dyn Fn(u32) -> String + 'static>;

/// Create a toggle button that reads/writes a boolean config key.
#[allow(clippy::too_many_arguments)]
fn toggle_config(
    c: &UiColors,
    id: &'static str,
    enabled: bool,
    config_key: &'static str,
    on_toggled: Option<ToggleCallback>,
    cx: &mut Context<super::MusicPlayer>,
) -> AnyElement {
    let entity = cx.entity().downgrade();
    let bg = if enabled {
        c.accent.opacity(0.15)
    } else {
        Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.0,
            a: 0.0,
        }
    };
    let border_color = if enabled { c.accent } else { c.border };
    Button::new(id)
        .label(if enabled { "✓" } else { "✖" })
        .compact()
        .ghost()
        .text_size(px(11.0))
        .h(px(20.0))
        .min_w(px(28.0))
        .bg(bg)
        .border_1()
        .border_color(border_color)
        .rounded(px(4.0))
        .on_click(move |_, _window, cx| {
            let new_val = !enabled;
            let _ = crate::config::Config::set(config_key, if new_val { "true" } else { "false" });
            if let Some(ref callback) = on_toggled {
                let _ = entity.update(cx, |this, cx| {
                    (callback)(new_val, this, _window, cx);
                });
            } else {
                let _ = entity.update(cx, |_, cx| {
                    cx.notify();
                });
            }
        })
        .into_any_element()
}

/// A horizontal slider that reads/writes a numeric config key.
#[allow(clippy::too_many_arguments)]
fn slider_config(
    c: &UiColors,
    id: &'static str,
    value: u32,
    min: u32,
    max: u32,
    step: u32,
    config_key: &'static str,
    label_fn: Option<LabelFn>,
    cx: &mut Context<super::MusicPlayer>,
) -> AnyElement {
    let display = if let Some(ref f) = label_fn {
        f(value)
    } else {
        value.to_string()
    };
    // Guard against a stale config value outside [min, max] (u32 underflow /
    // division by zero when computing the fill width).
    let v = value.clamp(min, max);
    let fill_width = if max > min {
        (v - min) as f32 / (max - min) as f32 * 120.0
    } else {
        0.0
    };
    let entity = cx.entity().downgrade();
    h_flex()
        .items_center()
        .gap_2()
        .child({
            let e = entity.clone();
            let key = config_key;
            let k2 = key;
            Button::new((id, 0u64))
                .label("−")
                .compact()
                .ghost()
                .on_click(move |_, _, cx| {
                    let new_val = value.saturating_sub(step).max(min);
                    let _ = crate::config::Config::set(k2, &new_val.to_string());
                    let _ = e.update(cx, |_, cx| cx.notify());
                })
        })
        .child(
            div()
                .w(px(120.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(c.border)
                .child(
                    div()
                        .h(px(6.0))
                        .rounded(px(3.0))
                        .bg(c.accent)
                        .w(px(fill_width)),
                ),
        )
        .child({
            let e = entity.clone();
            let key = config_key;
            Button::new((id, 1u64))
                .label("+")
                .compact()
                .ghost()
                .on_click(move |_, _, cx| {
                    let new_val = (value + step).min(max);
                    let _ = crate::config::Config::set(key, &new_val.to_string());
                    let _ = e.update(cx, |_, cx| cx.notify());
                })
        })
        .child(
            div()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child(display),
        )
        .into_any_element()
}

/// A radio button group item that writes a string config value.
fn radio_config(
    c: &UiColors,
    id: &'static str,
    selected: bool,
    label: &'static str,
    config_key: &'static str,
    config_value: &'static str,
    cx: &mut Context<super::MusicPlayer>,
) -> AnyElement {
    let entity = cx.entity().downgrade();
    Button::new(id)
        .label(if selected { "●" } else { "○" })
        .compact()
        .ghost()
        .on_click(move |_, _, cx| {
            let _ = crate::config::Config::set(config_key, config_value);
            let _ = entity.update(cx, |_, cx| cx.notify());
        })
        .child(txt(
            label,
            12.0,
            if selected { c.text_title } else { c.text },
        ))
        .into_any_element()
}

/// A dropdown selector that reads/writes a string config key.
fn dropdown_config(
    _c: &UiColors,
    _id: &'static str,
    current: &str,
    options: &[(&'static str, &'static str)],
    config_key: &'static str,
    cx: &mut Context<super::MusicPlayer>,
) -> AnyElement {
    let entity = cx.entity().downgrade();
    let current_label = options
        .iter()
        .find(|(v, _)| *v == current)
        .map(|(_, l)| l.to_string())
        .unwrap_or_else(|| current.to_string());
    let entity_clone = entity.clone();
    let key = config_key.to_string();
    let current_owned = current.to_string();
    let options_owned: Vec<(String, String)> = options
        .iter()
        .map(|(v, l)| (v.to_string(), l.to_string()))
        .collect();
    let btn_id: SharedString = format!("dd_{}", config_key).into();
    Button::new(btn_id)
        .label(current_label)
        .compact()
        .ghost()
        .bg(Hsla {
            h: 0.0,
            s: 0.0,
            l: 0.1,
            a: 0.5,
        })
        .on_click(move |_, _, cx| {
            let current_idx = options_owned
                .iter()
                .position(|(v, _)| *v == current_owned)
                .unwrap_or(0);
            let next_idx = (current_idx + 1) % options_owned.len();
            let new_val = options_owned[next_idx].0.clone();
            let _ = crate::config::Config::set(&key, &new_val);
            // For language switching, update the global language atom
            if key == "general.language" {
                let lang = match new_val.as_str() {
                    "en" => crate::gui::i18n::Lang::EnUs,
                    _ => crate::gui::i18n::Lang::ZhCn,
                };
                crate::gui::i18n::set_global_lang(lang);
            }
            let _ = entity_clone.update(cx, |_, cx| cx.notify());
        })
        .into_any_element()
}

// ---------------------------------------------------------------------------
// General settings
// ---------------------------------------------------------------------------
fn render_general_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    vec![
        // Group 1: Application Settings
        section_title(c, tr.settings_section_app),
        setting_row(
            c,
            tr.settings_check_update,
            toggle_config(
                c,
                "gen_check_update",
                cfg.general.check_update_when_start,
                "general.check_update_when_start",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_update_source,
            dropdown_config(
                c,
                "gen_update_src",
                &cfg.general.update_source,
                &[("github", "Github"), ("gitee", "Gitee")],
                "general.update_source",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_auto_run,
            toggle_config(c, "gen_auto_run", cfg.general.auto_run, "general.auto_run", None, cx),
        ),
        setting_row(
            c,
            tr.settings_lang_label,
            dropdown_config(
                c,
                "gen_lang",
                &cfg.general.language,
                &[("", "简体中文"), ("en", "English")],
                "general.language",
                cx,
            ),
        ),
        // Group 2: Config and Data Files
        section_title(c, tr.settings_section_config_file),
        setting_row(
            c,
            tr.settings_label_portable_mode,
            toggle_config(
                c,
                "gen_portable",
                cfg.general.portable_mode,
                "general.portable_mode",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_open_config_dir,
            Button::new("gen_open_cfg")
                .label(tr.settings_btn_open)
                .compact()
                .ghost()
                .on_click(|_, _, _| {
                    let dir = crate::config::get_config_dir();
                    #[cfg(windows)]
                    {
                        let _ = std::process::Command::new("explorer")
                            .arg(dir.to_string_lossy().to_string())
                            .spawn();
                    }
                    #[cfg(not(windows))]
                    {
                        let _ = std::process::Command::new("xdg-open")
                            .arg(dir.to_string_lossy().to_string())
                            .spawn();
                    }
                })
                .into_any_element(),
        ),
        // Group 3: When Closing Window
        section_title(c, tr.settings_section_close),
        setting_row(
            c,
            tr.settings_label_minimize_to_tray,
            radio_config(
                c,
                "gen_close_min",
                cfg.general.minimize_to_notify_icon,
                tr.settings_label_minimize_to_tray,
                "general.minimize_to_notify_icon",
                "true",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_exit,
            radio_config(
                c,
                "gen_close_exit",
                !cfg.general.minimize_to_notify_icon,
                tr.settings_label_exit,
                "general.minimize_to_notify_icon",
                "false",
                cx,
            ),
        ),
        // Group 4: Download Settings
        section_title(c, tr.settings_section_download),
        setting_row(
            c,
            tr.settings_label_auto_download_lyric,
            toggle_config(
                c,
                "gen_auto_dl_lyric",
                cfg.general.auto_download_lyric,
                "general.auto_download_lyric",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_lyric_save_pos,
            dropdown_config(
                c,
                "gen_dl_lyric_pos",
                "lyrics_folder",
                &[
                    ("lyrics_folder", tr.settings_option_lyric_save_lyrics),
                    ("song_dir", tr.settings_option_lyric_save_song),
                ],
                "general.auto_download_lyric_path",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_download_trans_format,
            dropdown_config(
                c,
                "gen_dl_trans_fmt",
                "lrc",
                &[("lrc", "LRC"), ("srt", "SRT"), ("txt", "TXT")],
                "general.download_trans_format",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_auto_download_cover,
            toggle_config(
                c,
                "gen_auto_dl_cover",
                cfg.general.auto_download_album_cover,
                "general.auto_download_album_cover",
                None,
                cx,
            ),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Appearance settings
// ---------------------------------------------------------------------------
fn render_appearance_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();

    vec![
        // Group 1: Visual Effects
        section_title(c, tr.settings_section_visual),
        setting_row(
            c,
            tr.settings_show_spectrum,
            toggle_config(
                c,
                "app_spectrum",
                cfg.appearance.show_spectrum,
                "appearance.show_spectrum",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_spectrum_low_center,
            toggle_config(
                c,
                "app_spec_low_mid",
                cfg.appearance.spectrum_low_freq_center,
                "appearance.spectrum_low_freq_center",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_spectrum_height,
            slider_config(
                c,
                "app_spec_h",
                cfg.appearance.spectrum_height,
                12,
                120,
                4,
                "appearance.spectrum_height",
                Some(Box::new(|v| format!("{}px", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_show_cover,
            toggle_config(
                c,
                "app_show_cover",
                cfg.appearance.show_album_cover,
                "appearance.show_album_cover",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_cover_fit,
            dropdown_config(
                c,
                "app_cover_fit",
                &cfg.appearance.album_cover_fit,
                &[
                    ("cover", tr.settings_option_cover_fit),
                    ("fill", tr.settings_option_cover_fill),
                    ("contain", tr.settings_option_cover_contain),
                    ("none", tr.settings_option_cover_none),
                ],
                "appearance.album_cover_fit",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_lyric_bg,
            toggle_config(
                c,
                "app_lyric_bg",
                cfg.appearance.lyric_background,
                "appearance.lyric_background",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_rounded,
            toggle_config(c, "app_rounded", cfg.appearance.rounded_style, "appearance.rounded_style", None, cx),
        ),
        setting_row(
            c,
            tr.settings_dark_mode,
            toggle_config(
                c,
                "app_dark",
                cfg.appearance.dark_mode,
                "appearance.dark_mode",
                Some(Box::new(move |new_val, this, _window, cx| {
                    let mut new_cfg = crate::config::Config::load();
                    new_cfg.appearance.dark_mode = new_val;
                    let theme_name =
                        crate::gui::theme::ThemeName::from_config(&new_cfg.appearance.theme);
                    this.colours = crate::gui::theme::UiColors::build(new_val, &theme_name);
                    cx.notify();
                })),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_window_opacity,
            slider_config(
                c,
                "app_opacity",
                cfg.appearance.window_transparency,
                20,
                100,
                5,
                "appearance.window_transparency",
                Some(Box::new(|v| {
                    if v >= 100 {
                        tr.settings_label_opaque.to_string()
                    } else {
                        format!("{}%", v)
                    }
                })),
                cx,
            ),
        ),
        // Group 2: Background Settings
        section_title(c, tr.settings_section_background),
        setting_row(
            c,
            tr.settings_label_enable_bg,
            toggle_config(
                c,
                "app_bg_enable",
                cfg.appearance.background_transparency > 0,
                "appearance.background_enabled",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_desktop_bg,
            toggle_config(
                c,
                "app_bg_desktop",
                cfg.appearance.use_desktop_background,
                "appearance.use_desktop_background",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_bg_opacity,
            slider_config(
                c,
                "app_bg_opacity",
                cfg.appearance.background_transparency,
                10,
                100,
                5,
                "appearance.background_transparency",
                Some(Box::new(|v| format!("{}%", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_bg_cover,
            toggle_config(
                c,
                "app_bg_cover",
                cfg.appearance.album_cover_as_background,
                "appearance.album_cover_as_background",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_gaussian_blur,
            toggle_config(
                c,
                "app_bg_blur",
                cfg.appearance.background_blur,
                "appearance.background_blur",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_blur_radius,
            slider_config(
                c,
                "app_bg_blur_r",
                cfg.appearance.background_blur_radius,
                1,
                50,
                1,
                "appearance.background_blur_radius",
                Some(Box::new(|v| format!("{}", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_theme_label,
            dropdown_config(
                c,
                "app_theme",
                &cfg.appearance.theme,
                &[
                    ("default", tr.theme_default),
                    ("ocean", tr.theme_ocean),
                    ("forest", tr.theme_forest),
                    ("lavender", tr.theme_lavender),
                    ("sunset", tr.theme_sunset),
                    ("midnight", tr.theme_midnight),
                    ("autumn", tr.theme_autumn),
                    ("spring", tr.theme_spring),
                ],
                "appearance.theme",
                cx,
            ),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Playback settings
// ---------------------------------------------------------------------------
fn render_playback_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();

    vec![
        // Group 1: Playback Options
        section_title(c, tr.settings_section_play_options),
        setting_row(
            c,
            tr.settings_label_error_stop,
            toggle_config(
                c,
                "pb_stop_err",
                cfg.play.stop_when_error,
                "play.stop_when_error",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_auto_play,
            toggle_config(
                c,
                "pb_auto_play",
                cfg.play.auto_play_when_start,
                "play.auto_play_when_start",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_taskbar_progress,
            toggle_config(
                c,
                "pb_taskbar_progress",
                cfg.play.show_taskbar_progress,
                "play.show_taskbar_progress",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_taskbar_icon,
            toggle_config(
                c,
                "pb_taskbar_icon",
                cfg.play.show_taskbar_status_icon,
                "play.show_taskbar_status_icon",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_fade,
            toggle_config(
                c,
                "pb_fade",
                cfg.play.fade_effect,
                "play.fade_effect",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_fade_time,
            slider_config(
                c,
                "pb_fade_time",
                cfg.play.fade_time,
                0,
                2000,
                50,
                "play.fade_time",
                Some(Box::new(|v| format!("{} ms", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_continue_on_switch,
            toggle_config(
                c,
                "pb_continue",
                cfg.play.continue_when_switch_playlist,
                "play.continue_when_switch_playlist",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_system_media,
            toggle_config(
                c,
                "pb_sys_media",
                cfg.play.use_system_media_controls,
                "play.use_system_media_controls",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_remember_pos,
            toggle_config(
                c,
                "pb_remember_pos",
                cfg.play.remember_last_position,
                "play.remember_last_position",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_default_volume,
            slider_config(
                c,
                "pb_vol",
                cfg.play.default_volume,
                0,
                100,
                5,
                "play.default_volume",
                Some(Box::new(|v| format!("{}%", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_volume_step,
            slider_config(
                c,
                "pb_vol_step",
                cfg.play.volume_step,
                1,
                10,
                1,
                "play.volume_step",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_merge_versions,
            toggle_config(
                c,
                "play_merge",
                cfg.play.merge_song_different_versions,
                "play.merge_song_different_versions",
                None,
                cx,
            ),
        ),
        // Group 2: Playback Kernel
        section_title(c, tr.settings_section_play_kernel),
        setting_row(
            c,
            tr.settings_label_kernel,
            h_flex()
                .gap_2()
                .child(radio_config(
                    c,
                    "pb_bass",
                    cfg.play.engine == "bass",
                    "BASS",
                    "play.engine",
                    "bass",
                    cx,
                ))
                .child(radio_config(
                    c,
                    "pb_mci",
                    cfg.play.engine == "mci",
                    "MCI",
                    "play.engine",
                    "mci",
                    cx,
                ))
                .child(radio_config(
                    c,
                    "pb_ffmpeg",
                    cfg.play.engine == "ffmpeg",
                    "FFMPEG",
                    "play.engine",
                    "ffmpeg",
                    cx,
                ))
                .child(radio_config(
                    c,
                    "pb_rodio",
                    cfg.play.engine == "rodio",
                    "Rodio",
                    "play.engine",
                    "rodio",
                    cx,
                ))
                .child(radio_config(
                    c,
                    "pb_symphonia",
                    cfg.play.engine == "symphonia",
                    "Symphonia",
                    "play.engine",
                    "symphonia",
                    cx,
                ))
                .into_any_element(),
        ),
        // Group 3: Playback Device
        section_title(c, tr.settings_section_play_device),
        setting_row(
            c,
            tr.settings_option_default_device,
            dropdown_config(
                c,
                "pb_device",
                &cfg.play.output_device,
                &[("", tr.settings_option_default_device)],
                "play.output_device",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_replaygain,
            dropdown_config(
                c,
                "pb_rg",
                &cfg.play.replaygain,
                &[
                    ("off", tr.settings_option_replaygain_off),
                    ("track", "Track"),
                    ("album", "Album"),
                    ("auto", tr.settings_option_replaygain_auto),
                ],
                "play.replaygain",
                cx,
            ),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Hotkey settings
// ---------------------------------------------------------------------------
fn render_hotkeys_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    let hotkeys = [
        (tr.hotkey_play_pause, "Space"),
        (tr.hotkey_stop, "Ctrl+S"),
        (tr.hotkey_forward, "Ctrl+Shift+Right"),
        (tr.hotkey_rewind, "Ctrl+Shift+Left"),
        (tr.hotkey_prev, "Ctrl+Left"),
        (tr.hotkey_next, "Ctrl+Right"),
        (tr.hotkey_vol_up, "Ctrl+Up"),
        (tr.hotkey_vol_down, "Ctrl+Down"),
        (tr.hotkey_exit, "Alt+F4"),
        (tr.hotkey_show_hide, "Ctrl+H"),
        (tr.hotkey_desktop_lyric, "Ctrl+L"),
        (tr.hotkey_favourite, "Ctrl+D"),
    ];
    let mut children = vec![
        section_title(c, tr.settings_section_hotkeys),
        setting_row(
            c,
            tr.settings_hk_enable,
            toggle_config(
                c,
                "hk_enabled",
                cfg.hotkey.enabled,
                "hotkey.enabled",
                None,
                cx,
            ),
        ),
        section_title(c, tr.settings_label_hotkey_func_shortcut),
    ];
    // Table header
    children.push(
        h_flex()
            .w_full()
            .h(px(24.0))
            .items_center()
            .px_4()
            .child(
                div()
                    .w(px(200.0))
                    .text_size(px(11.0))
                    .text_color(c.text_title)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr.hotkey_function),
            )
            .child(
                div()
                    .flex_grow()
                    .text_size(px(11.0))
                    .text_color(c.text_title)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(tr.hotkey_shortcut),
            )
            .into_any_element(),
    );
    for (func, key) in &hotkeys {
        children.push(
            h_flex()
                .w_full()
                .h(px(24.0))
                .items_center()
                .px_4()
                .border_b(px(1.0))
                .border_color(c.border)
                .child(
                    div()
                        .w(px(200.0))
                        .text_size(px(12.0))
                        .text_color(c.text)
                        .child(*func),
                )
                .child(
                    div()
                        .flex_grow()
                        .text_size(px(12.0))
                        .text_color(c.accent)
                        .child(*key),
                )
                .into_any_element(),
        );
    }
    children
}

// ---------------------------------------------------------------------------
// Media library settings
// ---------------------------------------------------------------------------
fn render_media_lib_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    let lib = crate::media::MediaLib::load();

    let mut children = vec![
        // Group 1: Media Library Options
        section_title(c, tr.settings_section_ml_options),
        setting_row(
            c,
            tr.settings_label_disable_delete,
            toggle_config(
                c,
                "ml_no_delete",
                cfg.media_lib.disable_disk_delete,
                "media_lib.disable_disk_delete",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_merge_single,
            toggle_config(
                c,
                "ml_merge_single",
                cfg.media_lib.merge_single_category,
                "media_lib.merge_single_category",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_min_duration,
            slider_config(
                c,
                "ml_min_dur",
                cfg.media_lib.min_duration_secs as u32,
                0,
                300,
                5,
                "media_lib.min_duration_secs",
                Some(Box::new(|v| {
                    tr.settings_unit_seconds.replace("{}", &v.to_string())
                })),
                cx,
            ),
        ),
        // Group 2: Media Library Update Options
        section_title(c, tr.settings_section_ml_update),
        setting_row(
            c,
            tr.settings_label_remove_missing,
            toggle_config(
                c,
                "ml_remove_missing",
                cfg.media_lib.remove_missing_files,
                "media_lib.remove_missing_files",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_auto_scan,
            toggle_config(
                c,
                "ml_autoscan",
                cfg.media_lib.auto_scan,
                "media_lib.auto_scan",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_force_reload,
            Button::new("ml_reload")
                .label(tr.settings_btn_force_reload)
                .compact()
                .ghost()
                .on_click(|_, _, _| {
                    crate::runtime().spawn_blocking(|| {
                        let cfg = crate::config::Config::load();
                        let mut lib = crate::media::MediaLib::load();
                        for dir in &cfg.media_lib.media_dirs {
                            if let Ok(entries) = crate::media::scan_directory(dir, true, None) {
                                for e in entries {
                                    lib.upsert(e);
                                }
                            }
                        }
                        let _ = lib.save();
                    });
                })
                .into_any_element(),
        ),
        // Group 3: Media Library Directories
        section_title(c, tr.settings_section_ml_dirs),
    ];

    let entity = cx.entity().downgrade();
    for (i, dir) in cfg.media_lib.media_dirs.iter().enumerate() {
        let dir_clone = dir.clone();
        let btn_id: SharedString = format!("ml_del_{}", i).into();
        let cfg_dirs = cfg.media_lib.media_dirs.clone();
        let ent = entity.clone();
        children.push(
            h_flex()
                .w_full()
                .h(px(28.0))
                .items_center()
                .gap_2()
                .px_4()
                .child(
                    div()
                        .w(px(20.0))
                        .text_size(px(11.0))
                        .text_color(c.text_dim)
                        .child(format!("{}", i + 1)),
                )
                .child(
                    div()
                        .flex_grow()
                        .text_size(px(12.0))
                        .text_color(c.text)
                        .child(dir_clone.clone()),
                )
                .child(
                    Button::new(btn_id)
                        .label(tr.settings_btn_delete)
                        .compact()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            let new_dirs: Vec<String> = cfg_dirs
                                .iter()
                                .filter(|d| **d != dir_clone)
                                .cloned()
                                .collect();
                            let _ = crate::config::Config::set(
                                "media_lib.media_dirs",
                                &new_dirs.join(","),
                            );
                            let _ = ent.update(cx, |_, cx| cx.notify());
                        })
                        .into_any_element(),
                )
                .into_any_element(),
        );
    }

    children.push(setting_row(
        c,
        tr.settings_label_add_dir,
        Button::new("ml_add")
            .label(tr.settings_btn_add)
            .compact()
            .ghost()
            .on_click({
                let ent = entity.clone();
                move |_, _, cx| {
                    super::run_blocking_dialog_app(
                        cx,
                        &ent,
                        || rfd::FileDialog::new().pick_folder(),
                        |dir, _this, cx| {
                            if let Some(dir) = dir {
                                let mut cfg = crate::config::Config::load();
                                let path = dir.to_string_lossy().to_string();
                                if !cfg.media_lib.media_dirs.iter().any(|d| d == &path) {
                                    cfg.media_lib.media_dirs.push(path);
                                    let _ = crate::config::Config::set(
                                        "media_lib.media_dirs",
                                        &cfg.media_lib.media_dirs.join(","),
                                    );
                                }
                                cx.notify();
                            }
                        },
                    );
                }
            })
            .into_any_element(),
    ));

    children.push(section_title(c, tr.settings_section_data_mgmt));
    children.push(setting_row(
        c,
        tr.settings_label_media_count,
        txt(
            &tr.settings_label_media_count
                .replace("{}", &lib.entries.len().to_string()),
            12.0,
            c.accent,
        )
        .into_any_element(),
    ));

    children
}

// ---------------------------------------------------------------------------
// Lyrics settings
// ---------------------------------------------------------------------------
fn render_lyrics_settings(
    tr: &Tr,
    c: &UiColors,
    cx: &mut Context<super::MusicPlayer>,
) -> Vec<AnyElement> {
    let cfg = crate::config::Config::load();
    vec![
        // Group 1: Lyric Options
        section_title(c, tr.settings_section_lyric_options),
        setting_row(
            c,
            tr.settings_label_inner_lyric,
            toggle_config(
                c,
                "ly_inner",
                cfg.lyric.use_inner_lyric_first,
                "lyric.use_inner_lyric_first",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_fuzzy_match,
            toggle_config(
                c,
                "ly_fuzzy",
                cfg.lyric.fuzzy_match,
                "lyric.fuzzy_match",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.menu_show_translation,
            toggle_config(
                c,
                "ly_translate",
                cfg.lyric.show_translate,
                "lyric.show_translate",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_show_info_when_none,
            toggle_config(
                c,
                "ly_show_info",
                cfg.lyric.show_song_info_when_no_lyric,
                "lyric.show_song_info_when_no_lyric",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_lyric_folder,
            txt(".\\lyrics\\", 12.0, c.accent).into_any_element(),
        ),
        setting_row(
            c,
            tr.settings_label_lyric_adjust,
            dropdown_config(
                c,
                "ly_adjust",
                &cfg.lyric.after_adjust,
                &[
                    ("ask", tr.settings_option_adjust_ask),
                    ("auto_save", tr.settings_option_adjust_save),
                    ("discard", tr.settings_option_adjust_discard),
                ],
                "lyric.after_adjust",
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_karaoke,
            toggle_config(c, "ly_karaoke", cfg.lyric.karaoke_display, "lyric.karaoke_display", None, cx),
        ),
        setting_row(
            c,
            tr.settings_label_hide_empty,
            toggle_config(c, "ly_no_empty", cfg.lyric.hide_empty_lines, "lyric.hide_empty_lines", None, cx),
        ),
        // Group 2: Window Lyrics
        section_title(c, tr.settings_section_window_lyric),
        setting_row(
            c,
            tr.settings_label_line_spacing,
            slider_config(
                c,
                "ly_line_spacing",
                cfg.lyric.line_spacing,
                0,
                20,
                1,
                "lyric.line_spacing",
                Some(Box::new(|v| format!("{}px", v))),
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_alignment,
            dropdown_config(
                c,
                "ly_align",
                &cfg.lyric.alignment,
                &[
                    ("auto", tr.settings_option_align_auto),
                    ("left", tr.settings_option_align_left),
                    ("center", tr.settings_option_align_center),
                    ("right", tr.settings_option_align_right),
                ],
                "lyric.alignment",
                cx,
            ),
        ),
        // Group 3: Desktop Lyrics
        section_title(c, tr.settings_section_desktop_lyric),
        setting_row(
            c,
            tr.settings_label_show_desktop_lyric,
            toggle_config(c, "ly_desktop_show", cfg.desktop_lyric.show, "desktop_lyric.show", None, cx),
        ),
        setting_row(
            c,
            tr.settings_label_hide_when_none,
            toggle_config(
                c,
                "ly_desktop_hide_no",
                cfg.desktop_lyric.hide_when_no_lyric,
                "desktop_lyric.hide_when_no_lyric",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_hide_when_paused,
            toggle_config(
                c,
                "ly_desktop_hide_pause",
                cfg.desktop_lyric.hide_when_paused,
                "desktop_lyric.hide_when_paused",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_lock_desktop,
            toggle_config(
                c,
                "ly_desktop_lock",
                cfg.desktop_lyric.locked,
                "desktop_lyric.locked",
                None,
                cx,
            ),
        ),
        setting_row(
            c,
            tr.settings_label_double_line,
            toggle_config(
                c,
                "ly_desktop_double",
                cfg.desktop_lyric.double_line,
                "desktop_lyric.double_line",
                None,
                cx,
            ),
        ),
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
                .w(px(64.0))
                .h(px(64.0))
                .rounded(px(16.0))
                .bg(c.accent)
                .flex()
                .items_center()
                .justify_center()
                .child(txt("♪", 28.0, gpui::white())),
        )
        .child(txt("HackMagic Music Player", 18.0, c.text_title))
        .child(txt(tr.about_version, 12.0, c.text_dim))
        .child(txt(tr.about_tagline, 11.0, c.text_dim))
        .child(
            div()
                .mt_4()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child(tr.about_original),
        )
        .child(
            div()
                .text_size(px(11.0))
                .text_color(c.text_dim)
                .child(tr.about_platforms),
        )
        .child(
            div()
                .mt_4()
                .text_size(px(10.0))
                .text_color(c.text_dim)
                .child(tr.about_copyright),
        )
}

/// Render the search panel with an editable search input field.
pub fn render_search_panel(
    tr: &Tr,
    c: &UiColors,
    query: &str,
    search_input: &Entity<InputState>,
    player: std::sync::Arc<crate::core::player::Player>,
    _window: &mut Window,
    _cx: &mut Context<super::MusicPlayer>,
) -> impl IntoElement {
    let results = if query.trim().is_empty() {
        vec![]
    } else {
        let lib = crate::media::MediaLib::load();
        lib.search(query)
            .iter()
            .map(|e| SearchResult {
                title: e.title.clone(),
                artist: e.artist.clone(),
                _album: e.album.clone(),
                path: e.file_path.clone(),
            })
            .collect()
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
                .px_4()
                .py_2()
                .bg(c.control_bar_bg)
                .gap_2()
                .child(txt(tr.menu_find, 14.0, c.text_title))
                .child(
                    h_flex()
                        .flex_grow()
                        .h(px(24.0))
                        .bg(c.panel)
                        .rounded(px(4.0))
                        .px_2()
                        .child(Input::new(search_input).w_full()),
                )
                .child(txt(&format!("{}", results.len()), 11.0, c.text_dim)),
        )
        .child(
            v_flex()
                .flex_1()
                .w_full()
                .children(results.iter().enumerate().map(|(i, r)| {
                    let is_even = i % 2 == 0;
                    let bg = if is_even {
                        c.playlist_item
                    } else {
                        c.playlist_item_hover
                    };
                    let title = if r.title.is_empty() {
                        r.path.clone()
                    } else {
                        r.title.clone()
                    };
                    let path = r.path.clone();
                    let p_click = player.clone();
                    let p_menu = player.clone();
                    h_flex()
                        .w_full()
                        .h(px(28.0))
                        .px_4()
                        .gap_3()
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
                                let p2 = p_click.clone();
                                let path3 = path2.clone();
                                crate::runtime().spawn_blocking(move || {
                                    let _ = p2.play_file(&path3);
                                });
                            }
                        })
                        .context_menu({
                            let s_play = tr.ctrl_toggle_play;
                            let s_add = tr.menu_add_to_playlist;
                            let s_open = tr.menu_open_file_location;
                            let s_copy = tr.menu_copy_path;
                            move |menu, _w, _cx| {
                                let p_play = path.clone();
                                let p_open = path.clone();
                                let p_add = path.clone();
                                let p_copy = path.clone();
                                let p_play2 = p_menu.clone();
                                let p_menu_add = p_menu.clone();
                                menu.item(PopupMenuItem::new(s_play).on_click(move |_, _, _| {
                                    let p = p_play2.clone();
                                    let path = p_play.clone();
                                    crate::runtime().spawn_blocking(move || {
                                        let _ = p.play_file(&path);
                                    });
                                }))
                                .item(PopupMenuItem::new(s_add).on_click(move |_, _, _| {
                                    let p = p_menu_add.clone();
                                    let path = p_add.clone();
                                    let mut pl = p.playlist_mut();
                                    pl.add_track(crate::core::playlist::Track::new(&path));
                                }))
                                .separator()
                                .item(PopupMenuItem::new(s_open).on_click(move |_, _, _| {
                                    let path = std::path::Path::new(&p_open);
                                    if let Some(_parent) = path.parent() {
                                        #[cfg(windows)]
                                        {
                                            let _ = std::process::Command::new("explorer")
                                                .arg("/select,")
                                                .arg(&p_open)
                                                .spawn();
                                        }
                                        #[cfg(not(windows))]
                                        {
                                            let _ = std::process::Command::new("xdg-open")
                                                .arg(&p_open)
                                                .spawn();
                                        }
                                    }
                                    tracing::info!("[Search] 打开文件位置: {}", p_open);
                                }))
                                .item(
                                    PopupMenuItem::new(s_copy).on_click(move |_, _, _| {
                                        super::copy_text_to_clipboard(&p_copy);
                                    }),
                                )
                            }
                        })
                })),
        )
}

struct SearchResult {
    title: String,
    artist: String,
    _album: String,
    path: String,
}
