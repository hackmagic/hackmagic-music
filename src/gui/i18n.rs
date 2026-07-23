//! Internationalization (i18n) system.


/// Supported languages.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    EnUs,
    ZhCn,
}

impl Lang {
    pub fn from_config(s: &str) -> Self {
        match s {
            "zh-CN" | "zh" | "zh_CN" | "chinese" => Lang::ZhCn,
            _ => Lang::EnUs,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Lang::EnUs => "en-US",
            Lang::ZhCn => "zh-CN",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Lang::EnUs => "English",
            Lang::ZhCn => "简体中文",
        }
    }
}

/// All translatable UI strings.
#[derive(Clone)]
pub struct Locale {
    pub lang: Lang,
    pub tr: &'static Tr,
}

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

static GLOBAL_LANG: AtomicU8 = AtomicU8::new(0);

pub fn global_tr() -> &'static Tr {
    static EN: OnceLock<&'static Tr> = OnceLock::new();
    static ZH: OnceLock<&'static Tr> = OnceLock::new();
    match GLOBAL_LANG.load(Ordering::Relaxed) {
        1 => *ZH.get_or_init(|| Tr::ZH),
        _ => *EN.get_or_init(|| Tr::EN),
    }
}

pub fn set_global_lang(lang: Lang) {
    GLOBAL_LANG.store(match lang { Lang::EnUs => 0, Lang::ZhCn => 1 }, Ordering::Relaxed);
}

/// Static translation table (one per language).
#[derive(Clone)]
pub struct Tr {
    // -- Window --
    pub app_title: &'static str,
    pub engine_label: &'static str,

    // -- Menu --
    pub menu_file: &'static str,
    pub menu_playback: &'static str,
    pub menu_playlist: &'static str,
    pub menu_lyric: &'static str,
    pub menu_view: &'static str,
    pub menu_tools: &'static str,
    pub menu_settings: &'static str,
    pub menu_help: &'static str,
    // -- File menu items --
    pub menu_open_file: &'static str,
    pub menu_open_folder: &'static str,
    pub menu_open_url: &'static str,
    pub menu_open_playlist: &'static str,
    pub menu_exit: &'static str,
    // -- Playback menu items --
    pub ctrl_stop: &'static str,
    pub ctrl_rew: &'static str,
    pub ctrl_ff: &'static str,
    pub menu_speed_up: &'static str,
    pub menu_slow_down: &'static str,
    pub menu_original_speed: &'static str,
    pub menu_cycle_repeat: &'static str,
    pub menu_ab_repeat: &'static str,
    // -- Playlist menu items --
    pub menu_add_file: &'static str,
    pub menu_add_folder: &'static str,
    pub menu_add_url: &'static str,
    pub menu_clear_list: &'static str,
    pub menu_remove_duplicates: &'static str,
    pub menu_remove_invalid: &'static str,
    pub menu_reload_playlist: &'static str,
    pub menu_save_as_new: &'static str,
    pub menu_delete_from_disk: &'static str,
    pub menu_locate_current: &'static str,
    // -- Lyric menu items --
    pub menu_reload_lyric: &'static str,
    pub menu_copy_current_line: &'static str,
    pub menu_copy_all_lyric: &'static str,
    pub menu_edit_lyric: &'static str,
    pub menu_show_translation: &'static str,
    pub menu_show_desktop_lyric: &'static str,
    pub menu_download_lyric: &'static str,
    pub menu_batch_download_lyric: &'static str,
    // -- View menu items --
    pub menu_toggle_playlist: &'static str,
    pub menu_float_playlist: &'static str,
    pub menu_toggle_menubar: &'static str,
    pub menu_toggle_statusbar: &'static str,
    pub menu_always_on_top: &'static str,
    pub menu_mini_mode: &'static str,
    pub menu_fullscreen: &'static str,
    pub menu_toggle_dark_mode: &'static str,
    pub menu_toggle_light_mode: &'static str,
    // -- Tools menu items --
    pub menu_find: &'static str,
    pub menu_equalizer: &'static str,
    // -- Help menu items --
    pub menu_help_content: &'static str,
    pub menu_about: &'static str,

    // -- Navigation sidebar --
    pub nav_now_playing: &'static str,
    pub nav_play_queue: &'static str,
    pub nav_recent: &'static str,
    pub nav_folders: &'static str,
    pub nav_playlists: &'static str,
    pub nav_favorites: &'static str,
    pub nav_media_lib: &'static str,

