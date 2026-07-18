pub mod theme;
pub mod styles;
pub mod i18n;
pub mod layout;
pub mod responsive;
pub mod desktop_lyrics;
pub mod lyric_editor;
pub mod lyric_download;
pub mod dialogs;

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use gpui::*;
use gpui_component::{h_flex, v_flex, IconName, Root};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::slider::{Slider, SliderState, SliderValue};
use gpui_component::tooltip::Tooltip;
use gpui_component::input::{Input, InputState};
use i18n::{Locale, Tr};
use theme::UiColors;
use crate::config::Config;
use crate::core::engine_trait::EngineType;
use crate::core::player::Player;
use crate::core::playlist::Track;
use responsive::{LayoutMode, ResponsiveState};

static ACTIVE_PANEL: AtomicU8 = AtomicU8::new(0);
static MINI_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STATUSBAR_VISIBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static MEDIA_LIB_SCANNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RPC_SERVER_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SLEEP_TIMER_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    Playlist,
    MediaLib,
    Search,
    FileBrowser,
    Lyrics,
    LyricEditor,
    LyricDownload,
    Equalizer,
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
            5 => Panel::LyricEditor,
            6 => Panel::LyricDownload,
            7 => Panel::Equalizer,
            8 => Panel::Settings,
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
            Panel::LyricEditor => 5,
            Panel::LyricDownload => 6,
            Panel::Settings => 8,
            Panel::Equalizer => 7,
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
            let url_state = cx.new(|c| InputState::new(window, c));
            let content = cx.new(|cx| MusicPlayer::new(cx, url_state));
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
    album: String,
    position: f64,
    duration: f64,
    volume: u32,
    is_playing: bool,
    is_muted: bool,
    is_favourite: bool,
    maximized: bool,
    volume_slider: Entity<SliderState>,
    speed_slider: Entity<SliderState>,
    responsive: ResponsiveState,
    eq_open: bool,
    lyric_visible: bool,
    media_lib_open: bool,
    lyric_state: desktop_lyrics::LyricsState,
    last_lpc_path: String,
    editor_state: lyric_editor::LyricEditorState,
    show_editor: bool,
    download_state: lyric_download::LyricDownloadState,
    pending_download_rx: Option<std::sync::mpsc::Receiver<lyric_download::DownloadEvent>>,
    pending_download_tx: Option<std::sync::mpsc::Sender<lyric_download::DownloadEvent>>,
    current_track_path_for_download: String,
    /// Playlist search/filter state
    playlist_filter_text: String,
    playlist_filter_mode: PlaylistFilterMode,
    playlist_sort_field: PlaylistSortField,
    playlist_sort_asc: bool,
    /// Playlist drag-drop reorder state
    playlist_drag_from: Option<usize>,
    playlist_drag_to: Option<usize>,
    playlist_drag_active: bool,
    /// Media library panel state
    media_lib_category: MediaLibCategory,
    media_lib_selected: Option<String>,
    media_lib_search: String,
    /// Equalizer state
    eq_enabled: bool,
    eq_sliders: Vec<Entity<SliderState>>,
    eq_preset_name: String,
    /// Settings dialog state
    settings_tab: dialogs::SettingsTab,
    /// Open URL dialog state
    url_dialog_open: bool,
    url_state: Entity<InputState>,
}

/// Filter mode for the playlist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistFilterMode {
    All,
    Artist,
    Album,
    Genre,
    Favorites,
}

/// Playlist column sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistSortField {
    Title,
    Artist,
    Album,
    Duration,
}

/// Media library browsing category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaLibCategory {
    AllTracks,
    Artists,
    Albums,
    Genres,
    Years,
    FileTypes,
    Bitrates,
    Recent,
    Rating,
}

impl MusicPlayer {
    fn new(cx: &mut Context<Self>, url_state: Entity<InputState>) -> Self {
        let cfg = Config::load();
        let engine = EngineType::from_str(&cfg.play.engine);
        let player = Arc::new(Player::new(engine));

        // BASS init may fail (missing DLLs); fall back to FFmpeg and use that as
        // the active player so playback actually works.
        let player = if let Err(e) = player.init() {
            tracing::warn!("BASS init failed ({}), trying FFmpeg", e);
            let fallback = Arc::new(Player::new(EngineType::Ffmpeg));
            if let Err(e2) = fallback.init() {
                tracing::error!("Fatal: no audio engine: {e2}");
                std::process::exit(1);
            }
            fallback.set_volume(cfg.play.default_volume).ok();
            fallback
        } else {
            player.set_volume(cfg.play.default_volume).ok();
            player
        };

        let dark = cfg.appearance.dark_mode;
        let theme_name = theme::ThemeName::from_config(&cfg.appearance.theme);
        let colours = UiColors::build(dark, &theme_name);
        let lang = i18n::Lang::from_config(&cfg.general.language);
        let tr = Locale::new(lang).tr;

        let vol = cfg.play.default_volume;
        let volume_slider = cx.new(|_cx| {
            SliderState::new().min(0.0).max(100.0).step(1.0).default_value(vol as f32)
        });

        let speed_val = cfg.play.speed;
        let speed_slider = cx.new(|_cx| {
            SliderState::new().min(0.5).max(2.0).step(0.05).default_value(speed_val as f32)
        });
        let _sp_slider = speed_slider.clone();
        let sp_player = player.clone();
        let _ = cx.subscribe(&speed_slider, move |_, entity, _window, cx| {
            let val = entity.read(cx).value();
            if let SliderValue::Single(v) = val {
                let _ = sp_player.set_speed(v);
            }
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
                let total = dirs.len();
                for (i, dir) in dirs.iter().enumerate() {
                    tracing::info!("[MediaLib] 扫描进度: {}/{} 目录", i + 1, total);
                    match crate::media::scan_directory(dir, true, None) {
                        Ok(entries) => {
                            let mut lib = crate::media::MediaLib::load();
                            for e in entries { lib.upsert(e); }
                            let _ = lib.save();
                        }
                        Err(e) => tracing::warn!("Scan failed {}: {}", dir, e),
                    }
                }
                tracing::info!("[MediaLib] 初始化扫描完成");
            });
        }

