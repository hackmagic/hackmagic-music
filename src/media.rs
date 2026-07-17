//! Media library scanner and cache.
//! Scans directories for audio files and stores metadata in a JSON cache.

use crate::error::{PlayerError, Result};
use crate::tag;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Audio file extensions supported for scanning
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "wav", "ogg", "opus", "m4a", "aac", "wma",
    "ape", "dsf", "dff", "mpc", "tta", "wv", "spx", "aiff", "aif",
];

/// A cached media library entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibEntry {
    pub file_path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub track_number: u32,
    pub year: u32,
    pub duration_secs: u64,
    pub bitrate: u32,
    pub is_favourite: bool,
    pub play_count: u64,
    pub last_played: String,  // ISO 8601
    #[serde(default)]
    pub song_id_netease: i64,
    #[serde(default)]
    pub song_id_qq_music: String,
}

/// The media library database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MediaLib {
    pub entries: Vec<LibEntry>,
    pub last_scan: String,  // ISO 8601
}

impl MediaLib {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            last_scan: String::new(),
        }
    }

    /// Load from JSON cache
    pub fn load() -> Self {
        let path = get_db_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            MediaLib::new()
        }
    }

    /// Save to JSON cache
    pub fn save(&self) -> Result<()> {
        let path = get_db_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| PlayerError::Other(format!("Serialization error: {e}")))?;
        std::fs::write(path, content)
            .map_err(|e| PlayerError::Other(format!("Cannot write media lib: {e}")))?;
        Ok(())
    }

    /// Check if a path is already indexed
    #[cfg(test)]
    pub fn is_indexed(&self, file_path: &str) -> bool {
        self.entries.iter().any(|e| e.file_path == file_path)
    }

    /// Add or update an entry
    pub fn upsert(&mut self, entry: LibEntry) {
        if let Some(pos) = self.entries.iter().position(|e| e.file_path == entry.file_path) {
            self.entries[pos] = entry;
        } else {
            self.entries.push(entry);
        }
    }

    /// Search by keyword
    pub fn search(&self, keyword: &str) -> Vec<&LibEntry> {
        let kw = keyword.to_lowercase();
        self.entries.iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&kw)
                    || e.artist.to_lowercase().contains(&kw)
                    || e.album.to_lowercase().contains(&kw)
                    || e.genre.to_lowercase().contains(&kw)
                    || e.file_path.to_lowercase().contains(&kw)
            })
            .collect()
    }

    /// Get entries for an artist
    pub fn by_artist(&self, name: Option<&str>) -> Vec<&LibEntry> {
        match name {
            Some(n) => self.entries.iter().filter(|e| e.artist.to_lowercase() == n.to_lowercase()).collect(),
            None => {
                // Group by artist
                self.entries.iter().collect()
            }
        }
    }

    /// Get entries for an album
    pub fn by_album(&self, name: Option<&str>) -> Vec<&LibEntry> {
        match name {
            Some(n) => self.entries.iter().filter(|e| e.album.to_lowercase() == n.to_lowercase()).collect(),
            None => self.entries.iter().collect(),
        }
    }

    /// Get unique artists
    pub fn artists(&self) -> Vec<String> {
        let mut set: Vec<String> = self.entries.iter().map(|e| e.artist.clone()).collect();
        set.sort();
        set.dedup();
        set.into_iter().filter(|s| !s.is_empty()).collect()
    }

    /// Get unique albums
    pub fn albums(&self) -> Vec<String> {
        let mut set: Vec<String> = self.entries.iter().map(|e| e.album.clone()).collect();
        set.sort();
        set.dedup();
        set.into_iter().filter(|s| !s.is_empty()).collect()
    }

    /// Get unique genres
    pub fn genres(&self) -> Vec<String> {
        let mut set: Vec<String> = self.entries.iter().map(|e| e.genre.clone()).collect();
        set.sort();
        set.dedup();
        set.into_iter().filter(|s| !s.is_empty()).collect()
    }

    /// Get unique years (sorted descending)
    pub fn years(&self) -> Vec<u32> {
        let mut set: Vec<u32> = self.entries.iter().map(|e| e.year).filter(|y| *y > 0).collect();
        set.sort_by(|a, b| b.cmp(a));
        set.dedup();
        set
    }

    /// Get unique file types (extensions)
    pub fn file_types(&self) -> Vec<String> {
        let mut set: Vec<String> = self.entries.iter()
            .filter_map(|e| std::path::Path::new(&e.file_path).extension())
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .collect();
        set.sort();
        set.dedup();
        set
    }

    /// Get unique bitrates (sorted)
    pub fn bitrates(&self) -> Vec<u32> {
        let mut set: Vec<u32> = self.entries.iter().map(|e| e.bitrate).filter(|b| *b > 0).collect();
        set.sort_unstable();
        set.dedup();
        set
    }

    /// Get entries by genre
    pub fn by_genre(&self, genre: &str) -> Vec<&LibEntry> {
        self.entries.iter().filter(|e| e.genre.to_lowercase() == genre.to_lowercase()).collect()
    }

    /// Get entries by year
    pub fn by_year(&self, year: u32) -> Vec<&LibEntry> {
        self.entries.iter().filter(|e| e.year == year).collect()
    }

    /// Get entries by file type (extension)
    pub fn by_file_type(&self, ext: &str) -> Vec<&LibEntry> {
        self.entries.iter().filter(|e| {
            std::path::Path::new(&e.file_path).extension()
                .is_some_and(|x| x.to_string_lossy().to_lowercase() == ext.to_lowercase())
        }).collect()
    }

    /// Get entries by bitrate
    pub fn by_bitrate(&self, bitrate: u32) -> Vec<&LibEntry> {
        self.entries.iter().filter(|e| e.bitrate == bitrate).collect()
    }

    /// Get recently played entries (sorted by `last_played` desc)
    pub fn recent(&self, limit: usize) -> Vec<&LibEntry> {
        let mut recent: Vec<&LibEntry> = self.entries.iter()
            .filter(|e| !e.last_played.is_empty())
            .collect();
        recent.sort_by(|a, b| b.last_played.cmp(&a.last_played));
        recent.truncate(limit);
        recent
    }

    /// Get favourite entries
    pub fn favourites(&self) -> Vec<&LibEntry> {
        self.entries.iter().filter(|e| e.is_favourite).collect()
    }

    /// Statistics
    pub fn stats(&self) -> HashMap<String, usize> {
        let mut map = HashMap::new();
        map.insert("total_tracks".to_string(), self.entries.len());
        map.insert("total_artists".to_string(), self.artists().len());
        map.insert("total_albums".to_string(), self.albums().len());
        map.insert("total_genres".to_string(), self.genres().len());
        let total_duration: u64 = self.entries.iter().map(|e| e.duration_secs).sum();
        map.insert("total_duration_secs".to_string(), total_duration as usize);
        map
    }
}

