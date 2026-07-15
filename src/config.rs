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
    pub hotkey: HotkeyConfig,
    pub lastfm: LastfmConfig,
    pub midi: MidiConfig,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub id3v2_first: bool,
    pub auto_download_lyric: bool,
    pub auto_download_album_cover: bool,
    pub check_update_when_start: bool,
    pub minimize_to_notify_icon: bool,
    pub language: String,
    pub portable_mode: bool,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            id3v2_first: false,
            auto_download_lyric: false,
            auto_download_album_cover: true,
            check_update_when_start: true,
            minimize_to_notify_icon: false,
            language: String::new(),
            portable_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlayConfig {
    pub engine: String,               // "bass", "mci", "ffmpeg"
    pub stop_when_error: bool,
    pub auto_play_when_start: bool,
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
    pub merge_song_different_versions: bool,  // 合并同一歌曲的多版本（文件夹/媒体库模式）
}

impl Default for PlayConfig {
    fn default() -> Self {
        Self {
            engine: if cfg!(target_os = "windows") { "bass".to_string() } else { "ffmpeg".to_string() },
            stop_when_error: true,
            auto_play_when_start: false,
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
            merge_song_different_versions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub dark_mode: bool,
    pub window_transparency: u32,       // 0-100
    pub show_spectrum: bool,
    pub background_transparency: u32,
    pub spectrum_columns: u32,          // 4, 8, 16, 32, 64, 128
    pub fft_size: u32,                  // 256, 512, 1024, 2048
    pub spectrum_style: String,         // "log" or "linear"
    pub theme: String,                  // theme name: "default", "ocean", "forest", etc.
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            dark_mode: false,
            window_transparency: 100,
            show_spectrum: true,
            background_transparency: 80,
            spectrum_columns: 128,
            fft_size: 512,
            spectrum_style: "log".to_string(),
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
}

impl Default for LyricConfig {
    fn default() -> Self {
        Self {
            fuzzy_match: true,
            show_translate: true,
            use_inner_lyric_first: false,
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
}

impl Default for MediaLibConfig {
    fn default() -> Self {
        Self {
            display_format: "title".to_string(),
            min_duration_secs: 0,
            auto_scan: false,
            media_dirs: Vec::new(),
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


impl Config {
    pub fn load() -> Self {
        let path = get_config_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            toml::from_str(&content).unwrap_or_default()
        } else {
            let cfg = Config::default();
            let _ = cfg.save();
            cfg
        }
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
            "general.portable_mode" => Some(cfg.general.portable_mode.to_string()),
            "play.engine" => Some(cfg.play.engine.clone()),
            "play.stop_when_error" => Some(cfg.play.stop_when_error.to_string()),
            "play.auto_play_when_start" => Some(cfg.play.auto_play_when_start.to_string()),
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
            "appearance.spectrum_columns" => Some(cfg.appearance.spectrum_columns.to_string()),
            "appearance.fft_size" => Some(cfg.appearance.fft_size.to_string()),
            "appearance.spectrum_style" => Some(cfg.appearance.spectrum_style.clone()),
            "appearance.background_transparency" => Some(cfg.appearance.background_transparency.to_string()),
            "appearance.theme" => Some(cfg.appearance.theme.clone()),
            "lyric.fuzzy_match" => Some(cfg.lyric.fuzzy_match.to_string()),
            "lyric.show_translate" => Some(cfg.lyric.show_translate.to_string()),
            "lyric.use_inner_lyric_first" => Some(cfg.lyric.use_inner_lyric_first.to_string()),
            "media_lib.display_format" => Some(cfg.media_lib.display_format.clone()),
            "media_lib.min_duration_secs" => Some(cfg.media_lib.min_duration_secs.to_string()),
            "media_lib.auto_scan" => Some(cfg.media_lib.auto_scan.to_string()),
            "media_lib.media_dirs" => Some(cfg.media_lib.media_dirs.join(",")),
            "lastfm.enabled" => Some(cfg.lastfm.enabled.to_string()),
            "lastfm.username" => Some(cfg.lastfm.username.clone()),
            "lastfm.api_key" => Some(cfg.lastfm.api_key.clone()),
            "lastfm.shared_secret" => Some("***".to_string()),
            "lastfm.auto_scrobble" => Some(cfg.lastfm.auto_scrobble.to_string()),
            "midi.soundfont" => Some(cfg.midi.soundfont.clone()),
            "midi.enabled" => Some(cfg.midi.enabled.to_string()),
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
            "play.always_on_top" => cfg.play.always_on_top = value == "true",
            "appearance.dark_mode" | "dark_mode" => cfg.appearance.dark_mode = value == "true",
            "appearance.spectrum_columns" => { if let Ok(v) = value.parse() { cfg.appearance.spectrum_columns = v; } }
            "appearance.fft_size" => { if let Ok(v) = value.parse() { cfg.appearance.fft_size = v; } }
            "appearance.spectrum_style" | "spectrum_style" => cfg.appearance.spectrum_style = value.to_string(),
            "appearance.theme" | "theme" => cfg.appearance.theme = value.to_string(),
            "lyric.fuzzy_match" => cfg.lyric.fuzzy_match = value == "true",
            "lyric.show_translate" => cfg.lyric.show_translate = value == "true",
            "lyric.use_inner_lyric_first" => cfg.lyric.use_inner_lyric_first = value == "true",
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