        Self {
            player,
            colours,
            tr,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            position: 0.0,
            duration: 0.0,
            volume: cfg.play.default_volume,
            is_playing: false,
            is_muted: false,
            is_favourite: false,
            maximized: false,
            volume_slider,
            speed_slider,
            responsive: ResponsiveState::new(1200.0, 800.0),
            eq_open: false,
            lyric_visible: true,
            media_lib_open: false,
            lyric_state: desktop_lyrics::LyricsState::new(),
            last_lpc_path: String::new(),
            editor_state: lyric_editor::LyricEditorState::new(),
            show_editor: false,
            download_state: lyric_download::LyricDownloadState::new(),
            pending_download_rx: None,
            pending_download_tx: None,
            current_track_path_for_download: String::new(),
            playlist_filter_text: String::new(),
            playlist_filter_mode: PlaylistFilterMode::All,
            playlist_sort_field: PlaylistSortField::Title,
            playlist_sort_asc: true,
            playlist_drag_from: None,
            playlist_drag_to: None,
            playlist_drag_active: false,
            media_lib_category: MediaLibCategory::AllTracks,
            media_lib_selected: None,
            media_lib_search: String::new(),
            eq_enabled: false,
            eq_sliders: (0..10).map(|_| cx.new(|_| SliderState::new().min(-12.0).max(12.0).step(0.5).default_value(0.0))).collect(),
            eq_preset_name: "自定义".to_string(),
            settings_tab: dialogs::SettingsTab::General,
            url_dialog_open: false,
            url_state,
        }
    }

    fn poll_player_state(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.position = self.player.position().as_secs_f64();
        self.duration = self.player.duration().as_secs_f64();
        self.volume = self.player.volume();
        self.is_playing = self.player.state() == crate::core::engine_trait::EngineState::Playing;
        self.is_muted = self.volume == 0;

        // Process pending download events
        self.poll_download_events();

        let pl = self.player.playlist();
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
            let album = if !track.album.is_empty() {
                track.album.clone()
            } else {
                String::new()
            };
            if self.title != display { self.title = display; }
            if self.artist != artist { self.artist = artist; }
            if self.album != album { self.album = album; }
            self.is_favourite = track.is_favourite;

            // Auto-load lyrics when track changes
            let track_path = track.file_path.clone();
            if track_path != self.last_lpc_path {
                self.last_lpc_path = track_path.clone();
                self.current_track_path_for_download = track_path.clone();
                self.load_lyrics_for_track(&track_path);
            }
            // Update download state keyword from current track
            if !track.title.is_empty() || !track.artist.is_empty() {
                if self.download_state.keyword.is_empty() || self.download_state.track_title != track.title {
                    self.download_state.auto_fill(&track.title, &track.artist);
                }
            }
        }

        // Update lyrics progress every frame
        self.lyric_state.recompute((self.position * 1000.0) as u64);
    }

    /// Search for and load lyrics for the given audio file path.
    fn load_lyrics_for_track(&mut self, audio_path: &str) {
        if audio_path.is_empty() {
            return;
        }
        let path = std::path::Path::new(audio_path);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = path.parent().unwrap_or(std::path::Path::new("."));

        // Search order: sibling .lrc, then lyrics/ subdirectory
        let sibling_lrc = parent.join(format!("{}.lrc", file_stem));
        let sibling_lrc_lower = parent.join(format!("{}.lrc", file_stem.to_lowercase()));

        let lyrics_dir = parent.join("lyrics");
        let lyrics_dir_lrc = lyrics_dir.join(format!("{}.lrc", file_stem));
        let lyrics_dir_lrc_lower = lyrics_dir.join(format!("{}.lrc", file_stem.to_lowercase()));

        for candidate in [&sibling_lrc, &sibling_lrc_lower, &lyrics_dir_lrc, &lyrics_dir_lrc_lower] {
            if candidate.exists() {
                match crate::lyric::load_lyric_file(candidate.to_str().unwrap_or("")) {
                    Ok(lyrics) => {
                        tracing::info!("[Lyric] Loaded: {:?}", candidate);
                        self.lyric_state.update(Some(lyrics), (self.position * 1000.0) as u64);
                        return;
                    }
                    Err(e) => {
                        tracing::warn!("[Lyric] Parse error {}: {}", candidate.display(), e);
                    }
                }
            }
        }

        // No lyrics found — clear state
        tracing::debug!("[Lyric] No lyrics found for: {}", audio_path);
        self.lyric_state.update(None, (self.position * 1000.0) as u64);
    }

    /// Process pending download events from background thread.
    fn poll_download_events(&mut self) {
        if let Some(rx) = &self.pending_download_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    lyric_download::DownloadEvent::SearchComplete(results) => {
                        self.download_state.searching = false;
                        match results {
                            Ok(items) => {
                                let count = items.len();
                                self.download_state.results = items;
                                self.download_state.selected_index = None;
                                self.download_state.status = format!("找到 {} 条结果", count);
                                tracing::info!("[LyricDownload] 搜索完成: {} 条结果", count);
                            }
                            Err(e) => {
                                self.download_state.status = e.clone();
                                tracing::warn!("[LyricDownload] 搜索失败: {}", e);
                            }
                        }
                    }
                    lyric_download::DownloadEvent::DownloadComplete { song_id, result } => {
                        self.download_state.downloading = false;
                        match result {
                            Ok(lrc_text) => {
                                // Determine save path
                                let save_path = self.compute_lyric_save_path();
                                match std::fs::write(&save_path, &lrc_text) {
                                    Ok(()) => {
                                        self.download_state.status =
                                            format!("下载成功: {}", save_path.display());
                                        tracing::info!(
                                            "[Lyric下载] 保存到: {}",
                                            save_path.display()
                                        );
                                        // Also update current lyrics
                                        match crate::lyric::load_lyric_file(
                                            save_path.to_str().unwrap_or_default(),
                                        ) {
                                            Ok(lyrics) => {
                                                self.lyric_state.update(
                                                    Some(lyrics),
                                                    (self.position * 1000.0) as u64,
                                                );
                                            }
                                            Err(e) => {
                                                tracing::warn!(
                                                    "[Lyric下载] 加载歌词失败: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        self.download_state.status =
                                            format!("保存失败: {}", e);
                                        tracing::error!(
                                            "[Lyric下载] 保存失败 {}: {}",
                                            save_path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                self.download_state.status = e.clone();
                                tracing::warn!("[Lyric下载] 下载失败: {}", e);
                            }
                        }
                        let _ = song_id;
                    }
                    lyric_download::DownloadEvent::ProgressUpdate { current, total } => {
                        self.download_state.progress = if total > 0 {
                            current as f32 / total as f32
                        } else {
                            0.0
                        };
                        self.download_state.total_tracks = total;
                        self.download_state.status =
                            format!("正在下载... {}/{}", current, total);
                    }
                }
            }
            // Clear channel if done
            if self.pending_download_rx.as_ref().map_or(false, |rx| rx.try_recv().is_err()) {
                // Check if we have any more events pending - if not, clear after a short check
                if !self.download_state.searching && !self.download_state.downloading {
                    self.pending_download_rx = None;
                }
            }
        }
    }

    /// Compute the save path for the lyric file.
    fn compute_lyric_save_path(&self) -> std::path::PathBuf {
        if self.current_track_path_for_download.is_empty() {
            // Fallback to lyrics directory in current dir
            return std::path::PathBuf::from("lyrics")
                .join("downloaded.lrc");
        }

        let track_path = std::path::Path::new(&self.current_track_path_for_download);
        let file_stem = track_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        if self.download_state.save_to_song_dir {
            // Save next to the audio file
            let parent = track_path.parent().unwrap_or(std::path::Path::new("."));
            parent.join(format!("{}.lrc", file_stem))
        } else {
            // Save to lyrics/ subdirectory
            let parent = track_path.parent().unwrap_or(std::path::Path::new("."));
            parent.join("lyrics").join(format!("{}.lrc", file_stem))
        }
    }
}

impl Render for MusicPlayer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // NOTE: do NOT call cx.notify() here — calling it every frame causes
        // an infinite re-render loop and triggers GPUI's
        // "RefCell already borrowed" panic. State changes that need a
        // repaint should call cx.notify() from the event handler that
        // produced the change, not from render itself.
        self.poll_player_state(window, cx);
        let tr = self.tr;
        let c = &self.colours;

        // Update responsive state based on window size
        let window_bounds = window.bounds();
        let window_width: f32 = window_bounds.size.width.into();
        let window_height: f32 = window_bounds.size.height.into();
        self.responsive.update(window_width, window_height);
        let layout_mode = self.responsive.mode;

        // Mini mode: compact player window
        if MINI_MODE.load(Ordering::Relaxed) {
            let play_label = if self.is_playing { "⏸" } else { "▶" };
            let player_p = self.player.clone();
            let player_n = self.player.clone();
            let player_rw = self.player.clone();
            let player_ff = self.player.clone();
            let player_s = self.player.clone();
            let vol = self.volume;
            let pos_str_mini = format!("{:02}:{:02} / {:02}:{:02}",
                (self.position as u32) / 60, (self.position as u32) % 60,
                (self.duration as u32) / 60, (self.duration as u32) % 60);
            let seek_pct = if self.duration > 0.0 { (self.position / self.duration * 100.0) as f32 } else { 0.0 };

            return v_flex().size_full().bg(c.bg).gap_4()
                .child(v_flex().flex_grow().items_center().justify_center().gap_2()
                    .child(div().size(px(200.0)).rounded(px(16.0)).bg(c.panel)) // album art placeholder
                    .child(layout::txt(&self.title, 16.0, c.text_title))
                    .child(layout::txt(&self.artist, 12.0, c.text_dim))
                )
                .child(v_flex().w_full().px_4().gap_2()
                    .child(h_flex().w_full().h(px(4.0)).bg(c.progress_track)
                        .child(div().h_full().w(DefiniteLength::Fraction(seek_pct / 100.0)).bg(c.accent))
                    )
                    .child(layout::txt(&pos_str_mini, 9.0, c.text_dim))
                )
                .child(h_flex().items_center().justify_center().gap_4().pb_4()
                    .child(Button::new("mini_rw").label("⏪").ghost().on_click(move |_, _, _| {
                        let pos = player_rw.position();
                        let _ = player_rw.seek(pos.saturating_sub(std::time::Duration::from_secs(5)));
                    }))
                    .child(Button::new("mini_prev").icon(IconName::ChevronLeft).ghost().on_click(move |_, _, _| { let _ = player_p.prev(); }))
                    .child(Button::new("mini_play").label(play_label).primary().on_click(move |_, _, _| { let _ = if player_n.is_playing() { player_n.toggle_pause() } else { player_n.play_at_index(player_n.playlist().current_index().unwrap_or(0)) }; }))
                    .child(Button::new("mini_next").icon(IconName::ChevronRight).ghost().on_click(move |_, _, _| { let _ = player_s.next(); }))
                    .child(Button::new("mini_ff").label("⏩").ghost().on_click(move |_, _, _| {
                        let dur = player_ff.duration();
                        let pos = player_ff.position();
                        let _ = player_ff.seek((pos + std::time::Duration::from_secs(5)).min(dur));
                    }))
                    .child(layout::txt(&format!("{:02}%", vol), 9.0, c.text_dim))
                )
                .into_any_element();
        }

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

        // -- spectrum --
        const SPECTRUM_BARS: usize = 48;

        let player_play = self.player.clone();
        let player_prev = self.player.clone();
        let player_next = self.player.clone();
        let player_stop = self.player.clone();
        let player_seek = self.player.clone();
        let player_rew = self.player.clone();
        let player_ff = self.player.clone();
        let player_repeat = self.player.clone();
        let repeat_mode_config = self.player.repeat_mode();
        let repeat_label = match repeat_mode_config {
            crate::core::playlist::RepeatMode::LoopPlaylist => "🔁",
            crate::core::playlist::RepeatMode::LoopTrack => "🔂",
            crate::core::playlist::RepeatMode::PlayShuffle => "🔀",
            _ => "➡️",
        };
        let raw_spec = self.player.calculate_spectrum();
        let raw_peaks = self.player.spectrum_peak_data();
        let panel = Panel::from_u8(ACTIVE_PANEL.load(Ordering::Relaxed));
        let playlist_tracks = self.player.playlist().tracks().to_vec();
        let current_idx = self.player.playlist().current_index();

        let sidebar = if layout_mode.show_sidebar() {
            let active = panel;
            let sidebar_width = layout_mode.sidebar_width();
            let btn_size = layout_mode.button_size();
            let make_btn = |id: &'static str, icon: IconName, p: Panel, c: &UiColors| {
                let bg = if active == p { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
                Button::new(id)
                    .icon(icon)
                    .ghost()
                    .compact()
                    .w(px(btn_size)).h(px(btn_size))
                    .rounded(px(8.0))
                    .bg(bg)
                    .on_click(move |_, _, _cx| {
                        ACTIVE_PANEL.store(p.to_u8(), Ordering::Relaxed);
                    })
            };
            Some(
                v_flex()
                    .items_center()
                    .w(px(sidebar_width))
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
                    .child(make_btn("lyr_edit", IconName::File, Panel::LyricEditor, c))
                    .child(make_btn("lyr_download", IconName::ArrowDown, Panel::LyricDownload, c))
                    .child(make_btn("eq", IconName::Settings, Panel::Equalizer, c))
            )
        } else {
            None
        };

        let content_area = div().flex_grow().h_full().child(match panel {
            Panel::Playlist => self.render_playlist(&playlist_tracks, current_idx, layout_mode, window, cx).into_any_element(),
            Panel::MediaLib => self.render_media_lib_panel(c, window, cx).into_any_element(),
            Panel::Lyrics => desktop_lyrics::render_lyrics_panel(&self.lyric_state, c).into_any_element(),
            Panel::LyricEditor => self.render_lyric_editor_panel(c, window, cx).into_any_element(),
            Panel::Equalizer => self.render_equalizer_panel(c, window, cx).into_any_element(),
            Panel::Search => dialogs::render_search_panel(c, &self.playlist_filter_text, window, cx).into_any_element(),
            Panel::Settings => dialogs::render_settings_panel(c, self.settings_tab, window, cx).into_any_element(),
            Panel::LyricDownload => self.render_lyric_download_panel(c, window, cx).into_any_element(),
            Panel::FileBrowser => self.render_file_browser(c, window, cx).into_any_element(),
            _ => layout::content_area(c, tr).into_any_element(),
        });

        let control_bar_h = layout_mode.control_bar_height();
        let progress_h = layout_mode.progress_bar_height();
        let title_size = layout_mode.title_font_size();
        let artist_size = layout_mode.artist_font_size();
        let vol_width = layout_mode.volume_slider_width();

        // Build main layout with responsive adjustments
        let mut main_layout = v_flex().size_full().bg(c.bg);

        // Title bar - always shown
        main_layout = main_layout.child(layout::title_bar(c, tr));

        // Menu bar - only shown in BIG and NARROW modes
        if layout_mode.show_menubar() {
            main_layout = main_layout.child(self.render_menu_bar(c, tr, window, cx));
        }

        // Main content area
        let mut content_flex = h_flex().w_full().flex_grow();
        if let Some(sidebar_el) = sidebar {
            content_flex = content_flex.child(sidebar_el);
        }
        content_flex = content_flex.child(content_area);
        main_layout = main_layout.child(content_flex);

        // Prepare button states
        let player_mute = self.player.clone();

        // Control bar at bottom
        main_layout = main_layout.child(
            v_flex()
                .w_full()
                .bg(c.control_bar_bg)
                .child({
                    let dur = self.duration;
                    h_flex()
                        .id("progress-bar")
                        .w_full()
                        .h(px(progress_h))
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
                    self.render_spectrum_strip(&raw_spec, &raw_peaks, SPECTRUM_BARS, c)
                )
                .child(
                    h_flex()
                        .items_center()
                        .w_full()
                        .px_4().gap_4()
                        .h(px(control_bar_h))
                        // Left: playback controls
                        .child(
                            h_flex().items_center().gap_3()
                                .child(Button::new("prev").icon(IconName::ChevronLeft).ghost().compact().on_click(move |_, _, _| { let _ = player_prev.prev(); }))
                                .child(Button::new("rew").label("⏪").ghost().compact().on_click(move |_, _, _| {
                                    let pos = player_rew.position();
                                    let new_pos = pos.saturating_sub(std::time::Duration::from_secs(5));
                                    let _ = player_rew.seek(new_pos);
                                }))
                                .child(Button::new("play").label(play_label).primary().compact().on_click(move |_, _, _| {
                                    tracing::info!("[GUI] Play button clicked, is_playing={}", player_play.is_playing());
                                    // Toggle pause if playing, otherwise play current/first track
                                    let is_playing = player_play.is_playing();
                                    if is_playing {
                                        tracing::info!("[GUI] toggle_pause");
                                        let _ = player_play.toggle_pause();
                                    } else {
                                        let idx = player_play.playlist().current_index().unwrap_or(0);
                                        tracing::info!("[GUI] play_at_index({}), playlist len={}", idx, player_play.playlist().len());
                                        let res = player_play.play_at_index(idx);
                                        tracing::info!("[GUI] play_at_index result: {:?}", res.as_ref().map(|_| ()).map_err(|e| e.to_string()));
                                        let _ = res;
                                    }
                                }))
                                .child(Button::new("ff").label("⏩").ghost().compact().on_click(move |_, _, _| {
                                    let dur = player_ff.duration();
                                    let pos = player_ff.position();
                                    let new_pos = (pos + std::time::Duration::from_secs(5)).min(dur);
                                    let _ = player_ff.seek(new_pos);
                                }))
                                .child(Button::new("next").icon(IconName::ChevronRight).ghost().compact().on_click(move |_, _, _| { let _ = player_next.next(); }))
                                .child(Button::new("repeat").label(repeat_label).ghost().compact().on_click(move |_, _, _| {
                                    use crate::core::playlist::RepeatMode;
                                    let modes = [RepeatMode::PlayOrder, RepeatMode::LoopPlaylist, RepeatMode::LoopTrack, RepeatMode::PlayShuffle, RepeatMode::PlayRandom];
                                    let current = player_repeat.repeat_mode();
                                    let idx = modes.iter().position(|m| *m == current).unwrap_or(0);
                                    let next = modes[(idx + 1) % modes.len()];
                                    player_repeat.set_repeat_mode(next);
                                }))
                                .child(Button::new("stop").label("⏹").ghost().compact().on_click(move |_, _, _| { let _ = player_stop.stop(); })),
                        )
                        // Center: track info
                        .child(
                            v_flex().flex_grow().gap_1()
                                .child(crate::gui::layout::txt(&self.title, title_size, c.text_title))
                                .child(crate::gui::layout::txt(&self.artist, artist_size, c.text_dim)),
                        )
                        // Right: extra buttons and controls
                        .child(
                            h_flex().items_center().gap_2()
                                .child(layout::txt(&pos_str, artist_size, c.text_dim))
                                // Speed slider (fine-grained speed control)
                                .child(h_flex().items_center().gap_1().child(
                                    layout::txt(&format!("{:.2}x", self.player.speed()), 9.0, c.accent)
                                ).child(Slider::new(&self.speed_slider).horizontal().w(px(60.0))))
                                // Mute button
                                .child(Button::new("mute").label(if self.is_muted { "X" } else { "V" }).ghost().compact().on_click(move |_, _, _| {
                                    let vol = player_mute.volume();
                                    if vol > 0 {
                                        let _ = player_mute.set_volume(0);
                                    } else {
                                        let _ = player_mute.set_volume(80);
                                    }
                                }))
                                // Volume slider
                                .child(Slider::new(&self.volume_slider).horizontal().w(px(vol_width)))
                                // Favourite button
                                .child(Button::new("favourite").label(if self.is_favourite { "♥" } else { "♡" }).ghost().compact())
                                // Lyrics toggle button
                                .child(Button::new("lyric_btn").icon(IconName::BookOpen).ghost().compact())
                                // Settings button
                                .child(Button::new("settings_btn").icon(IconName::Settings).ghost().compact())
                                // Equalizer button
                                .child(Button::new("eq_btn").label("EQ").ghost().compact())
                                // Fullscreen button
                                .child(Button::new("fullscreen").label("⛶").ghost().compact().on_click(|_, window, _| {
                                    window.toggle_fullscreen();
                                })),
                        ),
                ),
        );

        // Extra toolbar row for BIG mode (favourite, playlist ops, media library, etc.)
        if matches!(layout_mode, LayoutMode::Big) {
            let track_count = playlist_tracks.len();
            let total_dur = self.player.playlist().total_duration_str();
            main_layout = main_layout.child(
                h_flex()
                    .items_center()
                    .w_full()
                    .h(px(28.0))
                    .px_4().gap_3()
                    .bg(c.control_bar_bg)
                    .child(Button::new("media_lib_btn").icon(IconName::Folder).ghost().compact().label("媒体库")
                        .on_click(cx.listener(|this, _, _window, _cx| {
                            this.media_lib_open = !this.media_lib_open;
                            tracing::info!("[Playlist] 媒体库 toggled: {}", this.media_lib_open);
                        })))
                    .child(Button::new("add_files_btn").icon(IconName::Plus).ghost().compact().label("添加")
                        .on_click(cx.listener(|this, _, _window, _cx| {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                                .add_filter("播放列表", &["m3u", "m3u8"])
                                .pick_file()
                            {
                                let path = file.to_string_lossy().to_string();
                                if path.ends_with(".m3u") || path.ends_with(".m3u8") {
                                    // Import M3U playlist
                                    match crate::core::playlist::Playlist::import_m3u(&path) {
                                        Ok(tracks) => {
                                            let count = tracks.len();
                                            this.player.playlist_mut().add_tracks(tracks);
                                            tracing::info!("[Playlist] 导入 {} 首从 {}", count, path);
                                        }
                                        Err(e) => tracing::error!("[Playlist] 导入失败: {}", e),
                                    }
                                } else {
                                    // Add audio file
                                    let _ = this.player.play_file(&path);
                                }
                            }
                        })))
                    .child(Button::new("import_pl_btn").icon(IconName::ArrowDown).ghost().compact().label("导入")
                        .on_click(cx.listener(|this, _, _window, _cx| {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("播放列表", &["m3u", "m3u8", "wpl", "ttpl", "playlist"])
                                .pick_file()
                            {
                                let path = file.to_string_lossy().to_string();
                                let ext = std::path::Path::new(&path).extension()
                                    .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                let result: Result<Vec<crate::core::playlist::Track>, String> = match ext.as_str() {
                                    "m3u" | "m3u8" => crate::core::playlist::Playlist::import_m3u(&path).map_err(|e| e.to_string()),
                                    "wpl" | "ttpl" | "playlist" => {
                                        match crate::playlist_format::read_playlist(&path) {
                                            Ok(paths) => Ok(paths.into_iter().map(|p| crate::core::playlist::Track::new(&p)).collect()),
                                            Err(e) => Err(e.to_string()),
                                        }
                                    }
                                    _ => Err("不支持的格式".to_string()),
                                };
                                let tracks = match result {
                                    Ok(t) => t,
                                    Err(e) => {
                                        tracing::error!("[Playlist] 导入失败: {}", e);
                                        return;
                                    }
                                };
                                let count = tracks.len();
                            }
                        })))
                    .child(Button::new("save_pl_btn").icon(IconName::File).ghost().compact().label("导出")
                        .on_click(cx.listener(|this, _, _window, _cx| {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("M3U播放列表", &["m3u8"])
                                .add_filter("WPL播放列表", &["wpl"])
                                .add_filter("TTPL播放列表", &["ttpl"])
                                .add_filter("原生播放列表", &["playlist"])
                                .set_file_name("playlist.m3u8")
                                .save_file()
                            {
                                let path = file.to_string_lossy().to_string();
                                let ext = std::path::Path::new(&path).extension()
                                    .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                let result = match ext.as_str() {
                                    "m3u" | "m3u8" => this.player.playlist().export_m3u(&path, true).map_err(|e| e.to_string()),
                                    "wpl" => crate::playlist_format::write_playlist(&path, this.player.playlist().tracks(), None).map_err(|e| e.to_string()),
                                    "ttpl" => crate::playlist_format::write_playlist(&path, this.player.playlist().tracks(), None).map_err(|e| e.to_string()),
                                    "playlist" => crate::playlist_format::write_playlist(&path, this.player.playlist().tracks(), None).map_err(|e| e.to_string()),
                                    _ => Err("不支持的格式".to_string()),
                                };
                                match result {
                                    Ok(()) => tracing::info!("[Playlist] 导出到 {}", path),
                                    Err(e) => tracing::error!("[Playlist] 导出失败: {}", e),
                                }
                            }
                        })))
                    .child(div().flex_grow())
                    .child(layout::txt(&format!("{} 首 | {}", track_count, total_dur), 10.0, c.text_dim))
            );
        }

        // Status bar - only shown in BIG mode
        if layout_mode.show_statusbar() {
            let track_count = playlist_tracks.len();
            let total_dur = self.player.playlist().total_duration_str();
            let repeat_desc = self.player.repeat_mode().description();
            let engine_name = self.player.engine_name();
            let file_type = if let Some(track) = self.player.playlist().current_track() {
                if !track.file_type.is_empty() {
                    track.file_type.clone()
                } else {
                    "unknown".to_string()
                }
            } else {
                "--".to_string()
            };
            main_layout = main_layout.child(
                h_flex()
                    .items_center()
                    .w_full()
                    .h(px(theme::STATUSBAR_HEIGHT))
                    .px_3().gap_4()
                    .bg(c.statusbar_bg)
                    .child(layout::txt(&format!("� {} 首", track_count), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("� {}", total_dur), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("� {}", file_type), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("� {}", repeat_desc), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("⚙ {}", engine_name), 10.0, c.text_dim))
                    .child(div().flex_grow())
                    .child(
                        h_flex().items_center().gap_2()
                            .child(div().size(px(6.0)).rounded(px(3.0)).bg(
                                if RPC_SERVER_RUNNING.load(Ordering::Relaxed) {
                                    Hsla { h: 120.0/360.0, s: 0.8, l: 0.45, a: 1.0 }
                                } else {
                                    Hsla { h: 0.0, s: 0.8, l: 0.45, a: 1.0 }
                                }
                            ))
                            .child(layout::txt(if RPC_SERVER_RUNNING.load(Ordering::Relaxed) { "RPC 在线" } else { "RPC 离线" }, 10.0, c.text_dim))
                    )
            );
        }

        div().child(main_layout)
            .context_menu({
                let p = self.player.clone();
                move |menu, _w, _cx| {
                    let p1 = p.clone();
                    let p2 = p.clone();
                    let p3 = p.clone();
                    let p4 = p.clone();
                    let p5 = p.clone();
                    menu.item(PopupMenuItem::new("播放/暂停").on_click(move |_, _, _| { let _ = p1.toggle_pause(); }))
                        .item(PopupMenuItem::new("停止").on_click(move |_, _, _| { let _ = p2.stop(); }))
                        .item(PopupMenuItem::new("上一曲").on_click(move |_, _, _| { let _ = p3.prev(); }))
                        .item(PopupMenuItem::new("下一曲").on_click(move |_, _, _| { let _ = p4.next(); }))
                        .separator()
                        .item(PopupMenuItem::new("循环模式").on_click(move |_, _, _| {
                            use crate::core::playlist::RepeatMode;
                            let next = match p5.repeat_mode() {
                                RepeatMode::PlayOrder => RepeatMode::LoopPlaylist,
                                RepeatMode::LoopPlaylist => RepeatMode::LoopTrack,
                                RepeatMode::LoopTrack => RepeatMode::PlayShuffle,
                                RepeatMode::PlayShuffle => RepeatMode::PlayRandom,
                                RepeatMode::PlayRandom => RepeatMode::PlayTrack,
                                RepeatMode::PlayTrack => RepeatMode::PlayOrder,
                            };
                            p5.set_repeat_mode(next);
                        }))
                }
            })
            .into_any_element()
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
        // Copy i18n strings to satisfy 'static lifetime for closures
        let s_file = tr.menu_file;
        let s_open_file = tr.menu_open_file;
        let s_open_folder = tr.menu_open_folder;
        let s_save_as_new = tr.menu_save_as_new;
        let s_exit = tr.menu_exit;

        let s_playback = tr.menu_playback;
        let s_playlist = tr.menu_playlist;

        h_flex()
            .items_center()
            .w_full()
            .h(px(theme::MENUBAR_HEIGHT))
            .px_2().gap_1()
            .bg(c.control_bar_bg)
            // File menu
            .child({
                let player = self.player.clone();
                layout::menu_dropdown(s_file, IconName::Folder, move |menu, _w, _cx| {
                    let p = player.clone();
                    menu.item(PopupMenuItem::new(s_open_file).on_click(move |_, _, _| {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape", "cue"])
                            .pick_file()
                        {
                            let _ = p.play_file(path.to_str().unwrap_or_default());
                        }
                    }))
                    .item(PopupMenuItem::new(s_open_folder).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                let dir = folder.to_str().unwrap_or_default();
                                match crate::media::scan_directory(dir, true, None) {
                                    Ok(entries) => {
                                        // Add all tracks to playlist
                                        let mut pl = p.playlist_mut();
                                        let first_idx = pl.len();
                                        for e in &entries {
                                            pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                        }
                                        drop(pl);
                                        // Save to media library
                                        let mut lib = crate::media::MediaLib::load();
                                        for e in &entries { lib.upsert(e.clone()); }
                                        let _ = lib.save();
                                        // Play first track
                                        if !entries.is_empty() {
                                            let _ = p.play_at_index(first_idx);
                                        }
                                        tracing::info!("[Menu] Loaded {} tracks from folder", entries.len());
                                    }
                                    Err(e) => tracing::warn!("Scan folder failed: {}", e),
                                }
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_save_as_new).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            if let Some(file) = rfd::FileDialog::new()
                                .add_filter("M3U 播放列表", &["m3u", "m3u8"])
                                .set_file_name("playlist.m3u")
                                .save_file()
                            {
                                let path = file.to_string_lossy().to_string();
                                match p.playlist_mut().export_m3u(&path, true) {
                                    Ok(()) => tracing::info!("[Menu] 另存为新播放列表: {}", path),
                                    Err(e) => tracing::error!("[Menu] 保存失败: {}", e),
                                }
                            }
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_exit).on_click(|_, _, _| std::process::exit(0)))
                })
            })
            // Playback menu
            .child({
                let player = self.player.clone();
                layout::menu_dropdown(s_playback, IconName::ChevronRight, move |menu, _w, _cx| {
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
                    let p14 = player.clone();
                    menu.item(PopupMenuItem::new("播放/暂停").on_click(move |_, _, _| { let _ = p1.toggle_pause(); }))
                        .item(PopupMenuItem::new("停止").on_click(move |_, _, _| { let _ = p2.stop(); }))
                        .item(PopupMenuItem::new("上一曲").on_click(move |_, _, _| { let _ = p3.prev(); }))
                        .item(PopupMenuItem::new("下一曲").on_click(move |_, _, _| { let _ = p4.next(); }))
                        .separator()
                        .item(PopupMenuItem::new("快退5秒").on_click({
                            let p = p14.clone();
                            move |_, _, _| {
                                let pos = p.position();
                                let _ = p.seek(std::time::Duration::from_secs_f64((pos.as_secs_f64() - 5.0).max(0.0)));
                            }
                        }))
                        .item(PopupMenuItem::new("快进5秒").on_click(move |_, _, _| {
                            let pos = p14.position();
                            let _ = p14.seek(std::time::Duration::from_secs_f64(pos.as_secs_f64() + 5.0));
                        }))
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
            // Playlist menu
            .child({
                let player = self.player.clone();
                layout::menu_dropdown(s_playlist, IconName::SquareTerminal, move |menu, _, _| {
                    let p1 = player.clone();
                    let p2 = player.clone();
                    let p3 = player.clone();
                    menu.item(PopupMenuItem::new("添加文件").on_click(move |_, _, _| {
                        if let Some(paths) = rfd::FileDialog::new()
                            .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                            .pick_files()
                        {
                            let mut pl = p1.playlist_mut();
                            for path in &paths {
                                pl.add_track(crate::core::playlist::Track::new(path.to_str().unwrap_or_default()));
                            }
                        }
                    }))
                    .item(PopupMenuItem::new("添加文件夹").on_click(move |_, _, _| {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            if let Ok(entries) = crate::media::scan_directory(folder.to_str().unwrap_or_default(), true, None) {
                                let mut pl = p2.playlist_mut();
                                for e in entries {
                                    pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                }
                            }
                        }
                    }))
                    .item(PopupMenuItem::new("从媒体库添加").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let lib = crate::media::MediaLib::load();
                            let count = lib.entries.len();
                            let mut pl = p.playlist_mut();
                            for e in &lib.entries {
                                pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                            }
                            tracing::info!("[Playlist] 从媒体库添加 {} 首", count);
                        }
                    }))
                    .item(PopupMenuItem::new("添加URL").on_click(move |_, _, _| {
                        tracing::info!("[Playlist] 添加URL - 打开URL对话框");
                    }))
                    .item(PopupMenuItem::new("删除选中").on_click(move |_, _, _| {
                        // Placeholder: would need selection tracking
                        tracing::info!("Delete selected tracks");
                    }))
                    .item(PopupMenuItem::new("清空播放列表").on_click(move |_, _, _| {
                        p3.playlist_mut().clear();
                    }))
                    .separator()
                    .item(PopupMenuItem::new("移除重复").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let removed = p.playlist_mut().dedup();
                            tracing::info!("Removed {} duplicate tracks", removed);
                        }
                    }))
                    .item(PopupMenuItem::new("移除失效文件").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let removed = p.playlist_mut().clean();
                            tracing::info!("Removed {} missing tracks", removed);
                        }
                    }))
                    .item(PopupMenuItem::new("修复路径错误").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let pl = p.playlist();
                            let mut fixed = 0;
                            for i in 0..pl.len() {
                                if let Some(track) = pl.get(i) {
                                    let name = if !track.title.is_empty() { &track.title } else { &track.file_name };
                                    if !std::path::Path::new(&track.file_path).exists() {
                                        if let Some(new_path) = rfd::FileDialog::new()
                                            .set_title(&format!("修复: {}", name))
                                            .pick_file()
                                        {
                                            let new_path_str = new_path.to_string_lossy().to_string();
                                            if let Some(t) = p.playlist_mut().get_mut(i) {
                                                t.file_path = new_path_str;
                                            }
                                            fixed += 1;
                                        }
                                    }
                                }
                            }
                            tracing::info!("Fixed {} paths", fixed);
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new("保存播放列表").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            if let Err(e) = p.save_current_playlist() {
                                tracing::warn!("Save playlist failed: {}", e);
                            }
                        }
                    }))
                    .item(PopupMenuItem::new("另存为新播放列表").on_click(move |_, _, _| {
                        // Would need dialog for name input
                        tracing::info!("Save as new playlist");
                    }))
                    .separator()
                    // Sort submenu
                    .item(PopupMenuItem::new("排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            // Default sort by title ascending
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Title, false);
                            tracing::info!("Playlist sorted by title");
                        }
                    }))
                    .item(PopupMenuItem::new("按艺术家排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Artist, false);
                            tracing::info!("Playlist sorted by artist");
                        }
                    }))
                    .item(PopupMenuItem::new("按专辑排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Album, false);
                            tracing::info!("Playlist sorted by album");
                        }
                    }))
                    .item(PopupMenuItem::new("按时长排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Time, false);
                            tracing::info!("Playlist sorted by duration");
                        }
                    }))
                    .item(PopupMenuItem::new("按文件名排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::FileName, false);
                            tracing::info!("Playlist sorted by filename");
                        }
                    }))
                    .item(PopupMenuItem::new("随机排序").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Random, false);
                            tracing::info!("Playlist shuffled");
                        }
                    }))
                    .item(PopupMenuItem::new("倒序排列").on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().reverse();
                            tracing::info!("Playlist reversed");
                        }
                    }))
                })
            })
            // Lyric menu
            .child({
                let player = self.player.clone();
                let s_lyric_label = tr.menu_lyric;
                let s_reload_lyric = tr.menu_reload_lyric;
                let s_copy_line = tr.menu_copy_current_line;
                let s_copy_all = tr.menu_copy_all_lyric;
                let s_edit_lyric = tr.menu_edit_lyric;
                let s_download_lyric = tr.menu_download_lyric;
                let s_batch_download = tr.menu_batch_download_lyric;
                let s_show_trans = tr.menu_show_translation;
                let s_show_desktop = tr.menu_show_desktop_lyric;
                layout::menu_dropdown(s_lyric_label, IconName::BookOpen, move |menu, _, _| {
                    let p1 = player.clone();
                    let p2 = player.clone();
                    let p3 = player.clone();
                    menu.item(PopupMenuItem::new(s_reload_lyric).on_click(move |_, _, _| {
                        // Reload lyrics: search for .lrc file in same directory as current track
                        if let Some(track) = p1.playlist().current_track() {
                            let path = std::path::Path::new(&track.file_path);
                            if let Some(parent) = path.parent() {
                                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                let lrc_path = parent.join(format!("{}.lrc", stem));
                                if lrc_path.exists() {
                                    match crate::lyric::parser::load_lyric_file(lrc_path.to_str().unwrap_or_default()) {
                                        Ok(lyrics) => {
                                            tracing::info!("Reloaded lyrics: {} lines", lyrics.len());
                                            // In a real implementation, would update UI state with new lyrics
                                        }
                                        Err(e) => tracing::warn!("Failed to reload lyrics: {}", e),
                                    }
                                } else {
                                    tracing::info!("No lyric file found at {:?}", lrc_path);
                                }
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_copy_line).on_click(move |_, _, _| {
                        // Copy current lyric line to clipboard
                        if let Some(_track) = p2.playlist().current_track() {
                            // Would need to get lyrics state and current position
                            let pos_ms = (p2.position().as_secs_f64() * 1000.0) as u64;
                            tracing::info!("Copy current lyric at position {}ms", pos_ms);
                            // In real implementation, would use arboard or similar to set clipboard
                        }
                    }))
                    .item(PopupMenuItem::new(s_copy_all).on_click(move |_, _, _| {
                        tracing::info!("Copy all lyrics to clipboard");
                    }))
                    .item(PopupMenuItem::new(s_edit_lyric).on_click(move |_, _, _| {
                        // Open lyric editor - would launch external editor or embedded editor
                        if let Some(track) = p3.playlist().current_track() {
                            let path = std::path::Path::new(&track.file_path);
                            if let Some(parent) = path.parent() {
                                let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                let lrc_path = parent.join(format!("{}.lrc", stem));
                                let path_str = lrc_path.to_string_lossy().to_string();
                                tracing::info!("Opening lyric editor for: {}", path_str);
                                // Would open editor window here
                                #[cfg(windows)]
                                {
                                    let _ = std::process::Command::new("notepad").arg(&path_str).spawn();
                                }
                                #[cfg(not(windows))]
                                {
                                    let _ = std::process::Command::new("xdg-open").arg(&path_str).spawn();
                                }
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_download_lyric).on_click(move |_, _, _| {
                        tracing::info!("Search and download lyrics online");
                        // Would integrate with online lyric download (netease/qq music APIs)
                    }))
                    .item(PopupMenuItem::new(s_batch_download).on_click(move |_, _, _| {
                        tracing::info!("Batch download lyrics for all tracks");
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_show_trans).on_click(|_, _, _| {
                        tracing::info!("Toggle lyric translation display");
                    }))
                    .item(PopupMenuItem::new(s_show_desktop).on_click(|_, _, _| {
                        tracing::info!("Toggle desktop lyrics window");
                        // Would toggle desktop lyrics overlay
                    }))
                    .item(PopupMenuItem::new("桌面歌词锁定").on_click(|_, _, _| {
                        tracing::info!("Lock/unlock desktop lyrics position");
                    }))
                    .separator()
                    .item(PopupMenuItem::new("歌词前进0.5秒").on_click(|_, _, _| {
                        tracing::info!("Shift lyrics forward 0.5s");
                    }))
                    .item(PopupMenuItem::new("歌词后退0.5秒").on_click(|_, _, _| {
                        tracing::info!("Shift lyrics backward 0.5s");
                    }))
                })
            })
            // View menu
            .child({
                let s_view_label = tr.menu_view;
                let s_toggle_playlist = tr.menu_toggle_playlist;
                let s_float_playlist = tr.menu_float_playlist;
                let s_toggle_menubar = tr.menu_toggle_menubar;
                let s_toggle_statusbar = tr.menu_toggle_statusbar;
                let s_mini_mode = tr.menu_mini_mode;
                let s_fullscreen = tr.menu_fullscreen;
                let s_toggle_dark = tr.menu_toggle_dark_mode;
                let s_always_on_top = tr.menu_always_on_top;
                layout::menu_dropdown(s_view_label, IconName::LayoutDashboard, move |menu, _, _| {
                let cfg = crate::config::Config::load();
                let dark_mode = cfg.appearance.dark_mode;
                menu.item(PopupMenuItem::new(s_toggle_playlist).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(0, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_float_playlist).on_click(|_, _, _| {
                    tracing::info!("Toggle floating playlist window");
                }))
                .item(PopupMenuItem::new(s_toggle_menubar).on_click(|_, _, _| {
                    tracing::info!("Toggle menu bar visibility (BIG/NARROW modes only)");
                }))
                .item(PopupMenuItem::new(s_toggle_statusbar).on_click(|_, _, _| {
                    tracing::info!("Toggle status bar visibility (BIG mode only)");
                }))
                .separator()
                .item(PopupMenuItem::new(s_mini_mode).on_click(|_, window, _| {
                    let is_mini = MINI_MODE.fetch_xor(true, Ordering::Relaxed);
                    if is_mini {
                        window.resize(gpui::Size { width: px(1200.0), height: px(800.0) });
                    } else {
                        window.resize(gpui::Size { width: px(340.0), height: px(520.0) });
                    }
                }))
                .item(PopupMenuItem::new(s_fullscreen).on_click(|_, window, _| {
                    window.toggle_fullscreen();
                }))
                .separator()
                .item(PopupMenuItem::new(if dark_mode { "浅色模式" } else { s_toggle_dark }).on_click(|_, _, _| {
                    // Toggle dark mode in config and reload
                    let mut cfg = crate::config::Config::load();
                    cfg.appearance.dark_mode = !cfg.appearance.dark_mode;
                    let _ = cfg.save();
                    tracing::info!("Dark mode toggled to: {}", cfg.appearance.dark_mode);
                    // In real implementation, would trigger UI reload/repaint
                }))
                .item(PopupMenuItem::new("切换主题颜色").on_click(|_, _, _| {
                    // Cycle through all 8 theme colors
                    let mut cfg = crate::config::Config::load();
                    cfg.appearance.theme = match cfg.appearance.theme.as_str() {
                        "ocean" => "forest".to_string(),
                        "forest" => "lavender".to_string(),
                        "lavender" => "sunset".to_string(),
                        "sunset" => "midnight".to_string(),
                        "midnight" => "autumn".to_string(),
                        "autumn" => "spring".to_string(),
                        "spring" => "default".to_string(),
                        _ => "ocean".to_string(),
                    };
                    let _ = cfg.save();
                    tracing::info!("Theme switched to: {}", cfg.appearance.theme);
                }))
                .item(PopupMenuItem::new(s_always_on_top).on_click(|_, _, _| {
                    tracing::info!("Toggle always on top");
                }))
                })
            })
            // Tools menu
            .child({
                let player = self.player.clone();
                let s_tools_label = tr.menu_tools;
                let s_media_lib = tr.media_lib_title;
                let s_find = tr.menu_find;
                let s_equalizer = tr.menu_equalizer;
                let s_settings = tr.menu_settings;
                layout::menu_dropdown(s_tools_label, IconName::Settings, move |menu, _, _| {
                let player_eq = player.clone();
                menu.item(PopupMenuItem::new(s_media_lib).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(2, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_find).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(3, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new("探索文件路径").on_click(|_, _, _| {
                    ACTIVE_PANEL.store(1, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new("歌曲信息").on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            tracing::info!("Track info: {} - {} ({})", track.title, track.artist, track.album);
                        }
                    }
                }))
                .item(PopupMenuItem::new(s_equalizer).on_click(|_, _, _| {
                    tracing::info!("Open equalizer dialog");
                    // Would show equalizer overlay/dialog
                }))
                .item(PopupMenuItem::new("格式转换").on_click(|_, _, _| {
                    tracing::info!("Open format converter");
                    // Would show format conversion dialog
                }))
                .item(PopupMenuItem::new("繁简转换").on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            let simplified = crate::charset::to_simplified_chinese(&track.title);
                            let traditional = crate::charset::to_traditional_chinese(&track.title);
                            tracing::info!("[Convert] 简体: {}, 繁体: {}", simplified, traditional);
                        }
                    }
                }))
                .item(PopupMenuItem::new("在线获取标签").on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            tracing::info!("[OnlineTag] 在线获取标签: {}", track.file_path);
                            let path = track.file_path.clone();
                            std::thread::spawn(move || {
                                let args = crate::cli::TagOnlineArgs {
                                    file: path,
                                    service: "netease".to_string(),
                                    auto: true,
                                    cover: false,
                                };
                                let _ = crate::commands::track::cmd_tag(&crate::cli::TagArgs {
                                    action: crate::cli::TagAction::Online(args),
                                });
                            });
                        }
                    }
                }))
                .item(PopupMenuItem::new("封面预览").on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            match crate::tag::writer::read_pictures(&track.file_path) {
                                Ok(pics) => {
                                    let count = pics.len();
                                    let info = if count > 0 {
                                        format!("该曲目包含 {} 张封面图片\n格式: {}\n大小: {} KB",
                                            count, pics[0].0, pics[0].1.len() / 1024)
                                    } else {
                                        "该曲目没有内嵌封面".to_string()
                                    };
                                    let _ = rfd::MessageDialog::new()
                                        .set_title("封面预览")
                                        .set_description(&info)
                                        .show();
                                }
                                Err(e) => {
                                    let _ = rfd::MessageDialog::new()
                                        .set_title("封面预览")
                                        .set_description(&format!("读取封面失败: {}", e))
                                        .show();
                                }
                            }
                        }
                    }
                }))
                .item(PopupMenuItem::new("定时停止").on_click(|_, _, _| {
                    if SLEEP_TIMER_ACTIVE.load(Ordering::Relaxed) {
                        SLEEP_TIMER_ACTIVE.store(false, Ordering::Relaxed);
                        tracing::info!("[SleepTimer] 已取消");
                    } else {
                        SLEEP_TIMER_ACTIVE.store(true, Ordering::Relaxed);
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_secs(3600)); // 1 hour default
                            if SLEEP_TIMER_ACTIVE.load(Ordering::Relaxed) {
                                tracing::info!("[SleepTimer] 定时停止播放");
                                // Would stop playback here
                            }
                            SLEEP_TIMER_ACTIVE.store(false, Ordering::Relaxed);
                        });
                        tracing::info!("[SleepTimer] 已设置1小时后停止播放");
                    }
                }))
                .item(PopupMenuItem::new("文件关联").on_click(|_, _, _| {
                    crate::commands::system::cmd_file_assoc(&crate::cli::FileAssocArgs {
                        action: crate::cli::FileAssocAction::Register,
                    }).ok();
                }))
                .separator()
                .item(PopupMenuItem::new(s_settings).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(5, Ordering::Relaxed);
                }))
                })
            })
            // Help menu
            .child({
                let s_help_label = tr.menu_help;
                let s_help_content = tr.menu_help_content;
                let s_about = tr.menu_about;
                layout::menu_dropdown(s_help_label, IconName::Info, move |menu, _, _| {
                menu.item(PopupMenuItem::new(s_help_content).on_click(|_, _, _| {
                    tracing::info!("Open local help documentation");
                    #[cfg(windows)]
                    let _ = std::process::Command::new("cmd").args(&["/c", "start", "https://github.com/zhongyang219/MusicPlayer2/wiki"]).spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open").arg("https://github.com/zhongyang219/MusicPlayer2/wiki").spawn();
                    #[cfg(all(not(windows), not(target_os = "macos")))]
                    let _ = std::process::Command::new("xdg-open").arg("https://github.com/zhongyang219/MusicPlayer2/wiki").spawn();
                }))
                .item(PopupMenuItem::new("在线帮助").on_click(|_, _, _| {
                    tracing::info!("Opening online help");
                    #[cfg(windows)]
                    let _ = std::process::Command::new("cmd").args(&["/c", "start", "https://github.com/zhongyang219/MusicPlayer2"]).spawn();
                    #[cfg(not(windows))]
                    let _ = std::process::Command::new("xdg-open").arg("https://github.com/zhongyang219/MusicPlayer2").spawn();
                }))
                .item(PopupMenuItem::new("检查更新").on_click(|_, _, _| {
                    crate::commands::system::check_update_background();
                }))
                .separator()
                .item(PopupMenuItem::new("支持的格式").on_click(|_, _, _| {
                    let formats = crate::audio_common::supported_extensions();
                    tracing::info!("Supported formats: {:?}", formats);
                }))
                .item(PopupMenuItem::new(s_about).on_click(|_, _, _| {
                    tracing::info!("About HackMagic Music Player v1.0.0");
                    tracing::info!("Based on MusicPlayer2 by zhongyang219");
                    tracing::info!("Rewritten in Rust with GPUI");
                    // Would show about dialog with version info
                }))
                })
            })
    }

    /// Render the lyric editor panel.
    fn render_lyric_editor_panel(
        &self,
        c: &UiColors,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = &self.editor_state;
        let row_count = state.rows.len();
        let dirty_marker = if state.dirty { " *" } else { "" };

        let mut children: Vec<gpui::AnyElement> = Vec::new();

        // === Toolbar ===
        let save_label = format!("💾 保存{}", dirty_marker);
        let toolbar = h_flex()
            .w_full()
            .px_3().py_2()
            .gap_2()
            .bg(c.control_bar_bg)
            .items_center()
            .child(Button::new("lyr_save").label(&save_label).compact().primary()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 保存功能待实现");
                }))
            .child(Button::new("lyr_save_as").label("另存为...").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 另存为功能待实现");
                }))
            .child(Button::new("lyr_open").label("� 打开").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 打开文件功能待实现");
                }))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_add").label("➕ 插入行").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 插入行功能待实现");
                }))
            .child(Button::new("lyr_delete").label("� 删除行").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 删除行功能待实现");
                }))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_back_500").label("� -500ms").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] -500ms 调整功能待实现");
                }))
            .child(Button::new("lyr_back_100").label("� -100ms").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] -100ms 调整功能待实现");
                }))
            .child(Button::new("lyr_fwd_100").label("▶ +100ms").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] +100ms 调整功能待实现");
                }))
            .child(Button::new("lyr_fwd_500").label("⏩ +500ms").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] +500ms 调整功能待实现");
                }))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_shift_all").label("� 全部偏移").compact()
                .on_click(|_, _, _| {
                    tracing::info!("[LyricEditor] 全部偏移功能待实现");
                }))
            .child(div().flex_grow())
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(c.text_dim)
                    .child(format!("{} 行{}", row_count, dirty_marker))
            )
            .into_any_element();
        children.push(toolbar);

        // === File path display ===
        if !state.file_path.is_empty() {
            children.push(
                h_flex()
                    .w_full()
                    .px_3().py_1()
                    .bg(c.panel_alt)
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .flex_grow()
                            .child(state.file_path.clone())
                    )
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(if state.dirty { c.accent } else { c.text_dim })
                            .child(if state.dirty { "● 未保存" } else { "✓ 已保存" })
                    )
                    .into_any_element()
            );
        }

        // === Lyrics list ===
        if state.rows.is_empty() {
            children.push(
                v_flex()
                    .flex_grow()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(c.text_dim)
                            .child("� 歌词编辑器为空")
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.text_dim)
                            .child("打开 LRC 文件或从当前曲目加载歌词")
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .child("提示：选中的行可以使用时间偏移按钮调整时序")
                    )
                    .into_any_element()
            );
        } else {
            let mut list = v_flex().w_full().flex_grow();

            for (i, row) in state.rows.iter().enumerate() {
                let is_selected = state.selected_row == Some(i);
                let bg = if is_selected {
                    c.playlist_item_selected
                } else if i % 2 == 0 {
                    c.panel
                } else {
                    c.panel_alt
                };

                let row_id = format!("lyr_row_{}", i);
                list = list.child(
                    h_flex()
                        .id(("lyr_row", i as u64))
                        .w_full()
                        .px_2().py_1()
                        .gap_2()
                        .bg(bg)
                        .items_center()
                        .hover(|s| s.bg(c.playlist_item_selected.opacity(0.5)))
                        .cursor(gpui::CursorStyle::PointingHand)
                        .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                            let _ = &row_id;
                            tracing::info!("[LyricEditor] 选择行 {}", i);
                        })
                        .child(
                            div()
                                .w(px(28.0))
                                .text_size(px(9.0))
                                .text_color(if is_selected { c.text } else { c.text_dim })
                                .text_right()
                                .child(format!("{:03}", i + 1))
                        )
                        .child(
                            div()
                                .w(px(78.0))
                                .text_size(px(11.0))
                                .text_color(if is_selected { c.accent } else { c.text_dim })
                                .font_family("monospace".to_string())
                                .child(row.timestamp.clone())
                        )
                        .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
                        .child(
                            div()
                                .flex_grow()
                                .text_size(px(12.0))
                                .text_color(c.text)
                                .child(if row.text.is_empty() {
                                    "(空)".to_string()
                                } else {
                                    row.text.clone()
                                })
                        )
                        .child(if is_selected {
                            div()
                                .w(px(16.0))
                                .text_size(px(10.0))
                                .text_color(c.accent)
                                .child("◉")
                        } else {
                            div()
                        })
                );
            }

            children.push(list.into_any_element());
        }

        // === Status bar ===
        let selection_info = match state.selected_row {
            Some(idx) => format!("选中: #{:03}", idx + 1),
            None => "未选中".to_string(),
        };
        children.push(
            h_flex()
                .w_full()
                .px_3().py_1()
                .bg(c.control_bar_bg)
                .items_center()
                .gap_2()
                .child(div().text_size(px(9.0)).text_color(c.text_dim).child(selection_info))
                .child(div().flex_grow())
                .child(div().text_size(px(9.0)).text_color(c.text_dim).child("◀/▶ 调整时序 • ➕/� 增删行 • 💾 保存"))
                .into_any_element()
        );

        v_flex()
            .size_full()
            .flex_grow()
            .bg(c.bg)
            .children(children)
    }

    /// Render the lyric download panel with full event handling.
    fn render_lyric_download_panel(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state = &self.download_state;
        let mut children: Vec<gpui::AnyElement> = Vec::new();

        // === Header ===
        children.push(
            div()
                .w_full()
                .px_3().py_2()
                .bg(c.control_bar_bg)
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(c.text)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("歌词下载")
                )
                .into_any_element()
        );

        // === Search bar ===
        let searching = state.searching;
        let source_netease = state.source == "netease";
        let keyword = state.keyword.clone();

        // Note: pending_download_tx is initialized per-search inside the click handler

        children.push(
            h_flex()
                .w_full()
                .px_3().py_2()
                .gap_2()
                .bg(c.panel)
                .items_center()
                .child(
                    // Source selector
                    h_flex()
                        .gap_1()
                        .child(
                            Button::new("src_netease")
                                .label("● 网易云")
                                .compact()
                                .ghost()
                                .bg(if source_netease { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                .on_click(cx.listener(move |this, _, _window, _cx| {
                                    this.download_state.source = "netease".to_string();
                                    tracing::info!("[LyricDownload] 切换到网易云");
                                }))
                        )
                        .child(
                            Button::new("src_qqmusic")
                                .label("○ QQ音乐")
                                .compact()
                                .ghost()
                                .bg(if !source_netease { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                .on_click(cx.listener(move |this, _, _window, _cx| {
                                    this.download_state.source = "qqmusic".to_string();
                                    tracing::info!("[LyricDownload] 切换到QQ音乐");
                                }))
                    )
            )
            .child(
                    // Keyword display (shows current search term)
                    div()
                        .flex_grow()
                        .text_size(px(11.0))
                        .text_color(if keyword.is_empty() { c.text_dim } else { c.text })
                        .child(if keyword.is_empty() {
                            "输入歌曲名或艺术家...".to_string()
                        } else {
                            keyword
                        })
                )
                .child(
                    Button::new("lyd_search")
                        .label(if searching { "搜索中..." } else { "搜索" })
                        .compact()
                        .bg(if searching { c.progress_track } else { c.accent })
                        .on_click(cx.listener(move |this, _, _window, _cx| {
                            if this.download_state.keyword.trim().is_empty() {
                                this.download_state.status = "请输入搜索关键词".to_string();
                                return;
                            }
                            if this.download_state.searching {
                                return;
                            }
                            this.download_state.searching = true;
                            this.download_state.status = "正在搜索...".to_string();
                            this.download_state.results.clear();
                            this.download_state.selected_index = None;

                            let keyword = this.download_state.keyword.clone();
                            let source = this.download_state.source.clone();

                            // Create channel for this search
                            let (tx, rx) = std::sync::mpsc::channel();
                            this.pending_download_rx = Some(rx);
                            this.pending_download_tx = Some(tx.clone());

                            let display_kw = keyword.clone();
                            let is_qq = source == "qqmusic";

                            // Spawn background thread to perform search
                            std::thread::spawn(move || {
                                let rt = match tokio::runtime::Runtime::new() {
                                    Ok(rt) => rt,
                                    Err(e) => {
                                        let _ = tx.send(
                                            lyric_download::DownloadEvent::SearchComplete(
                                                Err(format!("创建运行时失败: {}", e)),
                                            ),
                                        );
                                        return;
                                    }
                                };
                                let result = rt.block_on(async {
                                    lyric_download::search_lyrics(&source, &keyword).await
                                });
                                let _ = tx.send(
                                    lyric_download::DownloadEvent::SearchComplete(result),
                                );
                            });

                            tracing::info!(
                                "[LyricDownload] 开始搜索: '{}' 来自 {}",
                                display_kw,
                                if is_qq { "QQ音乐" } else { "网易云" }
                            );
                        }))
                )
                .into_any_element()
        );

        // === Options bar ===
        let include_translation = state.include_translation;
        let save_to_song_dir = state.save_to_song_dir;
        children.push(
            h_flex()
                .w_full()
                .px_3().py_1()
                .gap_3()
                .bg(c.panel_alt)
                .items_center()
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(c.text_dim)
                        .child("选项：")
                )
                .child(
                    Button::new("lyd_translate")
                        .label(if include_translation { "✓ 包含翻译" } else { "翻译" })
                        .compact()
                        .bg(if !include_translation { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } } else { c.accent })
                        .ghost()
                        .on_click(cx.listener(move |this, _, _window, _cx| {
                            this.download_state.include_translation = !this.download_state.include_translation;
                            tracing::info!(
                                "[LyricDownload] 包含翻译: {}",
                                this.download_state.include_translation
                            );
                        }))
                )
                .child(
                    Button::new("lyd_save_loc")
                        .label(if save_to_song_dir { "保存到歌曲目录" } else { "保存到歌词目录" })
                        .compact()
                        .bg(if !save_to_song_dir { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } } else { c.accent })
                        .ghost()
                        .on_click(cx.listener(move |this, _, _window, _cx| {
                            this.download_state.save_to_song_dir = !this.download_state.save_to_song_dir;
                            tracing::info!(
                                "[LyricDownload] 保存位置: {}",
                                if this.download_state.save_to_song_dir { "歌曲目录" } else { "歌词目录" }
                            );
                        }))
                )
                .child(div().flex_grow())
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(c.text_dim)
                        .child(format!("来源: {}", if source_netease { "网易云音乐" } else { "QQ音乐" }))
                )
                .into_any_element()
        );

        // === Status bar (status message) ===
        if !state.status.is_empty() {
            children.push(
                h_flex()
                    .w_full()
                    .px_3().py_1()
                    .bg(c.panel_alt)
                    .items_center()
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.accent)
                            .child(state.status.clone())
                    )
                    .into_any_element()
            );
        }

        // === Results area ===
        if state.results.is_empty() && !state.searching {
            children.push(
                v_flex()
                    .flex_grow()
                    .size_full()
                    .px_4().py_8()
                    .justify_center()
                    .items_center()
                    .gap_3()
                    .bg(c.bg)
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(c.text_dim)
                            .child("🔍 搜索歌词")
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.text_dim)
                            .child("输入歌曲名称和艺术家，然后按搜索")
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .child("支持网易云音乐和QQ音乐搜索")
                    )
                    .into_any_element()
            );
        } else if state.searching {
            children.push(
                v_flex()
                    .flex_grow()
                    .size_full()
                    .px_4().py_8()
                    .justify_center()
                    .items_center()
                    .gap_2()
                    .bg(c.bg)
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(c.accent)
                            .child("搜索中...")
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .child("正在从在线音乐库搜索歌词")
                    )
                    .into_any_element()
            );
        } else {
            // Results list
            let result_count = state.results.len();
            let downloading = state.downloading;
            let source = state.source.clone();
            let include_translation = state.include_translation;

            let mut list = v_flex().w_full().flex_grow();

            for (i, result) in state.results.iter().enumerate() {
                let is_selected = state.selected_index == Some(i);
                let bg = if is_selected {
                    c.playlist_item_selected
                } else if i % 2 == 0 {
                    c.panel
                } else {
                    c.panel_alt
                };

                let info_line = if !result.artist.is_empty() && !result.album.is_empty() {
                    format!("{} - {}", result.artist, result.album)
                } else if !result.artist.is_empty() {
                    result.artist.clone()
                } else if !result.album.is_empty() {
                    result.album.clone()
                } else {
                    "未知".to_string()
                };

                let title = result.title.clone();
                let song_id = result.id.clone();
                let src = source.clone();
                let inc_trans = include_translation;

                // Pre-clone for the download closure to avoid borrow issues
                let dl_title = title.clone();
                let dl_song_id = song_id.clone();
                let dl_src = src.clone();

                list = list.child(
                    h_flex()
                        .id(("lyr_dld_result", i as u64))
                        .w_full()
                        .px_3().py_2()
                        .gap_2()
                        .bg(bg)
                        .items_center()
                        .hover(|s| s.bg(c.playlist_item_selected.opacity(0.5)))
                        .cursor(CursorStyle::PointingHand)
                        .on_click(cx.listener(move |this, _, _window, _cx| {
                            this.download_state.selected_index = Some(i);
                            tracing::info!("[LyricDownload] 选中结果 #{}", i);
                        }))
                        .child(
                            // Index indicator
                            div()
                                .w(px(24.0))
                                .text_size(px(9.0))
                                .text_color(if is_selected { c.text } else { c.text_dim })
                                .text_right()
                                .child(format!("{:02}", i + 1))
                        )
                        .child(
                            // Cover placeholder
                            div()
                                .w(px(32.0)).h(px(32.0))
                                .rounded(px(4.0))
                                .bg(c.panel_alt)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .text_color(c.text_dim)
                                        .child("♪")
                                )
                        )
                        .child({
                            div()
                                .flex_grow()
                                .child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(c.text)
                                        .child(title.clone())
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(c.text_dim)
                                        .mt(px(1.0))
                                        .child(info_line)
                                )
                        })
                        .child(
                            // Download button
                            Button::new(("lyd_dld_btn", i as u64))
                                .label(if downloading && is_selected { "下载中" } else { "下载" })
                                .compact()
                                .bg(if downloading { c.progress_track } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                .ghost()
                                .on_click(cx.listener(move |this, _, _window, _cx| {
                                    if this.download_state.downloading {
                                        return;
                                    }
                                    this.download_state.downloading = true;
                                    this.download_state.selected_index = Some(i);

                                    let display_title = dl_title.clone();
                                    let song_id_val = dl_song_id.clone();
                                    this.download_state.status = format!("正在下载: {}", display_title);

                                    let source = dl_src.clone();
                                    let include_translation = inc_trans;
                                    let song_id = song_id_val.clone();
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    this.pending_download_rx = Some(rx);
                                    this.pending_download_tx = Some(tx.clone());

                                    std::thread::spawn(move || {
                                        let rt = match tokio::runtime::Runtime::new() {
                                            Ok(rt) => rt,
                                            Err(e) => {
                                                let _ = tx.send(
                                                    lyric_download::DownloadEvent::DownloadComplete {
                                                        song_id: song_id.clone(),
                                                        result: Err(format!("创建运行时失败: {}", e)),
                                                    },
                                                );
                                                return;
                                            }
                                        };
                                        let result = rt.block_on(async {
                                            lyric_download::download_lyric(
                                                &source,
                                                &song_id,
                                                include_translation,
                                            )
                                            .await
                                        });
                                        let _ = tx.send(
                                            lyric_download::DownloadEvent::DownloadComplete {
                                                song_id,
                                                result,
                                            },
                                        );
                                    });

                                    tracing::info!("[Lyric下载] 开始下载: {} (ID: {})", display_title, song_id_val);
                                }))
                        )
                );
            }

            // Progress bar for batch download
            if state.total_tracks > 0 {
                let pct = (state.progress * 100.0) as u32;
                let progress_current = (state.progress * state.total_tracks as f32) as usize;
                list = list.child(
                    h_flex()
                        .w_full()
                        .px_3().py_1()
                        .bg(c.control_bar_bg)
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .w(px(80.0))
                                .text_size(px(9.0))
                                .text_color(c.text_dim)
                                .child(format!("{}/{}", progress_current, state.total_tracks))
                        )
                        .child(
                            div()
                                .flex_grow()
                                .h(px(4.0))
                                .bg(c.progress_track)
                                .rounded(px(2.0))
                                .child(
                                    div()
                                        .h_full()
                                        .w(DefiniteLength::Fraction(state.progress.clamp(0.0, 1.0)))
                                        .bg(c.accent)
                                        .rounded(px(2.0))
                                )
                        )
                        .child(
                            div()
                                .w(px(36.0))
                                .text_size(px(9.0))
                                .text_color(c.text_dim)
                                .child(format!("{}%", pct))
                        )
                );
            }

            children.push(list.into_any_element());

            // Add a label with result count
            children.push(
                h_flex()
                    .w_full()
                    .px_3().py_1()
                    .bg(c.panel_alt)
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(c.text_dim)
                            .child(format!("找到 {} 条结果", result_count))
                    )
                    .child(div().flex_grow())
                    .child(
                        div()
                            .text_size(px(9.0))
                            .text_color(c.text_dim)
                            .child("单击选中 • 点击下载")
                    )
                    .into_any_element()
            );
        }

        // === Bottom status bar ===
        let status_text = if state.results.is_empty() {
            if state.searching { "正在搜索...".to_string() } else { "就绪".to_string() }
        } else {
            format!("{} 条结果", state.results.len())
        };

        children.push(
            h_flex()
                .w_full()
                .px_3().py_1()
                .bg(c.control_bar_bg)
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(c.text_dim)
                        .child(status_text)
                )
                .child(div().flex_grow())
                .child({
                    let path_text = if self.current_track_path_for_download.is_empty() {
                        "先播放一首歌曲以自动保存歌词到其目录".to_string()
                    } else {
                        let path_display = self.current_track_path_for_download.clone();
                        let display = if path_display.len() > 60 {
                            format!("...{}", &path_display[path_display.len()-57..])
                        } else {
                            path_display
                        };
                        format!("保存到: {}", display)
                    };
                    div()
                        .text_size(px(9.0))
                        .text_color(c.text_dim)
                        .child(path_text)
                })
                .into_any_element()
        );

        v_flex()
            .size_full()
            .flex_grow()
            .bg(c.bg)
            .children(children)
    }

    fn render_playlist(
        &self,
        tracks: &[Track],
        current_idx: Option<usize>,
        layout_mode: LayoutMode,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        let c = &self.colours;
        let player = self.player.clone();
        let filter_text = self.playlist_filter_text.clone();
        let filter_mode = self.playlist_filter_mode;
        let drag_active = self.playlist_drag_active;
        let drag_to = self.playlist_drag_to;

        // Filter tracks based on search text and filter mode
        let filtered_indices: Vec<usize> = tracks.iter().enumerate().filter_map(|(i, track)| {
            let matches_mode = match filter_mode {
                PlaylistFilterMode::All => true,
                PlaylistFilterMode::Artist => !track.artist.is_empty(),
                PlaylistFilterMode::Album => !track.album.is_empty(),
                PlaylistFilterMode::Genre => !track.genre.is_empty(),
                PlaylistFilterMode::Favorites => track.is_favourite,
            };

            if !matches_mode {
                return None;
            }

            if filter_text.trim().is_empty() {
                return Some(i);
            }

            let kw = filter_text.to_lowercase();
            let matches_search = track.title.to_lowercase().contains(&kw)
                || track.artist.to_lowercase().contains(&kw)
                || track.album.to_lowercase().contains(&kw)
                || track.file_name.to_lowercase().contains(&kw)
                || track.genre.to_lowercase().contains(&kw);

            if matches_search { Some(i) } else { None }
        }).collect();

        let total_count = tracks.len();
        let filtered_count = filtered_indices.len();

        // Adjust row height and font size based on layout mode and config
        let cfg = crate::config::Config::load();
        let row_height = cfg.appearance.playlist_row_height as f32;
        let font_size = match layout_mode {
            LayoutMode::Big => 12.0,
            LayoutMode::Narrow => 11.0,
            LayoutMode::Small => 10.0,
        };
        let header_size = match layout_mode {
            LayoutMode::Big => 14.0,
            LayoutMode::Narrow => 13.0,
            LayoutMode::Small => 12.0,
        };

        // Header text based on filter state
        let header_text = if filter_text.is_empty() && filter_mode == PlaylistFilterMode::All {
            format!("播放列表 ({} 首)", total_count)
        } else {
            format!("筛选结果: {} / {}", filtered_count, total_count)
        };

        v_flex()
            .flex_grow()
            .h_full()
            .bg(c.bg)
            .child(
                v_flex()
                    .w_full()
                    .bg(c.control_bar_bg)
                    .child(
                        h_flex()
                            .items_center().justify_between()
                            .w_full()
                            .px_4().py_2()
                            .child(layout::txt(&header_text, header_size, c.text_title))
                    )
                    // Search and filter bar
                    .child(
                        h_flex()
                            .w_full()
                            .px_4().py_1()
                            .gap_2()
                            .bg(c.panel_alt)
                            .items_center()
                            .child(
                                // Search input area
                                div()
                                    .flex_grow()
                                    .h(px(24.0))
                                    .bg(c.bg)
                                    .border_1()
                                    .border_color(c.border)
                                    .rounded(px(4.0))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.0))
                                    .text_color(if filter_text.is_empty() { c.text_dim } else { c.text })
                                    .child(if filter_text.is_empty() {
                                        "� 搜索歌曲、艺术家或专辑...".to_string()
                                    } else {
                                        filter_text.clone()
                                    })
                            )
                            .child(
                                // Filter: All
                                Button::new("pl_filter_all")
                                    .label(if filter_mode == PlaylistFilterMode::All { "● 全部" } else { "全部" })
                                    .compact()
                                    .bg(if filter_mode == PlaylistFilterMode::All { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, _cx| {
                                        this.playlist_filter_mode = PlaylistFilterMode::All;
                                    }))
                            )
                            .child(
                                // Filter: Artists
                                Button::new("pl_filter_artist")
                                    .label(if filter_mode == PlaylistFilterMode::Artist { "● 艺术家" } else { "艺术家" })
                                    .compact()
                                    .bg(if filter_mode == PlaylistFilterMode::Artist { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, _cx| {
                                        this.playlist_filter_mode = PlaylistFilterMode::Artist;
                                    }))
                            )
                            .child(
                                // Filter: Album
                                Button::new("pl_filter_album")
                                    .label(if filter_mode == PlaylistFilterMode::Album { "● 专辑" } else { "专辑" })
                                    .compact()
                                    .bg(if filter_mode == PlaylistFilterMode::Album { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, _cx| {
                                        this.playlist_filter_mode = PlaylistFilterMode::Album;
                                    }))
                            )
                            .child(
                                // Filter: Favorites
                                Button::new("pl_filter_fav")
                                    .label(if filter_mode == PlaylistFilterMode::Favorites { "♥ 收藏" } else { "收藏" })
                                    .compact()
                                    .bg(if filter_mode == PlaylistFilterMode::Favorites { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                    .ghost()
                                    .on_click(cx.listener(|this, _, _window, _cx| {
                                        this.playlist_filter_mode = PlaylistFilterMode::Favorites;
                                    }))
                            )
                    )
            )
            // Column headers
            .child(
                h_flex()
                    .w_full()
                    .h(px(24.0))
                    .px_4().gap_3()
                    .bg(c.panel_alt)
                    .items_center()
                    .child(div().w(px(10.0))) // align with drag handle
                    .child(
                        Button::new("sort_title")
                            .label((if self.playlist_sort_field == PlaylistSortField::Title {
                                if self.playlist_sort_asc { "标题 ▲" } else { "标题 ▼" }
                            } else { "标题" }).to_string())
                            .compact().ghost()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .flex_grow()
                            .on_click(cx.listener(|this, _, _window, _cx| {
                                if this.playlist_sort_field == PlaylistSortField::Title {
                                    this.playlist_sort_asc = !this.playlist_sort_asc;
                                } else {
                                    this.playlist_sort_field = PlaylistSortField::Title;
                                    this.playlist_sort_asc = true;
                                }
                                let _ = this.player.playlist_mut().sort(crate::core::playlist::SortMode::Title, !this.playlist_sort_asc);
                            }))
                    )
                    .child(
                        if matches!(layout_mode, LayoutMode::Big) {
                            Button::new("sort_artist")
                                .label((if self.playlist_sort_field == PlaylistSortField::Artist {
                                    if self.playlist_sort_asc { "艺术家 ▲" } else { "艺术家 ▼" }
                                } else { "艺术家" }).to_string())
                                .compact().ghost()
                                .text_size(px(10.0))
                                .text_color(c.text_dim)
                                .w(px(80.0))
                                .on_click(cx.listener(|this, _, _window, _cx| {
                                    if this.playlist_sort_field == PlaylistSortField::Artist {
                                        this.playlist_sort_asc = !this.playlist_sort_asc;
                                    } else {
                                        this.playlist_sort_field = PlaylistSortField::Artist;
                                        this.playlist_sort_asc = true;
                                    }
                                    let _ = this.player.playlist_mut().sort(crate::core::playlist::SortMode::Artist, !this.playlist_sort_asc);
                                })).into_any_element()
                        } else {
                            div().into_any_element()
                        }
                    )
                    .child(
                        if matches!(layout_mode, LayoutMode::Big) {
                            Button::new("sort_album")
                                .label((if self.playlist_sort_field == PlaylistSortField::Album {
                                    if self.playlist_sort_asc { "专辑 ▲" } else { "专辑 ▼" }
                                } else { "专辑" }).to_string())
                                .compact().ghost()
                                .text_size(px(10.0))
                                .text_color(c.text_dim)
                                .w(px(80.0))
                                .on_click(cx.listener(|this, _, _window, _cx| {
                                    if this.playlist_sort_field == PlaylistSortField::Album {
                                        this.playlist_sort_asc = !this.playlist_sort_asc;
                                    } else {
                                        this.playlist_sort_field = PlaylistSortField::Album;
                                        this.playlist_sort_asc = true;
                                    }
                                    let _ = this.player.playlist_mut().sort(crate::core::playlist::SortMode::Album, !this.playlist_sort_asc);
                                })).into_any_element()
                        } else {
                            div().into_any_element()
                        }
                    )
                    .child(
                        Button::new("sort_dur")
                            .label((if self.playlist_sort_field == PlaylistSortField::Duration {
                                if self.playlist_sort_asc { "时长 ▲" } else { "时长 ▼" }
                            } else { "时长" }).to_string())
                            .compact().ghost()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .w(px(48.0))
                            .on_click(cx.listener(|this, _, _window, _cx| {
                                if this.playlist_sort_field == PlaylistSortField::Duration {
                                    this.playlist_sort_asc = !this.playlist_sort_asc;
                                } else {
                                    this.playlist_sort_field = PlaylistSortField::Duration;
                                    this.playlist_sort_asc = true;
                                }
                                let _ = this.player.playlist_mut().sort(crate::core::playlist::SortMode::Time, !this.playlist_sort_asc);
                            }))
                    )
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
                                let is_fav = track.is_favourite;
                                let ctx_player = player.clone();

                                // Highlight track as drop target when dragging
                                let is_drop_target = drag_active && drag_to == Some(i);
                                let row_bg = if is_drop_target {
                                    c.accent.opacity(0.3)
                                } else {
                                    bg
                                };

                                h_flex()
                                    .id(("track", i))
                                    .items_center()
                                    .w_full()
                                    .h(px(row_height))
                                    .px_4().gap_3()
                                    .bg(row_bg)
                                    .hover(|s| s.bg(if is_drop_target { c.accent.opacity(0.4) } else { c.playlist_item_selected }))
                                    .cursor(gpui::CursorStyle::PointingHand)
                                    // Drag handle (grip dots)
                                    .child(
                                        div()
                                            .w(px(10.0))
                                            .h(px(row_height - 4.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .cursor(gpui::CursorStyle::PointingHand)
                                            .text_color(c.text_dim)
                                            .text_size(px(10.0))
                                            .child("⠿")
                                            .on_mouse_down(gpui::MouseButton::Left, {
                                                let v = view.clone();
                                                let idx = i;
                                                move |_, _, cx| {
                                                    let _ = v.update(cx, |this, _cx| {
                                                        this.playlist_drag_active = true;
                                                        this.playlist_drag_from = Some(idx);
                                                        this.playlist_drag_to = Some(idx);
                                                    });
                                                }
                                            })
                                            .on_mouse_up(gpui::MouseButton::Left, {
                                                let v = view.clone();
                                                move |_, _, cx| {
                                                    let _ = v.update(cx, |this, _cx| {
                                                        if this.playlist_drag_active {
                                                            if let (Some(from), Some(to)) = (this.playlist_drag_from, this.playlist_drag_to) {
                                                                if from != to {
                                                                    let _ = this.player.playlist_mut().move_track(from, to);
                                                                }
                                                            }
                                                        }
                                                        this.playlist_drag_active = false;
                                                        this.playlist_drag_from = None;
                                                        this.playlist_drag_to = None;
                                                    });
                                                }
                                            })
                                    )
                                    .child(h_flex().flex_grow().child(layout::txt(&display_name, font_size, text_color)))
                                    .child(if is_fav { layout::txt("♥", font_size, c.accent) } else { layout::txt("", font_size, c.text_dim) })
                                    .child(layout::txt(&dur_str, font_size - 1.0, c.text_dim))
                                    .on_click({
                                        let p = player.clone();
                                        let idx = i;
                                        move |_, _, _| {
                                            // Play track at index (don't duplicate)
                                            let _ = p.play_at_index(idx);
                                        }
                                    })
                                    .on_mouse_move({
                                        let v = view.clone();
                                        let idx = i;
                                        move |_, _, cx| {
                                            let _ = v.update(cx, |this, _cx| {
                                                if this.playlist_drag_active && this.playlist_drag_to != Some(idx) {
                                                    this.playlist_drag_to = Some(idx);
                                                }
                                            });
                                        }
                                    })
                                    .context_menu(move |menu, _w, _cx| {
                                        // Pre-clone all state needed by inner closures
                                        let p_play = ctx_player.clone();
                                        let p_remove = ctx_player.clone();
                                        let p_fav = ctx_player.clone();
                                        let p_next = ctx_player.clone();
                                        let p_r1 = ctx_player.clone();
                                        let p_r2 = ctx_player.clone();
                                        let p_r3 = ctx_player.clone();
                                        let p_r4 = ctx_player.clone();
                                        let p_r5 = ctx_player.clone();
                                        let p_r0 = ctx_player.clone();
                                        let fp_loc = file_path.clone();
                                        let fp_copy = file_path.clone();
                                        let dn = display_name.clone();
                                        let fav = is_fav;
                                        let idx = i;

                                        menu.item(PopupMenuItem::new("播放").on_click(move |_, _, _| {
                                            let _ = p_play.play_at_index(idx);
                                        }))
                                        .item(PopupMenuItem::new("下一首播放").on_click(move |_, _, _| {
                                            p_next.push_next_track(idx);
                                            tracing::info!("[Playlist] Track #{} queued as next", idx);
                                        }))
                                        .separator()
                                        .item(PopupMenuItem::new("移除").on_click(move |_, _, _| {
                                            p_remove.playlist_mut().remove(idx);
                                            tracing::info!("[Playlist] Removed track #{}", idx);
                                        }))
                                        .item(PopupMenuItem::new(if fav { "取消收藏" } else { "收藏" }).on_click(move |_, _, _| {
                                            let new_state = p_fav.playlist_mut().toggle_favourite(idx);
                                            tracing::info!("[Playlist] Track #{} favourite: {}", idx, new_state);
                                        }))
                                        .separator()
                                        .item(PopupMenuItem::new("评级: ★☆☆☆☆").on_click(move |_, _, _| {
                                            p_r1.playlist_mut().set_rating(idx, 1);
                                        }))
                                        .item(PopupMenuItem::new("评级: ★★☆☆☆").on_click(move |_, _, _| {
                                            p_r2.playlist_mut().set_rating(idx, 2);
                                        }))
                                        .item(PopupMenuItem::new("评级: ★★★☆☆").on_click(move |_, _, _| {
                                            p_r3.playlist_mut().set_rating(idx, 3);
                                        }))
                                        .item(PopupMenuItem::new("评级: ★★★★☆").on_click(move |_, _, _| {
                                            p_r4.playlist_mut().set_rating(idx, 4);
                                        }))
                                        .item(PopupMenuItem::new("评级: ★★★★★").on_click(move |_, _, _| {
                                            p_r5.playlist_mut().set_rating(idx, 5);
                                        }))
                                        .item(PopupMenuItem::new("清除评级").on_click(move |_, _, _| {
                                            p_r0.playlist_mut().set_rating(idx, 0);
                                        }))
                                        .separator()
                                        .item(PopupMenuItem::new("打开文件位置").on_click({
                                            let fp_loc2 = fp_loc.clone();
                                            let ctx_player2 = ctx_player.clone();
                                            let idx2 = idx;
                                            let dn2 = dn.clone();
                                            let fav2 = fav;
                                            move |_, _, _| {
                                            let path = std::path::Path::new(&fp_loc2);
                                            if let Some(_parent) = path.parent() {
                                                #[cfg(windows)]
                                                {
                                                    let _ = std::process::Command::new("explorer")
                                                        .arg("/select,")
                                                        .arg(&fp_loc2)
                                                        .spawn();
                                                }
                                                #[cfg(target_os = "macos")]
                                                {
                                                    let _ = std::process::Command::new("open")
                                                        .arg("-R")
                                                        .arg(&fp_loc2)
                                                        .spawn();
                                                }
                                                #[cfg(all(not(windows), not(target_os = "macos")))]
                                                {
                                                    if let Some(p) = parent.to_str() {
                                                        let _ = std::process::Command::new("xdg-open")
                                                            .arg(p)
                                                            .spawn();
                                                    }
                                                }
                                            }
                                            tracing::info!("[Playlist] Opening file location: {}", fp_loc2);
                                        }}))
                                        .item(PopupMenuItem::new("复制路径").on_click(move |_, _, _| {
                                            tracing::info!("[Playlist] Copy path: {}", fp_copy);
                                        }))
                                        .item(PopupMenuItem::new("属性").on_click({
                                        let ctx_player2 = ctx_player.clone();
                                        let fp_loc2 = fp_loc.clone();
                                        let idx2 = idx;
                                        let dn2 = dn.clone();
                                        let fav2 = fav;
                                        move |_, _, _| {
                                        let file_size = std::path::Path::new(&fp_loc2).metadata().ok()
                                            .map(|m| {
                                                let bytes = m.len();
                                                if bytes > 1024 * 1024 {
                                                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                                                } else if bytes > 1024 {
                                                    format!("{:.0} KB", bytes as f64 / 1024.0)
                                                } else {
                                                    format!("{} B", bytes)
                                                }
                                            }).unwrap_or_else(|| "--".to_string());
                                        let mod_time = std::path::Path::new(&fp_loc2).metadata().ok()
                                            .and_then(|m| m.modified().ok())
                                            .map(|t| {
                                                let secs = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                                // Convert to a readable format (year-month-day)
                                                let days = secs / 86400;
                                                let year = 1970 + (days / 365) as u32;
                                                let month = ((days % 365) / 30 + 1) as u32;
                                                let day = ((days % 365) % 30 + 1) as u32;
                                                format!("{}-{:02}-{:02}", year, month, day)
                                            }).unwrap_or_else(|| "--".to_string());
                                        let info = format!(
                                            "标题: {}\n艺术家: {}\n专辑: {}\n文件名: {}\n路径: {}\n时长: {}\n收藏: {}\n类型: {}\n比特率: {}\n采样率: {}\n\n--- 高级信息 ---\n文件大小: {}\n修改日期: {}",
                                            dn2, ctx_player2.playlist().get(idx2).map(|t| &t.artist).unwrap_or(&"".to_string()),
                                            ctx_player2.playlist().get(idx2).map(|t| &t.album).unwrap_or(&"".to_string()),
                                            ctx_player2.playlist().get(idx2).map(|t| &t.file_name).unwrap_or(&"--".to_string()),
                                            fp_loc2,
                                            ctx_player2.playlist().get(idx2).map(|t| { let d = t.duration; format!("{:02}:{:02}", d.as_secs()/60, d.as_secs()%60) }).unwrap_or_default(),
                                            if fav2 { "是" } else { "否" },
                                            ctx_player2.playlist().get(idx2).map(|t| &t.file_type).unwrap_or(&"--".to_string()),
                                            "--",
                                            "--",
                                            file_size,
                                            mod_time,
                                        );
                                        let _ = rfd::MessageDialog::new()
                                            .set_title("歌曲属性")
                                            .set_description(&info)
                                            .show();
                                    }}))
                                    })
                            }))
                    )
            )
    }

    /// Render the file browser panel for browsing the local filesystem.
    fn render_file_browser(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let player = self.player.clone();
        v_flex()
            .flex_grow()
            .h_full()
            .bg(c.bg)
            .child(
                h_flex().items_center().justify_between().w_full().px_4().py_2().bg(c.control_bar_bg)
                    .child(layout::txt("文件浏览器", 14.0, c.text_title))
                    .child(
                        Button::new("fb_open_folder").label("打开文件夹").compact().ghost()
                            .on_click(cx.listener(move |this, _, _w, _cx| {
                                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                    let dir = folder.to_string_lossy().to_string();
                                    if let Ok(entries) = crate::media::scan_directory(&dir, true, None) {
                                        let mut pl = this.player.playlist_mut();
                                        let first_idx = pl.len();
                                        for e in &entries {
                                            pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                        }
                                        if !entries.is_empty() {
                                            let _ = this.player.play_at_index(first_idx);
                                        }
                                        tracing::info!("[FileBrowser] 打开文件夹: {} ({} 首)", dir, entries.len());
                                    }
                                }
                            }))
                    )
            )
            .child(
                v_flex().flex_grow().w_full().p_4()
                    .child(layout::txt("点击'打开文件夹'按钮浏览音乐文件", 12.0, c.text_dim))
                    .child(layout::txt("支持格式: mp3, flac, wav, ogg, aac, m4a, wma, ape 等", 11.0, c.text_dim))
            )
    }

    /// Render the media library panel with category browsing.
    fn render_media_lib_panel(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state_entity = cx.entity().clone();
        let lib = crate::media::MediaLib::load();
        let total_tracks = lib.entries.len();
        let total_dur_secs: u64 = lib.entries.iter().map(|e| e.duration_secs).sum();
        let total_dur = format!(
            "{}:{:02}:{:02}",
            total_dur_secs / 3600,
            (total_dur_secs % 3600) / 60,
            total_dur_secs % 60
        );
        let cat = self.media_lib_category;
        let sel = self.media_lib_selected.clone();

        let (sidebar_items, list_items): (Vec<String>, Vec<ViewEntry>) = match cat {
            MediaLibCategory::AllTracks => (
                Vec::new(),
                lib.entries.iter().map(ViewEntry::from).collect(),
            ),
            MediaLibCategory::Artists => {
                let artists = lib.artists();
                let items = sel.as_ref()
                    .map(|n| lib.by_artist(Some(n)).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (artists, items)
            }
            MediaLibCategory::Albums => {
                let albums = lib.albums();
                let items = sel.as_ref()
                    .map(|n| lib.by_album(Some(n)).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (albums, items)
            }
            MediaLibCategory::Genres => {
                let genres = lib.genres();
                let items = sel.as_ref()
                    .map(|n| lib.by_genre(n).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (genres, items)
            }
            MediaLibCategory::Years => {
                let years: Vec<String> = lib.years().iter().map(|y| y.to_string()).collect();
                let items = sel.as_ref()
                    .and_then(|y_str| y_str.parse::<u32>().ok())
                    .map(|year| lib.by_year(year).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (years, items)
            }
            MediaLibCategory::FileTypes => {
                let types = lib.file_types();
                let items = sel.as_ref()
                    .map(|ext| lib.by_file_type(ext).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (types, items)
            }
            MediaLibCategory::Bitrates => {
                let bitrates = lib.bitrates();
                let items = sel.as_ref()
                    .and_then(|b| b.parse::<u32>().ok())
                    .map(|b| lib.by_bitrate(b).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (bitrates.iter().map(|b| format!("{} kbps", b)).collect::<Vec<_>>(), items)
            }
            MediaLibCategory::Recent => {
                let recent = lib.recent(50);
                let items: Vec<String> = recent.iter().map(|e| e.file_path.clone()).collect();
                (items, Vec::new())
            }
            MediaLibCategory::Rating => {
                let ratings = lib.ratings();
                let items = sel.as_ref()
                    .and_then(|r| r.parse::<u32>().ok())
                    .map(|r| lib.by_rating(r).iter().map(|e| ViewEntry::from(*e)).collect())
                    .unwrap_or_default();
                (ratings.iter().map(|r| format!("{} 星", r)).collect::<Vec<_>>(), items)
            }
        };

        let mk_btn = |label: &'static str, id: &'static str, target: MediaLibCategory| {
            let is_active = cat == target;
            let bg = if is_active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
            Button::new(id).label(label).compact().ghost().bg(bg)
                .on_click(cx.listener(move |this, _, _w, _cx| {
                    this.media_lib_category = target;
                    this.media_lib_selected = None;
                }))
        };

        let header_text = format!("媒体库 ({} 首 | {})", total_tracks, total_dur);
        let show_sidebar = !matches!(cat, MediaLibCategory::AllTracks) && !sidebar_items.is_empty();

        let sidebar_el = if show_sidebar {
            let items: Vec<String> = sidebar_items.clone();
            div().w(px(150.0)).h_full().bg(c.panel_alt).border_r(px(1.0)).border_color(c.border)
                .child(v_flex().w_full().h_full().py_1()
                    .children(items.into_iter().map(|item| {
                        let is_active = sel.as_ref().map(|s| s.as_str()) == Some(item.as_str());
                        let bg = if is_active { c.accent.opacity(0.2) } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
                        div().px_3().py_1().bg(bg).text_size(px(11.0))
                            .text_color(if is_active { c.accent } else { c.text })
                            .cursor(gpui::CursorStyle::PointingHand)
                            .hover(|s| s.bg(c.playlist_item_selected))
                            .child(if item.is_empty() { "(未知)".to_string() } else { item.clone() })
                             .on_mouse_up(gpui::MouseButton::Left, {
                                 let item2 = item.clone();
                                 cx.listener(move |this, _, _w, _cx| {
                                     this.media_lib_selected = Some(item2.clone());
                                 })
                             })
                    }))
                )
        } else {
            div().w(px(0.0))
        };

        v_flex().flex_grow().h_full().bg(c.bg)
            .child(
                h_flex().items_center().justify_between().w_full().px_4().py_2().bg(c.control_bar_bg)
                    .child(layout::txt(&header_text, 13.0, c.text_title))
            )
            .child(
                h_flex().w_full().px_4().py_1().gap_2().bg(c.panel_alt).border_b(px(1.0)).border_color(c.border).items_center()
                    .child(mk_btn("全部曲目", "ml_cat_all", MediaLibCategory::AllTracks))
                    .child(mk_btn("艺术家", "ml_cat_artist", MediaLibCategory::Artists))
                    .child(mk_btn("专辑", "ml_cat_album", MediaLibCategory::Albums))
                    .child(mk_btn("流派", "ml_cat_genre", MediaLibCategory::Genres))
                    .child(mk_btn("年份", "ml_cat_year", MediaLibCategory::Years))
                    .child(mk_btn("文件类型", "ml_cat_ftype", MediaLibCategory::FileTypes))
                    .child(mk_btn("比特率", "ml_cat_bitrate", MediaLibCategory::Bitrates))
                    .child(mk_btn("最近播放", "ml_cat_recent", MediaLibCategory::Recent))
                    .child(mk_btn("评级", "ml_cat_rating", MediaLibCategory::Rating))
                    .child(div().flex_grow())
                    .child(
                        Button::new("ml_refresh_btn").label("刷新").compact().ghost()
                            .on_click(cx.listener(move |_this, _, _w, _cx| {
                                let _ = state_entity;
                                std::thread::spawn(|| {
                                    let cfg = crate::config::Config::load();
                                    let mut lib = crate::media::MediaLib::load();
                                    for dir in &cfg.media_lib.media_dirs {
                                        if let Ok(entries) = crate::media::scan_directory(dir, true, None) {
                                            for e in entries { lib.upsert(e); }
                                        }
                                    }
                                    let _ = lib.save();
                                    tracing::info!("[MediaLib] 扫描完成: {} 首", lib.entries.len());
                                });
                            }))
                    )
            )
            .child(
                h_flex().flex_grow().w_full()
                    .child(sidebar_el)
                    .child(
                        v_flex().flex_grow().h_full().bg(c.bg)
                            .children(list_items.iter().enumerate().map(|(i, item)| {
                                let is_even = i % 2 == 0;
                                let bg = if is_even { c.playlist_item } else { c.playlist_item_hover };
                                let title = item.title.clone();
                                let path = item.file_path.clone();
                                let dur_str = format!("{:02}:{:02}", item.duration / 60, item.duration % 60);
                                h_flex().w_full().h(px(26.0)).px_4().gap_3().bg(bg)
                                    .hover(|s| s.bg(c.playlist_item_selected))
                                    .items_center().cursor(gpui::CursorStyle::PointingHand)
                                    .child(layout::txt(&title, 11.0, c.text))
                                    .child(div().flex_grow())
                                    .child(layout::txt(&item.artist, 10.0, c.text_dim))
                                    .child(layout::txt(&dur_str, 10.0, c.text_dim))
                                    .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                                        tracing::info!("[MediaLib] 播放: {}", path);
                                    })
                            }))
                    )
            )
    }

    /// Render the 10-band equalizer panel.
    fn render_equalizer_panel(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        const BAND_LABELS: &[&str] = &[
            "32Hz", "64Hz", "125Hz", "250Hz", "500Hz",
            "1kHz", "2kHz", "4kHz", "8kHz", "16kHz",
        ];

        // Preset definition: name -> gains
        let presets: Vec<(&str, &[f32; 10])> = vec![
            ("平坦", &[0.0_f32; 10]),
            ("流行", &[3.0, 2.0, 0.0, -1.0, -2.0, 0.0, 2.0, 3.0, 3.0, 2.0]),
            ("摇滚", &[4.0, 3.0, -1.0, -2.0, 1.0, 3.0, 4.0, 5.0, 5.0, 4.0]),
            ("古典", &[3.0, 2.0, 0.0, 0.0, -1.0, -1.0, 0.0, 2.0, 3.0, 4.0]),
            ("爵士", &[3.0, 2.0, 0.0, 2.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0]),
            ("电子", &[4.0, 3.0, 1.0, 0.0, -2.0, 0.0, 2.0, 4.0, 5.0, 4.0]),
            ("低音增强", &[6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
            ("人声增强", &[-2.0, -1.0, 0.0, 0.0, 3.0, 4.0, 3.0, 1.0, -1.0, -2.0]),
        ];

        // Header with enable toggle
        let enabled = self.eq_enabled;
        let preset_name = self.eq_preset_name.clone();

        let header = h_flex()
            .w_full()
            .px_4().py_2()
            .bg(c.control_bar_bg)
            .items_center()
            .gap_4()
            .child(layout::txt("均衡器", 14.0, c.text_title))
            .child(
                Button::new("eq_enable")
                    .label(if enabled { "� 已启用" } else { "🔇 已禁用" })
                    .compact()
                    .bg(if enabled { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                    .ghost()
                    .on_click(cx.listener(move |this, _, _w, _cx| {
                        this.eq_enabled = !this.eq_enabled;
                        tracing::info!("[EQ] enabled={}", this.eq_enabled);
                    }))
            )
            .child(div().flex_grow())
            .child(layout::txt(&format!("预设: {}", preset_name), 11.0, c.text_dim))
            .child(
                Button::new("eq_save_preset").label("保存预设").compact().ghost()
                    .on_click(cx.listener(move |this, _, _w, _cx| {
                        let gains: Vec<String> = this.eq_sliders.iter().map(|s| {
                            format!("{:.1}", s.read(_cx).value())
                        }).collect();
                        this.eq_preset_name = "自定义".to_string();
                        tracing::info!("[EQ] 保存自定义预设: gains={:?}", gains);
                    }))
            );

        // Preset list (scrollable)
        let preset_buttons: Vec<String> = presets.iter().map(|(n, _)| n.to_string()).collect();
        let presets_el = h_flex()
            .w_full()
            .px_4().py_2()
            .bg(c.panel_alt)
            .border_b(px(1.0))
            .border_color(c.border)
            .items_center()
            .gap_2()
            .children(preset_buttons.iter().enumerate().map(|(pi, name)| {
                let is_active = preset_name == *name;
                let n = name.clone();
                Button::new(("eq_preset", pi as u64))
                    .label(n.clone())
                    .compact()
                    .bg(if is_active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                    .ghost()
                    .on_click(cx.listener(move |this, _, _w, _cx| {
                        this.eq_preset_name = n.clone();
                    }))
            }));

        // EQ band sliders
        let sliders_row = h_flex()
            .flex_1()
            .w_full()
            .px_4().py_4()
            .gap_3()
            .items_end()
            .children(self.eq_sliders.iter().enumerate().map(|(i, slider)| {
                let label = BAND_LABELS[i.min(BAND_LABELS.len() - 1)];
                div()
                    .w(px(48.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    // Value display
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .child(format!("+12"))
                    )
                    // Vertical slider
                    .child(
                        div()
                            .w(px(12.0))
                            .h(px(150.0))
                            .bg(c.progress_track)
                            .border(px(1.0))
                            .border_color(c.border)
                            .child(
                                Slider::new(slider).vertical()
                            )
                    )
                    // Frequency label
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text)
                            .child(label)
                    )
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text_dim)
                            .child(format!("-12"))
                    )
            }));

        v_flex()
            .flex_1()
            .h_full()
            .bg(c.bg)
            .child(header)
            .child(presets_el)
            // EQ curve visualization
            .child(
                div()
                    .w_full()
                    .h(px(60.0))
                    .px_4().py_2()
                    .bg(c.panel_alt)
                    .child(
                        h_flex().w_full().h_full().items_end().gap_1()
                            .children(self.eq_sliders.iter().map(|slider| {
                                let val = 0.0_f32; // placeholder - would read slider value
                                let h = ((val + 12.0) / 24.0 * 50.0).max(2.0);
                                div().w(px(20.0)).h(px(h)).bg(c.accent).rounded(px(1.0))
                            }))
                    )
            )
            .child(sliders_row)
    }

    /// Render a spectrum visualization strip.
    fn render_spectrum_strip(
        &self,
        raw_spec: &[f32],
        raw_peaks: &[f32],
        cols: usize,
        c: &UiColors,
    ) -> impl IntoElement {
        let bar_spacing = 2.0;
        let bar_count = cols.min(64);
        let bar_w = 4.0;
        let strip_h = 36.0;
        let max_h = strip_h - 4.0;

        // sample spectrum data down to bar_count
        let bars: Vec<f32> = if raw_spec.len() >= bar_count {
            let step = raw_spec.len() / bar_count;
            (0..bar_count)
                .map(|i| {
                    let idx = (i * step).min(raw_spec.len() - 1);
                    (raw_spec[idx] * max_h).min(max_h).max(1.0)
                })
                .collect()
        } else {
            raw_spec
                .iter()
                .map(|v| (v * max_h).min(max_h).max(1.0))
                .chain(std::iter::repeat(1.0).take(bar_count.saturating_sub(raw_spec.len())))
                .take(bar_count)
                .collect()
        };

        let peak_vals: Vec<f32> = if raw_peaks.len() >= bar_count {
            let step = raw_peaks.len() / bar_count;
            (0..bar_count)
                .map(|i| {
                    let idx = (i * step).min(raw_peaks.len() - 1);
                    (raw_peaks[idx] * max_h).min(max_h)
                })
                .collect()
        } else {
            vec![0.0; bar_count]
        };

        h_flex()
            .w_full()
            .h(px(strip_h))
            .px_4()
            .items_end()
            .gap(px(bar_spacing))
            .children(bars.iter().enumerate().map(|(i, &bar_h)| {
                let peak_h = peak_vals[i];
                let show_peak = peak_h > bar_h + 2.0;
                v_flex()
                    .items_center()
                    .h(px(max_h))
                    .w(px(bar_w))
                    .child(if show_peak {
                        div()
                            .w(px(3.0))
                            .h(px(2.0))
                            .bg(c.accent)
                            .into_any_element()
                    } else {
                        div().w(px(3.0)).h(px(0.0)).into_any_element()
                    })
                    .child(
                        div()
                            .w_full()
                            .h(px(bar_h))
                            .bg(c.accent)
                            .rounded(px(1.0))
                    )
            }))
    }
}

struct ViewEntry {
    file_path: String,
    title: String,
    artist: String,
    album: String,
    duration: u64,
}

impl<'a> From<&'a crate::media::LibEntry> for ViewEntry {
    fn from(e: &'a crate::media::LibEntry) -> Self {
        Self {
            file_path: e.file_path.clone(),
            title: e.title.clone(),
            artist: e.artist.clone(),
            album: e.album.clone(),
            duration: e.duration_secs,
        }
    }
}
