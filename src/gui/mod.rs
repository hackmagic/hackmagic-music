pub mod theme;
pub mod styles;
pub mod i18n;
pub mod layout;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use gpui::*;
use gpui_component::{h_flex, v_flex, IconName, Root};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::PopupMenuItem;
use gpui_component::slider::{Slider, SliderState, SliderValue};
use i18n::{Locale, Tr};
use theme::UiColors;
use crate::config::Config;
use crate::core::engine_trait::EngineType;
use crate::core::player::Player;
use crate::core::playlist::Track;

static ACTIVE_PANEL: AtomicU8 = AtomicU8::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Playlist,
    MediaLib,
    Search,
    FileBrowser,
    Lyrics,
    Settings,
}

impl Panel {
    fn from_u8(v: u8) -> Self {
        match v {
            0 => Panel::Playlist,
            1 => Panel::FileBrowser,
            2 => Panel::MediaLib,
            3 => Panel::Search,
            4 => Panel::Lyrics,
            5 => Panel::Settings,
            _ => Panel::Playlist,
        }
    }
    fn to_u8(self) -> u8 {
        match self {
            Panel::Playlist => 0,
            Panel::FileBrowser => 1,
            Panel::MediaLib => 2,
            Panel::Search => 3,
            Panel::Lyrics => 4,
            Panel::Settings => 5,
        }
    }
}

pub fn run(cx: &mut App) {
    let _ = cx.open_window(WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: Some("HackMagic Music Player".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::default(),
            size: gpui::Size { width: px(1200.0), height: px(800.0) },
        })),
        window_min_size: Some(gpui::Size { width: px(760.0), height: px(480.0) }),
        ..Default::default()
    }, |window, cx| {
        cx.new(|cx| {
            let content = cx.new(|cx| MusicPlayer::new(cx));
            Root::new(content, window, cx)
        })
    });
}

pub struct MusicPlayer {
    player: Arc<Player>,
    colours: UiColors,
    tr: &'static Tr,
    title: String,
    artist: String,
    position: f64,
    duration: f64,
    volume: u32,
    is_playing: bool,
    maximized: bool,
    volume_slider: Entity<SliderState>,
}

impl MusicPlayer {
    fn new(cx: &mut Context<Self>) -> Self {
        let cfg = Config::load();
        let engine = EngineType::from_str(&cfg.play.engine);
        let player = Arc::new(Player::new(engine));

        if let Err(e) = player.init() {
            tracing::warn!("BASS init failed ({}), trying FFmpeg", e);
            let fallback = Arc::new(Player::new(EngineType::Ffmpeg));
            if let Err(e2) = fallback.init() {
                tracing::error!("Fatal: no audio engine: {e2}");
                std::process::exit(1);
            }
            fallback.set_volume(cfg.play.default_volume).ok();
        } else {
            player.set_volume(cfg.play.default_volume).ok();
        }

        let dark = cfg.appearance.dark_mode;
        let theme_name = theme::ThemeName::from_config(&cfg.appearance.theme);
        let colours = UiColors::build(dark, &theme_name);
        let lang = i18n::Lang::from_config(&cfg.general.language);
        let tr = Locale::new(lang).tr;

        let vol = cfg.play.default_volume;
        let volume_slider = cx.new(|_cx| {
            SliderState::new().min(0.0).max(100.0).step(1.0).default_value(vol as f32)
        });

        let _ = cx.subscribe(&volume_slider, |this, entity, _window, cx| {
            let val = entity.read(cx).value();
            if let SliderValue::Single(v) = val {
                this.volume = v as u32;
                let _ = this.player.set_volume(this.volume);
            }
        });

        if cfg.media_lib.auto_scan && !cfg.media_lib.media_dirs.is_empty() {
            let dirs = cfg.media_lib.media_dirs.clone();
            std::thread::spawn(move || {
                for dir in &dirs {
                    match crate::media::scan_directory(dir, true, None) {
                        Ok(entries) => {
                            let mut lib = crate::media::MediaLib::load();
                            for e in entries { lib.upsert(e); }
                            let _ = lib.save();
                        }
                        Err(e) => tracing::warn!("Scan failed {}: {}", dir, e),
                    }
                }
            });
        }

        Self {
            player,
            colours,
            tr,
            title: String::new(),
            artist: String::new(),
            position: 0.0,
            duration: 0.0,
            volume: cfg.play.default_volume,
            is_playing: false,
            maximized: false,
            volume_slider,
        }
    }

