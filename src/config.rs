use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Root configuration structure, serialized as TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    pub general: GeneralConfig,
    pub play: PlayConfig,
    pub appearance: AppearanceConfig,
    pub lyric: LyricConfig,
    pub media_lib: MediaLibConfig,
    pub desktop_lyric: DesktopLyricConfig,
    pub hotkey: HotkeyConfig,
    pub lastfm: LastfmConfig,
    pub midi: MidiConfig,
    pub eq: EqConfig,
    pub source: SourceConfig,
    pub ai: AiConfig,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub id3v2_first: bool,
    pub id3v2_version: String,         // "v2.3" or "v2.4"
    pub auto_download_lyric: bool,
    pub auto_download_album_cover: bool,
    pub check_update_when_start: bool,
    pub update_source: String,           // "github" or "gitee"
    pub auto_run: bool,                  // 开机自动运行
    pub minimize_to_notify_icon: bool,
    pub language: String,
    pub portable_mode: bool,
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: u32,
    pub window_height: u32,
    pub window_maximized: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            id3v2_first: false,
            id3v2_version: "v2.4".to_string(),
            auto_download_lyric: false,
            auto_download_album_cover: true,
            check_update_when_start: true,
            update_source: "github".to_string(),
            auto_run: false,
            minimize_to_notify_icon: false,
            language: String::new(),
            portable_mode: false,
            window_x: -1,
            window_y: -1,
            window_width: 1200,
            window_height: 800,
            window_maximized: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayConfig {
    pub engine: String,               // "bass", "mci", "ffmpeg", "rodio", "symphonia"
    pub stop_when_error: bool,
    pub auto_play_when_start: bool,
    pub show_taskbar_progress: bool,
    pub show_taskbar_status_icon: bool,
    pub continue_when_switch_playlist: bool,
    pub use_system_media_controls: bool,
    pub remember_last_position: bool,
    pub output_device: String,
    pub fade_effect: bool,
    pub fade_time: u32,               // ms
    pub default_volume: u32,          // 0-100
    pub volume_step: u32,
    pub mouse_volume_step: u32,
    pub volume_map: u32,              // 0-100
    pub always_on_top: bool,
    pub replaygain: String,           // "off", "track", "album"
    pub output_mode: String,          // "directsound", "wasapi", "wasapi_exclusive"
    pub wasapi_device: i32,           // -1 = default
    pub use_ffmpeg: bool,
    pub ffmpeg_cache_len: u32,        // FFmpeg 缓存长度（秒），默认 5 秒
    pub merge_song_different_versions: bool,  // 合并同一歌曲的多版本（文件夹/媒体库模式）
    pub speed: f64,  // 播放速度 0.5 ~ 2.0
}