    // -- Play queue panel --
    pub pq_title: &'static str,
    pub pq_search: &'static str,
    pub pq_sort: &'static str,
    pub pq_clear: &'static str,
    pub pq_empty: &'static str,
    pub pq_count: &'static str,             // "{} songs"
    pub pq_unknown: &'static str,

    // -- Player info (compact) --
    pub info_no_track: &'static str,
    pub info_open_file: &'static str,
    pub info_unknown_artist: &'static str,

    // -- Controls --
    pub ctrl_repeat: &'static str,
    pub ctrl_prev: &'static str,
    pub ctrl_play: &'static str,
    pub ctrl_pause: &'static str,
    pub ctrl_next: &'static str,
    pub ctrl_vol_down: &'static str,
    pub ctrl_vol_up: &'static str,
    pub ctrl_toggle_play: &'static str,

    // -- Playback menu (extended) --
    pub menu_rewind_5s: &'static str,
    pub menu_forward_5s: &'static str,
    pub menu_pitch_up: &'static str,
    pub menu_pitch_down: &'static str,
    pub menu_original_pitch: &'static str,
    pub menu_ab_set_a: &'static str,
    pub menu_ab_set_b: &'static str,
    pub menu_ab_continue: &'static str,
    pub menu_ab_clear: &'static str,

    // -- Playlist menu (extended) --
    pub menu_add_from_lib: &'static str,
    pub menu_remove_selected: &'static str,
    pub menu_repair_paths: &'static str,
    pub menu_save_playlist: &'static str,
    pub menu_sort: &'static str,
    pub menu_sort_artist: &'static str,
    pub menu_sort_album: &'static str,
    pub menu_sort_duration: &'static str,
    pub menu_sort_filename: &'static str,
    pub menu_sort_random: &'static str,
    pub menu_sort_reverse: &'static str,

    // -- Lyric menu (extended) --
    pub menu_show_lyric: &'static str,
    pub menu_hide_lyric: &'static str,
    pub menu_desktop_lock: &'static str,
    pub menu_lyric_advance: &'static str,
    pub menu_lyric_retreat: &'static str,
    pub menu_save_lyric_edit: &'static str,
    pub menu_associate_lyric: &'static str,
    pub menu_embed_lyric: &'static str,

    // -- Tools menu (extended) --
    pub menu_browse_dir: &'static str,
    pub menu_song_info: &'static str,
    pub menu_switch_theme: &'static str,

    // -- Context menu --
    pub menu_add_to_playlist: &'static str,
    pub menu_open_file_location: &'static str,
    pub menu_copy_path: &'static str,
    pub menu_play_next: &'static str,
    pub menu_remove_track: &'static str,
    pub menu_favourite: &'static str,
    pub menu_unfavourite: &'static str,
    pub menu_properties: &'static str,
    pub menu_clear_rating: &'static str,
    pub menu_find_similar: &'static str,
    pub menu_rating_1: &'static str,
    pub menu_rating_2: &'static str,
    pub menu_rating_3: &'static str,
    pub menu_rating_4: &'static str,
    pub menu_rating_5: &'static str,

    // -- Dialog content texts --
    pub cover_info: &'static str,         // "This track has {} cover image(s)\nFormat: {}\nSize: {} KB"
    pub cover_none: &'static str,         // "No embedded cover"
    pub cover_error: &'static str,        // "Failed to read cover: {}"
    pub format_unsupported: &'static str,// "Unsupported format"

    // -- Desktop lyric --
    pub menu_close_desktop_lyric: &'static str,
    pub lyrics_empty: &'static str,
    pub lyrics_hint: &'static str,
    pub lyrics_file_empty: &'static str,

    // -- Tools menu (more) --
    pub menu_format_convert: &'static str,
    pub menu_charset_convert: &'static str,
    pub menu_online_tags: &'static str,
    pub menu_cover_preview: &'static str,
    pub menu_timer_shutdown: &'static str,
    pub menu_file_association: &'static str,
    pub menu_listen_stats: &'static str,
    pub menu_dev_progress: &'static str,
    pub menu_create_shortcut: &'static str,
    pub menu_reinit_player: &'static str,