    fn poll_player_state(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        let p = &self.player;
        self.position = p.position().as_secs_f64();
        self.duration = p.duration().as_secs_f64();
        self.volume = p.volume();
        self.is_playing = p.state() == crate::core::engine_trait::EngineState::Playing;

        let pl = p.playlist();
        if let Some(track) = pl.current_track() {
            let display = if !track.title.is_empty() {
                track.title.clone()
            } else {
                track.file_name.clone()
            };
            let artist = if !track.artist.is_empty() {
                track.artist.clone()
            } else {
                "未知艺术家".into()
            };
            if self.title != display { self.title = display; }
            if self.artist != artist { self.artist = artist; }
        }
    }
}

impl Render for MusicPlayer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll_player_state(window, cx);
        cx.notify();
        let tr = self.tr;
        let c = &self.colours;

        let pos_str = format!(
            "{:02}:{:02} / {:02}:{:02}",
            (self.position as u32) / 60,
            (self.position as u32) % 60,
            (self.duration as u32) / 60,
            (self.duration as u32) % 60
        );
        let seek_pct = if self.duration > 0.0 {
            (self.position / self.duration * 100.0) as f32
        } else {
            0.0
        };
        let play_label = if self.is_playing { "⏸" } else { "▶" };

        let player_play = self.player.clone();
        let player_prev = self.player.clone();
        let player_next = self.player.clone();
        let player_stop = self.player.clone();
        let player_seek = self.player.clone();
        let player_repeat = self.player.clone();
        let repeat_mode = self.player.repeat_mode();
        let repeat_label = match repeat_mode {
            crate::core::playlist::RepeatMode::LoopPlaylist => "🔁",
            crate::core::playlist::RepeatMode::LoopTrack => "🔂",
            crate::core::playlist::RepeatMode::PlayShuffle => "🔀",
            _ => "➡️",
        };
        let speed = self.player.speed();
        let speed_str = if (speed - 1.0).abs() > 0.01 { format!(" ×{:.1}", speed) } else { String::new() };
        let panel = Panel::from_u8(ACTIVE_PANEL.load(Ordering::Relaxed));
        let playlist_tracks = self.player.playlist().tracks().to_vec();
        let current_idx = self.player.playlist().current_index();

        let sidebar = {
            let active = panel;
            let make_btn = |id: &'static str, icon: IconName, p: Panel, c: &UiColors| {
                let bg = if active == p { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
                Button::new(id)
                    .icon(icon)
                    .ghost()
                    .compact()
                    .w(px(40.0)).h(px(40.0))
                    .rounded(px(8.0))
                    .bg(bg)
                    .on_click(move |_, _, _cx| {
                        ACTIVE_PANEL.store(p.to_u8(), Ordering::Relaxed);
                    })
            };
            v_flex()
                .items_center()
                .w(px(52.0))
                .h_full()
                .py_2().gap_1()
                .bg(c.panel)
                .child(make_btn("queue", IconName::LayoutDashboard, Panel::Playlist, c))
                .child(make_btn("folders", IconName::Folder, Panel::FileBrowser, c))
                .child(make_btn("playlists", IconName::SquareTerminal, Panel::MediaLib, c))
                .child(make_btn("recent", IconName::Calendar, Panel::Search, c))
                .child(make_btn("media", IconName::File, Panel::Settings, c))
                .child(div().flex_grow())
                .child(make_btn("search", IconName::Search, Panel::Search, c))
                .child(make_btn("lyrics", IconName::BookOpen, Panel::Lyrics, c))
        };

        let content_area = div().flex_grow().h_full().child(match panel {
            Panel::Playlist => self.render_playlist(&playlist_tracks, current_idx).into_any_element(),
            _ => layout::content_area(c, tr).into_any_element(),
        });

        v_flex()
            .size_full()
            .bg(c.bg)
            .child(layout::title_bar(c, tr))
            .child(self.render_menu_bar(c, tr, window, cx))
            .child(
                h_flex().w_full().flex_grow()
                    .child(sidebar)
                    .child(content_area)
            )
            .child(
                v_flex()
                    .w_full()
                    .bg(c.control_bar_bg)
                    .child({
                        let dur = self.duration;
                        h_flex()
                            .id("progress-bar")
                            .w_full()
                            .h(px(theme::PROGRESS_BAR_HEIGHT))
                            .cursor(gpui::CursorStyle::PointingHand)
                            .child(
                                h_flex().w_full().h_full()
                                    .bg(c.progress_track)
                                    .child(
                                        div()
                                            .h_full()
                                            .w(DefiniteLength::Fraction(seek_pct / 100.0))
                                            .bg(c.accent),
                                    ),
                            )
                            .on_mouse_down(gpui::MouseButton::Left, move |e, window, _cx| {
                                let win_bounds = window.bounds();
                                let total_w: f32 = win_bounds.size.width.into();
                                let padding: f32 = 32.0;
                                let bar_w = total_w - padding;
                                if bar_w > 0.0 {
                                    let mouse_x: f32 = e.position.x.into();
                                    let ratio = ((mouse_x - padding / 2.0) / bar_w).clamp(0.0, 1.0);
                                    let seek_to = dur * ratio as f64;
                                    let _ = player_seek.seek(std::time::Duration::from_secs_f64(seek_to));
                                }
                            })
                    })
                    .child(
                        h_flex()
                            .items_center()
                            .w_full()
                            .px_4().gap_4()
                            .h(px(theme::CONTROL_BAR_HEIGHT))
                            .child(
                                h_flex().items_center().gap_3()
                                    .child(Button::new("prev").icon(IconName::ChevronLeft).ghost().compact().on_click(move |_, _, _| { let _ = player_prev.prev(); }))
                                    .child(Button::new("play").label(play_label).primary().compact().on_click(move |_, _, _| { let _ = player_play.toggle_pause(); }))
                                    .child(Button::new("next").icon(IconName::ChevronRight).ghost().compact().on_click(move |_, _, _| { let _ = player_next.next(); }))
                                    .child(Button::new("repeat").label(repeat_label).ghost().compact().on_click(move |_, _, _| {
                                        use crate::core::playlist::RepeatMode;
                                        let modes = [RepeatMode::PlayOrder, RepeatMode::LoopPlaylist, RepeatMode::LoopTrack, RepeatMode::PlayShuffle, RepeatMode::PlayRandom];
                                        let current = player_repeat.repeat_mode();
                                        let idx = modes.iter().position(|m| *m == current).unwrap_or(0);
                                        let next = modes[(idx + 1) % modes.len()];
                                        player_repeat.set_repeat_mode(next);
                                    })),
                            )
                            .child(
                                v_flex().flex_grow().gap_1()
                                    .child(crate::gui::layout::txt(&self.title, 13.0, c.text_title))
                                    .child(crate::gui::layout::txt(&self.artist, 11.0, c.text_dim)),
                            )
                            .child(
                                h_flex().items_center().gap_2()
                                    .child(layout::txt(&pos_str, 11.0, c.text_dim))
                                    .child(if !speed_str.is_empty() { layout::txt(&speed_str, 10.0, c.accent) } else { layout::txt("", 10.0, c.text_dim) })
                                    .child(Button::new("stop").label("⏹").ghost().compact().on_click(move |_, _, _| { let _ = player_stop.stop(); }))
                                    .child(Slider::new(&self.volume_slider).horizontal().w(px(96.0))),
                            ),
                    ),
            )
            .child(layout::status_bar(c, tr))
    }
}