/// Scan a directory for audio files.
/// `on_progress` is called with (`file_path`, `files_found_so_far`) for each audio file encountered.
/// Callback type for scan progress: (`file_path`, `files_found_so_far`).
type ProgressCb<'a> = &'a dyn Fn(&str, usize);

pub fn scan_directory(path: &str, recursive: bool, on_progress: Option<ProgressCb<'_>>) -> Result<Vec<LibEntry>> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(PlayerError::FileNotFound(path.to_string()));
    }

    let mut entries = Vec::new();
    scan_dir_recursive(p, recursive, &mut entries, on_progress)?;
    Ok(entries)
}

fn scan_dir_recursive(dir: &Path, recursive: bool, entries: &mut Vec<LibEntry>, on_progress: Option<ProgressCb<'_>>) -> Result<()> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| PlayerError::Other(format!("Cannot read dir {dir:?}: {e}")))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| PlayerError::Other(e.to_string()))?;
        let path = entry.path();

        if path.is_dir() {
            if recursive {
                scan_dir_recursive(&path, true, entries, on_progress)?;
            }
            continue;
        }

        // Check extension
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .unwrap_or_default();

        if !AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            continue;
        }

        // Read tags
        let path_str = path.to_str().unwrap_or_default().to_string();

        // Report progress before reading tags (show which file is being processed)
        if let Some(cb) = on_progress {
            cb(&path_str, entries.len());
        }

        // 读取标签失败时也要把文件加入列表（用文件名作为标题），避免只加载部分文件
        let track = tag::reader::read_tags(&path_str).unwrap_or_default();
        let min_dur = crate::config::Config::load().media_lib.min_duration_secs;
        if min_dur > 0 && track.duration.as_secs() > 0 && track.duration.as_secs() < min_dur {
            continue;
        }
        // 标题为空时回退到文件名（不含扩展名）
        let title = if track.title.is_empty() {
            path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()
        } else {
            track.title
        };
        entries.push(LibEntry {
            file_path: path_str,
            title,
            artist: track.artist,
            album: track.album,
            genre: track.genre,
            track_number: track.track_number,
            year: track.year,
            duration_secs: track.duration.as_secs(),
            bitrate: track.bitrate,
            is_favourite: false,
            play_count: 0,
            last_played: String::new(),
            song_id_netease: track.song_id_netease,
            song_id_qq_music: track.song_id_qq_music,
        });
    }

    Ok(())
}