impl Default for PlayConfig {
    fn default() -> Self {
        Self {
            engine: if cfg!(target_os = "windows") { "bass".to_string() } else { "ffmpeg".to_string() },
            stop_when_error: true,
            auto_play_when_start: false,
            show_taskbar_progress: false,
            show_taskbar_status_icon: false,
            continue_when_switch_playlist: false,
            use_system_media_controls: false,
            remember_last_position: false,
            output_device: String::new(),
            fade_effect: true,
            fade_time: 500,
            default_volume: 80,
            volume_step: 3,
            mouse_volume_step: 2,
            volume_map: 100,
            always_on_top: false,
            replaygain: "off".to_string(),
            output_mode: "directsound".to_string(),
            wasapi_device: -1,
            use_ffmpeg: false,
            ffmpeg_cache_len: 5,
            merge_song_different_versions: true,
            speed: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub dark_mode: bool,
    pub window_transparency: u32,       // 0-100
    pub show_spectrum: bool,
    pub spectrum_low_freq_center: bool, // 低频部分显示在中间
    pub show_album_cover: bool,
    pub album_cover_fit: String,        // "cover", "fill", "contain", "none"
    pub lyric_background: bool,
    pub rounded_style: bool,
    pub use_desktop_background: bool,
    pub album_cover_as_background: bool,
    pub background_blur: bool,
    pub background_blur_radius: u32,
    pub background_transparency: u32,
    pub spectrum_columns: u32,          // 4, 8, 16, 32, 64, 128
    pub fft_size: u32,                  // 256, 512, 1024, 2048
    pub spectrum_style: String,         // "log" or "linear"
    pub spectrum_height: u32,           // 20-200, 频谱高度像素
    pub playlist_row_height: u32,       // 24-60, 播放列表行高
    pub theme: String,                  // theme name: "default", "ocean", "forest", etc.
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            dark_mode: false,
            window_transparency: 100,
            show_spectrum: true,
            spectrum_low_freq_center: false,
            show_album_cover: true,
            album_cover_fit: "cover".to_string(),
            lyric_background: true,
            rounded_style: true,
            use_desktop_background: false,
            album_cover_as_background: false,
            background_blur: false,
            background_blur_radius: 5,
            background_transparency: 80,
            spectrum_columns: 128,
            fft_size: 512,
            spectrum_style: "log".to_string(),
            spectrum_height: 36,
            playlist_row_height: 36,
            theme: "default".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LyricConfig {
    pub fuzzy_match: bool,
    pub show_translate: bool,
    pub use_inner_lyric_first: bool,
    pub show_song_info_when_no_lyric: bool,
    pub karaoke_display: bool,
    pub hide_empty_lines: bool,
    pub line_spacing: u32,              // 歌词行间距 px
    pub alignment: String,              // "auto", "left", "center", "right"
    pub after_adjust: String,           // "ask", "auto_save", "discard"
}

impl Default for LyricConfig {
    fn default() -> Self {
        Self {
            fuzzy_match: true,
            show_translate: true,
            use_inner_lyric_first: false,
            show_song_info_when_no_lyric: true,
            karaoke_display: false,
            hide_empty_lines: false,
            line_spacing: 0,
            alignment: "auto".to_string(),
            after_adjust: "ask".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaLibConfig {
    pub display_format: String,     // "file_name", "title", "artist_title", "title_artist"
    pub min_duration_secs: u64,     // 忽略短于该秒数的文件（0=不限制）
    pub auto_scan: bool,            // 启动时自动扫描媒体库
    pub media_dirs: Vec<String>,    // 扫描目录列表
    pub disable_disk_delete: bool,  // 禁止从磁盘删除文件
    pub merge_single_category: bool, // 合并单曲分类
    pub remove_missing_files: bool, // 移除缺失文件
}

impl Default for MediaLibConfig {
    fn default() -> Self {
        Self {
            display_format: "title".to_string(),
            min_duration_secs: 0,
            auto_scan: false,
            media_dirs: Vec::new(),
            disable_disk_delete: false,
            merge_single_category: false,
            remove_missing_files: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DesktopLyricConfig {
    pub show: bool,
    pub hide_when_no_lyric: bool,
    pub hide_when_paused: bool,
    pub locked: bool,
    pub double_line: bool,
}

impl Default for DesktopLyricConfig {
    fn default() -> Self {
        Self {
            show: false,
            hide_when_no_lyric: true,
            hide_when_paused: false,
            locked: false,
            double_line: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub enabled: bool,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LastfmConfig {
    pub enabled: bool,
    pub username: String,
    pub password: String,
    pub api_key: String,
    pub shared_secret: String,
    pub session_key: String,
    pub least_perdur: u32,
    pub least_dur: u32,
    pub auto_scrobble: bool,
}

impl Default for LastfmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            username: String::new(),
            password: String::new(),
            api_key: String::new(),
            shared_secret: String::new(),
            session_key: String::new(),
            least_perdur: 50,
            least_dur: 60,
            auto_scrobble: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct MidiConfig {
    pub soundfont: String,
    pub enabled: bool,
}

/// 10-band equalizer persistent state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct EqConfig {
    pub enabled: bool,
    /// 10 band gains in dB, range [-15, 15].
    pub gains: [i32; 10],
    /// Last selected preset name (empty = custom).
    pub preset: String,
}

/// Online music source settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SourceConfig {
    /// Comma-separated list of enabled source ids (empty = all enabled).
    pub enabled: Vec<String>,
    /// Default source id for search.
    pub default: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            enabled: Vec::new(),
            default: "internet_archive".to_string(),
        }
    }
}

/// AI / embedding settings.
///
/// Embeddings are produced by a third-party OpenAI-compatible API, which means
/// **the lyrics of whatever you play are sent to that service**. Both switches
/// default to `false`: nothing leaves the machine until you opt in.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    /// Master switch for the third-party embedding API. While `false`, every
    /// embedding call short-circuits to `None` (same as a missing API key).
    pub enabled: bool,
}

// 默认 false：不显式开启就不外传任何歌词。



impl Config {
    /// Read the configuration file. **Pure read — never writes to disk.**
    ///
    /// The old implementation wrote a default `config.toml` when the file was
    /// missing, which made a *reader* able to overwrite a *writer*:
    /// thread A reads the file (missing) → thread B writes the real config →
    /// thread A saves its default snapshot → B's config is silently gone.
    /// That race is not theoretical: `cargo test --lib` runs tests in
    /// parallel, the config path is derived from the process-global `APPDATA`,
    /// and `ai::tests::test_is_ai_enabled_reflects_config_toml` failed
    /// intermittently for exactly this reason (a concurrent `Config::load()`
    /// in another test clobbered the `[ai] enabled = true` it had just written).
    ///
    /// Materialising the file is now an explicit step — see
    /// [`Config::load_or_create`].
    pub fn load() -> Self {
        let path = get_config_path();
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Config::default(),
        }
    }

    /// Read the configuration, creating `config.toml` with the defaults when it
    /// does not exist yet.
    ///
    /// Used by the program entry points (CLI / GUI) so that a fresh install
    /// still gets a config file to edit. Library code should prefer [`Config::load`]
    /// so that reading never mutates the user's disk state.
    pub fn load_or_create() -> Self {
        let cfg = Self::load();
        if !get_config_path().exists() {
            let _ = cfg.save();
        }
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = get_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        std::fs::write(path, content)
    }

    pub fn get(key: &str) -> Option<String> {
        let cfg = Self::load();
        let val: Option<String> = match key {
            "general.id3v2_first" => Some(cfg.general.id3v2_first.to_string()),
            "general.auto_download_lyric" => Some(cfg.general.auto_download_lyric.to_string()),
            "general.auto_download_album_cover" => Some(cfg.general.auto_download_album_cover.to_string()),
            "general.check_update_when_start" => Some(cfg.general.check_update_when_start.to_string()),
            "general.minimize_to_notify_icon" => Some(cfg.general.minimize_to_notify_icon.to_string()),
            "general.language" => Some(cfg.general.language.clone()),
            "general.update_source" => Some(cfg.general.update_source.clone()),
            "general.auto_run" => Some(cfg.general.auto_run.to_string()),
            "general.portable_mode" => Some(cfg.general.portable_mode.to_string()),
            "general.window_x" => Some(cfg.general.window_x.to_string()),
            "general.window_y" => Some(cfg.general.window_y.to_string()),
            "general.window_width" => Some(cfg.general.window_width.to_string()),
            "general.window_height" => Some(cfg.general.window_height.to_string()),
            "general.window_maximized" => Some(cfg.general.window_maximized.to_string()),
            "play.engine" => Some(cfg.play.engine.clone()),
            "play.stop_when_error" => Some(cfg.play.stop_when_error.to_string()),
            "play.auto_play_when_start" => Some(cfg.play.auto_play_when_start.to_string()),
            "play.show_taskbar_progress" => Some(cfg.play.show_taskbar_progress.to_string()),
            "play.show_taskbar_status_icon" => Some(cfg.play.show_taskbar_status_icon.to_string()),
            "play.continue_when_switch_playlist" => Some(cfg.play.continue_when_switch_playlist.to_string()),
            "play.use_system_media_controls" => Some(cfg.play.use_system_media_controls.to_string()),
            "play.remember_last_position" => Some(cfg.play.remember_last_position.to_string()),
            "play.fade_effect" => Some(cfg.play.fade_effect.to_string()),
            "play.fade_time" => Some(cfg.play.fade_time.to_string()),
            "play.default_volume" => Some(cfg.play.default_volume.to_string()),
            "play.volume_step" => Some(cfg.play.volume_step.to_string()),
            "play.volume_map" => Some(cfg.play.volume_map.to_string()),
            "play.merge_song_different_versions" => Some(cfg.play.merge_song_different_versions.to_string()),
            "play.replaygain" => Some(cfg.play.replaygain.clone()),
            "play.output_mode" => Some(cfg.play.output_mode.clone()),
            "play.wasapi_device" | "play.output_device" => Some(cfg.play.wasapi_device.to_string()),
            "play.always_on_top" => Some(cfg.play.always_on_top.to_string()),
            "appearance.dark_mode" => Some(cfg.appearance.dark_mode.to_string()),
            "appearance.window_transparency" => Some(cfg.appearance.window_transparency.to_string()),
            "appearance.show_spectrum" => Some(cfg.appearance.show_spectrum.to_string()),
            "appearance.spectrum_height" => Some(cfg.appearance.spectrum_height.to_string()),
            "appearance.spectrum_low_freq_center" => Some(cfg.appearance.spectrum_low_freq_center.to_string()),
            "appearance.show_album_cover" => Some(cfg.appearance.show_album_cover.to_string()),
            "appearance.album_cover_fit" => Some(cfg.appearance.album_cover_fit.clone()),
            "appearance.lyric_background" => Some(cfg.appearance.lyric_background.to_string()),
            "appearance.rounded_style" => Some(cfg.appearance.rounded_style.to_string()),
            "appearance.use_desktop_background" => Some(cfg.appearance.use_desktop_background.to_string()),
            "appearance.album_cover_as_background" => Some(cfg.appearance.album_cover_as_background.to_string()),
            "appearance.background_blur" => Some(cfg.appearance.background_blur.to_string()),
            "appearance.background_blur_radius" => Some(cfg.appearance.background_blur_radius.to_string()),
            "appearance.spectrum_columns" => Some(cfg.appearance.spectrum_columns.to_string()),
            "appearance.fft_size" => Some(cfg.appearance.fft_size.to_string()),
            "appearance.spectrum_style" => Some(cfg.appearance.spectrum_style.clone()),
            "appearance.background_transparency" => Some(cfg.appearance.background_transparency.to_string()),
            "appearance.theme" => Some(cfg.appearance.theme.clone()),
            "lyric.fuzzy_match" => Some(cfg.lyric.fuzzy_match.to_string()),
            "lyric.show_translate" => Some(cfg.lyric.show_translate.to_string()),
            "lyric.use_inner_lyric_first" => Some(cfg.lyric.use_inner_lyric_first.to_string()),
            "lyric.show_song_info_when_no_lyric" => Some(cfg.lyric.show_song_info_when_no_lyric.to_string()),
            "lyric.karaoke_display" => Some(cfg.lyric.karaoke_display.to_string()),
            "lyric.hide_empty_lines" => Some(cfg.lyric.hide_empty_lines.to_string()),
            "lyric.line_spacing" => Some(cfg.lyric.line_spacing.to_string()),
            "lyric.alignment" => Some(cfg.lyric.alignment.clone()),
            "lyric.after_adjust" => Some(cfg.lyric.after_adjust.clone()),
            "media_lib.display_format" => Some(cfg.media_lib.display_format.clone()),
            "media_lib.min_duration_secs" => Some(cfg.media_lib.min_duration_secs.to_string()),
            "media_lib.auto_scan" => Some(cfg.media_lib.auto_scan.to_string()),
            "media_lib.media_dirs" => Some(cfg.media_lib.media_dirs.join(",")),
            "media_lib.disable_disk_delete" => Some(cfg.media_lib.disable_disk_delete.to_string()),
            "media_lib.merge_single_category" => Some(cfg.media_lib.merge_single_category.to_string()),
            "media_lib.remove_missing_files" => Some(cfg.media_lib.remove_missing_files.to_string()),
            "desktop_lyric.show" => Some(cfg.desktop_lyric.show.to_string()),
            "desktop_lyric.hide_when_no_lyric" => Some(cfg.desktop_lyric.hide_when_no_lyric.to_string()),
            "desktop_lyric.hide_when_paused" => Some(cfg.desktop_lyric.hide_when_paused.to_string()),
            "desktop_lyric.locked" => Some(cfg.desktop_lyric.locked.to_string()),
            "desktop_lyric.double_line" => Some(cfg.desktop_lyric.double_line.to_string()),
            "hotkey.enabled" => Some(cfg.hotkey.enabled.to_string()),
            "lastfm.enabled" => Some(cfg.lastfm.enabled.to_string()),
            "lastfm.username" => Some(cfg.lastfm.username.clone()),
            "lastfm.api_key" => Some(cfg.lastfm.api_key.clone()),
            "lastfm.shared_secret" => Some("***".to_string()),
            "lastfm.auto_scrobble" => Some(cfg.lastfm.auto_scrobble.to_string()),
            "midi.soundfont" => Some(cfg.midi.soundfont.clone()),
            "midi.enabled" => Some(cfg.midi.enabled.to_string()),
            "source.enabled" => Some(cfg.source.enabled.join(",")),
            "source.default" => Some(cfg.source.default.clone()),
            "ai.enabled" => Some(cfg.ai.enabled.to_string()),
            _ => None,
        };
        val
    }

    pub fn set(key: &str, value: &str) -> std::io::Result<()> {
        let mut cfg = Self::load();
        match key {
            "general.auto_download_lyric" => cfg.general.auto_download_lyric = value == "true",
            "general.auto_download_album_cover" => cfg.general.auto_download_album_cover = value == "true",
            "general.check_update_when_start" => cfg.general.check_update_when_start = value == "true",
            "general.minimize_to_notify_icon" => cfg.general.minimize_to_notify_icon = value == "true",
            "general.language" => cfg.general.language = value.to_string(),
            "general.update_source" => cfg.general.update_source = value.to_string(),
            "general.auto_run" => cfg.general.auto_run = value == "true",
            "general.portable_mode" => cfg.general.portable_mode = value == "true",
            "play.engine" => cfg.play.engine = value.to_string(),
            "play.default_volume" | "volume" => {
                if let Ok(v) = value.parse() { cfg.play.default_volume = v; }
            }
            "play.merge_song_different_versions" => cfg.play.merge_song_different_versions = value == "true",
            "play.replaygain" => cfg.play.replaygain = value.to_string(),
            "play.output_mode" => cfg.play.output_mode = value.to_string(),
            "play.wasapi_device" | "play.output_device" => { if let Ok(v) = value.parse() { cfg.play.wasapi_device = v; } }
            "play.volume_map" => {
                if let Ok(v) = value.parse() { cfg.play.volume_map = v; }
            }
            "play.fade_effect" | "fade" => cfg.play.fade_effect = value == "true",
            "play.fade_time" => {
                if let Ok(v) = value.parse() { cfg.play.fade_time = v; }
            }
            "play.stop_when_error" => cfg.play.stop_when_error = value == "true",
            "play.auto_play_when_start" => cfg.play.auto_play_when_start = value == "true",
            "play.show_taskbar_progress" => cfg.play.show_taskbar_progress = value == "true",
            "play.show_taskbar_status_icon" => cfg.play.show_taskbar_status_icon = value == "true",
            "play.continue_when_switch_playlist" => cfg.play.continue_when_switch_playlist = value == "true",
            "play.use_system_media_controls" => cfg.play.use_system_media_controls = value == "true",
            "play.remember_last_position" => cfg.play.remember_last_position = value == "true",
            "play.volume_step" => {
                if let Ok(v) = value.parse() { cfg.play.volume_step = v; }
            }
            "play.always_on_top" => cfg.play.always_on_top = value == "true",
            "appearance.dark_mode" | "dark_mode" => cfg.appearance.dark_mode = value == "true",
            "appearance.spectrum_columns" => { if let Ok(v) = value.parse() { cfg.appearance.spectrum_columns = v; } }
            "appearance.fft_size" => { if let Ok(v) = value.parse() { cfg.appearance.fft_size = v; } }
            "appearance.spectrum_style" | "spectrum_style" => cfg.appearance.spectrum_style = value.to_string(),
            "appearance.theme" | "theme" => cfg.appearance.theme = value.to_string(),
            "appearance.show_spectrum" => cfg.appearance.show_spectrum = value == "true",
            "appearance.window_transparency" => {
                if let Ok(v) = value.parse() { cfg.appearance.window_transparency = v; }
            }
            "appearance.background_transparency" => {
                if let Ok(v) = value.parse() { cfg.appearance.background_transparency = v; }
            }
            "appearance.spectrum_height" => {
                if let Ok(v) = value.parse() { cfg.appearance.spectrum_height = v; }
            }
            "appearance.spectrum_low_freq_center" => cfg.appearance.spectrum_low_freq_center = value == "true",
            "appearance.show_album_cover" => cfg.appearance.show_album_cover = value == "true",
            "appearance.album_cover_fit" => cfg.appearance.album_cover_fit = value.to_string(),
            "appearance.lyric_background" => cfg.appearance.lyric_background = value == "true",
            "appearance.rounded_style" => cfg.appearance.rounded_style = value == "true",
            "appearance.use_desktop_background" => cfg.appearance.use_desktop_background = value == "true",
            "appearance.album_cover_as_background" => cfg.appearance.album_cover_as_background = value == "true",
            "appearance.background_blur" => cfg.appearance.background_blur = value == "true",
            "appearance.background_blur_radius" => {
                if let Ok(v) = value.parse() { cfg.appearance.background_blur_radius = v; }
            }
            "lyric.fuzzy_match" => cfg.lyric.fuzzy_match = value == "true",
            "lyric.show_translate" => cfg.lyric.show_translate = value == "true",
            "lyric.use_inner_lyric_first" => cfg.lyric.use_inner_lyric_first = value == "true",
            "lyric.show_song_info_when_no_lyric" => cfg.lyric.show_song_info_when_no_lyric = value == "true",
            "lyric.karaoke_display" => cfg.lyric.karaoke_display = value == "true",
            "lyric.hide_empty_lines" => cfg.lyric.hide_empty_lines = value == "true",
            "lyric.line_spacing" => {
                if let Ok(v) = value.parse() { cfg.lyric.line_spacing = v; }
            }
            "lyric.alignment" => cfg.lyric.alignment = value.to_string(),
            "lyric.after_adjust" => cfg.lyric.after_adjust = value.to_string(),
            "lastfm.enabled" => cfg.lastfm.enabled = value == "true",
            "lastfm.username" => cfg.lastfm.username = value.to_string(),
            "lastfm.password" => cfg.lastfm.password = value.to_string(),
            "lastfm.api_key" => cfg.lastfm.api_key = value.to_string(),
            "lastfm.shared_secret" => cfg.lastfm.shared_secret = value.to_string(),
            "lastfm.session_key" => cfg.lastfm.session_key = value.to_string(),
            "lastfm.auto_scrobble" => cfg.lastfm.auto_scrobble = value == "true",
            "midi.soundfont" => cfg.midi.soundfont = value.to_string(),
            "midi.enabled" => cfg.midi.enabled = value == "true",
            "media_lib.min_duration_secs" | "min_duration" => {
                if let Ok(v) = value.parse() { cfg.media_lib.min_duration_secs = v; }
            }
            "media_lib.auto_scan" => cfg.media_lib.auto_scan = value == "true",
            "media_lib.media_dirs" => {
                cfg.media_lib.media_dirs = value.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
            }
            "media_lib.disable_disk_delete" => cfg.media_lib.disable_disk_delete = value == "true",
            "media_lib.merge_single_category" => cfg.media_lib.merge_single_category = value == "true",
            "media_lib.remove_missing_files" => cfg.media_lib.remove_missing_files = value == "true",
            "desktop_lyric.show" => cfg.desktop_lyric.show = value == "true",
            "desktop_lyric.hide_when_no_lyric" => cfg.desktop_lyric.hide_when_no_lyric = value == "true",
            "desktop_lyric.hide_when_paused" => cfg.desktop_lyric.hide_when_paused = value == "true",
            "desktop_lyric.locked" => cfg.desktop_lyric.locked = value == "true",
            "desktop_lyric.double_line" => cfg.desktop_lyric.double_line = value == "true",
            "hotkey.enabled" => cfg.hotkey.enabled = value == "true",
            "general.window_x" => { if let Ok(v) = value.parse() { cfg.general.window_x = v; } }
            "general.window_y" => { if let Ok(v) = value.parse() { cfg.general.window_y = v; } }
            "general.window_width" => { if let Ok(v) = value.parse() { cfg.general.window_width = v; } }
            "general.window_height" => { if let Ok(v) = value.parse() { cfg.general.window_height = v; } }
            "general.window_maximized" => cfg.general.window_maximized = value == "true",
            "source.enabled" => {
                cfg.source.enabled = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "source.default" => cfg.source.default = value.to_string(),
            "ai.enabled" => cfg.ai.enabled = value == "true",
            _ => {}
        }
        cfg.save()
    }
}

/// Get the platform-appropriate config directory (parent of config.toml).
///
/// - **Windows**: `%APPDATA%\hm`
/// - **macOS**: `~/Library/Application Support/hm`
/// - **Linux/other**: `$XDG_CONFIG_HOME/hm` or `~/.config/hm`
///
/// If `config.toml` already exists next to the executable, portable mode
/// is assumed and the executable directory is returned instead.
pub fn get_config_dir() -> PathBuf {
    // Check portable mode: if config.toml exists next to the executable, use that dir
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let portable_cfg = exe_dir.join("config.toml");
            if portable_cfg.exists() {
                return exe_dir.to_path_buf();
            }
        }
    }
    // Default: platform-appropriate config directory
    if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        appdata.join("hm")
    } else if cfg!(target_os = "macos") {
        // macOS convention: ~/Library/Application Support/AppName
        let home = std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
        home.join("Library")
            .join("Application Support")
            .join("hm")
    } else {
        // Linux / other Unix: XDG_CONFIG_HOME or ~/.config
        let xdg = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        let base = xdg.unwrap_or_else(|| {
            let home = std::env::var("HOME").map_or_else(|_| PathBuf::from("."), PathBuf::from);
            home.join(".config")
        });
        base.join("hm")
    }
}

fn get_config_path() -> PathBuf {
    get_config_dir().join("config.toml")
}

/// 跨模块共享的 APPDATA 测试隔离（`#[cfg(test)]`）。
///
/// 多个模块的测试都需要把 `APPDATA` 指到临时目录（play_stats 的统计文件、
/// ai 的 `[ai].enabled` 读取）。环境变量是进程级的，若各模块用各自的锁，
/// 并行测试会互相改写 `APPDATA` 造成偶发失败——因此锁必须全局唯一。
#[cfg(test)]
pub mod test_support {
    use std::sync::Mutex;

    static APPDATA_LOCK: Mutex<()> = Mutex::new(());

    pub struct AppdataGuard {
        prev: Option<String>,
        prev_xdg: Option<String>,
        _dir: tempfile::TempDir,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl Drop for AppdataGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var("APPDATA", v),
                None => std::env::remove_var("APPDATA"),
            }
            match &self.prev_xdg {
                Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
                None => std::env::remove_var("XDG_CONFIG_HOME"),
            }
        }
    }

    /// 把 `APPDATA`（Windows）与 `XDG_CONFIG_HOME`（Linux）指到一个全新的临时目录；
    /// 返回的守卫 Drop 时恢复原值。无法创建临时目录时返回 `None`（调用方应跳过而非误判）。
    ///
    /// 两个变量都要设：`get_config_dir()` 在 Windows 上读 `APPDATA`，在 Linux 上读
    /// `XDG_CONFIG_HOME`，只隔离一个会让另一平台的测试写到真实配置上。
    pub fn isolate_appdata() -> Option<AppdataGuard> {
        let lock = APPDATA_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let prev = std::env::var("APPDATA").ok();
        let prev_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let dir = tempfile::tempdir().ok()?;
        std::env::set_var("APPDATA", dir.path());
        std::env::set_var("XDG_CONFIG_HOME", dir.path());
        Some(AppdataGuard {
            prev,
            prev_xdg,
            _dir: dir,
            _lock: lock,
        })
    }
}

/// Recent folder and playlist history, saved to JSON
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecentHistory {
    pub folders: Vec<String>,
    pub playlists: Vec<String>,
}

impl RecentHistory {
    const MAX_RECENT: usize = 20;

    fn path() -> PathBuf {
        get_config_dir().join("recent.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }

    pub fn add_folder(&mut self, path: &str) {
        self.folders.retain(|f| f != path);
        self.folders.insert(0, path.to_string());
        self.folders.truncate(Self::MAX_RECENT);
        self.save();
    }

    pub fn add_playlist(&mut self, path: &str) {
        self.playlists.retain(|f| f != path);
        self.playlists.insert(0, path.to_string());
        self.playlists.truncate(Self::MAX_RECENT);
        self.save();
    }
}

/// Playback state saved on exit/stop and restored on startup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    pub last_playlist: String,
    pub track_index: usize,
    pub position_secs: f64,
    pub volume: u32,
    pub repeat_mode: String,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            last_playlist: "default".to_string(),
            track_index: 0,
            position_secs: 0.0,
            volume: 80,
            repeat_mode: "list".to_string(),
        }
    }
}

impl PlaybackState {
    fn state_path() -> PathBuf {
        get_config_dir().join("state.json")
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }

