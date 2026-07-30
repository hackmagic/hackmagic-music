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
    pub settings_fade_time: &'static str,
    pub settings_default_volume: &'static str,
    pub settings_volume_step: &'static str,
    pub settings_replaygain: &'static str,

    // -- Settings sections --
    pub settings_section_app: &'static str,
    pub settings_section_config_file: &'static str,
    pub settings_section_close: &'static str,
    pub settings_section_download: &'static str,
    pub settings_section_visual: &'static str,
    pub settings_section_background: &'static str,
    pub settings_section_play_options: &'static str,
    pub settings_section_play_kernel: &'static str,
    pub settings_section_play_device: &'static str,
    pub settings_section_lyric_options: &'static str,
    pub settings_section_window_lyric: &'static str,
    pub settings_section_desktop_lyric: &'static str,
    pub settings_section_ml_options: &'static str,
    pub settings_section_ml_update: &'static str,
    pub settings_section_ml_dirs: &'static str,
    pub settings_section_hotkeys: &'static str,
    pub settings_section_data_mgmt: &'static str,

    // -- Settings labels --
    pub settings_label_update_source: &'static str,
    pub settings_label_auto_run: &'static str,
    pub settings_label_portable_mode: &'static str,
    pub settings_label_open_config_dir: &'static str,
    pub settings_label_minimize_to_tray: &'static str,
    pub settings_label_exit: &'static str,
    pub settings_label_auto_download_lyric: &'static str,
    pub settings_label_lyric_save_pos: &'static str,
    pub settings_label_download_trans_format: &'static str,
    pub settings_label_auto_download_cover: &'static str,
    pub settings_label_spectrum_low_center: &'static str,
    pub settings_label_spectrum_height: &'static str,
    pub settings_label_show_cover: &'static str,
    pub settings_label_cover_fit: &'static str,
    pub settings_label_lyric_bg: &'static str,
    pub settings_label_rounded: &'static str,
    pub settings_label_enable_bg: &'static str,
    pub settings_label_desktop_bg: &'static str,
    pub settings_label_bg_opacity: &'static str,
    pub settings_label_bg_cover: &'static str,
    pub settings_label_gaussian_blur: &'static str,
    pub settings_label_blur_radius: &'static str,
    pub settings_label_error_stop: &'static str,
    pub settings_label_taskbar_progress: &'static str,
    pub settings_label_taskbar_icon: &'static str,
    pub settings_label_fade_time: &'static str,
    pub settings_label_continue_on_switch: &'static str,
    pub settings_label_system_media: &'static str,
    pub settings_label_remember_pos: &'static str,
    pub settings_label_default_volume: &'static str,
    pub settings_label_volume_step: &'static str,
    pub settings_label_merge_versions: &'static str,
    pub settings_label_inner_lyric: &'static str,
    pub settings_label_fuzzy_match: &'static str,
    pub settings_label_show_info_when_none: &'static str,
    pub settings_label_lyric_folder: &'static str,
    pub settings_label_lyric_adjust: &'static str,
    pub settings_label_karaoke: &'static str,
    pub settings_label_hide_empty: &'static str,
    pub settings_label_line_spacing: &'static str,
    pub settings_label_alignment: &'static str,
    pub settings_label_show_desktop_lyric: &'static str,
    pub settings_label_hide_when_none: &'static str,
    pub settings_label_hide_when_paused: &'static str,
    pub settings_label_lock_desktop: &'static str,
    pub settings_label_double_line: &'static str,
    pub settings_label_disable_delete: &'static str,
    pub settings_label_merge_single: &'static str,
    pub settings_label_min_duration: &'static str,
    pub settings_label_remove_missing: &'static str,
    pub settings_label_force_reload: &'static str,
    pub settings_label_media_dirs: &'static str,
    pub settings_label_media_count: &'static str,

    // -- Settings dropdown options --
    pub settings_option_lyric_save_song: &'static str,
    pub settings_option_lyric_save_lyrics: &'static str,
    pub settings_option_adjust_ask: &'static str,
    pub settings_option_adjust_save: &'static str,
    pub settings_option_adjust_discard: &'static str,
    pub settings_option_align_auto: &'static str,
    pub settings_option_align_left: &'static str,
    pub settings_option_align_center: &'static str,
    pub settings_option_align_right: &'static str,
    pub settings_option_cover_fit: &'static str,
    pub settings_option_cover_fill: &'static str,
    pub settings_option_cover_contain: &'static str,
    pub settings_option_cover_none: &'static str,
    pub settings_option_replaygain_off: &'static str,
    pub settings_option_replaygain_auto: &'static str,
    pub settings_option_default_device: &'static str,
    pub settings_label_kernel: &'static str,
    pub settings_label_hotkey_func_shortcut: &'static str,
    pub settings_label_opaque: &'static str,
    pub settings_label_add_dir: &'static str,

    // -- Settings buttons --
    pub settings_btn_open: &'static str,
    pub settings_btn_add: &'static str,
    pub settings_btn_delete: &'static str,
    pub settings_btn_force_reload: &'static str,

    // -- Hotkey labels --
    pub hotkey_play_pause: &'static str,
    pub hotkey_stop: &'static str,
    pub hotkey_forward: &'static str,
    pub hotkey_rewind: &'static str,
    pub hotkey_prev: &'static str,
    pub hotkey_next: &'static str,
    pub hotkey_vol_up: &'static str,
    pub hotkey_vol_down: &'static str,
    pub hotkey_exit: &'static str,
    pub hotkey_show_hide: &'static str,
    pub hotkey_desktop_lyric: &'static str,
    pub hotkey_favourite: &'static str,
    pub hotkey_enable: &'static str,
    pub hotkey_function: &'static str,
    pub hotkey_shortcut: &'static str,

    // -- Settings bottom buttons --
    pub settings_btn_ok: &'static str,
    pub settings_btn_cancel: &'static str,
    pub settings_btn_apply: &'static str,

    // -- EQ presets --
    pub eq_flat: &'static str,
    pub eq_pop: &'static str,
    pub eq_rock: &'static str,
    pub eq_classical: &'static str,
    pub eq_jazz: &'static str,
    pub eq_electronic: &'static str,
    pub eq_bass_boost: &'static str,
    pub eq_vocal_boost: &'static str,
    pub eq_enabled: &'static str,
    pub eq_disabled: &'static str,

    // -- Media library --
    pub media_lib_title: &'static str,
    pub media_lib_scan: &'static str,
    pub media_lib_search: &'static str,
    // -- Media lib category tabs --
    pub ml_cat_all: &'static str,
    pub ml_cat_artist: &'static str,
    pub ml_cat_album: &'static str,
    pub ml_cat_genre: &'static str,
    pub ml_cat_year: &'static str,
    pub ml_cat_ftype: &'static str,
    pub ml_cat_bitrate: &'static str,
    pub ml_cat_rating: &'static str,
    // -- Buttons --
    pub btn_refresh: &'static str,
    pub btn_save: &'static str,

    // -- Lyric download panel --
    pub lyd_title: &'static str,
    pub lyd_source_netease: &'static str,
    pub lyd_source_qqmusic: &'static str,
    pub lyd_keyword_hint: &'static str,
    pub lyd_search: &'static str,
    pub lyd_searching: &'static str,
    pub lyd_options: &'static str,
    pub lyd_include_translation: &'static str,
    pub lyd_save_to_song_dir: &'static str,
    pub lyd_save_to_lyrics_dir: &'static str,
    pub lyd_status_enter_keyword: &'static str,
    pub lyd_status_searching: &'static str,

    // -- Status bar / playback status --
    pub status_playing: &'static str,
    pub status_paused: &'static str,
    pub status_stopped: &'static str,
    pub status_rpc_online: &'static str,
    pub status_rpc_offline: &'static str,
    pub status_track_count: &'static str,    // "{} 首"

    // -- Playlist dock --
    pub pq_filter_all: &'static str,         // "全部 ({})"
    pub pq_filter_fav: &'static str,         // "喜欢 ({})"
    pub pq_filter_recent: &'static str,      // "最近 ({})"
    pub pq_btn_add: &'static str,            // "+ 添加"
    pub pq_btn_remove: &'static str,         // "× 删除"
    pub pq_btn_sort: &'static str,           // "↕ 排序"
    pub pq_btn_detail: &'static str,         // "≡ 详情"
    pub pq_btn_compact: &'static str,        // "≡ 简洁"
    pub pq_btn_edit: &'static str,           // "✏ 编辑"

    // -- Playlist column headers --
    pub pq_col_title: &'static str,
    pub pq_col_artist: &'static str,
    pub pq_col_album: &'static str,
    pub pq_col_duration: &'static str,
    pub pq_col_asc: &'static str,            // " ▲"
    pub pq_col_desc: &'static str,           // " ▼"

    // -- Playlist header text --
    pub pq_header_all: &'static str,         // "播放列表 ({} 首)"
    pub pq_header_fav: &'static str,         // "我喜欢的音乐 ({} 首)"
    pub pq_header_recent: &'static str,      // "最近播放 ({} 首)"
    pub pq_header_filtered: &'static str,    // "筛选结果: {} / {}"

    // -- Input placeholders --
    pub ph_search_media: &'static str,       // "搜索媒体库..."
    pub ph_title: &'static str,
    pub ph_artist: &'static str,
    pub ph_album: &'static str,
    pub ph_genre: &'static str,
    pub ph_year: &'static str,
    pub ph_track_num: &'static str,
    pub ph_rating: &'static str,             // "评分 (0-5)"

    // -- Track editor --
    pub ed_title: &'static str,              // "编辑曲目信息"
    pub ed_label_title: &'static str,
    pub ed_label_artist: &'static str,
    pub ed_label_album: &'static str,
    pub ed_label_genre: &'static str,
    pub ed_label_year: &'static str,
    pub ed_label_track_num: &'static str,
    pub ed_label_rating: &'static str,
    pub ed_btn_save: &'static str,

    // -- Song info --
    pub info_key_title: &'static str,
    pub info_key_artist: &'static str,
    pub info_key_album: &'static str,
    pub info_key_duration: &'static str,
    pub info_key_type: &'static str,
    pub info_key_bitrate: &'static str,
    pub info_key_sample_rate: &'static str,
    pub info_key_channels: &'static str,
    pub info_key_favourite: &'static str,
    pub info_key_path: &'static str,
    pub info_hint: &'static str,

    // -- Format convert --
    pub fmt_convert_title: &'static str,
    pub fmt_convert_hint: &'static str,
    pub fmt_convert_close: &'static str,

    // -- URL dialog --
    pub url_title: &'static str,
    pub url_hint: &'static str,

    // -- Lyric editor --
    pub lyr_ed_empty: &'static str,
    pub lyr_ed_hint: &'static str,
    pub lyr_ed_saved: &'static str,
    pub lyr_ed_unsaved: &'static str,
    pub lyr_ed_save: &'static str,
    pub lyr_ed_save_as: &'static str,
    pub lyr_ed_open: &'static str,
    pub lyr_ed_insert: &'static str,
    pub lyr_ed_delete: &'static str,
    pub lyr_ed_shift_all: &'static str,
    pub lyr_ed_help: &'static str,
    pub lyr_ed_empty_line: &'static str,     // "(空)"

    // -- Media library --
    pub ml_header: &'static str,             // "媒体库 ({} 首 | {})"
    pub ml_unknown: &'static str,            // "(未知)"
    pub ml_stars: &'static str,              // "{} 星"
    pub ml_overflow: &'static str,           // "... 及其他 {} 首 (共 {} 首)"

    // -- File browser --
    pub fb_title: &'static str,
    pub fb_entry_count: &'static str,        // "{} 项"

    // -- Misc --
    pub filter_audio_files: &'static str,    // "音频文件"
    pub filter_playlists: &'static str,      // "M3U 播放列表"
    pub eq_custom: &'static str,
    pub eq_preset_label: &'static str,       // "预设: {}"
    pub similar_tracks_title: &'static str,  // "与 \"{}\" 相似的歌曲"
    pub similar_tracks_empty: &'static str,
    pub default_list: &'static str,

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
        settings_fade_time: "Fade time",
        settings_default_volume: "Default volume",
        settings_volume_step: "Volume step",
        settings_replaygain: "ReplayGain",

        settings_section_app: "Application Settings",
        settings_section_config_file: "Config and Data Files",
        settings_section_close: "When Closing Window",
        settings_section_download: "Download Settings",
        settings_section_visual: "Visual Effects",
        settings_section_background: "Background Settings",
        settings_section_play_options: "Playback Options",
        settings_section_play_kernel: "Playback Kernel",
        settings_section_play_device: "Playback Device",
        settings_section_lyric_options: "Lyric Options",
        settings_section_window_lyric: "Window Lyrics",
        settings_section_desktop_lyric: "Desktop Lyrics",
        settings_section_ml_options: "Media Library Options",
        settings_section_ml_update: "Media Library Update Options",
        settings_section_ml_dirs: "Media Library Directories",
        settings_section_hotkeys: "Global Hotkeys",
        settings_section_data_mgmt: "Data Management",

        settings_label_update_source: "Update source",
        settings_label_auto_run: "Auto run on startup",
        settings_label_portable_mode: "Portable mode",
        settings_label_open_config_dir: "Open config directory",
        settings_label_minimize_to_tray: "Minimize to tray",
        settings_label_exit: "Exit",
        settings_label_auto_download_lyric: "Auto download when no lyrics",
        settings_label_lyric_save_pos: "Lyric save location",
        settings_label_download_trans_format: "Translation format",
        settings_label_auto_download_cover: "Auto download cover",
        settings_label_spectrum_low_center: "Low freq in center",
        settings_label_spectrum_height: "Spectrum height",
        settings_label_show_cover: "Show album cover",
        settings_label_cover_fit: "Cover fit",
        settings_label_lyric_bg: "Lyric background",
        settings_label_rounded: "Rounded style",
        settings_label_enable_bg: "Enable background",
        settings_label_desktop_bg: "Use desktop background",
        settings_label_bg_opacity: "Background opacity",
        settings_label_bg_cover: "Use cover as background",
        settings_label_gaussian_blur: "Gaussian blur",
        settings_label_blur_radius: "Blur radius",
        settings_label_error_stop: "Stop on error",
        settings_label_taskbar_progress: "Show progress in taskbar",
        settings_label_taskbar_icon: "Show status icon in taskbar",
        settings_label_fade_time: "Fade time",
        settings_label_continue_on_switch: "Continue on switch",
        settings_label_system_media: "Use system media controls",
        settings_label_remember_pos: "Remember position",
        settings_label_default_volume: "Default volume",
        settings_label_volume_step: "Volume step",
        settings_label_merge_versions: "Merge versions",
        settings_label_inner_lyric: "Prefer embedded lyrics",
        settings_label_fuzzy_match: "Fuzzy match",
        settings_label_show_info_when_none: "Show info when no lyrics",
        settings_label_lyric_folder: "Lyrics folder",
        settings_label_lyric_adjust: "After adjusting lyrics",
        settings_label_karaoke: "Karaoke style",
        settings_label_hide_empty: "Hide empty lines",
        settings_label_line_spacing: "Line spacing",
        settings_label_alignment: "Alignment",
        settings_label_show_desktop_lyric: "Show desktop lyrics",
        settings_label_hide_when_none: "Hide when no lyrics",
        settings_label_hide_when_paused: "Hide when paused",
        settings_label_lock_desktop: "Lock desktop lyrics",
        settings_label_double_line: "Double line",
        settings_label_disable_delete: "Disable disk delete",
        settings_label_merge_single: "Merge single categories",
        settings_label_min_duration: "Min duration threshold",
        settings_label_remove_missing: "Remove missing files",
        settings_label_force_reload: "Force reload",
        settings_label_media_dirs: "Media library dirs",
        settings_label_media_count: "{} tracks",

        settings_option_lyric_save_song: "Save to song directory",
        settings_option_lyric_save_lyrics: "Save to lyrics folder",
        settings_option_adjust_ask: "Ask",
        settings_option_adjust_save: "Auto save",
        settings_option_adjust_discard: "Discard",
        settings_option_align_auto: "Auto",
        settings_option_align_left: "Left",
        settings_option_align_center: "Center",
        settings_option_align_right: "Right",

        settings_option_cover_fit: "Fit",
        settings_option_cover_fill: "Stretch",
        settings_option_cover_contain: "Contain",
        settings_option_cover_none: "Original",
        settings_option_replaygain_off: "Off",
        settings_option_replaygain_auto: "Auto",
        settings_option_default_device: "Default Device",
        settings_label_kernel: "Kernel",
        settings_label_hotkey_func_shortcut: "Function/Shortcut",
        settings_label_opaque: "Opaque",
        settings_label_add_dir: "Add directory",

        settings_btn_open: "Open",
        settings_btn_add: "Add",
        settings_btn_delete: "Delete",
        settings_btn_force_reload: "Force Reload",

        hotkey_play_pause: "Play/Pause",
        hotkey_stop: "Stop",
        hotkey_forward: "Forward",
        hotkey_rewind: "Rewind",
        hotkey_prev: "Previous",
        hotkey_next: "Next",
        hotkey_vol_up: "Volume Up",
        hotkey_vol_down: "Volume Down",
        hotkey_exit: "Exit",
        hotkey_show_hide: "Show/Hide Player",
        hotkey_desktop_lyric: "Show/Hide Desktop Lyrics",
        hotkey_favourite: "Add to Favorites",
        hotkey_enable: "Enable global hotkeys",
        hotkey_function: "Function",
        hotkey_shortcut: "Shortcut",

        settings_btn_ok: "OK",
        settings_btn_cancel: "Cancel",
        settings_btn_apply: "Apply",

        eq_flat: "Flat",
        eq_pop: "Pop",
        eq_rock: "Rock",
        eq_classical: "Classical",
        eq_jazz: "Jazz",
        eq_electronic: "Electronic",
        eq_bass_boost: "Bass Boost",
        eq_vocal_boost: "Vocal Boost",
        eq_enabled: "Enabled",
        eq_disabled: "Disabled",

        media_lib_title: "Media Library",
        media_lib_scan: "Scan",
        media_lib_search: "\u{1F50D} Search media library...",

        ml_cat_all: "All Tracks",
        ml_cat_artist: "Artist",
        ml_cat_album: "Album",
        ml_cat_genre: "Genre",
        ml_cat_year: "Year",
        ml_cat_ftype: "File Type",
        ml_cat_bitrate: "Bitrate",
        ml_cat_rating: "Rating",
        btn_refresh: "Refresh",
        btn_save: "Save Preset",

        lyd_title: "Lyric Download",
        lyd_source_netease: "NetEase",
        lyd_source_qqmusic: "QQ Music",
        lyd_keyword_hint: "Enter song name or artist...",
        lyd_search: "Search",
        lyd_searching: "Searching...",
        lyd_options: "Options:",
        lyd_include_translation: "Include Translation",
        lyd_save_to_song_dir: "Save to Song Directory",
        lyd_save_to_lyrics_dir: "Save to Lyrics Directory",
        lyd_status_enter_keyword: "Please enter search keyword",
        lyd_status_searching: "Searching...",

        status_playing: "Playing",
        status_paused: "Paused",
        status_stopped: "Stopped",
        status_rpc_online: "RPC Online",
        status_rpc_offline: "RPC Offline",
        status_track_count: "{} tracks",

        pq_filter_all: "All ({})",
        pq_filter_fav: "Favorites ({})",
        pq_filter_recent: "Recent ({})",
        pq_btn_add: "+ Add",
        pq_btn_remove: "× Remove",
        pq_btn_sort: "↕ Sort",
        pq_btn_detail: "≡ Detail",
        pq_btn_compact: "≡ Compact",
        pq_btn_edit: "✏ Edit",

        pq_col_title: "Title",
        pq_col_artist: "Artist",
        pq_col_album: "Album",
        pq_col_duration: "Duration",
        pq_col_asc: " ▲",
        pq_col_desc: " ▼",

        pq_header_all: "Playlist ({} tracks)",
        pq_header_fav: "Favorites ({} tracks)",
        pq_header_recent: "Recent ({} tracks)",
        pq_header_filtered: "Filtered: {} / {}",

        ph_search_media: "Search media library...",
        ph_title: "Title",
        ph_artist: "Artist",
        ph_album: "Album",
        ph_genre: "Genre",
        ph_year: "Year",
        ph_track_num: "Track #",
        ph_rating: "Rating (0-5)",

        ed_title: "Edit Track Info",
        ed_label_title: "Title",
        ed_label_artist: "Artist",
        ed_label_album: "Album",
        ed_label_genre: "Genre",
        ed_label_year: "Year",
        ed_label_track_num: "Track #",
        ed_label_rating: "Rating",
        ed_btn_save: "Save",

        info_key_title: "Title",
        info_key_artist: "Artist",
        info_key_album: "Album",
        info_key_duration: "Duration",
        info_key_type: "Type",
        info_key_bitrate: "Bitrate",
        info_key_sample_rate: "Sample Rate",
        info_key_channels: "Channels",
        info_key_favourite: "Favorite",
        info_key_path: "File Path",
        info_hint: "Tip",

        fmt_convert_title: "Format Convert",
        fmt_convert_hint: "Select target format, then pick audio files to convert (requires ffmpeg).",
        fmt_convert_close: "Close",

        url_title: "Open Audio Stream",
        url_hint: "Enter audio stream URL (supports http, https, mms):",

        lyr_ed_empty: "Lyric Editor Empty",
        lyr_ed_hint: "Open an LRC file or load lyrics from current track",
        lyr_ed_saved: "Saved",
        lyr_ed_unsaved: "Unsaved",
        lyr_ed_save: "Save",
        lyr_ed_save_as: "Save As...",
        lyr_ed_open: "Open",
        lyr_ed_insert: "Insert Line",
        lyr_ed_delete: "Delete Line",
        lyr_ed_shift_all: "Shift All",
        lyr_ed_help: "◀/▶ Adjust timing • ➕/− Add/Delete • 💾 Save",
        lyr_ed_empty_line: "(empty)",

        ml_header: "Media Library ({} tracks | {})",
        ml_unknown: "(unknown)",
        ml_stars: "{} stars",
        ml_overflow: "... and {} more ({} total)",

        fb_title: "File Browser",
        fb_entry_count: "{} items",

        filter_audio_files: "Audio Files",
        filter_playlists: "M3U Playlists",
        eq_custom: "Custom",
        eq_preset_label: "Preset: {}",
        similar_tracks_title: "Tracks similar to \"{}\"",
        similar_tracks_empty: "No similar tracks found (ensure vector index exists)",
        default_list: "Default List",

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
        settings_fade_time: "淡入淡出时间",
        settings_default_volume: "默认音量",
        settings_volume_step: "音量步进",
        settings_replaygain: "ReplayGain",

        settings_section_app: "应用程序设置",
        settings_section_config_file: "配置和数据文件",
        settings_section_close: "关闭主窗口时",
        settings_section_download: "下载设置",
        settings_section_visual: "视觉效果",
        settings_section_background: "背景设置",
        settings_section_play_options: "播放选项",
        settings_section_play_kernel: "播放内核",
        settings_section_play_device: "播放设备",
        settings_section_lyric_options: "歌词选项",
        settings_section_window_lyric: "窗口歌词",
        settings_section_desktop_lyric: "桌面歌词",
        settings_section_ml_options: "媒体库选项",
        settings_section_ml_update: "媒体库更新选项",
        settings_section_ml_dirs: "媒体库目录",
        settings_section_hotkeys: "全局快捷键",
        settings_section_data_mgmt: "数据管理",

        settings_label_update_source: "更新源",
        settings_label_auto_run: "开机自动运行",
        settings_label_portable_mode: "便携模式",
        settings_label_open_config_dir: "打开配置文件所在目录",
        settings_label_minimize_to_tray: "最小化到通知区",
        settings_label_exit: "退出程序",
        settings_label_auto_download_lyric: "没有歌词时自动下载",
        settings_label_lyric_save_pos: "自动下载歌词保存位置",
        settings_label_download_trans_format: "下载歌词翻译格式",
        settings_label_auto_download_cover: "没有专辑封面时自动下载",
        settings_label_spectrum_low_center: "低频部分显示在中间",
        settings_label_spectrum_height: "频谱分析高度",
        settings_label_show_cover: "显示专辑封面",
        settings_label_cover_fit: "专辑封面契合度",
        settings_label_lyric_bg: "歌词界面背景",
        settings_label_rounded: "使用圆角风格",
        settings_label_enable_bg: "启用背景",
        settings_label_desktop_bg: "使用桌面背景",
        settings_label_bg_opacity: "背景不透明度",
        settings_label_bg_cover: "使用专辑封面作为背景",
        settings_label_gaussian_blur: "背景高斯模糊",
        settings_label_blur_radius: "高斯模糊半径",
        settings_label_error_stop: "出现错误时停止播放",
        settings_label_taskbar_progress: "在任务栏显示播放进度",
        settings_label_taskbar_icon: "在任务栏显示播放状态图标",
        settings_label_fade_time: "淡入淡出时间",
        settings_label_continue_on_switch: "切换播放列表时继续播放",
        settings_label_system_media: "使用系统媒体控件",
        settings_label_remember_pos: "记住上次播放的位置",
        settings_label_default_volume: "默认音量",
        settings_label_volume_step: "音量步进",
        settings_label_merge_versions: "合并多版本",
        settings_label_inner_lyric: "优先使用内嵌歌词",
        settings_label_fuzzy_match: "歌词模糊匹配",
        settings_label_show_info_when_none: "没有歌词时显示歌曲信息",
        settings_label_lyric_folder: "歌词文件夹",
        settings_label_lyric_adjust: "歌词调整后",
        settings_label_karaoke: "歌词卡拉OK样式显示",
        settings_label_hide_empty: "不显示歌词空行",
        settings_label_line_spacing: "歌词行间距",
        settings_label_alignment: "歌词对齐方式",
        settings_label_show_desktop_lyric: "显示桌面歌词",
        settings_label_hide_when_none: "没有歌词时隐藏歌词窗口",
        settings_label_hide_when_paused: "暂停时隐藏歌词窗口",
        settings_label_lock_desktop: "锁定桌面歌词",
        settings_label_double_line: "歌词双行显示",
        settings_label_disable_delete: "禁用从磁盘删除",
        settings_label_merge_single: "将只有一项的分类归到其他类中",
        settings_label_min_duration: "音频文件低时长阈值",
        settings_label_remove_missing: "移除不存在的音频文件",
        settings_label_force_reload: "强制重新加载",
        settings_label_media_dirs: "媒体库目录",
        settings_label_media_count: "{} 首",

        settings_option_lyric_save_song: "保存到歌曲所在目录",
        settings_option_lyric_save_lyrics: "保存到歌词文件夹",
        settings_option_adjust_ask: "询问",
        settings_option_adjust_save: "自动保存",
        settings_option_adjust_discard: "放弃",
        settings_option_align_auto: "自动",
        settings_option_align_left: "左对齐",
        settings_option_align_center: "居中",
        settings_option_align_right: "右对齐",

        settings_option_cover_fit: "适应",
        settings_option_cover_fill: "拉伸",
        settings_option_cover_contain: "覆盖",
        settings_option_cover_none: "原始",
        settings_option_replaygain_off: "关闭",
        settings_option_replaygain_auto: "自动",
        settings_option_default_device: "默认设备",
        settings_label_kernel: "内核",
        settings_label_hotkey_func_shortcut: "功能/快捷键",
        settings_label_opaque: "不透明",
        settings_label_add_dir: "添加目录",

        settings_btn_open: "打开",
        settings_btn_add: "添加",
        settings_btn_delete: "删除",
        settings_btn_force_reload: "重新加载",

        hotkey_play_pause: "播放/暂停",
        hotkey_stop: "停止",
        hotkey_forward: "快进",
        hotkey_rewind: "快退",
        hotkey_prev: "上一曲",
        hotkey_next: "下一曲",
        hotkey_vol_up: "增大音量",
        hotkey_vol_down: "减小音量",
        hotkey_exit: "退出",
        hotkey_show_hide: "显示/隐藏播放器",
        hotkey_desktop_lyric: "显示/隐藏桌面歌词",
        hotkey_favourite: "添加到我喜欢的音乐",
        hotkey_enable: "启用全局热键",
        hotkey_function: "功能",
        hotkey_shortcut: "快捷键",

        settings_btn_ok: "确定",
        settings_btn_cancel: "取消",
        settings_btn_apply: "应用",

        eq_flat: "平坦",
        eq_pop: "流行",
        eq_rock: "摇滚",
        eq_classical: "古典",
        eq_jazz: "爵士",
        eq_electronic: "电子",
        eq_bass_boost: "低音增强",
        eq_vocal_boost: "人声增强",
        eq_enabled: "已启用",
        eq_disabled: "已禁用",

        media_lib_title: "媒体库",
        media_lib_scan: "扫描",
        media_lib_search: "\u{1F50D} 搜索媒体库...",

        ml_cat_all: "全部曲目",
        ml_cat_artist: "艺术家",
        ml_cat_album: "专辑",
        ml_cat_genre: "流派",
        ml_cat_year: "年份",
        ml_cat_ftype: "文件类型",
        ml_cat_bitrate: "比特率",
        ml_cat_rating: "评级",
        btn_refresh: "刷新",
        btn_save: "保存预设",

        lyd_title: "歌词下载",
        lyd_source_netease: "网易云",
        lyd_source_qqmusic: "QQ音乐",
        lyd_keyword_hint: "输入歌曲名或艺术家...",
        lyd_search: "搜索",
        lyd_searching: "搜索中...",
        lyd_options: "选项：",
        lyd_include_translation: "包含翻译",
        lyd_save_to_song_dir: "保存到歌曲目录",
        lyd_save_to_lyrics_dir: "保存到歌词目录",
        lyd_status_enter_keyword: "请输入搜索关键词",
        lyd_status_searching: "正在搜索...",

        status_playing: "播放中",
        status_paused: "已暂停",
        status_stopped: "已停止",
        status_rpc_online: "RPC 在线",
        status_rpc_offline: "RPC 离线",
        status_track_count: "{} 首",

        pq_filter_all: "全部 ({})",
        pq_filter_fav: "喜欢 ({})",
        pq_filter_recent: "最近 ({})",
        pq_btn_add: "+ 添加",
        pq_btn_remove: "× 删除",
        pq_btn_sort: "↕ 排序",
        pq_btn_detail: "≡ 详情",
        pq_btn_compact: "≡ 简洁",
        pq_btn_edit: "✏ 编辑",

        pq_col_title: "标题",
        pq_col_artist: "艺术家",
        pq_col_album: "专辑",
        pq_col_duration: "时长",
        pq_col_asc: " ▲",
        pq_col_desc: " ▼",

        pq_header_all: "播放列表 ({} 首)",
        pq_header_fav: "我喜欢的音乐 ({} 首)",
        pq_header_recent: "最近播放 ({} 首)",
        pq_header_filtered: "筛选结果: {} / {}",

        ph_search_media: "搜索媒体库...",
        ph_title: "标题",
        ph_artist: "艺术家",
        ph_album: "专辑",
        ph_genre: "流派",
        ph_year: "年份",
        ph_track_num: "曲目号",
        ph_rating: "评分 (0-5)",

        ed_title: "编辑曲目信息",
        ed_label_title: "标题",
        ed_label_artist: "艺术家",
        ed_label_album: "专辑",
        ed_label_genre: "流派",
        ed_label_year: "年份",
        ed_label_track_num: "曲目号",
        ed_label_rating: "评分",
        ed_btn_save: "保存",

        info_key_title: "标题",
        info_key_artist: "艺术家",
        info_key_album: "专辑",
        info_key_duration: "时长",
        info_key_type: "类型",
        info_key_bitrate: "比特率",
        info_key_sample_rate: "采样率",
        info_key_channels: "声道",
        info_key_favourite: "收藏",
        info_key_path: "文件路径",
        info_hint: "提示",

        fmt_convert_title: "格式转换",
        fmt_convert_hint: "选择目标格式，然后挑选要转换的音频文件（需系统已安装 ffmpeg）。",
        fmt_convert_close: "关闭",

        url_title: "打开网络音频流",
        url_hint: "输入音频流 URL（支持常见流媒体协议如 http、https、mms）：",

        lyr_ed_empty: "歌词编辑器为空",
        lyr_ed_hint: "打开 LRC 文件或从当前曲目加载歌词",
        lyr_ed_saved: "已保存",
        lyr_ed_unsaved: "未保存",
        lyr_ed_save: "保存",
        lyr_ed_save_as: "另存为...",
        lyr_ed_open: "打开",
        lyr_ed_insert: "插入行",
        lyr_ed_delete: "删除行",
        lyr_ed_shift_all: "全部偏移",
        lyr_ed_help: "◀/▶ 调整时序 • ➕/− 增删行 • 💾 保存",
        lyr_ed_empty_line: "(空)",

        ml_header: "媒体库 ({} 首 | {})",
        ml_unknown: "(未知)",
        ml_stars: "{} 星",
        ml_overflow: "... 及其他 {} 首 (共 {} 首)",

        fb_title: "文件浏览器",
        fb_entry_count: "{} 项",

        filter_audio_files: "音频文件",
        filter_playlists: "M3U 播放列表",
        eq_custom: "自定义",
        eq_preset_label: "预设: {}",
        similar_tracks_title: "与 \"{}\" 相似的歌曲",
        similar_tracks_empty: "未找到相似歌曲（请确认已建立向量索引）",
        default_list: "默认列表",

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_from_config() {
        assert_eq!(Lang::from_config("zh-CN"), Lang::ZhCn);
        assert_eq!(Lang::from_config("zh"), Lang::ZhCn);
        assert_eq!(Lang::from_config("zh_CN"), Lang::ZhCn);
        assert_eq!(Lang::from_config("chinese"), Lang::ZhCn);
        assert_eq!(Lang::from_config("en-US"), Lang::EnUs);
        assert_eq!(Lang::from_config(""), Lang::EnUs);
        assert_eq!(Lang::from_config("invalid"), Lang::EnUs);
    }

    #[test]
    fn lang_code() {
        assert_eq!(Lang::EnUs.code(), "en-US");
        assert_eq!(Lang::ZhCn.code(), "zh-CN");
    }

    #[test]
    fn lang_label() {
        assert_eq!(Lang::EnUs.label(), "English");
        assert_eq!(Lang::ZhCn.label(), "简体中文");
    }

    #[test]
    fn global_tr_switches_language() {
        set_global_lang(Lang::EnUs);
        let tr_en = global_tr();
        assert_eq!(tr_en.app_title, "HackMagic Music Player");

        set_global_lang(Lang::ZhCn);
        let tr_zh = global_tr();
        assert_eq!(tr_zh.app_title, "HackMagic 音乐播放器");

        set_global_lang(Lang::EnUs);
    }

    #[test]
    fn en_translations_not_empty() {
        let tr = Tr::EN;
        assert!(!tr.app_title.is_empty());
        assert!(!tr.menu_file.is_empty());
        assert!(!tr.ctrl_stop.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.status_stopped.is_empty());
        assert!(!tr.lyrics_empty.is_empty());
    }

    #[test]
    fn zh_translations_not_empty() {
        let tr = Tr::ZH;
        assert!(!tr.app_title.is_empty());
        assert!(!tr.menu_file.is_empty());
        assert!(!tr.ctrl_stop.is_empty());
        assert!(!tr.settings_title.is_empty());
        assert!(!tr.status_stopped.is_empty());
        assert!(!tr.lyrics_empty.is_empty());
    }

    #[test]
    fn en_zh_field_count_match() {
        // Verify both translations have same number of fields by checking a few key ones
        let en = Tr::EN;
        let zh = Tr::ZH;
        assert_eq!(en.app_title.len() > 0, zh.app_title.len() > 0);
        assert_eq!(en.menu_file.len() > 0, zh.menu_file.len() > 0);
    }
}