    // -- Help menu (more) --
    pub menu_online_help: &'static str,
    pub menu_check_update: &'static str,
    pub menu_supported_formats: &'static str,

    // -- About dialog --
    pub about_original: &'static str,
    pub about_platforms: &'static str,
    pub about_copyright: &'static str,

    // -- Repeat mode labels (displayed in status bar) --
    pub repeat_loop_pl: &'static str,
    pub repeat_loop_trk: &'static str,
    pub repeat_random: &'static str,
    pub repeat_shuffle: &'static str,
    pub repeat_order: &'static str,
    pub repeat_single: &'static str,

    // -- Status bar --
    pub status_fps: &'static str,           // "FPS: {:.0}"
    pub status_next: &'static str,           // "Next: {}"
    pub status_next_empty: &'static str,

    // -- Settings dialog --
    pub settings_title: &'static str,
    pub settings_tab_general: &'static str,
    pub settings_tab_appearance: &'static str,
    pub settings_tab_playback: &'static str,
    pub settings_tab_lyrics: &'static str,
    pub settings_tab_equalizer: &'static str,
    pub settings_tab_hotkeys: &'static str,
    pub settings_tab_media_lib: &'static str,
    pub settings_lang_label: &'static str,
    pub settings_auto_download: &'static str,
    pub settings_check_update: &'static str,
    pub settings_minimize_tray: &'static str,
    pub settings_theme_label: &'static str,
    pub settings_dark_mode: &'static str,
    pub settings_show_spectrum: &'static str,
    pub settings_window_opacity: &'static str,
    pub settings_always_status: &'static str,
    pub settings_engine_label: &'static str,
    pub settings_auto_play: &'static str,
    pub settings_fade: &'static str,
    pub settings_remember_pos: &'static str,
    pub settings_lyric_download: &'static str,
    pub settings_lyric_font: &'static str,
    pub settings_desktop_lyric: &'static str,
    pub settings_lyric_dual: &'static str,
    pub settings_hk_enable: &'static str,
    pub settings_hk_play_pause: &'static str,
    pub settings_hk_next: &'static str,
    pub settings_hk_prev: &'static str,
    pub settings_hk_vol_up: &'static str,
    pub settings_hk_vol_down: &'static str,
    pub settings_ml_folders: &'static str,
    pub settings_auto_scan: &'static str,
    pub settings_ml_ignore_short: &'static str,

    // -- Media library --
    pub media_lib_title: &'static str,
    pub media_lib_scan: &'static str,
    pub media_lib_search: &'static str,

    // -- File dialogs --
    pub dlg_open_title: &'static str,
    pub dlg_folder_title: &'static str,

    // -- Playlist item format --
    pub fmt_track: &'static str,            // "{}. {}"
}

impl Default for Locale {
    fn default() -> Self {
        Self { lang: Lang::EnUs, tr: &Tr::EN }
    }
}

impl Locale {
    pub fn new(lang: Lang) -> Self {
        let tr = match lang {
            Lang::EnUs => &Tr::EN,
            Lang::ZhCn => &Tr::ZH,
        };
        Self { lang, tr }
    }
}

// =========================================================================
// Translation tables
// =========================================================================

#[allow(non_upper_case_globals)]
impl Tr {
    const EN: &'static Tr = &Tr {
        app_title: "HackMagic Music Player",
        engine_label: "BASS",