    /// Record which playlist is the active one, leaving the other fields
    /// (track index, position, volume, repeat mode) untouched.
    ///
    /// Playlist commands change *which* playlist is current, but not the
    /// playback position inside it — so a full `save_playback_state()` would
    /// wrongly reset the position. This helper is the surgical alternative.
    ///
    /// Without it, `hm playlist load x` would leave `state.json` pointing at the
    /// previous playlist, and the next CLI invocation would restore that one
    /// instead of the one the user just selected.
    pub fn set_last_playlist(name: &str) {
        let mut state = Self::load();
        if state.last_playlist == name {
            return;
        }
        state.last_playlist = name.to_string();
        state.save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `set_last_playlist` must change **only** the playlist name: the playback
    /// position, volume and repeat mode belong to the session, not to the
    /// playlist switch, and must survive it.
    #[test]
    fn test_set_last_playlist_preserves_other_fields() {
        use super::test_support::isolate_appdata;
        let Some(_guard) = isolate_appdata() else { return; };

        let state = PlaybackState {
            last_playlist: "default".to_string(),
            track_index: 7,
            position_secs: 123.5,
            volume: 42,
            repeat_mode: "one".to_string(),
        };
        state.save();

        PlaybackState::set_last_playlist("other");

        let loaded = PlaybackState::load();
        assert_eq!(loaded.last_playlist, "other");
        assert_eq!(loaded.track_index, 7);
        assert_eq!(loaded.position_secs, 123.5);
        assert_eq!(loaded.volume, 42);
        assert_eq!(loaded.repeat_mode, "one");
    }

    /// Regression: `Config::load()` must be a **pure read** — it must never
    /// create or rewrite `config.toml`.
    ///
    /// It used to save a default config whenever the file was missing. Because
    /// the config path is derived from the process-global `APPDATA`, that write
    /// side effect let a *reader* clobber a *writer*: a parallel test calling
    /// `Config::load()` (e.g. `media::scan_directory`) could read "missing" and
    /// then save its default snapshot over the `[ai] enabled = true` that
    /// `ai::tests::test_is_ai_enabled_reflects_config_toml` had just written.
    /// That is exactly how that test failed intermittently.
    #[test]
    fn test_config_load_never_writes_to_disk() {
        use super::test_support::isolate_appdata;
        let Some(_guard) = isolate_appdata() else { return; };

        // Portable mode (config.toml next to the executable) points the config
        // path at the real config — skip rather than touch the user's file.
        let Some(appdata) = std::env::var("APPDATA").ok() else { return; };
        if get_config_dir() != std::path::Path::new(&appdata).join("hm") {
            return;
        }

        let cfg_path = get_config_path();
        let Some(parent) = cfg_path.parent() else { return; };
        std::fs::create_dir_all(parent).unwrap();

        // 1. A read must not create the file.
        assert!(!cfg_path.exists(), "precondition: config.toml must not exist");
        let cfg = Config::load();
        assert!(!cfg.ai.enabled);
        assert!(
            !cfg_path.exists(),
            "Config::load() must not create config.toml on a read"
        );

        // 2. A read must leave an existing file byte-identical.
        let content = "[ai]\nenabled = true\n";
        std::fs::write(&cfg_path, content).unwrap();
        let cfg = Config::load();
        assert!(cfg.ai.enabled, "existing config must be parsed");
        assert_eq!(
            std::fs::read_to_string(&cfg_path).unwrap(),
            content,
            "Config::load() must not rewrite config.toml"
        );

        // 3. `load_or_create` is the explicit way to materialise the file.
        std::fs::remove_file(&cfg_path).unwrap();
        let cfg = Config::load_or_create();
        assert!(!cfg.ai.enabled);
        assert!(
            cfg_path.exists(),
            "Config::load_or_create() must write config.toml"
        );
    }

    #[test]
    fn test_config_default_values() {
        let cfg = Config::default();

        // General
        assert!(!cfg.general.id3v2_first);
        assert!(!cfg.general.auto_download_lyric);
        assert!(cfg.general.auto_download_album_cover);
        assert!(cfg.general.check_update_when_start);
        assert!(!cfg.general.minimize_to_notify_icon);
        assert!(cfg.general.language.is_empty());
        assert!(!cfg.general.portable_mode);

        // Play
        if cfg!(target_os = "windows") {
            assert_eq!(cfg.play.engine, "bass");
        } else {
            assert_eq!(cfg.play.engine, "ffmpeg");
        }
        assert!(cfg.play.stop_when_error);
        assert!(!cfg.play.auto_play_when_start);
        assert!(cfg.play.fade_effect);
        assert_eq!(cfg.play.fade_time, 500);
        assert_eq!(cfg.play.default_volume, 80);
        assert_eq!(cfg.play.volume_step, 3);
        assert_eq!(cfg.play.volume_map, 100);
        assert!(!cfg.play.always_on_top);
        assert_eq!(cfg.play.replaygain, "off");
        assert_eq!(cfg.play.output_mode, "directsound");
        assert_eq!(cfg.play.wasapi_device, -1);
        assert!(!cfg.play.use_ffmpeg);
        assert!(cfg.play.merge_song_different_versions);

        // Appearance
        assert!(!cfg.appearance.dark_mode);
        assert_eq!(cfg.appearance.window_transparency, 100);
        assert!(cfg.appearance.show_spectrum);
        assert_eq!(cfg.appearance.background_transparency, 80);
        assert_eq!(cfg.appearance.spectrum_columns, 128);
        assert_eq!(cfg.appearance.fft_size, 512);
        assert_eq!(cfg.appearance.spectrum_style, "log");
        assert_eq!(cfg.appearance.theme, "default");

        // Lyric
        assert!(cfg.lyric.fuzzy_match);
        assert!(cfg.lyric.show_translate);
        assert!(!cfg.lyric.use_inner_lyric_first);

        // MediaLib
        assert_eq!(cfg.media_lib.display_format, "title");
        assert_eq!(cfg.media_lib.min_duration_secs, 0);
        assert!(!cfg.media_lib.auto_scan);
        assert!(cfg.media_lib.media_dirs.is_empty());

        // Hotkey
        assert!(cfg.hotkey.enabled);

        // LastFM
        assert!(!cfg.lastfm.enabled);
        assert!(cfg.lastfm.username.is_empty());
        assert_eq!(cfg.lastfm.least_perdur, 50);
        assert_eq!(cfg.lastfm.least_dur, 60);
        assert!(cfg.lastfm.auto_scrobble);

        // Midi
        assert!(cfg.midi.soundfont.is_empty());
        assert!(!cfg.midi.enabled);

        // Source
        assert!(cfg.source.enabled.is_empty());
        assert_eq!(cfg.source.default, "internet_archive");
    }

    #[test]
    fn test_config_toml_roundtrip() {
        let cfg = Config::default();
        let toml_str = toml::to_string_pretty(&cfg).expect("toml serialization failed");
        let deserialized: Config = toml::from_str(&toml_str).expect("toml deserialization failed");

        // Spot-check a few fields after roundtrip
        assert_eq!(deserialized.general.id3v2_first, cfg.general.id3v2_first);
        assert_eq!(deserialized.play.engine, cfg.play.engine);
        assert_eq!(deserialized.appearance.theme, cfg.appearance.theme);
        assert_eq!(deserialized.lyric.fuzzy_match, cfg.lyric.fuzzy_match);
        assert_eq!(deserialized.media_lib.display_format, cfg.media_lib.display_format);
        assert_eq!(deserialized.lastfm.auto_scrobble, cfg.lastfm.auto_scrobble);
        assert_eq!(deserialized.midi.enabled, cfg.midi.enabled);
    }

    #[test]
    fn test_config_source_section_toml_roundtrip() {
        // `source.enabled` (aliases) and `source.default` must survive a TOML
        // round-trip so the config actually drives target selection.
        let mut cfg = Config::default();
        cfg.source.enabled = vec!["fma".to_string(), "ia".to_string()];
        cfg.source.default = "fma".to_string();

        let toml_str = toml::to_string_pretty(&cfg).expect("toml serialization failed");
        let parsed: Config = toml::from_str(&toml_str).expect("toml deserialization failed");

        assert_eq!(parsed.source.enabled, vec!["fma".to_string(), "ia".to_string()]);
        assert_eq!(parsed.source.default, "fma");
    }

    #[test]
    fn test_config_source_default_missing_parses_to_default() {
        // A config that omits the `[source]` table entirely must still parse
        // and fall back to the documented default source.
        let parsed: Config = toml::from_str(r#"
[general]
language = "en"
"#).expect("parse failed");
        assert_eq!(parsed.source.default, "internet_archive");
        assert!(parsed.source.enabled.is_empty());
    }

    #[test]
    fn test_config_custom_toml_parse() {
        let toml_str = r#"
[general]
language = "zh-CN"
portable_mode = true

[play]
engine = "ffmpeg"
default_volume = 50
fade_time = 1000

[appearance]
dark_mode = true
theme = "ocean"
"#;
        let cfg: Config = toml::from_str(toml_str).expect("toml parsing failed");

        assert_eq!(cfg.general.language, "zh-CN");
        assert!(cfg.general.portable_mode);
        assert_eq!(cfg.play.engine, "ffmpeg");
        assert_eq!(cfg.play.default_volume, 50);
        assert_eq!(cfg.play.fade_time, 1000);
        assert!(cfg.appearance.dark_mode);
        assert_eq!(cfg.appearance.theme, "ocean");

        // Fields not specified should use defaults
        assert!(cfg.general.auto_download_album_cover);
        assert!(cfg.general.check_update_when_start);
        assert!(cfg.play.fade_effect);
        assert_eq!(cfg.play.replaygain, "off");
        assert!(cfg.lyric.fuzzy_match);
    }

    #[test]
    fn test_playback_state_default() {
        let state = PlaybackState::default();
        assert_eq!(state.last_playlist, "default");
        assert_eq!(state.track_index, 0);
        assert_eq!(state.position_secs, 0.0);
        assert_eq!(state.volume, 80);
        assert_eq!(state.repeat_mode, "list");
    }

    #[test]
    fn test_playback_state_json_roundtrip() {
        let state = PlaybackState {
            last_playlist: "my_playlist.m3u".to_string(),
            track_index: 5,
            position_secs: 123.45,
            volume: 60,
            repeat_mode: "one".to_string(),
        };
        let json = serde_json::to_string_pretty(&state).expect("json serialization failed");
        let deserialized: PlaybackState = serde_json::from_str(&json).expect("json deserialization failed");

        assert_eq!(deserialized.last_playlist, "my_playlist.m3u");
        assert_eq!(deserialized.track_index, 5);
        assert!((deserialized.position_secs - 123.45).abs() < 1e-10);
        assert_eq!(deserialized.volume, 60);
        assert_eq!(deserialized.repeat_mode, "one");
    }

    #[test]
    fn test_recent_history_default() {
        let hist = RecentHistory::default();
        assert!(hist.folders.is_empty());
        assert!(hist.playlists.is_empty());
    }

    #[test]
    fn test_recent_history_serde_roundtrip() {
        let mut hist = RecentHistory::default();
        hist.folders.push("/music/rock".to_string());
        hist.folders.push("/music/jazz".to_string());
        hist.playlists.push("favorites.m3u".to_string());

        let json = serde_json::to_string_pretty(&hist).expect("json serialization");
        let deserialized: RecentHistory = serde_json::from_str(&json).expect("json deserialization");

        assert_eq!(deserialized.folders, vec!["/music/rock", "/music/jazz"]);
        assert_eq!(deserialized.playlists, vec!["favorites.m3u"]);
    }
}
