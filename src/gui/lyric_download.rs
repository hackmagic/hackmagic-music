//! Lyric download panel — state and utility functions.
//!
//! The actual GPUI rendering lives in `MusicPlayer::render_lyric_download_panel`
//! in `gui/mod.rs` (which needs `&mut self` access to modify the state via listeners).

use crate::online;

/// Events produced by the background lyric download thread.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    SearchComplete(Result<Vec<online::SearchResult>, String>),
    DownloadComplete {
        song_id: String,
        result: Result<String, String>,
    },
    ProgressUpdate {
        current: usize,
        total: usize,
    },
}

/// State for the lyric download panel.
#[derive(Clone)]
pub struct LyricDownloadState {
    /// Search keyword input
    pub keyword: String,
    /// Search results
    pub results: Vec<online::SearchResult>,
    /// Currently selected result index
    pub selected_index: Option<usize>,
    /// Whether a search is in progress
    pub searching: bool,
    /// Whether a download is in progress
    pub downloading: bool,
    /// Status message for user feedback
    pub status: String,
    /// Whether to include translation
    pub include_translation: bool,
    /// Save to song directory vs lyrics directory
    pub save_to_song_dir: bool,
    /// Search source: "netease" or "qqmusic"
    pub source: String,
    /// Track info for auto-fill
    pub track_title: String,
    pub track_artist: String,
    /// Download progress (0.0-1.0) for batch mode
    pub progress: f32,
    /// Total tracks for batch
    pub total_tracks: usize,
}

impl LyricDownloadState {
    pub fn new() -> Self {
        Self {
            keyword: String::new(),
            results: Vec::new(),
            selected_index: None,
            searching: false,
            downloading: false,
            status: String::new(),
            include_translation: true,
            save_to_song_dir: true,
            source: "netease".to_string(),
            track_title: String::new(),
            track_artist: String::new(),
            progress: 0.0,
            total_tracks: 0,
        }
    }

    /// Auto-fill search fields from current track info.
    pub fn auto_fill(&mut self, title: &str, artist: &str) {
        self.track_title = title.to_string();
        self.track_artist = artist.to_string();
        if !title.is_empty() && !artist.is_empty() {
            self.keyword = format!("{} {}", title, artist);
        } else if !title.is_empty() {
            self.keyword = title.to_string();
        } else if !artist.is_empty() {
            self.keyword = artist.to_string();
        }
    }
}

impl Default for LyricDownloadState {
    fn default() -> Self {
        Self::new()
    }
}

/// Search lyrics online using the given source.
pub async fn search_lyrics(
    source: &str,
    keyword: &str,
) -> Result<Vec<online::SearchResult>, String> {
    if keyword.trim().is_empty() {
        return Err("请输入搜索关键词".to_string());
    }

    match source {
        "qqmusic" => {
            online::qqmusic_search(keyword)
                .await
                .map_err(|e| format!("QQ音乐搜索失败: {}", e))
        }
        _ => {
            online::netease_search(keyword)
                .await
                .map_err(|e| format!("网易云搜索失败: {}", e))
        }
    }
}

/// Download lyrics from the given source by ID.
pub async fn download_lyric(
    source: &str,
    song_id: &str,
    include_translation: bool,
) -> Result<String, String> {
    let lrc_text = match source {
        "qqmusic" => {
            online::qqmusic_download_lyric(song_id)
                .await
                .map_err(|e| format!("QQ音乐下载失败: {}", e))?
        }
        _ => {
            online::netease_download_lyric(song_id)
                .await
                .map_err(|e| format!("网易云下载失败: {}", e))?
        }
    };

    if !include_translation {
        // Remove translation lines if present
        let filtered: Vec<&str> = lrc_text
            .lines()
            .filter(|line| !line.starts_with("[t:"))
            .collect();
        Ok(filtered.join("\n"))
    } else {
        Ok(lrc_text)
    }
}
