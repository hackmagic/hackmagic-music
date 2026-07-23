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
use std::path::PathBuf;
use std::collections::VecDeque;
use std::hash::{Hash, Hasher};
use lru::LruCache;
use async_channel::unbounded;
use gpui::*;
use gpui_component::scroll::ScrollableElement;
use gpui_component::{h_flex, v_flex, IconName, Root};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::slider::{Slider, SliderState, SliderValue};
use gpui_component::input::{Input, InputState};
use i18n::{Locale, Tr};
use theme::{UiColors, LEFT_PANEL_WIDTH};
use crate::config::Config;
use crate::core::engine_trait::EngineType;
use crate::core::player::Player;
use crate::core::playlist::Track;
use responsive::{LayoutMode, ResponsiveState};

static ACTIVE_PANEL: AtomicU8 = AtomicU8::new(0);
static MINI_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static STATUSBAR_VISIBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static MENUBAR_VISIBLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
static MEDIA_LIB_SCANNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static RPC_SERVER_RUNNING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SLEEP_TIMER_ACTIVE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static ALWAYS_ON_TOP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static DESKTOP_LYRICS_WINDOW_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn runtime() -> &'static tokio::runtime::Runtime {
    use std::sync::OnceLock;
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime"))
}

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
        window_min_size: Some(gpui::Size { width: px(480.0), height: px(360.0) }),
        ..Default::default()
    }, |window, cx| {
        let url_state = cx.new(|c| InputState::new(window, c));
        let search_input = cx.new(|c| InputState::new(window, c).placeholder("搜索媒体库..."));
        let editor_title_input = cx.new(|c| InputState::new(window, c).placeholder("标题"));
        let editor_artist_input = cx.new(|c| InputState::new(window, c).placeholder("艺术家"));
        let editor_album_input = cx.new(|c| InputState::new(window, c).placeholder("专辑"));
        let editor_genre_input = cx.new(|c| InputState::new(window, c).placeholder("流派"));
        let editor_year_input = cx.new(|c| InputState::new(window, c).placeholder("年份"));
        let editor_track_num_input = cx.new(|c| InputState::new(window, c).placeholder("曲目号"));
        let editor_rating_input = cx.new(|c| InputState::new(window, c).placeholder("评分 (0-5)"));
        let content = cx.new(|cx| MusicPlayer::new(cx, url_state, search_input,
            editor_title_input, editor_artist_input, editor_album_input,
            editor_genre_input, editor_year_input, editor_track_num_input,
            editor_rating_input));
        cx.new(|c| {
            Root::new(content, window, c)
        })
    });
}

/// Modal dialogs shown as an overlay on top of the main window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModalKind {
    About,
    SongInfo,
    FormatConvert,
    TrackEditor,
}

/// Copy text to the system clipboard using platform utilities (no extra deps).
fn copy_text_to_clipboard(text: &str) {
    #[cfg(windows)]
    {
        use std::process::{Command, Stdio};
        let _ = Command::new("cmd")
            .args(["/c", "clip"])
            .stdin(Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    let _ = stdin.write_all(text.as_bytes());
                }
                child.wait()
            });
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("pbcopy").arg(text).status();
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xclip").arg("-selection").arg("clipboard").arg(text).status();
    }
}

/// Render a `Lyrics` object back into an LRC string so it can be saved
/// to disk or embedded into a tag. Mirrors the `[mm:ss.xx]text` format.
fn lyrics_to_lrc_string(lyrics: &crate::lyric::parser::Lyrics) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    if !lyrics.title.is_empty() {
        let _ = writeln!(out, "[ti:{}]", lyrics.title);
    }
    if !lyrics.artist.is_empty() {
        let _ = writeln!(out, "[ar:{}]", lyrics.artist);
    }
    if !lyrics.album.is_empty() {
        let _ = writeln!(out, "[al:{}]", lyrics.album);
    }
    if lyrics.offset_ms != 0 {
        let _ = writeln!(out, "[offset:{}]", lyrics.offset_ms);
    }
    for line in &lyrics.lines {
        let total_ms = line.time_ms;
        let mins = total_ms / 60_000;
        let secs = (total_ms % 60_000) / 1_000;
        let cs = (total_ms % 1_000) / 10;
        let _ = write!(out, "[{:02}:{:02}.{:02}]", mins, secs, cs);
        out.push_str(&line.text);
        out.push('\n');
    }
    out
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
    pending_track_path: Option<String>,
    editor_state: lyric_editor::LyricEditorState,
    show_editor: bool,
    download_state: lyric_download::LyricDownloadState,
    pending_download_rx: Option<std::sync::mpsc::Receiver<lyric_download::DownloadEvent>>,
    pending_download_tx: Option<std::sync::mpsc::Sender<lyric_download::DownloadEvent>>,
    current_track_path_for_download: String,
    /// Decoded album-art image path, refreshed on track change (None = no cover).
    album_art: Option<PathBuf>,
    /// Playlist search/filter state
    playlist_filter_text: String,
    playlist_filter_mode: PlaylistFilterMode,
    playlist_sort_field: PlaylistSortField,
    playlist_sort_asc: bool,
    playlist_view_mode: PlaylistViewMode,
    /// Playlist drag-drop reorder state
    playlist_drag_from: Option<usize>,
    playlist_drag_to: Option<usize>,
    playlist_drag_active: bool,
    /// Multi-selection for playlist delete-from-list / delete-from-disk.
    /// Empty = no selection. Maintained by render_playlist click handlers.
    playlist_selected: std::collections::HashSet<usize>,
    /// Recently played track file paths (most recent first), capped
    recent_tracks: VecDeque<String>,
    /// Media library cached data (loaded once per session, not on every render).
    media_lib_cache: Option<crate::media::MediaLib>,
    media_lib_category: MediaLibCategory,
    media_lib_selected: Option<String>,
    media_lib_search: String,
    /// Cached sidebar items (artists/albums/genres etc.) — recomputed only on category change.
    media_lib_sidebar_cache: Vec<String>,
    /// Cached list items for current selection — recomputed only on category/selection change.
    media_lib_list_cache: Vec<ViewEntry>,
    /// Tracks which (category, selected) the cache was built for.
    media_lib_cache_key: (MediaLibCategory, Option<String>),
    /// Cached total duration in seconds (recomputed when lib changes).
    media_lib_total_dur: u64,
    /// Equalizer state
    eq_enabled: bool,
    eq_sliders: Vec<Entity<SliderState>>,
    eq_preset_name: String,
    /// Current playback error message (shown in status bar / overlay)
    playback_error: Option<String>,
    settings_tab: dialogs::SettingsTab,
    /// Open URL dialog state
    url_dialog_open: bool,
    url_state: Entity<InputState>,
    /// Search panel input state (created in `run`, passed to `new`)
    search_input: Entity<InputState>,
    /// Lyrics offset in milliseconds (positive = shifted later)
    lyric_offset_ms: i64,
    /// Whether to show the lyric translation line
    lyric_show_translation: bool,
    /// Desktop lyrics overlay toggle
    desktop_lyrics_open: bool,
    /// Current modal dialog (None = no dialog open)
    modal: Option<ModalKind>,
    /// Track editor state (used when modal == Some(ModalKind::TrackEditor))
    editor_track_idx: Option<usize>,
    /// Scroll handle for the playlist list (drives virtual scrolling via uniform_list).
    playlist_scroll_handle: UniformListScrollHandle,
    /// Cached Config to avoid file I/O every frame in render_playlist.
    config_cache: crate::config::Config,
    /// LRU cache for extracted album art (track_path → temp_cover_path).
    cover_cache: std::sync::Mutex<LruCache<String, PathBuf>>,
    editor_title_input: Entity<InputState>,
    editor_artist_input: Entity<InputState>,
    editor_album_input: Entity<InputState>,
    editor_genre_input: Entity<InputState>,
    editor_year_input: Entity<InputState>,
    editor_track_num_input: Entity<InputState>,
    editor_rating_input: Entity<InputState>,
}

/// Filter mode for the playlist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistFilterMode {
    All,
    Artist,
    Album,
    Genre,
    Favorites,
    Recent,
}

/// Playlist column sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistSortField {
    Title,
    Artist,
    Album,
    Duration,
}

/// View mode for the playlist dock
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlaylistViewMode {
    Detail,
    Compact,
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

/// Run a blocking (Win32 modal) operation — notably `rfd` file dialogs — on a
/// separate OS thread, then deliver the result back to the GPUI main thread and
/// apply it via `apply`.
///
/// This is required because `rfd` drives its own nested Win32 message loop.
/// Invoking it synchronously inside a GPUI event handler re-enters the message
/// pump while `App` is already borrowed, which panics ("already borrowed")
/// inside the non-unwinding window procedure and aborts the process.
///
/// Two variants exist because `Context<MusicPlayer>` does not `DerefMut` to
/// `App`: in-view handlers receive `&mut Context<MusicPlayer>` (use
/// `run_blocking_dialog`), while menu callbacks receive `&mut App` (use
/// `run_blocking_dialog_app` and pass a `WeakEntity`).
fn run_blocking_dialog_app<T, F, A>(
    cx: &mut App,
    weak: &gpui::WeakEntity<MusicPlayer>,
    build: F,
    apply: A,
) where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    A: FnOnce(T, &mut MusicPlayer, &mut Context<MusicPlayer>) + 'static,
{
    let weak_clone = weak.clone();
    let (tx, rx) = unbounded::<T>();
    runtime().spawn_blocking(move || {
        let _ = tx.send(build());
    });
    cx.spawn(async move |cx| {
        // Wait for the background thread result. The `.await` yields back to
        // the GPUI executor, so by the time `update` runs we're outside any
        // outer `App::borrow_mut()` frame (e.g. render / event dispatch).
        // Without this yield, calling `weak.update` from a synchronous spawn
        // re-enters the GPUI message pump while `App` is already borrowed,
        // which panics ("RefCell already borrowed") inside the non-unwinding
        // window procedure and aborts the process.
        if let Ok(result) = rx.recv().await {
            // Yield once to ensure we don't run inside the same borrow frame.
            cx.background_executor()
                .timer(std::time::Duration::from_millis(0))
                .await;
            let _ = weak_clone.update(cx, |this, cx| apply(result, this, cx));
        }
    })
    .detach();
}

/// Typed-context variant — `Context::spawn` already provides the `WeakEntity`.
fn run_blocking_dialog<T, F, A>(
    cx: &mut Context<MusicPlayer>,
    build: F,
    apply: A,
) where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
    A: FnOnce(T, &mut MusicPlayer, &mut Context<MusicPlayer>) + 'static,
{
    let (tx, rx) = unbounded::<T>();
    runtime().spawn_blocking(move || {
        let _ = tx.send(build());
    });
    cx.spawn(async move |this, cx| {
        // Same yield-once trick as `run_blocking_dialog_app`: ensure we don't
        // run inside the same `App` borrow frame that triggered the dialog.
        if let Ok(result) = rx.recv().await {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(0))
                .await;
            let _ = this.update(cx, |this, cx| apply(result, this, cx));
        }
    })
    .detach();
}

impl MusicPlayer {
    fn new(
        cx: &mut Context<Self>,
        url_state: Entity<InputState>,
        search_input: Entity<InputState>,
        editor_title_input: Entity<InputState>,
        editor_artist_input: Entity<InputState>,
        editor_album_input: Entity<InputState>,
        editor_genre_input: Entity<InputState>,
        editor_year_input: Entity<InputState>,
        editor_track_num_input: Entity<InputState>,
        editor_rating_input: Entity<InputState>,
    ) -> Self {
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
            // 只在媒体库为空时自动扫描，避免每次启动都扫描大量文件导致卡顿
            let existing = crate::media::MediaLib::load();
            if existing.entries.is_empty() {
                // Capture a weak handle to self so the background scan can
                // refresh `media_lib_cache` and trigger a repaint when done.
                let _weak = cx.weak_entity();
                runtime().spawn_blocking(move || {
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
                    // Cache refresh deferred — cannot use cx in std::thread
                });
            } else {
                tracing::info!("[MediaLib] 跳过自动扫描（已有 {} 条记录）", existing.entries.len());
            }
        }