        menu_file: "File",
        menu_playback: "Playback",
        menu_playlist: "Playlist",
        menu_lyric: "Lyric",
        menu_view: "View",
        menu_tools: "Tools",
        menu_settings: "Settings",
        menu_help: "Help",
        menu_open_file: "Open File",
        menu_open_folder: "Open Folder",
        menu_open_url: "Open URL",
        menu_open_playlist: "Open Playlist",
        menu_exit: "Exit",
        ctrl_stop: "Stop",
        ctrl_rew: "Rewind",
        ctrl_ff: "Fast Forward",
        menu_speed_up: "Speed Up",
        menu_slow_down: "Slow Down",
        menu_original_speed: "Original Speed",
        menu_cycle_repeat: "Cycle Repeat Mode",
        menu_ab_repeat: "AB Repeat",
        menu_add_file: "Add File",
        menu_add_folder: "Add Folder",
        menu_add_url: "Add URL",
        menu_clear_list: "Clear List",
        menu_remove_duplicates: "Remove Duplicates",
        menu_remove_invalid: "Remove Invalid",
        menu_reload_playlist: "Reload Playlist",
        menu_save_as_new: "Save As New",
        menu_delete_from_disk: "Delete From Disk",
        menu_locate_current: "Locate Current",
        menu_reload_lyric: "Reload Lyric",
        menu_copy_current_line: "Copy Current Line",
        menu_copy_all_lyric: "Copy All Lyric",
        menu_edit_lyric: "Edit Lyric",
        menu_show_translation: "Show Translation",
        menu_show_desktop_lyric: "Show Desktop Lyric",
        menu_download_lyric: "Download Lyric",
        menu_batch_download_lyric: "Batch Download Lyric",
        menu_toggle_playlist: "Toggle Playlist",
        menu_float_playlist: "Float Playlist",
        menu_toggle_menubar: "Toggle Menu Bar",
        menu_toggle_statusbar: "Toggle Status Bar",
        menu_always_on_top: "Always on Top",
        menu_mini_mode: "Mini Mode",
        menu_fullscreen: "Fullscreen",
        menu_toggle_dark_mode: "Toggle Dark Mode",
        menu_toggle_light_mode: "Light Mode",
        menu_find: "Find",
        menu_equalizer: "Equalizer",
        menu_help_content: "Help",
        menu_about: "About",

        nav_now_playing: "NOW PLAYING",
        nav_play_queue: "PLAY QUEUE",
        nav_recent: "RECENTLY PLAYED",
        nav_folders: "FOLDERS",
        nav_playlists: "PLAYLISTS",
        nav_favorites: "FAVORITES",
        nav_media_lib: "MEDIA LIBRARY",

        pq_title: "PLAY QUEUE",
        pq_search: "\u{1F50D} Search songs...",
        pq_sort: "AZ",
        pq_clear: "CLR",
        pq_empty: "No songs, drag files here",
        pq_count: "{} songs",
        pq_unknown: "Unknown",

        info_no_track: "No track playing",
        info_open_file: "Open a file to start listening",
        info_unknown_artist: "Unknown Artist",

        ctrl_repeat: "REP",
        ctrl_prev: "|<",
        ctrl_play: ">",
        ctrl_pause: "||",
        ctrl_next: ">|",
        ctrl_vol_down: "-",
        ctrl_vol_up: "+",

        repeat_loop_pl: "Loop",
        repeat_loop_trk: "Single",
        repeat_random: "Random",
        repeat_shuffle: "Shuffle",
        repeat_order: "Order",
        repeat_single: "Single",

        status_fps: "FPS: {:.0}",
        status_next: "Next: {}",
        status_next_empty: "Next: --",

        settings_title: "Settings",
        settings_tab_general: "General",
        settings_tab_appearance: "Appearance",
        settings_tab_playback: "Playback",
        settings_tab_lyrics: "Lyrics",
        settings_tab_equalizer: "Equalizer",
        settings_tab_hotkeys: "Hotkeys",
        settings_tab_media_lib: "Media Library",
        settings_lang_label: "Language",
        settings_auto_download: "Auto-download lyrics & covers",
        settings_check_update: "Check for updates on startup",
        settings_minimize_tray: "Minimize to system tray",
        settings_theme_label: "Theme color",
        settings_dark_mode: "Dark mode",
        settings_show_spectrum: "Show spectrum analyzer",
        settings_window_opacity: "Window opacity",
        settings_always_status: "Always show status bar",
        settings_engine_label: "Audio engine",
        settings_auto_play: "Auto-play on startup",
        settings_fade: "Fade in/out on track change",
        settings_remember_pos: "Remember playback position",
        settings_lyric_download: "Auto-download lyrics",
        settings_lyric_font: "Lyrics font and size",
        settings_desktop_lyric: "Desktop lyrics settings",
        settings_lyric_dual: "Dual-line lyrics display",
        settings_hk_enable: "Enable global hotkeys",
        settings_hk_play_pause: "Play/Pause",
        settings_hk_next: "Next track",
        settings_hk_prev: "Previous track",
        settings_hk_vol_up: "Volume up",
        settings_hk_vol_down: "Volume down",
        settings_ml_folders: "Media library folders",
        settings_auto_scan: "Auto-scan on startup",
        settings_ml_ignore_short: "Ignore short tracks",