/// Entry for directory browsing
#[derive(Debug)]
pub struct DirEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub duration_secs: u64,
    pub is_audio: bool,
}

/// Browse a directory for audio files and subdirectories
pub fn browse_directory(path: &str, recursive: bool) -> Result<Vec<DirEntry>> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(PlayerError::FileNotFound(path.to_string()));
    }
    if !p.is_dir() {
        return Err(PlayerError::Other(format!("Not a directory: {path}")));
    }

    let mut entries = Vec::new();
    browse_dir_recursive(p, recursive, &mut entries, 0)?;
    Ok(entries)
}

fn browse_dir_recursive(
    dir: &Path,
    recursive: bool,
    entries: &mut Vec<DirEntry>,
    #[allow(clippy::only_used_in_recursion)] _depth: usize,
) -> Result<()> {
    let read_dir = std::fs::read_dir(dir)
        .map_err(|e| PlayerError::Other(format!("Cannot read dir {dir:?}: {e}")))?;

    let mut sub_entries: Vec<std::fs::DirEntry> = Vec::new();
    for entry in read_dir {
        let entry = entry.map_err(|e| PlayerError::Other(e.to_string()))?;
        sub_entries.push(entry);
    }
    // Sort: directories first, then files, alphabetically
    sub_entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        if a_is_dir == b_is_dir {
            a.file_name().cmp(&b.file_name())
        } else {
            b_is_dir.cmp(&a_is_dir)
        }
    });

    for entry in sub_entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let path_str = path.to_str().unwrap_or_default().to_string();
        let is_dir = path.is_dir();

        if is_dir {
            let size = std::fs::metadata(&path)
                .map(|m| m.len())
                .unwrap_or(0);
            entries.push(DirEntry {
                name: name + "/",
                path: path_str.clone(),
                is_dir: true,
                size,
                duration_secs: 0,
                is_audio: false,
            });
            if recursive {
                browse_dir_recursive(&path, true, entries, _depth + 1)?;
            }
        } else {
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .map(str::to_lowercase)
                .unwrap_or_default();
            let is_audio = crate::audio_common::supported_extensions().contains(&ext.as_str());
            let metadata = std::fs::metadata(&path).ok();
            let size = metadata.map_or(0, |m| m.len());
            let duration_secs = if is_audio {
                crate::tag::reader::read_tags(&path_str)
                    .map(|t| t.duration.as_secs())
                    .unwrap_or(0)
            } else {
                0
            };
            entries.push(DirEntry {
                name,
                path: path_str,
                is_dir: false,
                size,
                duration_secs,
                is_audio,
            });
        }
    }

    Ok(())
}