        // ── 30fps repaint loop: drives progress bar, spectrum, and lyrics
        //    updates during playback. Player state is polled from render() now,
        //    not here; this timer only processes deferred I/O and calls notify.
        //    The loading guard skips dispatches during BASS blocking operations.
        //    Kopuz reference: audio engine runs independently, UI reads state
        //    reactively via render(), never polls from background tasks.
        let player_loading = std::sync::Arc::clone(&player.loading);
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(33))
                    .await;
                // Skip updates while engine is performing a blocking operation (e.g. BASS open)
                if player_loading.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                // Process deferred track-change I/O (lyrics + cover) on background thread
                let pending_info = this.update(cx, |this, _| {
                    let path = this.pending_track_path.take();
                    let show_trans = this.lyric_show_translation;
                    let pos_ms = ((this.position * 1000.0) as i64 + this.lyric_offset_ms).max(0) as u64;
                    (path, show_trans, pos_ms)
                }).ok();
                if let Some((Some(ref p), show_trans, pos_ms)) = pending_info {
                    if !p.is_empty() {
                        let lyrics = Self::load_lyrics_raw(p, show_trans, pos_ms);
                        let cover = Self::do_extract_cover(p);
                        let _ = this.update(cx, |this, _| {
                            if let Some(l) = lyrics {
                                this.lyric_state.update(Some(l), pos_ms);
                            }
                            this.album_art = cover;
                        });
                    }
                }
                // Yield then notify render — render() now reads player state directly
                cx.background_executor()
                    .timer(std::time::Duration::ZERO)
                    .await;
                if this.update(cx, |_this, cx| cx.notify()).is_err() {
                    tracing::error!("[Timer] notify failed");
                }
            }
        })
        .detach();

        // Build the 10 EQ band sliders and wire each to the audio engine.
        // Initial values come from EqConfig (restored below via default_value).
        let eq_sliders: Vec<Entity<SliderState>> = (0..10)
            .map(|i| {
                let g = cfg.eq.gains.get(i).copied().unwrap_or(0).clamp(-12, 12) as f32;
                cx.new(|_cx| {
                    SliderState::new()
                        .min(-12.0)
                        .max(12.0)
                        .step(0.5)
                        .default_value(g)
                })
            })
            .collect();
        for (i, slider) in eq_sliders.iter().enumerate() {
            let _ = cx.subscribe(slider, move |this, entity, _window, cx| {
                if !this.eq_enabled {
                    return;
                }
                if let SliderValue::Single(v) = entity.read(cx).value() {
                    let _ = this.player.eq_set(i, v as i32);
                }
            });
        }

        // Restore EQ state from config: apply gains to player + enable engine.
        {
            let eq_cfg = &cfg.eq;
            let _ = player.eq_enable(eq_cfg.enabled);
            if eq_cfg.enabled {
                for (i, &g) in eq_cfg.gains.iter().enumerate() {
                    let _ = player.eq_set(i, g);
                }
            } else {
                for i in 0..10 {
                    let _ = player.eq_set(i, 0);
                }
            }
        }

        // Init hotkey message window so CLI commands can forward to us
        crate::hotkey::init(player.clone());

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
            pending_track_path: None,
            editor_state: lyric_editor::LyricEditorState::new(),
            show_editor: false,
            download_state: lyric_download::LyricDownloadState::new(),
            pending_download_rx: None,
            pending_download_tx: None,
            current_track_path_for_download: String::new(),
            album_art: None,
            playlist_filter_text: String::new(),
            playlist_filter_mode: PlaylistFilterMode::All,
            playlist_sort_field: PlaylistSortField::Title,
            playlist_sort_asc: true,
            playlist_view_mode: PlaylistViewMode::Detail,
            playlist_drag_from: None,
            playlist_drag_to: None,
            playlist_drag_active: false,
            playlist_selected: std::collections::HashSet::new(),
            recent_tracks: VecDeque::new(),
            media_lib_cache: Some(crate::media::MediaLib::load()),
            media_lib_category: MediaLibCategory::AllTracks,
            media_lib_selected: None,
            media_lib_search: String::new(),
            media_lib_sidebar_cache: Vec::new(),
            media_lib_list_cache: Vec::new(),
            media_lib_cache_key: (MediaLibCategory::Rating, Some("__init__".into())),
            media_lib_total_dur: 0,
            eq_enabled: false,
            eq_sliders,
            eq_preset_name: "自定义".to_string(),
            playback_error: None,
            settings_tab: dialogs::SettingsTab::General,
            url_dialog_open: false,
            url_state,
            search_input,
            lyric_offset_ms: 0,
            lyric_show_translation: false,
            desktop_lyrics_open: false,
            modal: None,
            editor_track_idx: None,
            editor_title_input,
            editor_artist_input,
            editor_album_input,
            editor_genre_input,
            editor_year_input,
            editor_track_num_input,
            editor_rating_input,
            playlist_scroll_handle: UniformListScrollHandle::default(),
            config_cache: cfg,
            cover_cache: std::sync::Mutex::new(LruCache::new(std::num::NonZeroUsize::new(100).unwrap())),
        }
    }

    fn poll_player_state(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.poll_player_state_inner();
    }

    /// Window-less variant used by the 30fps repaint timer (which runs inside
    /// `this.update(cx, …)` and has no `&mut Window` handle). See the comment
    /// at the spawn site for why polling moved out of `render()`.
    fn poll_player_state_in_render(&mut self, _cx: &mut Context<Self>) {
        self.poll_player_state_inner();
    }

    /// Shared body of `poll_player_state` and `poll_player_state_in_render`.
    /// Reads position/duration/volume/state directly from engine (atomic-fast),
    /// and track info / spectrum / peaks from the lock-free `EngineStatus` snapshot.
    fn poll_player_state_inner(&mut self) {
        self.position = self.player.position().as_secs_f64();
        self.duration = self.player.duration().as_secs_f64();
        self.volume = self.player.volume();
        self.is_playing = self.player.state() == crate::core::engine_trait::EngineState::Playing;
        self.is_muted = self.volume == 0;

        // Process pending download events
        self.poll_download_events();

        let snap = self.player.status.load();
        if snap.current_track_index.is_some() {
            let display = if !snap.current_track_title.is_empty() {
                snap.current_track_title.clone()
            } else {
                std::path::Path::new(&snap.current_track_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default()
            };
            let artist = if !snap.current_track_artist.is_empty() {
                snap.current_track_artist.clone()
            } else {
                "未知艺术家".into()
            };
            let album = if !snap.current_track_album.is_empty() {
                snap.current_track_album.clone()
            } else {
                String::new()
            };
            if self.title != display { self.title = display; }
            if self.artist != artist { self.artist = artist; }
            if self.album != album { self.album = album; }
            self.is_favourite = snap.current_track_is_favourite;

            // Auto-load lyrics when track changes
            let track_path = &snap.current_track_path;
            if *track_path != self.last_lpc_path {
                self.last_lpc_path = track_path.clone();
                self.current_track_path_for_download = track_path.clone();
                self.pending_track_path = Some(track_path.clone());
                // Track recently played (most recent first), capped at 50
                self.recent_tracks.retain(|p| p != track_path);
                self.recent_tracks.push_front(track_path.clone());
                if self.recent_tracks.len() > 50 {
                    self.recent_tracks.pop_back();
                }
            }
            // Update download state keyword from current track
            if !snap.current_track_title.is_empty() || !snap.current_track_artist.is_empty() {
                if self.download_state.keyword.is_empty() || self.download_state.track_title != snap.current_track_title {
                    self.download_state.auto_fill(&snap.current_track_title, &snap.current_track_artist);
                }
            }
        }

        // Update lyrics progress every frame (apply user offset adjustment)
        let lyric_ms = ((self.position * 1000.0) as i64 + self.lyric_offset_ms).max(0) as u64;
        self.lyric_state.recompute(lyric_ms);
    }

    /// Extract album art for an audio file: prefer an embedded picture, then a
    /// sidecar `cover`/`folder`/`album`/`front` image in the same directory.
    /// Embedded art is written to a temp file so it can be rendered via `gpui::img`.
    /// Results are LRU-cached by track path to avoid repeated extraction.
    fn extract_cover(&self, file_path: &str) -> Option<PathBuf> {
        {
            let mut cache = self.cover_cache.lock().unwrap();
            if let Some(cached) = cache.peek(file_path) {
                return Some(cached.clone());
            }
        }
        let cover = Self::do_extract_cover(file_path);
        if let Some(ref path) = cover {
            let mut cache = self.cover_cache.lock().unwrap();
            if let Some(evicted) = cache.push(file_path.to_string(), path.clone()) {
                let _ = std::fs::remove_file(&evicted.1);
            }
        }
        cover
    }

    fn do_extract_cover(file_path: &str) -> Option<PathBuf> {
        if let Ok(pics) = crate::tag::writer::read_pictures(file_path) {
            if let Some((ext, data)) = pics.into_iter().next() {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                file_path.hash(&mut hasher);
                let hash = hasher.finish();
                let tmp = std::env::temp_dir().join(format!("hm_cover_{:016x}.{}", hash, ext));
                if std::fs::write(&tmp, &data).is_ok() {
                    return Some(tmp);
                }
            }
        }
        let path = std::path::Path::new(file_path);
        if let Some(dir) = path.parent() {
            for name in ["cover", "folder", "album", "front"] {
                for ext in ["jpg", "jpeg", "png", "bmp", "webp"] {
                    let cand = dir.join(format!("{}.{}", name, ext));
                    if cand.exists() {
                        return Some(cand);
                    }
                }
            }
        }
        None
    }

    /// Render the album-art box at the given size (real cover, or a grey placeholder).
    fn album_art_element(&self, size: gpui::Pixels, c: &UiColors) -> impl IntoElement {
        match &self.album_art {
            Some(path) => gpui::img(path.clone())
                .size(size)
                .rounded(px(16.0))
                .into_any_element(),
            None => div()
                .size(size)
                .rounded(px(16.0))
                .bg(c.panel)
                .into_any_element(),
        }
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
                    Ok(mut lyrics) => {
                        // Honour the "show translation" preference
                        lyrics.translate_mode = if self.lyric_show_translation {
                            crate::lyric::TranslateMode::Separate
                        } else {
                            crate::lyric::TranslateMode::Hidden
                        };
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

    /// Stateless lyrics loader for background use (no &mut self needed).
    fn load_lyrics_raw(audio_path: &str, show_translation: bool, _position_ms: u64) -> Option<crate::lyric::Lyrics> {
        if audio_path.is_empty() {
            return None;
        }
        let path = std::path::Path::new(audio_path);
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let parent = path.parent().unwrap_or(std::path::Path::new("."));
        let sibling_lrc = parent.join(format!("{}.lrc", file_stem));
        let sibling_lrc_lower = parent.join(format!("{}.lrc", file_stem.to_lowercase()));
        let lyrics_dir = parent.join("lyrics");
        let lyrics_dir_lrc = lyrics_dir.join(format!("{}.lrc", file_stem));
        let lyrics_dir_lrc_lower = lyrics_dir.join(format!("{}.lrc", file_stem.to_lowercase()));
        for candidate in [&sibling_lrc, &sibling_lrc_lower, &lyrics_dir_lrc, &lyrics_dir_lrc_lower] {
            if candidate.exists() {
                match crate::lyric::load_lyric_file(candidate.to_str().unwrap_or("")) {
                    Ok(mut lyrics) => {
                        lyrics.translate_mode = if show_translation {
                            crate::lyric::TranslateMode::Separate
                        } else {
                            crate::lyric::TranslateMode::Hidden
                        };
                        tracing::info!("[Lyric] Loaded: {:?}", candidate);
                        return Some(lyrics);
                    }
                    Err(e) => tracing::warn!("[Lyric] Parse error {}: {}", candidate.display(), e),
                }
            }
        }
        None
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
        // Poll player state directly in render (no thread contention since this
        // runs on the main thread). Heavy I/O (lyrics, cover extraction) is
        // deferred to the background timer via pending_track_path.
        self.poll_player_state_inner();
        let tr = self.tr;
        let c = &self.colours;

        // Modal dialogs take over the whole window when open.
        if let Some(kind) = self.modal {
            return self.render_modal(kind, c, window, cx).into_any_element();
        }
        // Open URL dialog overlay
        if self.url_dialog_open {
            return self.render_url_dialog(c, window, cx).into_any_element();
        }
        // Desktop lyrics are now rendered in a separate floating window.
        // The window is opened by the menu toggle handler, not here.

        // Update responsive state based on window size
        let window_bounds = window.bounds();
        let window_width: f32 = window_bounds.size.width.into();
        let window_height: f32 = window_bounds.size.height.into();
        self.responsive.update(window_width, window_height);
        let layout_mode = self.responsive.mode;

        // Mini mode: compact player window — mirrors original
        // skins/miniMode/miniMode01.xml (334×54):
        //   [cover 46] [prev/play/next/repeat/play_time/showPlaylist] [fav/mini]
        //   <progressBar height="2"/>
        if MINI_MODE.load(Ordering::Relaxed) {
            let play_label = if self.is_playing { "⏸" } else { "▶" };
            let player_p = self.player.clone();
            let player_n = self.player.clone();
            let player_s = self.player.clone();
            let player_rew = self.player.clone();
            let player_ff = self.player.clone();
            let player_repeat = self.player.clone();
            let player_mute_m = self.player.clone();
            let _vol = self.volume;
            let pos_str_mini = format!(
                "{:02}:{:02} / {:02}:{:02}",
                (self.position as u32) / 60, (self.position as u32) % 60,
                (self.duration as u32) / 60, (self.duration as u32) % 60
            );
            let seek_pct = if self.duration > 0.0 {
                (self.position / self.duration * 100.0) as f32
            } else { 0.0 };
            let repeat_label_m = match self.player.repeat_mode() {
                crate::core::playlist::RepeatMode::LoopPlaylist => "🔁",
                crate::core::playlist::RepeatMode::LoopTrack => "🔂",
                crate::core::playlist::RepeatMode::PlayShuffle => "🔀",
                _ => "➡️",
            };
            let dur_m = self.duration;
            let player_seek_m = self.player.clone();

            return v_flex()
                .size_full()
                .bg(c.bg)
                .child(
                    // Top row: cover | controls | fav/mini
                    h_flex().w_full().flex_grow().items_center().px_2().gap_2()
                        // Left: small album cover (46px in original)
                        .child(self.album_art_element(px(56.0), c))
                        // Center: prev/play/next + repeat + play_time + showPlaylist
                        .child(
                            h_flex().flex_1().items_center().justify_center().gap_1()
                                .child(Button::new("mini_prev").label("◀").ghost().compact().on_click(move |_, _, _| { let p = player_p.clone(); runtime().spawn_blocking(move || { let _ = p.prev(); }); }))
                                .child(Button::new("mini_play").label(play_label).primary().compact().on_click(move |_, _, _| {
                                    if player_n.is_playing() {
                                        let _ = player_n.toggle_pause();
                                    } else if player_n.is_paused() {
                                        let _ = player_n.toggle_pause();
                                    } else {
                                        let p = player_n.clone();
                                        let idx = p.playlist().current_index().unwrap_or(0);
                                        runtime().spawn_blocking(move || { let _ = p.play_at_index(idx); });
                                    }
                                }))
                                .child(Button::new("mini_next").label("▶").ghost().compact().on_click(move |_, _, _| { let p = player_s.clone(); runtime().spawn_blocking(move || { let _ = p.next(); }); }))
                                .child(Button::new("mini_repeat").label(repeat_label_m).ghost().compact().on_click(move |_, _, _| {
                                    use crate::core::playlist::RepeatMode;
                                    let modes = [RepeatMode::PlayOrder, RepeatMode::LoopPlaylist, RepeatMode::LoopTrack, RepeatMode::PlayShuffle, RepeatMode::PlayRandom];
                                    let cur = player_repeat.repeat_mode();
                                    let idx = modes.iter().position(|m| *m == cur).unwrap_or(0);
                                    player_repeat.set_repeat_mode(modes[(idx + 1) % modes.len()]);
                                }))
                                .child(layout::txt(&pos_str_mini, 10.0, c.text_dim))
                                .child(Button::new("mini_rew").label("«").ghost().compact().on_click(move |_, _, _| {
                                    let pos = player_rew.position();
                                    let _ = player_rew.seek(pos.saturating_sub(std::time::Duration::from_secs(5)));
                                }))
                                .child(Button::new("mini_ff").label("»").ghost().compact().on_click(move |_, _, _| {
                                    let dur = player_ff.duration();
                                    let pos = player_ff.position();
                                    let _ = player_ff.seek((pos + std::time::Duration::from_secs(5)).min(dur));
                                }))
                                .child(Button::new("mini_list").label("☰").ghost().compact().on_click(|_, _, _| {
                                    ACTIVE_PANEL.store(0, Ordering::Relaxed);
                                }))
                        )
                        // Right: favorite + exit-mini button
                        .child(
                            h_flex().items_center().gap_1()
                                .child(Button::new("mini_fav").label(if self.is_favourite { "♥" } else { "♡" }).ghost().compact())
                                .child(Button::new("mini_mute").label(if self.is_muted { "🔇" } else { "🔊" }).ghost().compact().on_click(move |_, _, _| {
                                    let v = player_mute_m.volume();
                                    let _ = if v > 0 { player_mute_m.set_volume(0) } else { player_mute_m.set_volume(80) };
                                }))
                                .child(Button::new("mini_exit").label("⤢").ghost().compact().on_click(|_, _, _| {
                                    MINI_MODE.store(false, Ordering::Relaxed);
                                }))
                    )
            )
            .child(
                    // Bottom: 2px progress bar (miniMode01.xml: <progressBar height="2"/>)
                    h_flex()
                        .id("mini-progress")
                        .w_full()
                        .h(px(3.0))
                        .cursor(gpui::CursorStyle::PointingHand)
                        .bg(c.progress_track)
                        .child(div().h_full().w(DefiniteLength::Fraction(seek_pct / 100.0)).bg(c.accent))
                        .on_mouse_down(gpui::MouseButton::Left, move |e, window, _cx| {
                            let win_w: f32 = window.bounds().size.width.into();
                            let mouse_x: f32 = e.position.x.into();
                            let ratio = (mouse_x / win_w).clamp(0.0, 1.0);
                            let _ = player_seek_m.seek(std::time::Duration::from_secs_f64(dur_m * ratio as f64));
                        })
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
        let snap = self.player.status.load();
        let raw_spec = snap.spectrum.clone();
        let raw_peaks = snap.spectrum_peaks.clone();
        let playlist_tracks = self.player.playlist().tracks().to_vec();
        let current_idx = self.player.playlist().current_index();

        // Build main layout with responsive adjustments
        let mut main_layout = v_flex().size_full().bg(c.bg);

        // Title bar - always shown
        main_layout = main_layout.child(layout::title_bar(c, tr));

        // Menu bar - only shown in BIG and NARROW modes
        if layout_mode.show_menubar() && MENUBAR_VISIBLE.load(Ordering::Relaxed) {
            main_layout = main_layout.child(self.render_menu_bar(c, tr, window, cx));
        }

        // ── Main content area: left-right split matching original MusicPlayer2 ──
        // LEFT:  album art + title/artist + spectrum + lyrics + toolbar + transport + progress
        // RIGHT: playlist (search + toolbar + list) — Big mode only
        let player_mute = self.player.clone();
        let dur = self.duration;
        let vol_width = layout_mode.volume_slider_width();
        let has_track = !self.title.is_empty() || self.duration > 0.0;

        // ── Left panel: now-playing + controls ──────────────────────────────
        let left_panel = v_flex()
            .flex_grow()
            .h_full()
            .bg(c.bg)
            // Status line: "已暂停 081 ..." + format info
            .child(
                v_flex().w_full().px_4().pt_2().gap_0()
                    .child(
                        h_flex().items_center().gap_2()
                            .child(layout::txt(
                                &if self.is_playing { "播放中" } else { "已暂停" },
                                10.0, c.text_dim))
                            .child(layout::txt(&self.title, 10.0, c.text))
                    )
                    .child(layout::txt(&self.format_line(), 9.0, c.text_dim))
            )
            // Album cover (large, centered)
            .child(
                v_flex().flex_grow().items_center().justify_center()
                    .child(self.album_art_element(px(200.0), c))
            )
            // Track title + artist
            .child(
                v_flex().items_center().gap_1().px_4()
                    .child(layout::txt(&self.title, 14.0, c.text_title))
                    .child(layout::txt(&self.artist, 11.0, c.text_dim))
            )
            // Spectrum
            .child(
                if layout_mode.show_spectrum() {
                    self.render_spectrum_strip(&raw_spec, &raw_peaks, 48, c).into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            // Lyrics (karaoke style)
            .child(
                if has_track {
                    v_flex().flex_grow().h(px(120.0)).child(
                        desktop_lyrics::render_lyrics_panel(&self.lyric_state, self.tr, c).into_any_element()
                    ).into_any_element()
                } else {
                    div().into_any_element()
                }
            )
            // Toolbar icons row (matching original: playlist/settings/eq/favourite/search/translate/brightness/AB/lyrics)
            .child(
                h_flex().w_full().px_4().py_1().gap_2().items_center()
                    .child(Button::new("tb_playlist").label("☰").ghost().compact().on_click(|_, _, _| {
                        ACTIVE_PANEL.store(0, Ordering::Relaxed);
                    }))
                    .child(Button::new("tb_settings").label("⚙").ghost().compact().on_click(|_, _, _| {
                        ACTIVE_PANEL.store(8, Ordering::Relaxed);
                    }))
                    .child(Button::new("tb_eq").label("EQ").ghost().compact().on_click(|_, _, _| {
                        ACTIVE_PANEL.store(7, Ordering::Relaxed);
                    }))
                    .child(Button::new("tb_fav").label(if self.is_favourite { "♥" } else { "♡" }).ghost().compact())
                    .child(Button::new("tb_search").icon(IconName::Search).ghost().compact().on_click(|_, _, _| {
                        ACTIVE_PANEL.store(3, Ordering::Relaxed);
                    }))
                    .child(Button::new("tb_ab").label("A-B").ghost().compact())
                    .child(Button::new("tb_lyrics").label("词").ghost().compact().on_click(|_, _, _| {
                        ACTIVE_PANEL.store(4, Ordering::Relaxed);
                    }))
                    .child(div().flex_grow())
                    // Volume on the right side of toolbar
                    .child(Button::new("mute").label(if self.is_muted { "🔇" } else { "🔊" }).ghost().compact().on_click(move |_, _, _| {
                        let vol = player_mute.volume();
                        if vol > 0 {
                            let _ = player_mute.set_volume(0);
                        } else {
                            let _ = player_mute.set_volume(80);
                        }
                    }))
                    .child(Slider::new(&self.volume_slider).horizontal().w(px(vol_width)))
            )
            // Transport controls: stop / prev / rew / play / ff / next + time
            .child(
                h_flex().w_full().px_4().py_2().gap_3().items_center().justify_center()
                    .child(Button::new("stop").label("⏹").ghost().compact().on_click(move |_, _, _| { let _ = player_stop.stop(); }))
                    .child(Button::new("prev").label("⏮").ghost().compact().on_click(move |_, _, _| { let p = player_prev.clone(); runtime().spawn_blocking(move || { let _ = p.prev(); }); }))
                    .child(Button::new("rew").label("⏪").ghost().compact().on_click(move |_, _, _| {
                        let pos = player_rew.position();
                        let _ = player_rew.seek(pos.saturating_sub(std::time::Duration::from_secs(5)));
                    }))
                    .child(Button::new("play").label(play_label).primary().compact().on_click({
                        let p = player_play.clone();
                        move |_, _, _| {
                            if p.is_playing() {
                                let _ = p.toggle_pause();
                            } else if p.is_paused() {
                                let _ = p.toggle_pause();
                            } else if p.playlist().is_empty() {
                                let p_clone = p.clone();
                                runtime().spawn_blocking(move || {
                                    if let Some(file) = rfd::FileDialog::new()
                                        .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape", "cue"])
                                        .pick_file()
                                    {
                                        let path = file.to_string_lossy().to_string();
                                        let _ = p_clone.play_file(&path);
                                    }
                                });
                            } else {
                                let idx = p.playlist().current_index().unwrap_or(0);
                                let p2 = p.clone();
                                runtime().spawn_blocking(move || { let _ = p2.play_at_index(idx); });
                            }
                        }
                    }))
                    .child(Button::new("ff").label("⏩").ghost().compact().on_click(move |_, _, _| {
                        let dur = player_ff.duration();
                        let pos = player_ff.position();
                        let _ = player_ff.seek((pos + std::time::Duration::from_secs(5)).min(dur));
                    }))
                    .child(Button::new("next").label("⏭").ghost().compact().on_click(move |_, _, _| { let p = player_next.clone(); runtime().spawn_blocking(move || { let _ = p.next(); }); }))
                    .child(div().w(px(1.0)).h(px(20.0)).bg(c.border))
                    .child(layout::txt(&pos_str, 11.0, c.text_dim))
                    // Repeat mode
                    .child(Button::new("repeat").label(repeat_label).ghost().compact().on_click(move |_, _, _| {
                        use crate::core::playlist::RepeatMode;
                        let modes = [RepeatMode::PlayOrder, RepeatMode::LoopPlaylist, RepeatMode::LoopTrack, RepeatMode::PlayShuffle, RepeatMode::PlayRandom];
                        let current = player_repeat.repeat_mode();
                        let idx = modes.iter().position(|m| *m == current).unwrap_or(0);
                        let next = modes[(idx + 1) % modes.len()];
                        player_repeat.set_repeat_mode(next);
                    }))
            )
            // Progress bar (full width, click-to-seek)
            .child(
                h_flex()
                    .id("progress-bar")
                    .w_full()
                    .h(px(layout_mode.progress_bar_height()))
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
                        let win_w: f32 = window.bounds().size.width.into();
                        let mouse_x: f32 = e.position.x.into();
                        // Progress bar spans the full window width
                        let ratio = (mouse_x / win_w).clamp(0.0, 1.0);
                        let seek_to = dur * ratio as f64;
                        let _ = player_seek.seek(std::time::Duration::from_secs_f64(seek_to));
                    })
            );

        // ── Right panel: playlist dock (Big mode only) ─────────────────────
        let right_panel = if layout_mode.show_right_panel() {
            Some(self.render_playlist_dock(&playlist_tracks, current_idx, layout_mode, c, window, cx))
        } else {
            None
        };

        // Assemble: left panel + optional divider + right panel
        let mut content_flex = h_flex().w_full().flex_grow();
        content_flex = content_flex.child(left_panel);
        if let Some(right) = right_panel {
            content_flex = content_flex.child(div().w(px(1.0)).h_full().bg(c.border));
            content_flex = content_flex.child(right);
        }
        main_layout = main_layout.child(content_flex);

        // Status bar - only shown in BIG mode
        if layout_mode.show_statusbar() && STATUSBAR_VISIBLE.load(Ordering::Relaxed) {
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
                    .child(layout::txt(&format!("{} 首", track_count), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("{}", total_dur), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("{}", file_type), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("{}", repeat_desc), 10.0, c.text_dim))
                    .child(div().w(px(1.0)).h(px(12.0)).bg(c.border))
                    .child(layout::txt(&format!("{}", engine_name), 10.0, c.text_dim))
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
                let tr = self.tr;
                move |menu, _w, _cx| {
                    let p1 = p.clone();
                    let p2 = p.clone();
                    let p3 = p.clone();
                    let p4 = p.clone();
                    let p5 = p.clone();
                    let s_repeat_order = tr.repeat_order;
                    let s_repeat_random = tr.repeat_random;
                    let s_repeat_loop_pl = tr.repeat_loop_pl;
                    let s_repeat_loop_trk = tr.repeat_loop_trk;
                    let s_repeat_single = tr.repeat_single;
                    menu.item(PopupMenuItem::new(tr.ctrl_toggle_play).on_click(move |_, _, _| { let _ = p1.toggle_pause(); }))
                        .item(PopupMenuItem::new(tr.ctrl_stop).on_click(move |_, _, _| { let _ = p2.stop(); }))
                        .item(PopupMenuItem::new(tr.ctrl_prev).on_click(move |_, _, _| { let p = p3.clone(); runtime().spawn_blocking(move || { let _ = p.prev(); }); }))
                        .item(PopupMenuItem::new(tr.ctrl_next).on_click(move |_, _, _| { let p = p4.clone(); runtime().spawn_blocking(move || { let _ = p.next(); }); }))
                        .separator()
                        .item(PopupMenuItem::new(s_repeat_order).on_click({
                            let p = p5.clone();
                            move |_, _, _| { p.set_repeat_mode(crate::core::playlist::RepeatMode::PlayOrder); }
                        }))
                        .item(PopupMenuItem::new(s_repeat_random).on_click({
                            let p = p5.clone();
                            move |_, _, _| { p.set_repeat_mode(crate::core::playlist::RepeatMode::PlayRandom); }
                        }))
                        .item(PopupMenuItem::new(s_repeat_loop_pl).on_click({
                            let p = p5.clone();
                            move |_, _, _| { p.set_repeat_mode(crate::core::playlist::RepeatMode::LoopPlaylist); }
                        }))
                        .item(PopupMenuItem::new(s_repeat_loop_trk).on_click({
                            let p = p5.clone();
                            move |_, _, _| { p.set_repeat_mode(crate::core::playlist::RepeatMode::LoopTrack); }
                        }))
                        .item(PopupMenuItem::new(s_repeat_single).on_click({
                            let p = p5.clone();
                            move |_, _, _| { p.set_repeat_mode(crate::core::playlist::RepeatMode::PlayTrack); }
                        }))
                }
            })
            .into_any_element()
    }
}

impl MusicPlayer {
    /// Left navigation rail — vertical icon+text bar that swaps the main view.
    /// Mirrors the original `navigationBar orientation="vertical" icon_and_text`
    /// with item_list="now_playing,play_queue,recently_played,folder,playlist,
    /// my_favourite,media_lib" plus a settings button at the bottom.
    /// DEAD CODE — kept for reference but no longer called from render().
    #[allow(dead_code)]
    fn render_nav_rail(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.entity().downgrade();
        let active = Panel::from_u8(ACTIVE_PANEL.load(Ordering::Relaxed));

        // (id, label, target_panel) — use text labels since the gpui-component
        // IconName enum doesn't expose Music/List/History/Library/ListMusic/
        // SlidersHorizontal in this version.
        let items: [(&'static str, &'static str, Panel); 7] = [
            ("nav_now_playing", "正在播放",   Panel::Lyrics),
            ("nav_play_queue",  "播放队列",   Panel::Playlist),
            ("nav_recent",      "最近播放",   Panel::Playlist),
            ("nav_folder",      "文件夹",     Panel::FileBrowser),
            ("nav_playlist",    "播放列表",   Panel::Playlist),
            ("nav_favourite",   "我喜欢的",   Panel::Playlist),
            ("nav_media_lib",   "媒体库",     Panel::MediaLib),
        ];

        v_flex()
            .w(px(theme::NAV_RAIL_WIDTH))
            .h_full()
            .bg(c.panel_alt)
            .border_r_1()
            .border_color(c.border)
            .child(v_flex().w_full().px_2().py_2().gap_1()
                .children(items.map(|(id, label, target)| {
                    let selected = active == target;
                    let bg = if selected { c.accent.opacity(0.18) }
                             else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } };
                    let tc = if selected { c.accent } else { c.text };
                    let w = weak.clone();
                    let t = target.to_u8();
                    Button::new(id)
                        .label(label)
                        .w_full().justify_start()
                        .px_3().h(px(36.0))
                        .ghost()
                        .bg(bg)
                        .text_color(tc)
                        .on_click(move |_, _, cx| {
                            let _ = w.update(cx, |_, cx| {
                                ACTIVE_PANEL.store(t, Ordering::Relaxed);
                                cx.notify();
                            });
                        })
                }))
            )
            .child(div().flex_grow())
            .child(
                v_flex().w_full().px_2().py_2().gap_1()
                    .child({
                        let w = weak.clone();
                        Button::new("nav_search")
                            .label("查找")
                            .w_full().justify_start()
                            .px_3().h(px(36.0))
                            .ghost().text_color(if active == Panel::Search { c.accent } else { c.text })
                            .on_click(move |_, _, cx| {
                                let _ = w.update(cx, |_, cx| {
                                    ACTIVE_PANEL.store(3, Ordering::Relaxed);
                                    cx.notify();
                                });
                            })
                    })
                    .child({
                        let w = weak.clone();
                        Button::new("nav_eq")
                            .label("均衡器")
                            .w_full().justify_start()
                            .px_3().h(px(36.0))
                            .ghost().text_color(if active == Panel::Equalizer { c.accent } else { c.text })
                            .on_click(move |_, _, cx| {
                                let _ = w.update(cx, |_, cx| {
                                    ACTIVE_PANEL.store(7, Ordering::Relaxed);
                                    cx.notify();
                                });
                            })
                    })
                    .child({
                        let w = weak.clone();
                        Button::new("nav_settings")
                            .icon(IconName::Settings).label("设置")
                            .w_full().justify_start().items_center()
                            .px_3().h(px(36.0)).gap_2()
                            .ghost().text_color(if active == Panel::Settings { c.accent } else { c.text })
                            .on_click(move |_, _, cx| {
                                let _ = w.update(cx, |_, cx| {
                                    ACTIVE_PANEL.store(8, Ordering::Relaxed);
                                    cx.notify();
                                });
                            })
                    })
            )
    }

    /// "Now Playing" screen — the heart of the original MusicPlayer2 main
    /// view. Layout (matches 02_grooveMusic.xml "正在播放"):
    ///   ┌────────────────────────────────────────────┐
    ///   │  ┌──────────┐  title (scroll)              │
    ///   │  │          │  artist_album (scroll)       │
    ///   │  │  cover   │  spectrum (reflex, fixed)    │
    ///   │  │  110px   │  format                       │
    ///   │  └──────────┘                              │
    ///   │  lyrics (flex_grow, karaoke highlight)     │
    ///   └────────────────────────────────────────────┘
    fn render_now_playing(
        &self,
        raw_spec: &[f32],
        raw_peaks: &[f32],
        c: &UiColors,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let has_track = !self.title.is_empty() || self.duration > 0.0;

        v_flex()
            .flex_grow()
            .h_full()
            .bg(c.bg)
            .p_4()
            .gap_3()
            .child(
                // Top: cover (left) + title/artist/album/spectrum (right)
                h_flex().w_full().gap_4().items_start()
                    .child(
                        v_flex().items_center().gap_3()
                            .child(self.album_art_element(px(180.0), c))
                            .child(
                                h_flex().gap_2()
                                    .child(Button::new("np_fav").label(if self.is_favourite { "♥" } else { "♡" }).ghost().compact())
                                    .child(Button::new("np_info").icon(IconName::Info).ghost().compact())
                                    .child(Button::new("np_add").icon(IconName::Plus).ghost().compact())
                            )
                    )
                    .child(
                        v_flex().flex_grow().gap_2()
                            .child(layout::txt(&self.title, 18.0, c.text_title))
                            .child(layout::txt(&self.artist, 13.0, c.text_dim))
                            .child(layout::txt(&self.album, 12.0, c.text_dim))
                            .child(self.render_spectrum_strip(raw_spec, raw_peaks, 48, c))
                            .child(
                                // Format / file info line
                                h_flex().gap_3().child(layout::txt(
                                    &self.format_line(),
                                    11.0,
                                    c.text_dim,
                                ))
                            )
                    )
            )
            .child(
                // Bottom: synced lyrics (flex_grow fills remaining space)
                v_flex().flex_grow().h_full().child(
                    if has_track {
                        desktop_lyrics::render_lyrics_panel(&self.lyric_state, self.tr, c).into_any_element()
                    } else {
                        v_flex().size_full().justify_center().items_center().gap_2()
                            .child(div().text_size(px(16.0)).text_color(c.text_dim).child("未打开音乐文件"))
                            .child(div().text_size(px(12.0)).text_color(c.text_dim).child("点击 文件 > 打开文件 开始播放"))
                            .into_any_element()
                    }
                )
            )
    }

    /// One-line format/file info string shown on the now-playing screen.
    fn format_line(&self) -> String {
        if let Some(track) = self.player.playlist().current_track() {
            let ft = if !track.file_type.is_empty() { track.file_type.clone() } else { "--".into() };
            let bit = if track.bitrate > 0 { format!("{} kbps", track.bitrate) } else { "--".into() };
            let sr = if track.sample_rate > 0 { format!("{} Hz", track.sample_rate) } else { "--".into() };
            format!("{}  ·  {}  ·  {}", ft, bit, sr)
        } else {
            String::new()
        }
    }

    /// Right-docked playlist column for Big mode. Re-uses the existing
    /// `render_playlist` (the full track list) but wraps it with a header
    /// (title + search + add button) and a fixed dock width.
    fn render_playlist_dock(
        &self,
        tracks: &[Track],
        current_idx: Option<usize>,
        layout_mode: LayoutMode,
        c: &UiColors,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak = cx.entity().downgrade();

        v_flex()
            .w(px(theme::PLAYLIST_DOCK_WIDTH))
            .h_full()
            .bg(c.panel)
            .child(
                // Header: "播放队列" + search box + add button
                h_flex().items_center().justify_between()
                    .w_full().px_3().h(px(36.0))
                    .bg(c.panel_alt)
                    .child(layout::txt("播放队列", 13.0, c.text_title))
                    .child(
                        h_flex().gap_1()
                            .child(Button::new("dock_search").icon(IconName::Search).ghost().compact())
                            .child(Button::new("dock_add").icon(IconName::Plus).ghost().compact())
                    )
            )
            .child(
                // Filter chips: 默认列表 / 我喜欢的音乐 / 最近播放
                h_flex().w_full().px_2().py_1().gap_1()
                    .children({
                        let total = tracks.len();
                        let fav_count = tracks.iter().filter(|t| t.is_favourite).count();
                        let recent_count = self.recent_tracks.len();
                        let cur = self.playlist_filter_mode;
                        let w = weak.clone();
                        [
                            ("chip_all", PlaylistFilterMode::All,       format!("全部 ({})", total),       cur == PlaylistFilterMode::All),
                            ("chip_fav", PlaylistFilterMode::Favorites, format!("喜欢 ({})", fav_count),    cur == PlaylistFilterMode::Favorites),
                            ("chip_rec", PlaylistFilterMode::Recent,    format!("最近 ({})", recent_count), cur == PlaylistFilterMode::Recent),
                        ].into_iter().map(move |(id, m, label, selected)| {
                            let w = w.clone();
                            Button::new(id)
                                .label(label)
                                .compact().ghost()
                                .text_color(if selected { c.accent } else { c.text_dim })
                                .bg(if selected { c.accent.opacity(0.18) } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                                .on_click(move |_, _, cx| {
                                    let _ = w.update(cx, |this, cx| {
                                        this.playlist_filter_mode = m;
                                        cx.notify();
                                    });
                                })
                        })
                    })
            )
            .child(
                // Toolbar: +添加 / ×删除 / ↕排序 / ≡列表 / ✏编辑 / ⟳
                h_flex().w_full().px_3().py_1().gap_3().items_center()
                    .child(Button::new("pl_add").label("+ 添加").compact().ghost().text_size(px(10.0)).on_click({
                        let w = weak.clone();
                        let w2 = weak.clone();
                        move |_, _, cx| {
                            let _ = w.update(cx, |this, cx| {
                                let weak_ = w2.clone();
                                run_blocking_dialog_app(cx, &weak_,
                                    || rfd::FileDialog::new()
                                        .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                                        .add_filter("播放列表", &["m3u", "m3u8"])
                                        .pick_file(),
                                    |file, this, cx| {
                                        if let Some(file) = file {
                                            let path = file.to_string_lossy().to_string();
                                            if path.ends_with(".m3u") || path.ends_with(".m3u8") {
                                                if let Ok(tracks) = crate::core::playlist::Playlist::import_m3u(&path) {
                                                    this.player.playlist_mut().add_tracks(tracks);
                                                }
                                            } else {
                                                let _ = this.player.play_file(&path);
                                            }
                                        }
                                        cx.notify();
                                    });
                            });
                        }
                    }))
                    .child(Button::new("pl_remove").label("× 删除").compact().ghost().text_size(px(10.0)).on_click({
                        let w = weak.clone();
                        move |_, _, cx| {
                            let _ = w.update(cx, |this, cx| {
                                if let Some(idx) = this.player.playlist().current_index() {
                                    this.player.playlist_mut().remove(idx);
                                }
                                cx.notify();
                            });
                        }
                    }))
                    .child(Button::new("pl_sort").label("↕ 排序").compact().ghost().text_size(px(10.0)).on_click({
                        let w = weak.clone();
                        move |_, _, cx| {
                            let _ = w.update(cx, |this, cx| {
                                this.player.playlist_mut().sort(crate::core::playlist::SortMode::Title, false);
                                cx.notify();
                            });
                        }
                    }))
                    .child(Button::new("pl_list").label(if self.playlist_view_mode == PlaylistViewMode::Detail { "≡ 简洁" } else { "≡ 详情" }).compact().ghost().text_size(px(10.0)).on_click({
                        let w = weak.clone();
                        move |_, _, cx| {
                            let _ = w.update(cx, |this, cx| {
                                this.playlist_view_mode = match this.playlist_view_mode {
                                    PlaylistViewMode::Detail => PlaylistViewMode::Compact,
                                    PlaylistViewMode::Compact => PlaylistViewMode::Detail,
                                };
                                cx.notify();
                            });
                        }
                    }))
                    .child(Button::new("pl_edit").label("✏ 编辑").compact().ghost().text_size(px(10.0)).on_click({
                        let w = weak.clone();
                        let e_title = self.editor_title_input.clone();
                        let e_artist = self.editor_artist_input.clone();
                        let e_album = self.editor_album_input.clone();
                        let e_genre = self.editor_genre_input.clone();
                        let e_year = self.editor_year_input.clone();
                        let e_tnum = self.editor_track_num_input.clone();
                        let e_rating = self.editor_rating_input.clone();
                        move |_, window, cx| {
                            let data = w.update(cx, |this, _cx| {
                                this.player.playlist().current_index().and_then(|idx| {
                                    this.player.playlist().get(idx).map(|t| {
                                        (idx, t.title.clone(), t.artist.clone(), t.album.clone(), t.genre.clone(),
                                         if t.year > 0 { t.year.to_string() } else { String::new() },
                                         if t.track_number > 0 { t.track_number.to_string() } else { String::new() },
                                         if t.rating > 0 { t.rating.to_string() } else { String::new() })
                                    })
                                })
                            }).ok().flatten();
                            if let Some((idx, title, artist, album, genre, year, tnum, rating)) = data {
                                let _ = e_title.update(cx, |s, cx| s.set_value(&title, window, cx));
                                let _ = e_artist.update(cx, |s, cx| s.set_value(&artist, window, cx));
                                let _ = e_album.update(cx, |s, cx| s.set_value(&album, window, cx));
                                let _ = e_genre.update(cx, |s, cx| s.set_value(&genre, window, cx));
                                let _ = e_year.update(cx, |s, cx| s.set_value(&year, window, cx));
                                let _ = e_tnum.update(cx, |s, cx| s.set_value(&tnum, window, cx));
                                let _ = e_rating.update(cx, |s, cx| s.set_value(&rating, window, cx));
                                let _ = w.update(cx, |this, cx| {
                                    this.editor_track_idx = Some(idx);
                                    this.modal = Some(ModalKind::TrackEditor);
                                    cx.notify();
                                });
                            }
                        }
                    }))
                    .child(Button::new("pl_refresh").label("⟳").compact().ghost().text_size(px(10.0)))
            )
            .child(
                // The actual playlist (flex_grow fills the rest)
                v_flex().flex_grow().h_full().child(
                    self.render_playlist(tracks, current_idx, layout_mode, window, cx)
                )
            )
    }

    fn render_menu_bar(
        &self,
        c: &UiColors,
        tr: &Tr,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Weak handle to self so menu callbacks (which only receive &mut App)
        // can rebuild colors and request a repaint after toggling theme/dark mode.
        let weak = _cx.entity().downgrade();
        let weak_file = weak.clone();
        let weak_playlist = weak.clone();
        let weak_lyric = weak.clone();
        let _weak_view = weak.clone();
        let _weak_tools = weak.clone();
        let _weak_help = weak.clone();
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
                let s_open_url = tr.menu_open_url;
                let s_open_playlist = tr.menu_open_playlist;
                layout::menu_dropdown(s_file, IconName::Folder, move |menu, _w, _cx| {
                    let weak = weak_file.clone();
                    menu.item(PopupMenuItem::new(s_open_file).on_click({
                        let weak_ = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak_,
                                || rfd::FileDialog::new()
                                    .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape", "cue"])
                                    .pick_file(),
                                |file, this, cx| {
                                    if let Some(file) = file {
                                        let _ = this.player.play_file(file.to_str().unwrap_or_default());
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .item(PopupMenuItem::new(s_open_folder).on_click({
                        let weak_ = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak_,
                                || rfd::FileDialog::new().pick_folder(),
                                |folder, this, cx| {
                                    if let Some(folder) = folder {
                                        let dir = folder.to_str().unwrap_or_default();
                                        match crate::media::scan_directory(dir, true, None) {
                                            Ok(entries) => {
                                                let mut pl = this.player.playlist_mut();
                                                let first_idx = pl.len();
                                                for e in &entries {
                                                    pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                                }
                                                drop(pl);
                                                let mut lib = crate::media::MediaLib::load();
                                                for e in &entries { lib.upsert(e.clone()); }
                                                let _ = lib.save();
                                                this.media_lib_cache = Some(lib);
                                                this.media_lib_cache_key = (MediaLibCategory::AllTracks, None);
                                                if !entries.is_empty() {
                                                    let p = this.player.clone();
                                                    runtime().spawn_blocking(move || { let _ = p.play_at_index(first_idx); });
                                                }
                                                tracing::info!("[Menu] Loaded {} tracks from folder", entries.len());
                                            }
                                            Err(e) => tracing::warn!("Scan folder failed: {}", e),
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .item(PopupMenuItem::new(s_open_url).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.url_dialog_open = true;
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_open_playlist).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak,
                                || rfd::FileDialog::new()
                                    .add_filter("播放列表", &["m3u", "m3u8", "wpl", "ttpl", "playlist"])
                                    .pick_file(),
                                |file, this, cx| {
                                    if let Some(file) = file {
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
                                            _ => Err(crate::gui::i18n::global_tr().format_unsupported.to_string()),
                                        };
                                        match result {
                                            Ok(tracks) => {
                                                let mut pl = this.player.playlist_mut();
                                                pl.clear();
                                                pl.add_tracks(tracks);
                                                let p = this.player.clone();
                                                runtime().spawn_blocking(move || { let _ = p.play_at_index(0); });
                                                tracing::info!("[Menu] Loaded playlist: {}", path);
                                            }
                                            Err(e) => tracing::error!("[Menu] Failed to load playlist: {}", e),
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_save_as_new).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak,
                                || rfd::FileDialog::new()
                                    .add_filter("M3U 播放列表", &["m3u", "m3u8"])
                                    .set_file_name("playlist.m3u")
                                    .save_file(),
                                |file, this, cx| {
                                    if let Some(file) = file {
                                        let path = file.to_string_lossy().to_string();
                                        match this.player.playlist_mut().export_m3u(&path, true) {
                                            Ok(()) => tracing::info!("[Menu] 另存为新播放列表: {}", path),
                                            Err(e) => tracing::error!("[Menu] 保存失败: {}", e),
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_exit).on_click(|_, _, _| std::process::exit(0)))
                })
            })
            // Playback menu
            .child({
                let player = self.player.clone();
                let tr = self.tr;
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
                    let s_toggle = tr.ctrl_toggle_play;
                    let s_stop = tr.ctrl_stop;
                    let s_prev = tr.ctrl_prev;
                    let s_next = tr.ctrl_next;
                    let s_rewind5 = tr.menu_rewind_5s;
                    let s_forward5 = tr.menu_forward_5s;
                    let s_speed_up = tr.menu_speed_up;
                    let s_slow_down = tr.menu_slow_down;
                    let s_orig_speed = tr.menu_original_speed;
                    let s_pitch_up = tr.menu_pitch_up;
                    let s_pitch_down = tr.menu_pitch_down;
                    let s_orig_pitch = tr.menu_original_pitch;
                    let s_ab_a = tr.menu_ab_set_a;
                    let s_ab_b = tr.menu_ab_set_b;
                    let s_ab_cont = tr.menu_ab_continue;
                    let s_ab_clear = tr.menu_ab_clear;
                    menu.item(PopupMenuItem::new(s_toggle).on_click(move |_, _, _| { let _ = p1.toggle_pause(); }))
                        .item(PopupMenuItem::new(s_stop).on_click(move |_, _, _| { let _ = p2.stop(); }))
                        .item(PopupMenuItem::new(s_prev).on_click(move |_, _, _| { let p = p3.clone(); runtime().spawn_blocking(move || { let _ = p.prev(); }); }))
                        .item(PopupMenuItem::new(s_next).on_click(move |_, _, _| { let p = p4.clone(); runtime().spawn_blocking(move || { let _ = p.next(); }); }))
                        .separator()
                        .item(PopupMenuItem::new(s_rewind5).on_click({
                            let p = p14.clone();
                            move |_, _, _| {
                                let pos = p.position();
                                let _ = p.seek(std::time::Duration::from_secs_f64((pos.as_secs_f64() - 5.0).max(0.0)));
                            }
                        }))
                        .item(PopupMenuItem::new(s_forward5).on_click(move |_, _, _| {
                            let pos = p14.position();
                            let _ = p14.seek(std::time::Duration::from_secs_f64(pos.as_secs_f64() + 5.0));
                        }))
                        .separator()
                        .item(PopupMenuItem::new(s_speed_up).on_click(move |_, _, _| { let _ = p5.speed_up(); }))
                        .item(PopupMenuItem::new(s_slow_down).on_click(move |_, _, _| { let _ = p6.speed_down(); }))
                        .item(PopupMenuItem::new(s_orig_speed).on_click(move |_, _, _| { let _ = p7.set_speed(1.0); }))
                        .separator()
                        .item(PopupMenuItem::new(s_pitch_up).on_click(move |_, _, _| { let _ = p8.pitch_up(); }))
                        .item(PopupMenuItem::new(s_pitch_down).on_click(move |_, _, _| { let _ = p9.pitch_down(); }))
                        .item(PopupMenuItem::new(s_orig_pitch).on_click(move |_, _, _| { let _ = p10.set_pitch(0); }))
                        .separator()
                        .item(PopupMenuItem::new(s_ab_a).on_click(move |_, _, _| { let _ = p11.ab_set_a(); }))
                        .item(PopupMenuItem::new(s_ab_b).on_click(move |_, _, _| { let _ = p12.ab_set_b(); }))
                        .item(PopupMenuItem::new(s_ab_cont).on_click({
                            let p = p13.clone();
                            move |_, _, _| { let _ = p.ab_continue(); }
                        }))
                        .item(PopupMenuItem::new(s_ab_clear).on_click(move |_, _, _| { p13.ab_reset(); }))
                })
            })
            // Playlist menu
            .child({
                let player = self.player.clone();
                let weak = weak_playlist;
                let tr = self.tr;
                layout::menu_dropdown(s_playlist, IconName::SquareTerminal, move |menu, _, _| {
                    let s_add_file = tr.menu_add_file;
                    let s_add_folder = tr.menu_add_folder;
                    let s_add_from_lib = tr.menu_add_from_lib;
                    let s_add_url = tr.menu_add_url;
                    let s_remove_sel = tr.menu_remove_selected;
                    let s_delete_disk = tr.menu_delete_from_disk;
                    let s_clear_list = tr.menu_clear_list;
                    let s_remove_dups = tr.menu_remove_duplicates;
                    let s_remove_invalid = tr.menu_remove_invalid;
                    let s_repair_paths = tr.menu_repair_paths;
                    let s_save_pl = tr.menu_save_playlist;
                    let s_save_as_new = tr.menu_save_as_new;
                    let s_sort = tr.menu_sort;
                    let s_sort_artist = tr.menu_sort_artist;
                    let s_sort_album = tr.menu_sort_album;
                    let s_sort_duration = tr.menu_sort_duration;
                    let s_sort_filename = tr.menu_sort_filename;
                    let s_sort_random = tr.menu_sort_random;
                    let s_sort_reverse = tr.menu_sort_reverse;
                    menu                    .item(PopupMenuItem::new(s_add_file).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak,
                                || rfd::FileDialog::new()
                                    .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                                    .pick_files(),
                                |paths, this, cx| {
                                    if let Some(paths) = paths {
                                        let mut pl = this.player.playlist_mut();
                                        for path in &paths {
                                            pl.add_track(crate::core::playlist::Track::new(path.to_str().unwrap_or_default()));
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .item(PopupMenuItem::new(s_add_folder).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            run_blocking_dialog_app(cx, &weak,
                                || rfd::FileDialog::new().pick_folder(),
                                |folder, this, cx| {
                                    if let Some(folder) = folder {
                                        if let Ok(entries) = crate::media::scan_directory(folder.to_str().unwrap_or_default(), true, None) {
                                            let mut pl = this.player.playlist_mut();
                                            for e in entries {
                                                pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                            }
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .item(PopupMenuItem::new(s_add_from_lib).on_click({
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
                    .item(PopupMenuItem::new(s_add_url).on_click(move |_, _, _| {
                        tracing::info!("[Playlist] 添加URL - 打开URL对话框");
                    }))
                    .item(PopupMenuItem::new(s_remove_sel).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                if this.playlist_selected.is_empty() {
                                    tracing::info!("[Playlist] 删除选中: 无选中项");
                                    return;
                                }
                                // Collect selected indices descending so removal
                                // doesn't shift the indices still to remove.
                                let mut indices: Vec<usize> = this.playlist_selected.iter().copied().collect();
                                indices.sort_unstable_by(|a, b| b.cmp(a));
                                let mut pl = this.player.playlist_mut();
                                for idx in &indices {
                                    pl.remove(*idx);
                                }
                                let removed = indices.len();
                                this.playlist_selected.clear();
                                tracing::info!("[Playlist] 删除选中 {} 首", removed);
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_delete_disk).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                if this.playlist_selected.is_empty() {
                                    tracing::info!("[Playlist] 从磁盘删除: 无选中项");
                                    return;
                                }
                                let indices: Vec<usize> = {
                                    let mut v: Vec<usize> = this.playlist_selected.iter().copied().collect();
                                    v.sort_unstable_by(|a, b| b.cmp(a));
                                    v
                                };
                                let mut pl = this.player.playlist_mut();
                                let mut deleted = 0usize;
                                let mut failed = 0usize;
                                for idx in &indices {
                                    if let Some(track) = pl.get(*idx) {
                                        let path = track.file_path.clone();
                                        match std::fs::remove_file(&path) {
                                            Ok(()) => {
                                                pl.remove(*idx);
                                                deleted += 1;
                                            }
                                            Err(e) => {
                                                tracing::warn!("[Playlist] 从磁盘删除失败 {}: {}", path, e);
                                                failed += 1;
                                            }
                                        }
                                    }
                                }
                                this.playlist_selected.clear();
                                tracing::info!(
                                    "[Playlist] 从磁盘删除: {} 成功, {} 失败",
                                    deleted, failed
                                );
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_clear_list).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().clear();
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_remove_dups).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let removed = p.playlist_mut().dedup();
                            tracing::info!("Removed {} duplicate tracks", removed);
                        }
                    }))
                    .item(PopupMenuItem::new(s_remove_invalid).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            let removed = p.playlist_mut().clean();
                            tracing::info!("Removed {} missing tracks", removed);
                        }
                    }))
                    .item(PopupMenuItem::new(s_repair_paths).on_click({
                        let weak = weak.clone();
                        let player = player.clone();
                        move |_, _, cx| {
                            let weak3 = weak.clone();
                            let player2 = player.clone();
                            runtime().spawn_blocking(move || {
                                let pl = player2.playlist();
                                let missing: Vec<(usize, String)> = (0..pl.len())
                                    .filter_map(|i| {
                                        pl.get(i).filter(|t| !std::path::Path::new(&t.file_path).exists()).map(|t| {
                                            let name = if !t.title.is_empty() { t.title.clone() } else { t.file_name.clone() };
                                            (i, name)
                                        })
                                    })
                                    .collect();
                                drop(pl);
                                for (i, name) in missing {
                                    if let Some(new_path) = rfd::FileDialog::new().set_title(&format!("修复: {}", name)).pick_file() {
                                        let new_path_str = new_path.to_string_lossy().to_string();
                                        // Can't update player from here, will need to queue
                                        tracing::info!("[修复] 曲目 #{} -> {}", i, new_path_str);
                                    }
                                }
                            });
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_save_pl).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            if let Err(e) = p.save_current_playlist() {
                                tracing::warn!("Save playlist failed: {}", e);
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_save_as_new).on_click(move |_, _, _| {
                        // Would need dialog for name input
                        tracing::info!("Save as new playlist");
                    }))
                    .separator()
                    // Sort submenu
                    .item(PopupMenuItem::new(s_sort).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            // Default sort by title ascending
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Title, false);
                            tracing::info!("Playlist sorted by title");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_artist).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Artist, false);
                            tracing::info!("Playlist sorted by artist");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_album).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Album, false);
                            tracing::info!("Playlist sorted by album");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_duration).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Time, false);
                            tracing::info!("Playlist sorted by duration");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_filename).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::FileName, false);
                            tracing::info!("Playlist sorted by filename");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_random).on_click({
                        let p = player.clone();
                        move |_, _, _| {
                            p.playlist_mut().sort(crate::core::playlist::SortMode::Random, false);
                            tracing::info!("Playlist shuffled");
                        }
                    }))
                    .item(PopupMenuItem::new(s_sort_reverse).on_click({
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
                let weak = weak_lyric;
                let s_lyric_label = tr.menu_lyric;
                let s_reload_lyric = tr.menu_reload_lyric;
                let s_copy_line = tr.menu_copy_current_line;
                let s_copy_all = tr.menu_copy_all_lyric;
                let s_edit_lyric = tr.menu_edit_lyric;
                let s_download_lyric = tr.menu_download_lyric;
                let s_batch_download = tr.menu_batch_download_lyric;
                let s_show_trans = tr.menu_show_translation;
                let s_show_desktop = tr.menu_show_desktop_lyric;
                let s_desktop_lock = tr.menu_desktop_lock;
                let s_lyric_adv = tr.menu_lyric_advance;
                let s_lyric_ret = tr.menu_lyric_retreat;
                let s_save_lyric = tr.menu_save_lyric_edit;
                let s_assoc_lyric = tr.menu_associate_lyric;
                let s_embed_lyric = tr.menu_embed_lyric;
                let lyric_visible_now = self.lyric_visible;
                layout::menu_dropdown(s_lyric_label, IconName::BookOpen, {
                    let weak = weak.clone();
                    let player = player.clone();
                    move |menu, _, _| {
                    let p1 = player.clone();
                    let p2 = player.clone();
                    let p3 = player.clone();
                    let p4 = player.clone();
                    menu.item(PopupMenuItem::new(if lyric_visible_now { "隐藏歌词" } else { "显示歌词" }.to_string()).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.lyric_visible = !this.lyric_visible;
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_reload_lyric).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                if let Some(track) = this.player.playlist().current_track() {
                                    this.load_lyrics_for_track(&track.file_path);
                                }
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_copy_line).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, _cx| {
                                if let Some(lyrics) = &this.lyric_state.lyrics {
                                    if let Some(idx) = this.lyric_state.current_index {
                                        if let Some(line) = lyrics.lines.get(idx) {
                                            copy_text_to_clipboard(&lyrics.display_text(line));
                                            tracing::info!("[Lyric] Copied current line");
                                        }
                                    }
                                }
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_copy_all).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, _cx| {
                                if let Some(lyrics) = &this.lyric_state.lyrics {
                                    let all: Vec<String> = lyrics.lines.iter().map(|l| lyrics.display_text(l)).collect();
                                    copy_text_to_clipboard(&all.join("\n"));
                                    tracing::info!("[Lyric] Copied all lyrics");
                                }
                            }).ok();
                        }
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
                        if let Some(track) = p4.playlist().current_track() {
                            if !track.title.is_empty() || !track.artist.is_empty() {
                                tracing::info!("[LyricDownload] Searching for: {} - {}", track.artist, track.title);
                                // Would open lyric download panel
                                ACTIVE_PANEL.store(6, Ordering::Relaxed);
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_batch_download).on_click(move |_, _, _| {
                        tracing::info!("Batch download lyrics for all tracks");
                        // Would start batch download in background
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_show_trans).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.lyric_show_translation = !this.lyric_show_translation;
                                if let Some(track) = this.player.playlist().current_track() {
                                    this.load_lyrics_for_track(&track.file_path);
                                }
                                cx.notify();
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_show_desktop).on_click({
                        let weak = weak.clone();
                        let player = player.clone();
                        move |_, _, cx| {
                            let already_open = DESKTOP_LYRICS_WINDOW_OPEN.load(Ordering::Relaxed);
                            if !already_open {
                                DESKTOP_LYRICS_WINDOW_OPEN.store(true, Ordering::Relaxed);
                                let player = player.clone();
                                let _ = cx.open_window(
                                    WindowOptions {
                                        titlebar: Some(TitlebarOptions {
                                            title: Some("桌面歌词".into()),
                                            ..Default::default()
                                        }),
                                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                                            origin: Point::default(),
                                            size: gpui::Size { width: px(600.0), height: px(200.0) },
                                        })),
                                        window_min_size: Some(gpui::Size { width: px(300.0), height: px(80.0) }),
                                        ..Default::default()
                                    },
                                    |window, cx| {
                                        let view = cx.new(|cx| DesktopLyricsView::new(player, window, cx));
                                        cx.new(|cx| Root::new(view, window, cx))
                                    },
                                );
                            }
                        }
                    }))
                    .item(PopupMenuItem::new(s_desktop_lock).on_click(|_, _, _| {
                        tracing::info!("Lock/unlock desktop lyrics position");
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_lyric_adv).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.lyric_offset_ms += 500;
                                cx.notify();
                                tracing::info!("[Lyric] offset +0.5s -> {}ms", this.lyric_offset_ms);
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_lyric_ret).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, cx| {
                                this.lyric_offset_ms -= 500;
                                cx.notify();
                                tracing::info!("[Lyric] offset -0.5s -> {}ms", this.lyric_offset_ms);
                            }).ok();
                        }
                    }))
                    .separator()
                    .item(PopupMenuItem::new(s_save_lyric).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, _cx| {
                                // Write the in-memory lyrics back to the .lrc
                                // next to the current track.
                                if let Some(lyrics) = &this.lyric_state.lyrics.clone() {
                                    if let Some(track) = this.player.playlist().current_track() {
                                        let path = std::path::Path::new(&track.file_path);
                                        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                        let parent = path.parent().unwrap_or(std::path::Path::new("."));
                                        let lrc_path = parent.join(format!("{}.lrc", stem));
                                        let lrc_text = lyrics_to_lrc_string(lyrics);
                                        match std::fs::write(&lrc_path, lrc_text.as_bytes()) {
                                            Ok(()) => tracing::info!("[Lyric] 保存到: {}", lrc_path.display()),
                                            Err(e) => tracing::warn!("[Lyric] 保存失败: {}", e),
                                        }
                                    }
                                }
                            }).ok();
                        }
                    }))
                    .item(PopupMenuItem::new(s_assoc_lyric).on_click({
                        let weak = weak.clone();
                        move |_, window, cx| {
                            // Pick a .lrc file, then copy/rename it next to the
                            // current track so it gets auto-loaded.
                            run_blocking_dialog_app(cx, &weak,
                                || rfd::FileDialog::new().add_filter("LRC", &["lrc"]).pick_file(),
                                |path, this, cx| {
                                    if let Some(path) = path {
                                        if let Some(track) = this.player.playlist().current_track() {
                                            let audio = std::path::Path::new(&track.file_path);
                                            let stem = audio.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                            let parent = audio.parent().unwrap_or(std::path::Path::new("."));
                                            let dest = parent.join(format!("{}.lrc", stem));
                                            let _ = std::fs::copy(&path, &dest);
                                            this.load_lyrics_for_track(&track.file_path);
                                            tracing::info!("[Lyric] 关联本地歌词: {}", dest.display());
                                        }
                                    }
                                    cx.notify();
                                });
                        }
                    }))
                    .item(PopupMenuItem::new(s_embed_lyric).on_click({
                        let weak = weak.clone();
                        move |_, _, cx| {
                            weak.update(cx, |this, _cx| {
                                if let (Some(track), Some(lyrics)) = (
                                    this.player.playlist().current_track(),
                                    this.lyric_state.lyrics.clone(),
                                ) {
                                    let lrc_text = lyrics_to_lrc_string(&lyrics);
                                    match crate::tag::writer::set_tag_field(&track.file_path, "lyrics", &lrc_text) {
                                        Ok(()) => tracing::info!("[Lyric] 内嵌歌词成功: {}", track.file_path),
                                        Err(e) => tracing::warn!("[Lyric] 内嵌歌词失败: {}", e),
                                    }
                                }
                            }).ok();
                        }
                    }))
                    }})
            })
            // View menu
            .child({
                let weak = weak.clone();
                let s_view_label = tr.menu_view;
                let s_toggle_playlist = tr.menu_toggle_playlist;
                let s_float_playlist = tr.menu_float_playlist;
                let s_toggle_menubar = tr.menu_toggle_menubar;
                let s_toggle_statusbar = tr.menu_toggle_statusbar;
                let s_mini_mode = tr.menu_mini_mode;
                let s_fullscreen = tr.menu_fullscreen;
                let s_toggle_dark = tr.menu_toggle_dark_mode;
                let s_always_on_top = tr.menu_always_on_top;
                let s_switch_theme = tr.menu_switch_theme;
                let s_toggle_light = tr.menu_toggle_light_mode;
                let pl = self.player.clone();
                layout::menu_dropdown(s_view_label, IconName::LayoutDashboard, move |menu, _, _| {
                let cfg = crate::config::Config::load();
                let dark_mode = cfg.appearance.dark_mode;
                menu.item(PopupMenuItem::new(s_toggle_playlist).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(0, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_float_playlist).on_click({
                    let pl = pl.clone();
                    move |_, _, cx| {
                        open_floating_playlist(cx, pl.clone());
                    }
                }))
                .item(PopupMenuItem::new(s_toggle_menubar).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        MENUBAR_VISIBLE.fetch_xor(true, Ordering::Relaxed);
                        weak.update(cx, |_, cx| cx.notify()).ok();
                        tracing::info!("[View] Menu bar visibility toggled");
                    }
                }))
                .item(PopupMenuItem::new(s_toggle_statusbar).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        STATUSBAR_VISIBLE.fetch_xor(true, Ordering::Relaxed);
                        weak.update(cx, |_, cx| cx.notify()).ok();
                        tracing::info!("[View] Status bar visibility toggled");
                    }
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
                .item(PopupMenuItem::new(if dark_mode { s_toggle_light } else { s_toggle_dark }).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        let mut cfg = crate::config::Config::load();
                        cfg.appearance.dark_mode = !cfg.appearance.dark_mode;
                        let _ = cfg.save();
                        let dark = cfg.appearance.dark_mode;
                        let theme = theme::ThemeName::from_config(&cfg.appearance.theme);
                        weak.update(cx, |this, cx| {
                            this.colours = UiColors::build(dark, &theme);
                            cx.notify();
                        }).ok();
                        tracing::info!("Dark mode toggled to: {}", cfg.appearance.dark_mode);
                    }
                }))
                .item(PopupMenuItem::new(s_switch_theme).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
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
                        let dark = cfg.appearance.dark_mode;
                        let theme = theme::ThemeName::from_config(&cfg.appearance.theme);
                        weak.update(cx, |this, cx| {
                            this.colours = UiColors::build(dark, &theme);
                            cx.notify();
                        }).ok();
                        tracing::info!("Theme switched to: {}", cfg.appearance.theme);
                    }
                }))
                .item(PopupMenuItem::new(s_always_on_top).on_click(|_, _, _| {
                    toggle_always_on_top();
                }))
                })
            })
            // Tools menu
            .child({
                let player = self.player.clone();
                let weak = weak.clone();
                let s_tools_label = tr.menu_tools;
                let s_media_lib = tr.media_lib_title;
                let s_find = tr.menu_find;
                let s_equalizer = tr.menu_equalizer;
                let s_settings = tr.menu_settings;
                let s_browse_dir = tr.menu_browse_dir;
                let s_song_info = tr.menu_song_info;
                let s_format_convert = tr.menu_format_convert;
                let s_charset_convert = tr.menu_charset_convert;
                let s_online_tags = tr.menu_online_tags;
                let s_cover_preview = tr.menu_cover_preview;
                let s_timer_shutdown = tr.menu_timer_shutdown;
                let s_file_assoc = tr.menu_file_association;
                let s_listen_stats = tr.menu_listen_stats;
                let s_dev_progress = tr.menu_dev_progress;
                let s_create_shortcut = tr.menu_create_shortcut;
                let s_reinit_player = tr.menu_reinit_player;
                let s_cover_info = tr.cover_info;
                let s_cover_none = tr.cover_none;
                let s_cover_error = tr.cover_error;
                let s_format_unsupported = tr.format_unsupported;
                layout::menu_dropdown(s_tools_label, IconName::Settings, move |menu, _, _| {
                let player_eq = player.clone();
                menu.item(PopupMenuItem::new(s_media_lib).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(2, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_find).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(3, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_browse_dir).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(1, Ordering::Relaxed);
                }))
                .item(PopupMenuItem::new(s_song_info).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        weak.update(cx, |this, cx| {
                            this.modal = Some(ModalKind::SongInfo);
                            cx.notify();
                        }).ok();
                    }
                }))
                .item(PopupMenuItem::new(s_equalizer).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        weak.update(cx, |_, cx| {
                            ACTIVE_PANEL.store(7, Ordering::Relaxed);
                            cx.notify();
                        }).ok();
                        tracing::info!("[Tools] Open equalizer panel");
                    }
                }))
                .item(PopupMenuItem::new(s_format_convert).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        weak.update(cx, |this, cx| {
                            this.modal = Some(ModalKind::FormatConvert);
                            cx.notify();
                        }).ok();
                    }
                }))
                .item(PopupMenuItem::new(s_charset_convert).on_click({
                    let weak = weak.clone();
                    let p = player_eq.clone();
                    move |_, _, cx| {
                        if let Some(track) = p.playlist().current_track() {
                            let path = track.file_path.clone();
                            weak.update(cx, |this, cx| {
                                // Convert title/artist/album in tag
                                let simplified = crate::charset::to_simplified_chinese(&track.title);
                                let traditional = crate::charset::to_traditional_chinese(&track.title);
                                tracing::info!("[Convert] 简体: {}, 繁体: {}", simplified, traditional);
                                // Also convert lyrics in memory
                                if let Some(lyrics) = &this.lyric_state.lyrics {
                                    if !lyrics.is_empty() {
                                        let mut converted = lyrics.clone();
                                        let s = crate::charset::to_simplified_chinese(
                                            &lyrics.lines.iter().map(|l| lyrics.display_text(l)).collect::<Vec<_>>().join("\n")
                                        );
                                        if !s.is_empty() {
                                            // Update all lines with simplified text
                                            for (i, line) in converted.lines.iter_mut().enumerate() {
                                                if i < lyrics.lines.len() {
                                                    line.text = crate::charset::to_simplified_chinese(&line.text);
                                                }
                                            }
                                            this.lyric_state.update(Some(converted), (this.position * 1000.0) as u64);
                                            tracing::info!("[Convert] 歌词已转换");
                                        }
                                    }
                                }
                                // Write converted tags back to file
                                let _ = crate::tag::writer::set_tag_field(&path, "title", &crate::charset::to_simplified_chinese(&track.title));
                                let _ = crate::tag::writer::set_tag_field(&path, "artist", &crate::charset::to_simplified_chinese(&track.artist));
                                let _ = crate::tag::writer::set_tag_field(&path, "album", &crate::charset::to_simplified_chinese(&track.album));
                                cx.notify();
                            }).ok();
                        }
                    }
                }))
                .item(PopupMenuItem::new(s_online_tags).on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            tracing::info!("[OnlineTag] 在线获取标签: {}", track.file_path);
                            let path = track.file_path.clone();
                            runtime().spawn_blocking(move || {
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
                .item(PopupMenuItem::new(s_cover_preview).on_click({
                    let p = player_eq.clone();
                    move |_, _, _| {
                        if let Some(track) = p.playlist().current_track() {
                            match crate::tag::writer::read_pictures(&track.file_path) {
                                Ok(pics) => {
                                    let count = pics.len();
                                    let info = if count > 0 {
                                        s_cover_info
                                            .replace("{0}", &count.to_string())
                                            .replace("{1}", &pics[0].0)
                                            .replace("{2}", &(pics[0].1.len() / 1024).to_string())
                                    } else {
                                        s_cover_none.to_string()
                                    };
                                    let _ = rfd::MessageDialog::new()
                                        .set_title(s_cover_preview)
                                        .set_description(&info)
                                        .show();
                                }
                                Err(e) => {
                                    let _ = rfd::MessageDialog::new()
                                        .set_title(s_cover_preview)
                                        .set_description(&s_cover_error.replace("{0}", &e.to_string()))
                                        .show();
                                }
                            }
                        }
                    }
                }))
                .item(PopupMenuItem::new(s_timer_shutdown).on_click(|_, _, _| {
                    if SLEEP_TIMER_ACTIVE.load(Ordering::Relaxed) {
                        SLEEP_TIMER_ACTIVE.store(false, Ordering::Relaxed);
                        tracing::info!("[SleepTimer] 已取消");
                    } else {
                        SLEEP_TIMER_ACTIVE.store(true, Ordering::Relaxed);
                        runtime().spawn_blocking(|| {
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
                .item(PopupMenuItem::new(s_file_assoc).on_click(|_, _, _| {
                    crate::commands::system::cmd_file_assoc(&crate::cli::FileAssocArgs {
                        action: crate::cli::FileAssocAction::Register,
                    }).ok();
                }))
                .item(PopupMenuItem::new(s_listen_stats).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        let stats = crate::play_stats::top_stats(20);
                        let total_secs = crate::play_stats::total_listen_secs();
                        let total_plays = crate::play_stats::total_play_count();
                        let total_tracks = crate::play_stats::total_track_count();
                        let hours = total_secs / 3600;
                        let mins = (total_secs % 3600) / 60;
                        tracing::info!(
                            "=== 收听统计 === 总曲目: {}, 总播放: {}, 总时长: {}h{}m",
                            total_tracks, total_plays, hours, mins
                        );
                        for (i, (path, entry)) in stats.iter().enumerate() {
                            if i >= 10 { break; }
                            let h = entry.listen_secs / 3600;
                            let m = (entry.listen_secs % 3600) / 60;
                            tracing::info!("  #{:2} {:>4}h{:02}m  {:>4}次  {}", i+1, h, m, entry.play_count, path);
                        }
                    }
                }))
                .item(PopupMenuItem::new(s_dev_progress).on_click(|_, _, _| {
                    #[cfg(windows)]
                    let _ = std::process::Command::new("cmd").args(&["/c", "start", "docs\\缺陷分析与改进计划.md"]).spawn();
                    #[cfg(not(windows))]
                    let _ = std::process::Command::new("xdg-open").arg("docs/缺陷分析与改进计划.md").spawn();
                    tracing::info!("[Dev] 打开开发进度文档");
                }))
                .item(PopupMenuItem::new(s_create_shortcut).on_click(|_, _, _| {
                    #[cfg(windows)]
                    {
                        use std::process::Command;
                        let exe = std::env::current_exe().ok().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                        let desktop = std::env::var("USERPROFILE").unwrap_or_default() + "\\Desktop";
                        let lnk = format!("{}\\HackMagic Music Player.lnk", desktop);
                        let _ = Command::new("powershell")
                            .args(&["-Command", &format!("$ws = New-Object -ComObject WScript.Shell; $sc = $ws.CreateShortcut('{}'); $sc.TargetPath = '{}'; $sc.Save()", lnk, exe)])
                            .status();
                        tracing::info!("[Shortcut] 创建快捷方式: {}", lnk);
                    }
                    #[cfg(not(windows))]
                    {
                        tracing::info!("[Shortcut] 创建快捷方式 - 仅在 Windows 支持");
                    }
                }))
                .item(PopupMenuItem::new(s_reinit_player).on_click({
                    let weak = weak.clone();
                    let p = player_eq.clone();
                    move |_, _, cx| {
                        // Reset player state: stop, clear playlist, reset EQ, reset speed/pitch
                        let _ = p.stop();
                        p.playlist_mut().clear();
                        p.eq_reset().ok();
                        let _ = p.set_speed(1.0);
                        let _ = p.set_pitch(0);
                        p.eq_enable(false);
                        weak.update(cx, |this, cx| {
                            this.title.clear();
                            this.artist.clear();
                            this.album.clear();
                            this.position = 0.0;
                            this.duration = 0.0;
                            this.is_playing = false;
                            this.lyric_state.clear();
                            this.lyric_offset_ms = 0;
                            this.playlist_selected.clear();
                            cx.notify();
                        }).ok();
                        tracing::info!("[Player] 播放器已重新初始化");
                    }
                }))
                .separator()
                .item(PopupMenuItem::new(s_settings).on_click(|_, _, _| {
                    ACTIVE_PANEL.store(5, Ordering::Relaxed);
                }))
                })
            })
            // Help menu
            .child({
                let weak = weak.clone();
                let s_help_label = tr.menu_help;
                let s_help_content = tr.menu_help_content;
                let s_about = tr.menu_about;
                let s_online_help = tr.menu_online_help;
                let s_check_update = tr.menu_check_update;
                let s_supported_formats = tr.menu_supported_formats;
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
                .item(PopupMenuItem::new(s_online_help).on_click(|_, _, _| {
                    tracing::info!("Opening online help");
                    #[cfg(windows)]
                    let _ = std::process::Command::new("cmd").args(&["/c", "start", "https://github.com/zhongyang219/MusicPlayer2"]).spawn();
                    #[cfg(not(windows))]
                    let _ = std::process::Command::new("xdg-open").arg("https://github.com/zhongyang219/MusicPlayer2").spawn();
                }))
                .item(PopupMenuItem::new(s_check_update).on_click(|_, _, _| {
                    crate::commands::system::check_update_background();
                }))
                .separator()
                .item(PopupMenuItem::new(s_supported_formats).on_click(|_, _, _| {
                    let formats = crate::audio_common::supported_extensions();
                    tracing::info!("Supported formats: {:?}", formats);
                }))
                .item(PopupMenuItem::new(s_about).on_click({
                    let weak = weak.clone();
                    move |_, _, cx| {
                        weak.update(cx, |this, cx| {
                            this.modal = Some(ModalKind::About);
                            cx.notify();
                        }).ok();
                    }
                }))
                })
            })
    }

    /// Render a modal dialog overlay (About / Song Info / Format Convert).
    fn render_modal(
        &self,
        kind: ModalKind,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let close_btn = Button::new("modal_close")
            .label(self.tr.ctrl_stop)
            .ghost()
            .on_click(cx.listener(|this, _, _window, _cx| {
                this.modal = None;
            }));

        let card = match kind {
            ModalKind::About => {
                v_flex()
                    .w(px(420.0))
                    .max_h(px(520.0))
                    .rounded(px(12.0))
                    .bg(c.bg)
                    .p_6()
                    .gap_4()
                    .child(dialogs::render_about_dialog(self.tr, c))
                    .child(h_flex().w_full().justify_end().child(close_btn))
                    .into_any_element()
            }
            ModalKind::SongInfo => {
                let info = self.song_info_text();
                v_flex()
                    .w(px(460.0))
                    .max_h(px(560.0))
                    .rounded(px(12.0))
                    .bg(c.bg)
                    .p_6()
                    .gap_3()
                    .child(layout::txt(self.tr.menu_song_info, 16.0, c.text_title))
                    .child(div().w_full().h(px(1.0)).bg(c.divider))
                    .child(
                        v_flex().w_full().gap_1()
                            .children(info.iter().map(|(k, v)| {
                                h_flex().w_full().gap_2()
                                    .child(div().w(px(96.0)).child(layout::txt(k, 11.0, c.text_dim)))
                                    .child(layout::txt(v, 12.0, c.text))
                            }))
                    )
                    .child(h_flex().w_full().justify_end().child(close_btn))
                    .into_any_element()
            }
            ModalKind::FormatConvert => {
                self.render_format_convert_card(c, cx)
            }
            ModalKind::TrackEditor => {
                let weak = cx.entity().downgrade();
                let weak_save = weak.clone();
                v_flex()
                    .w(px(400.0))
                    .max_h(px(520.0))
                    .rounded(px(12.0))
                    .bg(c.bg)
                    .p_6()
                    .gap_3()
                    .child(layout::txt("编辑曲目信息", 16.0, c.text_title))
                    .child(div().w_full().h(px(1.0)).bg(c.divider))
                    .child(self.render_editor_field("标题", &self.editor_title_input, c))
                    .child(self.render_editor_field("艺术家", &self.editor_artist_input, c))
                    .child(self.render_editor_field("专辑", &self.editor_album_input, c))
                    .child(self.render_editor_field("流派", &self.editor_genre_input, c))
                    .child(self.render_editor_field("年份", &self.editor_year_input, c))
                    .child(self.render_editor_field("曲目号", &self.editor_track_num_input, c))
                    .child(self.render_editor_field("评分", &self.editor_rating_input, c))
                    .child(h_flex().w_full().justify_end().gap_2()
                        .child(close_btn)
                        .child(Button::new("editor_save").label("保存").on_click(move |_, _, cx| {
                            let _ = weak_save.update(cx, |this, cx| {
                                if let Some(idx) = this.editor_track_idx {
                                    let title = this.editor_title_input.read(cx).value().to_string();
                                    let artist = this.editor_artist_input.read(cx).value().to_string();
                                    let album = this.editor_album_input.read(cx).value().to_string();
                                    let genre = this.editor_genre_input.read(cx).value().to_string();
                                    let year: u32 = this.editor_year_input.read(cx).value().parse().unwrap_or(0);
                                    let track_num: u32 = this.editor_track_num_input.read(cx).value().parse().unwrap_or(0);
                                    let rating: u32 = this.editor_rating_input.read(cx).value().parse().unwrap_or(0);
                                    if let Some(track) = this.player.playlist_mut().get_mut(idx) {
                                        track.title = title;
                                        track.artist = artist;
                                        track.album = album;
                                        track.genre = genre;
                                        track.year = year;
                                        track.track_number = track_num;
                                        track.rating = rating.min(5);
                                    }
                                    this.modal = None;
                                }
                                cx.notify();
                            });
                        }))
                    )
                    .into_any_element()
            }
        };

        div().size_full()
            .bg(gpui::black().opacity(0.45))
            .flex()
            .items_center()
            .justify_center()
            .child(card)
    }

    /// Build a key/value list describing the currently playing track.
    fn song_info_text(&self) -> Vec<(String, String)> {
        let mut rows = Vec::new();
        if let Some(track) = self.player.playlist().current_track() {
            rows.push(("标题".into(), if track.title.is_empty() { track.file_name.clone() } else { track.title.clone() }));
            rows.push(("艺术家".into(), track.artist.clone()));
            rows.push(("专辑".into(), track.album.clone()));
            let d = track.duration;
            rows.push(("时长".into(), format!("{:02}:{:02}", d.as_secs() / 60, d.as_secs() % 60)));
            rows.push(("类型".into(), track.file_type.clone()));
            rows.push(("比特率".into(), if track.bitrate > 0 { format!("{} kbps", track.bitrate) } else { "--".into() }));
            rows.push(("采样率".into(), if track.sample_rate > 0 { format!("{} Hz", track.sample_rate) } else { "--".into() }));
            rows.push(("声道".into(), if track.channels > 0 { format!("{}", track.channels) } else { "--".into() }));
            rows.push(("收藏".into(), if track.is_favourite { "是".into() } else { "否".into() }));
            rows.push(("文件路径".into(), track.file_path.clone()));
        } else {
            rows.push(("提示".into(), "当前没有播放的曲目".into()));
        }
        rows
    }

    /// Render a labeled input row for the track editor modal.
    fn render_editor_field(&self, label: &str, input: &Entity<InputState>, c: &UiColors) -> impl IntoElement {
        h_flex().w_full().gap_2().items_center()
            .child(div().w(px(64.0)).child(layout::txt(label, 11.0, c.text_dim)))
            .child(
                h_flex().flex_grow().h(px(28.0)).bg(c.bg).rounded(px(4.0)).px_2().items_center()
                    .child(Input::new(input))
            )
    }

    /// Render the format-conversion modal card. Picking a target format opens a
    /// file picker and converts via the bundled ffmpeg (if available).
    fn render_format_convert_card(
        &self,
        c: &UiColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let close_btn = Button::new("fc_close2")
            .label("关闭")
            .ghost()
            .on_click(cx.listener(|this, _, _window, _cx| {
                this.modal = None;
            }));

        let formats: [(&'static str, &'static str); 6] = [
            ("fc_mp3", "mp3"),
            ("fc_flac", "flac"),
            ("fc_wav", "wav"),
            ("fc_ogg", "ogg"),
            ("fc_aac", "aac"),
            ("fc_m4a", "m4a"),
        ];
        let fmt_btns: Vec<AnyElement> = formats.iter().map(|(id, fmt)| {
            let fmt = *fmt;
            Button::new(*id)
                .label(format!("转为 .{}", fmt))
                .compact()
                .ghost()
                .on_click(cx.listener(move |this, _, _window, cx| {
                    let weak = cx.entity().downgrade();
                    let fmt = fmt;
                    run_blocking_dialog_app(cx, &weak,
                        || rfd::FileDialog::new()
                            .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                            .pick_file(),
                        move |file, this, cx| {
                            if let Some(file) = file {
                                let src = file.to_string_lossy().to_string();
                                let dst = std::path::Path::new(&src).with_extension(fmt).to_string_lossy().to_string();
                                match std::process::Command::new("ffmpeg")
                                    .args(["-y", "-i", &src, &dst])
                                    .status()
                                {
                                    Ok(status) if status.success() => {
                                        tracing::info!("[Convert] 转换成功: {} -> {}", src, dst);
                                    }
                                    Ok(status) => {
                                        tracing::error!("[Convert] ffmpeg 退出码: {:?}", status.code());
                                    }
                                    Err(e) => {
                                        tracing::error!("[Convert] 未找到 ffmpeg，或转换失败: {}", e);
                                    }
                                }
                            }
                            cx.notify();
                        });
                }))
                .into_any_element()
        }).collect();

        v_flex()
            .w(px(440.0))
            .max_h(px(520.0))
            .rounded(px(12.0))
            .bg(c.bg)
            .p_6()
            .gap_3()
            .child(layout::txt("格式转换", 16.0, c.text_title))
            .child(layout::txt("选择目标格式，然后挑选要转换的音频文件（需系统已安装 ffmpeg）。", 11.0, c.text_dim))
            .child(div().w_full().h(px(1.0)).bg(c.divider))
            .child(
                v_flex().w_full().gap_2()
                    .children(fmt_btns)
            )
            .child(h_flex().w_full().justify_end().child(close_btn))
            .into_any_element()
    }

    /// Render the "Open URL" dialog overlay. Shows a text input where the
    /// user can paste a URL (e.g. an http stream or podcast), then opens it
    /// via the player's `play_file` method (which delegates to the engine).
    fn render_url_dialog(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let close_btn = Button::new("url_close")
            .label("关闭")
            .ghost()
            .on_click(cx.listener(|this, _, _window, _cx| {
                this.url_dialog_open = false;
            }));

        v_flex()
            .size_full()
            .justify_center()
            .items_center()
            .bg(gpui::Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.4 })
            .child(
                v_flex()
                    .w(px(440.0))
                    .rounded(px(12.0))
                    .bg(c.bg)
                    .p_6()
                    .gap_4()
                    .child(layout::txt("打开网络音频流", 16.0, c.text_title))
                    .child(layout::txt("输入音频流 URL（支持常见流媒体协议如 http、https、mms）：", 11.0, c.text_dim))
                    .child(
                        div()
                            .w_full()
                            .h(px(32.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(c.border)
                            .bg(c.panel)
                            .px_3()
                            .child(
                                gpui_component::input::Input::new(&self.url_state)
                                    .w_full()
                            )
                    )
                    .child(
                        h_flex().w_full().justify_end().gap_2()
                            .child(close_btn)
                            .child(Button::new("url_play")
                                .label("播放")
                                .primary()
                                .on_click(cx.listener(|this, _, _window, _cx| {
                                    let url = this.url_state.read(_cx).value().to_string();
                                    if !url.is_empty() {
                                        let _ = this.player.play_file(&url);
                                        tracing::info!("[URL] 播放网络流: {}", url);
                                    }
                                    this.url_dialog_open = false;
                                })))
                    )
            )
            .into_any_element()
    }

    /// Render the lyric editor panel.
    fn render_lyric_editor_panel(
        &self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
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
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Err(e) = this.editor_state.save() {
                        tracing::warn!("[LyricEditor] 保存失败: {}", e);
                    }
                })))
            .child(Button::new("lyr_save_as").label("另存为...").compact()
                .on_click(cx.listener(|this, _, _window, cx| {
                    run_blocking_dialog(cx,
                        || rfd::FileDialog::new()
                            .set_file_name("lyric.lrc")
                            .add_filter("LRC", &["lrc"])
                            .save_file(),
                        |path, this, cx| {
                            if let Some(path) = path {
                                let p = path.to_string_lossy().to_string();
                                if let Err(e) = this.editor_state.save_as(&p) {
                                    tracing::warn!("[LyricEditor] 另存为失败: {}", e);
                                }
                            }
                            cx.notify();
                        });
                })))
            .child(Button::new("lyr_open").label("打开").compact()
                .on_click(cx.listener(|this, _, _window, cx| {
                    run_blocking_dialog(cx,
                        || rfd::FileDialog::new()
                            .add_filter("LRC", &["lrc"])
                            .pick_file(),
                        |path, this, cx| {
                            if let Some(path) = path {
                                let p = path.to_string_lossy().to_string();
                                if let Err(e) = this.editor_state.load_from_file(&p) {
                                    tracing::warn!("[LyricEditor] 打开失败: {}", e);
                                }
                            }
                            cx.notify();
                        });
                })))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_add").label("插入行").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    let at = this.editor_state.selected_row
                        .unwrap_or(this.editor_state.rows.len().saturating_sub(1));
                    this.editor_state.insert_row(at);
                })))
            .child(Button::new("lyr_delete").label("删除行").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Some(idx) = this.editor_state.selected_row {
                        this.editor_state.delete_row(idx);
                    }
                })))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_back_500").label("-500ms").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Some(idx) = this.editor_state.selected_row {
                        this.editor_state.adjust_timing(idx, -500);
                    }
                })))
            .child(Button::new("lyr_back_100").label("-100ms").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Some(idx) = this.editor_state.selected_row {
                        this.editor_state.adjust_timing(idx, -100);
                    }
                })))
            .child(Button::new("lyr_fwd_100").label("+100ms").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Some(idx) = this.editor_state.selected_row {
                        this.editor_state.adjust_timing(idx, 100);
                    }
                })))
            .child(Button::new("lyr_fwd_500").label("+500ms").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    if let Some(idx) = this.editor_state.selected_row {
                        this.editor_state.adjust_timing(idx, 500);
                    }
                })))
            .child(div().w(px(1.0)).h(px(16.0)).bg(c.divider))
            .child(Button::new("lyr_shift_all").label("全部偏移").compact()
                .on_click(cx.listener(|this, _, _window, _cx| {
                    this.editor_state.shift_all_timing(100);
                })))
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
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this: &mut MusicPlayer, _, _, _| {
                            this.editor_state.selected_row = Some(i);
                        }))
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

                            // Spawn background async task to perform search
                            runtime().spawn(async move {
                                let result = lyric_download::search_lyrics(&source, &keyword).await;
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

                                    runtime().spawn(async move {
                                        let result = lyric_download::download_lyric(
                                            &source,
                                            &song_id,
                                            include_translation,
                                        )
                                        .await;
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

    fn render_playlist_panel(
        &self,
        tracks: &[Track],
        _layout_mode: LayoutMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let c = &self.colours;
        let total = tracks.len();
        let fav_count = tracks.iter().filter(|t| t.is_favourite).count();
        let recent_count = self.recent_tracks.len();
        let cur = self.playlist_filter_mode;
        let weak = cx.entity().downgrade();

        let entries: [(&'static str, PlaylistFilterMode, &'static str, IconName, usize); 3] = [
            ("pl_default", PlaylistFilterMode::All, "默认列表", IconName::LayoutDashboard, total),
            ("pl_fav", PlaylistFilterMode::Favorites, "我喜欢的音乐", IconName::Heart, fav_count),
            ("pl_recent", PlaylistFilterMode::Recent, "最近播放", IconName::Calendar, recent_count),
        ];

        v_flex()
            .w(px(LEFT_PANEL_WIDTH))
            .h_full()
            .bg(c.panel)
            .child(
                h_flex()
                    .items_center().justify_between()
                    .w_full().px_3().h(px(36.0))
                    .bg(c.panel_alt)
                    .child(layout::txt("播放列表", 13.0, c.text_title))
                    .child(
                        Button::new("pl_add")
                            .icon(IconName::Plus).compact().ghost()
                            .text_color(c.text_dim)
                            .on_click(cx.listener(|this, _, _window, _cx| {
                                tracing::info!("[Playlist] Add playlist requested");
                                let _ = this;
                            }))
                    )
            )
            .child(
                v_flex().w_full().py_1()
                    .children(entries.iter().map(|(id, mode, name, icon, count)| {
                        let selected = cur == *mode;
                        let text_color = if selected { c.accent } else { c.text };
                        let m = *mode;
                        let w = weak.clone();
                        Button::new(*id)
                            .icon(icon.clone())
                            .label(format!("{}  ({})", name, count))
                            .w_full().justify_start()
                            .px_3().h(px(32.0))
                            .ghost()
                            .bg(if selected { c.accent.opacity(0.18) } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                            .text_color(text_color)
                            .on_click(move |_, _, cx| {
                                let _ = w.update(cx, |this, cx| {
                                    this.playlist_filter_mode = m;
                                    ACTIVE_PANEL.store(0, Ordering::Relaxed);
                                    cx.notify();
                                });
                            })
                    }))
            )
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

        // Filter tracks based on search text and filter mode
        let filtered_indices: Vec<usize> = tracks.iter().enumerate().filter_map(|(i, track)| {
            let matches_mode = match filter_mode {
                PlaylistFilterMode::All => true,
                PlaylistFilterMode::Artist => !track.artist.is_empty(),
                PlaylistFilterMode::Album => !track.album.is_empty(),
                PlaylistFilterMode::Genre => !track.genre.is_empty(),
                PlaylistFilterMode::Favorites => track.is_favourite,
                PlaylistFilterMode::Recent => self.recent_tracks.iter().any(|p| p == &track.file_path),
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
        let row_height = self.config_cache.appearance.playlist_row_height as f32;
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
        let header_text = if !filter_text.is_empty() {
            format!("筛选结果: {} / {}", filtered_count, total_count)
        } else if filter_mode == PlaylistFilterMode::All {
            format!("播放列表 ({} 首)", total_count)
        } else if filter_mode == PlaylistFilterMode::Favorites {
            format!("我喜欢的音乐 ({} 首)", filtered_count)
        } else if filter_mode == PlaylistFilterMode::Recent {
            format!("最近播放 ({} 首)", filtered_count)
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
                            .child(
                                h_flex().gap_1()
                                    .child(Button::new("pl_add").icon(IconName::Plus).compact().ghost()
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            run_blocking_dialog(cx,
                                                || -> Option<std::path::PathBuf> {
                                                    rfd::FileDialog::new()
                                                        .add_filter("音频文件", &["mp3", "flac", "wav", "ogg", "aac", "m4a", "wma", "ape"])
                                                        .pick_file()
                                                },
                                                |file, this, cx| {
                                                    if let Some(file) = file {
                                                        let _ = this.player.play_file(file.to_str().unwrap_or_default());
                                                    }
                                                    cx.notify();
                                                });
                                        })))
                                    .child(Button::new("pl_import").icon(IconName::ArrowDown).compact().ghost()
                                        .on_click(cx.listener(|this, _, _window, cx| {
                                            run_blocking_dialog(cx,
                                                || -> Option<std::path::PathBuf> {
                                                    rfd::FileDialog::new()
                                                        .add_filter("播放列表", &["m3u", "m3u8", "wpl", "ttpl", "playlist"])
                                                        .pick_file()
                                                },
                                                |file, this, cx| {
                                                    if let Some(file) = file {
                                                        let path = file.to_string_lossy().to_string();
                                                        let ext = std::path::Path::new(&path).extension()
                                                            .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                                                        if let Ok(tracks) = match ext.as_str() {
                                                            "m3u" | "m3u8" => crate::core::playlist::Playlist::import_m3u(&path).map_err(|e| e.to_string()),
                                                            "wpl" | "ttpl" | "playlist" => {
                                                                match crate::playlist_format::read_playlist(&path) {
                                                                    Ok(paths) => Ok(paths.into_iter().map(|p| crate::core::playlist::Track::new(&p)).collect()),
                                                                    Err(e) => Err(e.to_string()),
                                                                }
                                                            }
                                                            _ => Err(crate::gui::i18n::global_tr().format_unsupported.to_string()),
                                                        } {
                                                            let count = tracks.len();
                                                            this.player.playlist_mut().add_tracks(tracks);
                                                            tracing::info!("[Playlist] 导入 {} 首", count);
                                                        }
                                                    }
                                                    cx.notify();
                                                });
                                        })))
                                    .child(Button::new("pl_clear").label("清空").compact().ghost()
                                        .on_click(cx.listener(|this, _, _w, _cx| {
                                            this.player.playlist_mut().clear();
                                        })))
                            )
                    )
                    // Search and filter bar with proper Input
                    .child(
                        h_flex()
                            .w_full()
                            .px_4().py_1()
                            .gap_2()
                            .bg(c.panel_alt)
                            .items_center()
                            .child(
                                // Search input area
                                h_flex()
                                    .flex_grow()
                                    .h(px(24.0))
                                    .bg(c.bg)
                                    .rounded(px(4.0))
                                    .px_1()
                                    .items_center()
                                    .child(
                                        gpui_component::input::Input::new(&self.search_input)
                                    )
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
                if self.playlist_view_mode == PlaylistViewMode::Detail {
                    h_flex()
                    .w_full()
                    .h(px(24.0))
                    .px_4().gap_3()
                    .bg(c.panel_alt)
                    .items_center()
                    .child(div().w(px(10.0))) // align with drag handle
                    .child(div().w(px(28.0))) // align with row index column
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
                            }                            ))
                    )
                } else {
                    div().h(px(0.0))
                }
            )
            .child({
                let n = filtered_count;
                let font_size = font_size;
                let row_height = row_height;
                let layout_mode = layout_mode;
                let c2 = self.colours.clone();
                let cv = view.clone();
                let cp = player.clone();
                let ci = current_idx;
                let play_vm = self.playlist_view_mode;
                let da = self.playlist_drag_active;
                let dt = self.playlist_drag_to;
                let scroll_handle = self.playlist_scroll_handle.clone();
                let indices = filtered_indices;
                let tracks2 = tracks.to_vec();
                let s_play = self.tr.ctrl_play;
                let s_play_next = self.tr.menu_play_next;
                let s_remove = self.tr.menu_remove_track;
                let s_fav = self.tr.menu_favourite;
                let s_unfav = self.tr.menu_unfavourite;
                let s_clear_rating = self.tr.menu_clear_rating;
                let s_r1 = self.tr.menu_rating_1;
                let s_r2 = self.tr.menu_rating_2;
                let s_r3 = self.tr.menu_rating_3;
                let s_r4 = self.tr.menu_rating_4;
                let s_r5 = self.tr.menu_rating_5;
                let s_open_location = self.tr.menu_open_file_location;
                let s_copy_path = self.tr.menu_copy_path;
                let s_properties = self.tr.menu_properties;

                uniform_list("playlist_list", n, move |range, _window, _cx| {
                    range.map(|rel_i| {
                        let i = indices[rel_i];
                        let track = &tracks2[i];
                        let is_current = ci == Some(i);
                        let bg = if is_current { c2.playlist_playing } else if i % 2 == 0 { c2.playlist_item } else { c2.playlist_item_hover };
                        let text_color = if is_current { c2.accent } else { c2.text };
                        let display_name = if !track.title.is_empty() {
                            track.title.clone()
                        } else {
                            track.file_name.clone()
                        };
                        let duration = track.duration;
                        let dur_str = format!("{:02}:{:02}", duration.as_secs() / 60, duration.as_secs() % 60);
                        let file_path = track.file_path.clone();
                        let is_fav = track.is_favourite;
                        let ctx_player = cp.clone();

                        let is_drop_target = da && dt == Some(i);
                        let row_bg = if is_drop_target {
                            c2.accent.opacity(0.3)
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
                            .hover(|s| s.bg(if is_drop_target { c2.accent.opacity(0.4) } else { c2.playlist_item_selected }))
                            .cursor(gpui::CursorStyle::PointingHand)
                            .child(
                                div()
                                    .w(px(10.0))
                                    .h(px(row_height - 4.0))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor(gpui::CursorStyle::PointingHand)
                                    .text_color(c2.text_dim)
                                    .text_size(px(10.0))
                                    .child("⠿")
                                    .on_mouse_down(gpui::MouseButton::Left, {
                                        let v = cv.clone();
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
                                        let v = cv.clone();
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
                            .child(div().w(px(28.0)).child(layout::txt(&(i + 1).to_string(), font_size, c2.text_dim)))
                            .child(h_flex().flex_grow().child(layout::txt(&display_name, font_size, text_color)))
                            .child(if matches!(layout_mode, LayoutMode::Big) {
                                div().w(px(90.0)).child(layout::txt(&track.artist, font_size, c2.text_dim)).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(if matches!(layout_mode, LayoutMode::Big) {
                                div().w(px(90.0)).child(layout::txt(&track.album, font_size, c2.text_dim)).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(if is_fav { layout::txt("♥", font_size, c2.accent) } else { layout::txt("", font_size, c2.text_dim) })
                            .child(if play_vm == PlaylistViewMode::Detail {
                                layout::txt(&dur_str, font_size - 1.0, c2.text_dim).into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .on_click({
                                let v = cv.clone();
                                let idx = i;
                                let p = cp.clone();
                                move |e, _, _cx| {
                                    if e.modifiers().control {
                                        let _ = v.update(_cx, |this, _| {
                                            if this.playlist_selected.contains(&idx) {
                                                this.playlist_selected.remove(&idx);
                                            } else {
                                                this.playlist_selected.insert(idx);
                                            }
                                        });
                                    } else {
                                        let _ = v.update(_cx, |this, _| {
                                            this.playlist_selected.clear();
                                            this.playlist_selected.insert(idx);
                                        });
                                        let p2 = p.clone();
                                        runtime().spawn_blocking(move || { let _ = p2.play_at_index(idx); });
                                    }
                                }
                            })
                            .on_mouse_move({
                                let v = cv.clone();
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
                                let s_fav_label = if fav { s_unfav } else { s_fav };

                                menu.item(PopupMenuItem::new(s_play).on_click(move |_, _, _| {
                                    let p = p_play.clone();
                                    runtime().spawn_blocking(move || { let _ = p.play_at_index(idx); });
                                }))
                                .item(PopupMenuItem::new(s_play_next).on_click(move |_, _, _| {
                                    p_next.push_next_track(idx);
                                }))
                                .separator()
                                .item(PopupMenuItem::new(s_remove).on_click(move |_, _, _| {
                                    p_remove.playlist_mut().remove(idx);
                                }))
                                .item(PopupMenuItem::new(s_fav_label).on_click(move |_, _, _| {
                                    p_fav.playlist_mut().toggle_favourite(idx);
                                }))
                                .separator()
                                .item(PopupMenuItem::new(s_r1).on_click(move |_, _, _| {
                                    p_r1.playlist_mut().set_rating(idx, 1);
                                }))
                                .item(PopupMenuItem::new(s_r2).on_click(move |_, _, _| {
                                    p_r2.playlist_mut().set_rating(idx, 2);
                                }))
                                .item(PopupMenuItem::new(s_r3).on_click(move |_, _, _| {
                                    p_r3.playlist_mut().set_rating(idx, 3);
                                }))
                                .item(PopupMenuItem::new(s_r4).on_click(move |_, _, _| {
                                    p_r4.playlist_mut().set_rating(idx, 4);
                                }))
                                .item(PopupMenuItem::new(s_r5).on_click(move |_, _, _| {
                                    p_r5.playlist_mut().set_rating(idx, 5);
                                }))
                                .item(PopupMenuItem::new(s_clear_rating).on_click(move |_, _, _| {
                                    p_r0.playlist_mut().set_rating(idx, 0);
                                }))
                                .separator()
                                .item(PopupMenuItem::new(s_open_location).on_click({
                                    let fp_loc2 = fp_loc.clone();
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
                                }}))
                                .item(PopupMenuItem::new(s_copy_path).on_click(move |_, _, _| {
                                }))
                                .item(PopupMenuItem::new(s_properties).on_click({
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
                                        let days = secs / 86400;
                                        let year = 1970 + (days / 365) as u32;
                                        let month = ((days % 365) / 30 + 1) as u32;
                                        let day = ((days % 365) % 30 + 1) as u32;
                                        format!("{}-{:02}-{:02}", year, month, day)
                                    }).unwrap_or_else(|| "--".to_string());
                                let info = format!(
                                    "标题: {}\n艺术家: {}\n专辑: {}\n文件名: {}\n路径: {}\n时长: {}\n收藏: {}\n类型: {}\n比特率: {}\n采样率: {}\n\n--- 高级信息 ---\n文件大小: {}\n修改日期: {}",
                                    dn2, "", "", "", fp_loc2, "", if fav2 { "是" } else { "否" }, "", "--", "--", file_size, mod_time,
                                );
                                let _ = rfd::MessageDialog::new()
                                    .set_title(s_properties)
                                    .set_description(&info)
                                    .show();
                            }}))
                            })
                            .into_any_element()
                     }).collect::<Vec<_>>()
                })
                .track_scroll(scroll_handle)
                .flex_grow()
            })
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
                            .on_click(cx.listener(|this, _, _w, cx| {
                                run_blocking_dialog(cx,
                                    || rfd::FileDialog::new().pick_folder(),
                                    |folder, this, cx| {
                                        if let Some(folder) = folder {
                                            let dir = folder.to_string_lossy().to_string();
                                            if let Ok(entries) = crate::media::scan_directory(&dir, true, None) {
                                                let mut pl = this.player.playlist_mut();
                                                let first_idx = pl.len();
                                                for e in &entries {
                                                    pl.add_track(crate::core::playlist::Track::new(&e.file_path));
                                                }
                                                if !entries.is_empty() {
                                                    let p = this.player.clone();
                                                    runtime().spawn_blocking(move || { let _ = p.play_at_index(first_idx); });
                                                }
                                                tracing::info!("[FileBrowser] 打开文件夹: {} ({} 首)", dir, entries.len());
                                            }
                                        }
                                        cx.notify();
                                    });
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
    /// Recomputes sidebar/list data only on category/selection change
    /// (equivalent to MFC virtual list: data prepared once, not per-frame).
    fn render_media_lib_panel(
        &mut self,
        c: &UiColors,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Lazy-load cache if missing; refresh once per render is cheap because
        // MediaLib::load() is just a serde_json::from_slice of a small file.
        if self.media_lib_cache.is_none() {
            self.media_lib_cache = Some(crate::media::MediaLib::load());
        }

        let cat = self.media_lib_category;
        let sel = self.media_lib_selected.clone();

        // Recompute sidebar + list only when category or selection changes.
        if self.media_lib_cache_key != (cat, sel.clone()) {
            self.media_lib_cache_key = (cat, sel.clone());
            if let Some(lib) = self.media_lib_cache.as_ref() {
                self.media_lib_total_dur = lib.entries.iter().map(|e| e.duration_secs).sum();
                const MAX_ENTRIES: usize = 500;
                let (sidebar, list): (Vec<String>, Vec<ViewEntry>) = match cat {
                    MediaLibCategory::AllTracks => (
                        Vec::new(),
                        lib.entries.iter().take(MAX_ENTRIES).map(ViewEntry::from).collect(),
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
                self.media_lib_sidebar_cache = sidebar;
                self.media_lib_list_cache = list;
            }
        }

        // Read stats from cache (cheap: no per-entry iteration on every frame).
        let total_tracks = self.media_lib_cache.as_ref().map_or(0, |lib| lib.entries.len());
        let secs = self.media_lib_total_dur;
        let total_dur = format!("{}:{:02}:{:02}", secs / 3600, (secs % 3600) / 60, secs % 60);

        let sidebar_items = &self.media_lib_sidebar_cache;
        let list_items = &self.media_lib_list_cache;

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
            div().w(px(150.0)).h_full().bg(c.panel_alt).border_r(px(1.0)).border_color(c.border)
                .child(v_flex().w_full().h_full().py_1()
                    .children(sidebar_items.iter().map(|item| {
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
                            .on_click(cx.listener(move |_this, _e: &gpui::ClickEvent, _w, cx| {
                                let weak = cx.entity().downgrade();
                                let (tx, rx) = async_channel::unbounded();
                                runtime().spawn_blocking(move || {
                                    let cfg = crate::config::Config::load();
                                    let mut lib = crate::media::MediaLib::load();
                                    for dir in &cfg.media_lib.media_dirs {
                                        if let Ok(entries) = crate::media::scan_directory(dir, true, None) {
                                            for e in entries { lib.upsert(e); }
                                        }
                                    }
                                    let _ = lib.save();
                                    tracing::info!("[MediaLib] 扫描完成: {} 首", lib.entries.len());
                                    let _ = tx.send(lib);
                                });
                                cx.spawn(async move |this, cx| {
                                    let lib = rx.recv().await.ok();
                                    let _ = this.update(cx, |this, cx_| {
                                        if let Some(lib) = lib {
                                            this.media_lib_cache = Some(lib);
                                            // Invalidate cached sidebar/list so they rebuild from new data.
                                            this.media_lib_cache_key = (MediaLibCategory::AllTracks, None);
                                        }
                                        cx_.notify();
                                    });
                                }).detach();
                            }))
                    )
            )
            .child(
                h_flex().flex_grow().w_full()
                    .child(sidebar_el)
                    .child(
                        v_flex().flex_grow().h_full().bg(c.bg)
                            .children({
                                // Limit displayed tracks to avoid creating thousands of
                                // GPUI elements when the media library is large.
                                const MAX_DISPLAY: usize = 500;
                                let total = list_items.len();
                                let mut items: Vec<gpui::AnyElement> = Vec::with_capacity(list_items.len().min(MAX_DISPLAY) + 1);
                                items.extend(list_items.iter().take(MAX_DISPLAY).enumerate().map(|(i, item)| {
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
                                        .into_any_element()
                                }));
                                if total > MAX_DISPLAY {
                                    items.push(
                                        div().w_full().px_4().py_2().text_size(px(11.0)).text_color(c.text_dim)
                                            .child(format!("... 及其他 {} 首 (共 {} 首)", total - MAX_DISPLAY, total))
                .into_any_element()
        );
                                }
                                items
                            })
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
                        let _ = this.player.eq_enable(this.eq_enabled);
                        if this.eq_enabled {
                            for (i, slider) in this.eq_sliders.iter().enumerate() {
                                if let SliderValue::Single(v) = slider.read(_cx).value() {
                                    let _ = this.player.eq_set(i, v as i32);
                                }
                            }
                        } else {
                            for i in 0..10 {
                                let _ = this.player.eq_set(i, 0);
                            }
                        }
                        // Persist to config
                        let mut cfg = Config::load();
                        cfg.eq.enabled = this.eq_enabled;
                        if this.eq_enabled {
                            for (i, slider) in this.eq_sliders.iter().enumerate() {
                                if let SliderValue::Single(v) = slider.read(_cx).value() {
                                    cfg.eq.gains[i] = v as i32;
                                }
                            }
                        }
                        let _ = cfg.save();
                        tracing::info!("[EQ] enabled={} (persisted)", this.eq_enabled);
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
                    let presets = presets.clone();
                    Button::new(("eq_preset", pi as u64))
                        .label(n.clone())
                        .compact()
                        .bg(if is_active { c.accent } else { Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 } })
                        .ghost()
                        .on_click(cx.listener(move |this, _, _w, _cx| {
                            this.eq_preset_name = n.clone();
                            if let Some((_, gains)) = presets.iter().find(|(pn, _)| *pn == n.as_str()) {
                                for (i, g) in gains.iter().enumerate() {
                                    if let Some(slider) = this.eq_sliders.get(i) {
                                        slider.update(_cx, |s, cx2| {
                                            s.set_value(SliderValue::Single(*g), _w, cx2);
                                        });
                                    }
                                    if this.eq_enabled {
                                        let _ = this.player.eq_set(i, *g as i32);
                                    }
                                }
                                // Persist preset gains
                                let mut cfg = Config::load();
                                cfg.eq.preset = n.clone();
                                for (i, g) in gains.iter().enumerate() {
                                    cfg.eq.gains[i] = *g as i32;
                                }
                                let _ = cfg.save();
                            }
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
                let val = match slider.read(cx).value() {
                    SliderValue::Single(v) => v,
                    _ => 0.0,
                };
                div()
                    .w(px(56.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    // Value display (show actual gain value)
                    .child(
                        div()
                            .text_size(px(10.0))
                            .text_color(c.text)
                            .child(format!("{:.0}", val))
                    )
                    // Vertical slider
                    .child(
                        div()
                            .w(px(24.0))
                            .h(px(150.0))
                            .flex()
                            .items_center()
                            .justify_center()
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
                    .h(px(80.0))
                    .px_4().py_2()
                    .bg(c.panel_alt)
                    .child(
                        v_flex().w_full().h_full()
                            // Center line (0 dB reference)
                            .child(div().w_full().h(px(1.0)).bg(c.border))
                            // Bar area with positioning relative to center
                            .child(
                                h_flex().w_full().flex_grow().items_center().gap_1()
                                    .children(self.eq_sliders.iter().map(|slider| {
                                        let val = match slider.read(cx).value() {
                                            SliderValue::Single(v) => v,
                                            _ => 0.0,
                                        };
                                        let bar_h = ((val.abs()) / 12.0 * 30.0).max(1.0).min(30.0);
                                        let is_positive = val >= 0.0;
                                        let bar_color = if is_positive { c.accent } else { Hsla { h: 0.0, s: 0.8, l: 0.5, a: 1.0 } };
                                        div()
                                            .flex_grow()
                                            .h_full()
                                            .flex_col()
                                            .justify_center()
                                            .items_center()
                                            .child(
                                                div()
                                                    .w(px(16.0))
                                                    .h(px(bar_h))
                                                    .bg(bar_color)
                                                    .rounded(px(2.0))
                                                    .mt(if is_positive { px(0.0) } else { px(bar_h - bar_h) })
                                                    .mb(if is_positive { px(bar_h - bar_h) } else { px(0.0) })
                                            )
                                    }))
                            )
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

/// Open a separate, floating playlist window (original "浮动播放列表").
pub fn open_floating_playlist(app: &mut App, player: Arc<Player>) {
    let _ = app.open_window(
        WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some("播放列表".into()),
                ..Default::default()
            }),
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point::default(),
                size: gpui::Size { width: px(420.0), height: px(720.0) },
            })),
            window_min_size: Some(gpui::Size { width: px(280.0), height: px(320.0) }),
            ..Default::default()
        },
        |window, cx| {
            let view = cx.new(|_cx| FloatingPlaylistView::new(player.clone()));
            cx.new(|cx| Root::new(view, window, cx))
        },
    );
}

/// A standalone floating playlist window: lists the current playlist and lets
/// the user jump to any track. Shares the same `Arc<Player>` as the main window.
struct FloatingPlaylistView {
    player: Arc<Player>,
}

impl FloatingPlaylistView {
    fn new(player: Arc<Player>) -> Self {
        Self { player }
    }
}

impl Render for FloatingPlaylistView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let cfg = Config::load();
        let c = UiColors::build(cfg.appearance.dark_mode, &theme::ThemeName::from_config(&cfg.appearance.theme));
        let tracks = self.player.playlist().tracks().to_vec();
        let current = self.player.playlist().current_index();

        v_flex()
            .size_full()
            .bg(c.bg)
            .child(
                h_flex()
                    .items_center()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(c.panel_alt)
                    .child(layout::txt(&format!("播放列表 ({} 首)", tracks.len()), 13.0, c.text_title)),
            )
            .child(
                div()
                    .flex_grow()
                    .overflow_y_scrollbar()
                    .py_1()
                    .children(tracks.iter().enumerate().map(|(i, track)| {
                        let is_current = current == Some(i);
                        let bg = if is_current {
                            c.playlist_playing
                        } else if i % 2 == 0 {
                            c.playlist_item
                        } else {
                            c.playlist_item_hover
                        };
                        let text_color = if is_current { c.accent } else { c.text };
                        let display = if !track.title.is_empty() {
                            track.title.clone()
                        } else {
                            track.file_name.clone()
                        };
                        let dur = track.duration_str();
                        let player = self.player.clone();
                        Button::new(("fp_track", i))
                            .w_full()
                            .justify_start()
                            .px_3()
                            .h(px(30.0))
                            .ghost()
                            .bg(bg)
                            .text_color(text_color)
                            .on_click(move |_, _, _| {
                                let p = player.clone();
                                runtime().spawn_blocking(move || { let _ = p.play_at_index(i); });
                            })
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .w_full()
                                    .child(layout::txt(&display, 12.0, text_color))
                                    .child(div().flex_grow())
                                    .child(layout::txt(&dur, 11.0, c.text_dim)),
                            )
                    })),
            )
    }
}

/// Toggle the main window's always-on-top ("置顶") state. Windows only —
/// we locate the main window by its title and call `SetWindowPos`.
fn toggle_always_on_top() {
    let on = !ALWAYS_ON_TOP.fetch_xor(true, Ordering::Relaxed);
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            FindWindowW, SetWindowPos, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE,
        };
        use windows::Win32::Foundation::HWND;
        use windows::core::PCWSTR;
        let title: Vec<u16> = "HackMagic Music Player"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        if let Ok(hwnd) = unsafe { FindWindowW(None, PCWSTR::from_raw(title.as_ptr())) } {
            unsafe {
                let _ = SetWindowPos(
                    hwnd,
                    if on { Some(HWND_TOPMOST) } else { Some(HWND::default()) },
                    0, 0, 0, 0,
                    SWP_NOMOVE | SWP_NOSIZE,
                );
            }
            tracing::info!("[View] Always-on-top: {}", on);
        } else {
            tracing::warn!("[View] Always-on-top: main window not found");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = on;
        tracing::info!("[View] Always-on-top is only supported on Windows");
    }
}

/// A standalone floating window showing synced lyrics (desktop lyrics).
struct DesktopLyricsView {
    player: Arc<Player>,
    state: desktop_lyrics::LyricsState,
    last_track_path: String,
    locked: bool,
    double_line: bool,
    tr: &'static crate::gui::i18n::Tr,
}

impl DesktopLyricsView {
    fn new(player: Arc<Player>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let player_loading = std::sync::Arc::clone(&player.loading);
        // Start a periodic timer to repaint so lyrics update live
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(33))
                    .await;
                // Skip dispatches while engine is blocking (e.g. BASS open)
                if player_loading.load(std::sync::atomic::Ordering::SeqCst) {
                    continue;
                }
                let _ = this.update(cx, |_, _| {});
                cx.background_executor()
                    .timer(std::time::Duration::ZERO)
                    .await;
                let alive = this.update(cx, |_, cx| cx.notify()).is_ok();
                if !alive { break; }
            }
        }).detach();
        Self {
            player,
            state: desktop_lyrics::LyricsState::new(),
            last_track_path: String::new(),
            locked: false,
            double_line: true,
            tr: crate::gui::i18n::global_tr(),
        }
    }

    fn poll(&mut self) {
        let pos = self.player.position().as_secs_f64();
        let lyric_ms = (pos * 1000.0) as u64;

        // Check if track changed
        let current_path = self.player.playlist().current_track()
            .map(|t| t.file_path.clone())
            .unwrap_or_default();

        if current_path != self.last_track_path {
            self.last_track_path = current_path.clone();
            // Load lyrics for the new track
            if !current_path.is_empty() {
                let path = std::path::Path::new(&current_path);
                let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let parent = path.parent().unwrap_or(std::path::Path::new("."));

                let sibling_lrc = parent.join(format!("{}.lrc", file_stem));
                if sibling_lrc.exists() {
                    match crate::lyric::load_lyric_file(sibling_lrc.to_str().unwrap_or("")) {
                        Ok(lyrics) => {
                            self.state.update(Some(lyrics), lyric_ms);
                            return;
                        }
                        Err(e) => tracing::warn!("[DesktopLyrics] Parse error: {}", e),
                    }
                }
            }
            self.state.update(None, lyric_ms);
        } else {
            self.state.recompute(lyric_ms);
        }
    }

    /// Render a single lyric line (used in locked mode).
    fn render_lyric_line(&self, c: &UiColors) -> impl IntoElement {
        if !self.state.visible || self.state.lyrics.is_none() {
            return div().into_any_element();
        }
        let lyrics = self.state.lyrics.as_ref().unwrap();
        let current_idx = self.state.current_index.unwrap_or(0);
        if current_idx >= lyrics.len() {
            return div().into_any_element();
        }
        let line = &lyrics.lines[current_idx];
        let progress = self.state.progress;
        let display_text = lyrics.display_text(line);

        v_flex().items_center().gap_1()
            .child(
                desktop_lyrics::karaoke_line(&display_text, progress, 18.0, FontWeight::BOLD, c)
            )
            .child(if self.double_line && lyrics.translate_mode == crate::lyric::TranslateMode::Separate && !line.translate.is_empty() {
                div().text_center().text_size(px(13.0)).text_color(c.text_dim).child(line.translate.clone())
            } else {
                div()
            })
            .into_any_element()
    }
}

impl Render for DesktopLyricsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll player state
        self.poll();

        let cfg = Config::load();
        let c = UiColors::build(cfg.appearance.dark_mode, &theme::ThemeName::from_config(&cfg.appearance.theme));

        // When locked, use a transparent background and render only the lyrics text
        if self.locked {
            return v_flex()
                .size_full()
                .bg(gpui::Hsla { h: 0.0, s: 0.0, l: 0.0, a: 0.0 })
                .child(
                    v_flex()
                        .flex_grow()
                        .justify_center()
                        .items_center()
                        .gap_1()
                        .child(self.render_lyric_line(&c))
                )
                .into_any_element();
        }

        desktop_lyrics::render_lyrics_overlay(&self.state, self.tr, &c).into_any_element()
    }
}