        media_lib_title: "Media Library",
        media_lib_scan: "Scan",
        media_lib_search: "\u{1F50D} Search media library...",

        dlg_open_title: "Select Audio File",
        dlg_folder_title: "Select Music Folder",

        fmt_track: "{}. {}",

        ctrl_toggle_play: "Play / Pause",
        menu_rewind_5s: "Rewind 5s",
        menu_forward_5s: "Forward 5s",
        menu_pitch_up: "Pitch Up",
        menu_pitch_down: "Pitch Down",
        menu_original_pitch: "Original Pitch",
        menu_ab_set_a: "Set Point A",
        menu_ab_set_b: "Set Point B",
        menu_ab_continue: "AB Repeat Continue",
        menu_ab_clear: "Clear AB Loop",
        menu_add_from_lib: "Add from Library",
        menu_remove_selected: "Remove Selected",
        menu_repair_paths: "Repair Paths",
        menu_save_playlist: "Save Playlist",
        menu_sort: "Sort",
        menu_sort_artist: "By Artist",
        menu_sort_album: "By Album",
        menu_sort_duration: "By Duration",
        menu_sort_filename: "By Filename",
        menu_sort_random: "Randomize",
        menu_sort_reverse: "Reverse Order",
        menu_show_lyric: "Show Lyrics",
        menu_hide_lyric: "Hide Lyrics",
        menu_desktop_lock: "Lock Desktop Lyrics",
        menu_lyric_advance: "Advance +0.5s",
        menu_lyric_retreat: "Retreat -0.5s",
        menu_save_lyric_edit: "Save Lyric Changes",
        menu_associate_lyric: "Associate Local Lyric",
        menu_embed_lyric: "Embed Lyrics into File",
        menu_browse_dir: "Browse File Path",
        menu_song_info: "Song Info",
        menu_switch_theme: "Switch Theme Color",
        menu_add_to_playlist: "Add to Playlist",
        menu_open_file_location: "Open File Location",
        menu_copy_path: "Copy Path",
        menu_play_next: "Play Next",
        menu_remove_track: "Remove",
        menu_favourite: "Favorite",
        menu_unfavourite: "Unfavorite",
        menu_properties: "Properties",
        menu_clear_rating: "Clear Rating",
        menu_find_similar: "Find Similar",
        menu_rating_1: "Rating: ★☆☆☆☆",
        menu_rating_2: "Rating: ★★☆☆☆",
        menu_rating_3: "Rating: ★★★☆☆",
        menu_rating_4: "Rating: ★★★★☆",
        menu_rating_5: "Rating: ★★★★★",
    cover_info: "This track has {0} cover image(s)\nFormat: {1}\nSize: {2} KB",
    cover_none: "No embedded cover",
    cover_error: "Failed to read cover: {0}",
    format_unsupported: "Unsupported format",
        menu_close_desktop_lyric: "Close Desktop Lyrics",
        lyrics_empty: "No lyrics",
        lyrics_hint: "Open a music file to show lyrics",
        lyrics_file_empty: "Lyrics file is empty",
        menu_format_convert: "Format Convert",
        menu_charset_convert: "SC/TC Convert",
        menu_online_tags: "Get Tags Online",
        menu_cover_preview: "Cover Preview",
        menu_timer_shutdown: "Sleep Timer",
        menu_file_association: "File Association",
        menu_listen_stats: "Listening Stats",
        menu_dev_progress: "Dev Progress",
        menu_create_shortcut: "Create Shortcut",
        menu_reinit_player: "Reinitialize Player",
        menu_online_help: "Online Help",
        menu_check_update: "Check Update",
        menu_supported_formats: "Supported Formats",
        about_original: "Original: MusicPlayer2 by zhongyang219",
        about_platforms: "Platforms: Windows / macOS / Linux",
        about_copyright: "© 2026 HackMagic Team",
    };

    const ZH: &'static Tr = &Tr {
        app_title: "HackMagic 音乐播放器",
        engine_label: "BASS",

        menu_file: "文件",
        menu_playback: "播放",
        menu_playlist: "播放列表",
        menu_lyric: "歌词",
        menu_view: "视图",
        menu_tools: "工具",
        menu_settings: "设置",
        menu_help: "帮助",
        menu_open_file: "打开文件",
        menu_open_folder: "打开文件夹",
        menu_open_url: "打开URL",
        menu_open_playlist: "打开播放列表",
        menu_exit: "退出",
        ctrl_stop: "停止",
        ctrl_rew: "快退",
        ctrl_ff: "快进",
        menu_speed_up: "加速",
        menu_slow_down: "减速",
        menu_original_speed: "原速",
        menu_cycle_repeat: "循环模式",
        menu_ab_repeat: "AB重复",
        menu_add_file: "添加文件",
        menu_add_folder: "添加文件夹",
        menu_add_url: "添加URL",
        menu_clear_list: "清空列表",
        menu_remove_duplicates: "移除相同曲目",
        menu_remove_invalid: "移除无效项目",
        menu_reload_playlist: "重新载入",
        menu_save_as_new: "另存为新播放列表",
        menu_delete_from_disk: "从磁盘删除",
        menu_locate_current: "定位到当前",
        menu_reload_lyric: "重新载入歌词",
        menu_copy_current_line: "复制当前行",
        menu_copy_all_lyric: "复制全部歌词",
        menu_edit_lyric: "编辑歌词",
        menu_show_translation: "显示翻译",
        menu_show_desktop_lyric: "显示桌面歌词",
        menu_download_lyric: "下载歌词",
        menu_batch_download_lyric: "批量下载歌词",
        menu_toggle_playlist: "显示/隐藏播放列表",
        menu_float_playlist: "浮动播放列表",
        menu_toggle_menubar: "显示菜单栏",
        menu_toggle_statusbar: "显示状态栏",
        menu_always_on_top: "总在最前",
        menu_mini_mode: "迷你模式",
        menu_fullscreen: "全屏",
        menu_toggle_dark_mode: "深色模式",
        menu_toggle_light_mode: "浅色模式",
        menu_find: "查找",
        menu_equalizer: "均衡器",
        menu_help_content: "帮助",
        menu_about: "关于",

        nav_now_playing: "正在播放",
        nav_play_queue: "播放队列",
        nav_recent: "最近播放",
        nav_folders: "文件夹",
        nav_playlists: "播放列表",
        nav_favorites: "我喜欢的音乐",
        nav_media_lib: "媒体库",

        pq_title: "播放队列",
        pq_search: "\u{1F50D} 搜索歌曲...",
        pq_sort: "排序",
        pq_clear: "清空",
        pq_empty: "暂无歌曲，拖拽文件到此处",
        pq_count: "共 {} 首",
        pq_unknown: "未知",

        info_no_track: "暂无播放",
        info_open_file: "打开文件开始欣赏音乐",
        info_unknown_artist: "未知艺术家",

        ctrl_repeat: "模式",
        ctrl_prev: "上一首",
        ctrl_play: "播放",
        ctrl_pause: "暂停",
        ctrl_next: "下一首",
        ctrl_vol_down: "减",
        ctrl_vol_up: "加",

        repeat_loop_pl: "列表循环",
        repeat_loop_trk: "单曲循环",
        repeat_random: "随机播放",
        repeat_shuffle: "洗牌",
        repeat_order: "顺序播放",
        repeat_single: "单曲播放",

        status_fps: "帧率: {:.0}",
        status_next: "下一首: {}",
        status_next_empty: "下一首: --",

        settings_title: "设置",
        settings_tab_general: "常规",
        settings_tab_appearance: "外观",
        settings_tab_playback: "播放",
        settings_tab_lyrics: "歌词",
        settings_tab_equalizer: "均衡器",
        settings_tab_hotkeys: "快捷键",
        settings_tab_media_lib: "媒体库",
        settings_lang_label: "语言",
        settings_auto_download: "自动下载歌词与封面",
        settings_check_update: "启动时检查更新",
        settings_minimize_tray: "最小化到系统托盘",
        settings_theme_label: "主题颜色",
        settings_dark_mode: "深色模式",
        settings_show_spectrum: "显示频谱分析",
        settings_window_opacity: "窗口透明度",
        settings_always_status: "总是显示状态栏",
        settings_engine_label: "播放引擎",
        settings_auto_play: "启动时自动播放",
        settings_fade: "切歌时淡入淡出",
        settings_remember_pos: "记忆播放位置",
        settings_lyric_download: "自动下载歌词",
        settings_lyric_font: "歌词字体与大小",
        settings_desktop_lyric: "桌面歌词设置",
        settings_lyric_dual: "双行歌词显示",
        settings_hk_enable: "启用全局快捷键",
        settings_hk_play_pause: "播放/暂停",
        settings_hk_next: "下一曲",
        settings_hk_prev: "上一曲",
        settings_hk_vol_up: "音量增加",
        settings_hk_vol_down: "音量减少",
        settings_ml_folders: "媒体库文件夹",
        settings_auto_scan: "启动时自动扫描",
        settings_ml_ignore_short: "忽略短文件",

        media_lib_title: "媒体库",
        media_lib_scan: "扫描",
        media_lib_search: "\u{1F50D} 搜索媒体库...",

        dlg_open_title: "选择音频文件",
        dlg_folder_title: "选择音乐文件夹",

        fmt_track: "{}. {}",

        ctrl_toggle_play: "播放/暂停",
        menu_rewind_5s: "快退5秒",
        menu_forward_5s: "快进5秒",
        menu_pitch_up: "升高音调",
        menu_pitch_down: "降低音调",
        menu_original_pitch: "原始音调",
        menu_ab_set_a: "设置 A 点",
        menu_ab_set_b: "设置 B 点",
        menu_ab_continue: "AB复读继续",
        menu_ab_clear: "清除 AB 循环",
        menu_add_from_lib: "从媒体库添加",
        menu_remove_selected: "删除选中",
        menu_repair_paths: "修复路径错误",
        menu_save_playlist: "保存播放列表",
        menu_sort: "排序",
        menu_sort_artist: "按艺术家排序",
        menu_sort_album: "按专辑排序",
        menu_sort_duration: "按时长排序",
        menu_sort_filename: "按文件名排序",
        menu_sort_random: "随机排序",
        menu_sort_reverse: "倒序排列",
        menu_show_lyric: "显示歌词",
        menu_hide_lyric: "隐藏歌词",
        menu_desktop_lock: "桌面歌词锁定",
        menu_lyric_advance: "歌词前进0.5秒",
        menu_lyric_retreat: "歌词后退0.5秒",
        menu_save_lyric_edit: "保存歌词改动",
        menu_associate_lyric: "关联本地歌词",
        menu_embed_lyric: "内嵌歌词到文件",
        menu_browse_dir: "探索文件路径",
        menu_song_info: "歌曲信息",
        menu_switch_theme: "切换主题颜色",
        menu_add_to_playlist: "添加到播放列表",
        menu_open_file_location: "打开文件位置",
        menu_copy_path: "复制路径",
        menu_play_next: "下一首播放",
        menu_remove_track: "移除",
        menu_favourite: "收藏",
        menu_unfavourite: "取消收藏",
        menu_properties: "属性",
        menu_clear_rating: "清除评级",
        menu_find_similar: "查找相似歌曲",
        menu_rating_1: "评级: ★☆☆☆☆",
        menu_rating_2: "评级: ★★☆☆☆",
        menu_rating_3: "评级: ★★★☆☆",
        menu_rating_4: "评级: ★★★★☆",
        menu_rating_5: "评级: ★★★★★",
    cover_info: "该曲目包含 {0} 张封面图片\n格式: {1}\n大小: {2} KB",
    cover_none: "该曲目没有内嵌封面",
    cover_error: "读取封面失败: {0}",
    format_unsupported: "不支持的格式",
        menu_close_desktop_lyric: "关闭桌面歌词",
        lyrics_empty: "暂无歌词",
        lyrics_hint: "打开音乐文件以显示歌词",
        lyrics_file_empty: "歌词文件为空",
        menu_format_convert: "格式转换",
        menu_charset_convert: "繁简转换",
        menu_online_tags: "在线获取标签",
        menu_cover_preview: "封面预览",
        menu_timer_shutdown: "定时停止",
        menu_file_association: "文件关联",
        menu_listen_stats: "收听统计",
        menu_dev_progress: "开发进度",
        menu_create_shortcut: "创建快捷方式",
        menu_reinit_player: "重新初始化播放器",
        menu_online_help: "在线帮助",
        menu_check_update: "检查更新",
        menu_supported_formats: "支持的格式",
        about_original: "原始项目: MusicPlayer2 by zhongyang219",
        about_platforms: "支持: Windows / macOS / Linux",
        about_copyright: "© 2026 HackMagic Team",
    };
}