fn get_db_path() -> std::path::PathBuf {
    let mut path = crate::config::get_config_dir();
    path.push("media_lib.json");
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(title: &str, artist: &str, album: &str, genre: &str, file_path: &str, year: u32) -> LibEntry {
        LibEntry {
            file_path: file_path.to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            genre: genre.to_string(),
            track_number: 0,
            year,
            duration_secs: 200,
            bitrate: 320,
            is_favourite: false,
            play_count: 0,
            last_played: String::new(),
            song_id_netease: 0,
            song_id_qq_music: String::new(),
        }
    }

    fn test_lib() -> MediaLib {
        MediaLib {
            entries: vec![
                make_entry("Bohemian Rhapsody", "Queen", "A Night at the Opera", "Rock", "C:\\music\\queen.mp3", 1975),
                make_entry("Imagine", "John Lennon", "Imagine", "Soft Rock", "C:\\music\\imagine.flac", 1971),
                make_entry("Hotel California", "Eagles", "Hotel California", "Rock", "C:\\music\\hotel.ogg", 1976),
                make_entry("Yesterday", "The Beatles", "Help!", "Pop", "C:\\music\\yesterday.mp3", 1965),
            ],
            last_scan: String::new(),
        }
    }

    // ── search() tests ──────────────────────────────────────────────

    #[test]
    fn test_search_by_title() {
        let lib = test_lib();
        let results = lib.search("bohemian");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Bohemian Rhapsody");
    }

    #[test]
    fn test_search_by_artist() {
        let lib = test_lib();
        let results = lib.search("queen");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].artist, "Queen");
    }

    #[test]
    fn test_search_by_album() {
        let lib = test_lib();
        let results = lib.search("hotel california");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].album, "Hotel California");
    }

    #[test]
    fn test_search_by_genre() {
        let lib = test_lib();
        // "rock" matches genre "Rock" (2 entries) + genre "Soft Rock" (1 entry) = 3
        let results = lib.search("rock");
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_search_by_file_path() {
        let lib = test_lib();
        let results = lib.search("imagine.flac");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Imagine");
    }

    #[test]
    fn test_search_case_insensitive() {
        let lib = test_lib();
        let upper = lib.search("QUEEN");
        let lower = lib.search("queen");
        let mixed = lib.search("QuEeN");
        assert_eq!(upper.len(), 1);
        assert_eq!(lower.len(), 1);
        assert_eq!(mixed.len(), 1);
    }

    #[test]
    fn test_search_no_match() {
        let lib = test_lib();
        let results = lib.search("zzzznotexist");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_empty_keyword() {
        let lib = test_lib();
        // Empty string matches everything because `contains("")` is always true
        let results = lib.search("");
        assert_eq!(results.len(), lib.entries.len());
    }

    #[test]
    fn test_search_partial_match() {
        let lib = test_lib();
        let results = lib.search("yes");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Yesterday");
    }

    #[test]
    fn test_search_empty_library() {
        let lib = MediaLib::new();
        let results = lib.search("anything");
        assert!(results.is_empty());
    }

    // ── favourites() tests ──────────────────────────────────────────

    #[test]
    fn test_favourites_none() {
        let lib = test_lib();
        let favs = lib.favourites();
        assert!(favs.is_empty());
    }

    #[test]
    fn test_favourites_some() {
        let mut lib = test_lib();
        // Mark the first and third entries as favourites
        lib.entries[0].is_favourite = true;
        lib.entries[2].is_favourite = true;
        let favs = lib.favourites();
        assert_eq!(favs.len(), 2);
        assert!(favs.iter().any(|e| e.title == "Bohemian Rhapsody"));
        assert!(favs.iter().any(|e| e.title == "Hotel California"));
    }

    #[test]
    fn test_favourites_all() {
        let mut lib = test_lib();
        for entry in lib.entries.iter_mut() {
            entry.is_favourite = true;
        }
        let favs = lib.favourites();
        assert_eq!(favs.len(), lib.entries.len());
    }

    #[test]
    fn test_favourites_empty_library() {
        let lib = MediaLib::new();
        let favs = lib.favourites();
        assert!(favs.is_empty());
    }

    #[test]
    fn test_favourites_after_upsert() {
        let mut lib = MediaLib::new();
        // Add an entry via upsert (default: not favourite)
        let entry = make_entry("Test Song", "Test Artist", "Test Album", "Pop", "C:\\test.mp3", 2024);
        lib.upsert(entry);
        assert!(lib.favourites().is_empty());

        // Mark as favourite by upserting a modified copy
        let mut entry2 = make_entry("Test Song", "Test Artist", "Test Album", "Pop", "C:\\test.mp3", 2024);
        entry2.is_favourite = true;
        lib.upsert(entry2);
        assert_eq!(lib.favourites().len(), 1);
    }
}