impl MusicPlayer {
    fn render_menu_bar(
        &self,
        c: &UiColors,
        tr: &Tr,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        h_flex()
            .items_center()
            .w_full()
            .h(px(theme::MENUBAR_HEIGHT))
            .px_2().gap_1()
            .bg(c.control_bar_bg)
            .child({
                let player = self.player.clone();
                layout::menu_dropdown(tr.menu_file, IconName::Folder, move |menu, _w, _cx| {
                    let p = player.clone();
                    menu.item(PopupMenuItem::new("打开文件").on_click(move |_, _, _| {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                            .pick_file()
                        {
                            let _ = p.play_file(path.to_str().unwrap_or_default());
                        }
                    }))
                    .item(PopupMenuItem::new("打开文件夹").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                let dir = folder.to_str().unwrap_or_default();
                                match crate::media::scan_directory(dir, true, None) {
                                    Ok(entries) => {
                                        let mut lib = crate::media::MediaLib::load();
                                        let first_path = entries.first().map(|e| e.file_path.clone());
                                        for e in &entries { lib.upsert(e.clone()); }
                                        let _ = lib.save();
                                        if let Some(path) = first_path {
                                            let _ = p.play_file(&path);
                                        }
                                    }
                                    Err(e) => tracing::warn!("Scan folder failed: {}", e),
                                }
                            }
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new("退出").on_click(|_, _, _| std::process::exit(0)))
                })
            })
            .child({
                let player = self.player.clone();
                layout::menu_dropdown(tr.menu_playback, IconName::ChevronRight, move |menu, _w, _cx| {
                    let p1 = player.clone();
                    let p2 = player.clone();
                    let p3 = player.clone();
                    let p4 = player.clone();
                    let p5 = player.clone();
                    let p6 = player.clone();
                    let p7 = player.clone();
                    let p8 = player.clone();
                    let p9 = player.clone();
                    let p10 = player.clone();
                    let p11 = player.clone();
                    let p12 = player.clone();
                    let p13 = player.clone();
                    let _p14 = player.clone();
                    menu.item(PopupMenuItem::new("播放/暂停").on_click(move |_, _, _| { let _ = p1.toggle_pause(); }))
                        .item(PopupMenuItem::new("停止").on_click(move |_, _, _| { let _ = p2.stop(); }))
                        .item(PopupMenuItem::new("上一曲").on_click(move |_, _, _| { let _ = p3.prev(); }))
                        .item(PopupMenuItem::new("下一曲").on_click(move |_, _, _| { let _ = p4.next(); }))
                        .separator()
                        .item(PopupMenuItem::new("加速").on_click(move |_, _, _| { let _ = p5.speed_up(); }))
                        .item(PopupMenuItem::new("减速").on_click(move |_, _, _| { let _ = p6.speed_down(); }))
                        .item(PopupMenuItem::new("原始速度").on_click(move |_, _, _| { let _ = p7.set_speed(1.0); }))
                        .separator()
                        .item(PopupMenuItem::new("升高音调").on_click(move |_, _, _| { let _ = p8.pitch_up(); }))
                        .item(PopupMenuItem::new("降低音调").on_click(move |_, _, _| { let _ = p9.pitch_down(); }))
                        .item(PopupMenuItem::new("原始音调").on_click(move |_, _, _| { let _ = p10.set_pitch(0); }))
                        .separator()
                        .item(PopupMenuItem::new("设置 A 点").on_click(move |_, _, _| { let _ = p11.ab_set_a(); }))
                        .item(PopupMenuItem::new("设置 B 点").on_click(move |_, _, _| { let _ = p12.ab_set_b(); }))
                        .item(PopupMenuItem::new("清除 AB 循环").on_click(move |_, _, _| { p13.ab_reset(); }))
                })
            })
            .child(layout::menu_dropdown(tr.menu_playlist, IconName::SquareTerminal, |menu, _, _| {
                menu.item(PopupMenuItem::new("添加").on_click(|_, _, _| {}))
                    .item(PopupMenuItem::new("删除").on_click(|_, _, _| {}))
                    .separator()
                    .item(PopupMenuItem::new("另存为新播放列表").on_click(|_, _, _| {}))
            }))
            .child(layout::menu_dropdown(tr.menu_lyric, IconName::BookOpen, |menu, _, _| {
                menu.item(PopupMenuItem::new("重新加载歌词").on_click(|_, _, _| {}))
                    .item(PopupMenuItem::new("编辑歌词").on_click(|_, _, _| {}))
                    .separator()
                    .item(PopupMenuItem::new("显示桌面歌词").on_click(|_, _, _| {}))
            }))
            .child(layout::menu_dropdown(tr.menu_view, IconName::LayoutDashboard, |menu, _, _| {
                menu.item(PopupMenuItem::new("显示播放列表").on_click(|_, _, _| {}))
                    .separator()
                    .item(PopupMenuItem::new("深色模式").on_click(|_, _, _| {}))
            }))
            .child(layout::menu_dropdown(tr.menu_tools, IconName::Settings, |menu, _, _| {
                menu.item(PopupMenuItem::new("媒体库").on_click(|_, _, _| {}))
                    .item(PopupMenuItem::new("均衡器").on_click(|_, _, _| {}))
                    .separator()
                    .item(PopupMenuItem::new("选项设置").on_click(|_, _, _| {}))
            }))
            .child(layout::menu_dropdown(tr.menu_help, IconName::Info, |menu, _, _| {
                menu.item(PopupMenuItem::new("帮助 (F1)").on_click(|_, _, _| {}))
                    .separator()
                    .item(PopupMenuItem::new("关于").on_click(|_, _, _| {}))
            }))
    }

    fn render_playlist(
        &self,
        tracks: &[Track],
        current_idx: Option<usize>,
    ) -> impl IntoElement {
        let c = &self.colours;
        let player = self.player.clone();

        v_flex()
            .flex_grow()
            .h_full()
            .bg(c.bg)
            .child(
                h_flex()
                    .items_center().justify_between()
                    .w_full()
                    .px_4().py_3()
                    .child(layout::txt(&format!("播放列表 ({} 首)", tracks.len()), 14.0, c.text_title))
            )
            .child(
                div().flex_grow()
                    .child(
                        v_flex().w_full()
                            .children(tracks.iter().enumerate().map(|(i, track)| {
                                let is_current = current_idx == Some(i);
                                let bg = if is_current { c.playlist_playing } else if i % 2 == 0 { c.playlist_item } else { c.playlist_item_hover };
                                let text_color = if is_current { c.accent } else { c.text };
                                let display_name = if !track.title.is_empty() {
                                    track.title.clone()
                                } else {
                                    track.file_name.clone()
                                };
                                let duration = track.duration;
                                let dur_str = format!("{:02}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60);
                                let file_path = track.file_path.clone();
                                let p = player.clone();

                                h_flex()
                                    .id(("track", i))
                                    .items_center()
                                    .w_full()
                                    .h(px(36.0))
                                    .px_4().gap_3()
                                    .bg(bg)
                                    .hover(|s| s.bg(c.playlist_item_selected))
                                    .cursor(gpui::CursorStyle::PointingHand)
                                    .child(h_flex().flex_grow().child(layout::txt(&display_name, 12.0, text_color)))
                                    .child(layout::txt(&dur_str, 11.0, c.text_dim))
                                    .on_click(move |_, _, _| {
                                        let _ = p.play_file(&file_path);
                                    })
                            }))
                    )
            )
    }
}
