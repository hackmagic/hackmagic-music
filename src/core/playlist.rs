//! Playlist management
//! Mirrors the original `CPlayer` playlist functionality
#![allow(dead_code)]

use crate::error::{PlayerError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use rand::seq::SliceRandom;
use rand::thread_rng;

/// Maximum number of tracks allowed in a playlist
pub const MAX_TRACK_COUNT: usize = 99999;

/// Playlist mode, matching original `CPlayer::PlaylistMode`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistMode {
    Folder,
    Playlist,
    MediaLib,
}

/// Repeat mode, matching original `RepeatMode`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    PlayOrder,    // 顺序播放
    PlayShuffle,  // 无序播放 (once through shuffled)
    PlayRandom,   // 随机播放 (true random with history)
    LoopPlaylist, // 列表循环
    LoopTrack,    // 单曲循环
    PlayTrack,    // 单曲播放
}

impl RepeatMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "order" | "顺序" => RepeatMode::PlayOrder,
            "shuffle" | "无序" | "打乱" => RepeatMode::PlayShuffle,
            "random" | "随机" => RepeatMode::PlayRandom,
            "loop" | "列表循环" | "list" => RepeatMode::LoopPlaylist,
            "track" | "单曲循环" | "once" => RepeatMode::LoopTrack,
            "play_track" | "单曲播放" => RepeatMode::PlayTrack,
            _ => RepeatMode::LoopPlaylist,
        }
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn to_str(&self) -> &'static str {
        match self {
            RepeatMode::PlayOrder => "order",
            RepeatMode::PlayShuffle => "shuffle",
            RepeatMode::PlayRandom => "random",
            RepeatMode::LoopPlaylist => "loop",
            RepeatMode::LoopTrack => "track",
            RepeatMode::PlayTrack => "play_track",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            RepeatMode::PlayOrder => "顺序播放",
            RepeatMode::PlayShuffle => "无序播放",
            RepeatMode::PlayRandom => "随机播放",
            RepeatMode::LoopPlaylist => "列表循环",
            RepeatMode::LoopTrack => "单曲循环",
            RepeatMode::PlayTrack => "单曲播放",
        }
    }
}

/// Sort mode for playlist
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Default,
    Title,
    Artist,
    Album,
    Track,
    FileName,
    Path,
    Time,
    Bitrate,
    Genre,
    Year,
    ListenCount,
    Rating,
    Random,
}

impl SortMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "title" | "标题" => SortMode::Title,
            "artist" | "艺术家" => SortMode::Artist,
            "album" | "专辑" => SortMode::Album,
            "track" | "音轨" => SortMode::Track,
            "file" | "filename" | "文件名" => SortMode::FileName,
            "path" | "路径" => SortMode::Path,
            "time" | "时长" => SortMode::Time,
            "bitrate" | "比特率" => SortMode::Bitrate,
            "genre" | "流派" => SortMode::Genre,
            "year" | "年份" => SortMode::Year,
            "listen" | "listen_count" | "播放次数" => SortMode::ListenCount,
            "rating" | "评分" => SortMode::Rating,
            "random" | "随机" => SortMode::Random,
            _ => SortMode::Default,
        }
    }
}

/// A single track in the playlist
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Track {
    pub file_path: String,
    pub file_name: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub track_number: u32,
    pub year: u32,
    pub duration: Duration,
    pub bitrate: u32,
    pub file_type: String,
    pub is_cue: bool,
    pub cue_file_path: String,
    pub cue_track_number: u32,
    pub start_pos: Duration,
    pub end_pos: Duration,
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub channels: u8,
    pub is_favourite: bool,
    pub rating: u32,
    pub listen_count: u32,
    pub lyric_file: String,
    pub lyric_offset: i64,
    #[serde(default)]
    pub song_id_netease: i64,
    #[serde(default)]
    pub song_id_qq_music: String,
    #[serde(default)]
    pub flags: u32,
    // ReplayGain (parsed dB value, e.g. -2.34)
    pub track_gain: f32,
    pub track_peak: f32,
    pub album_gain: f32,
    pub album_peak: f32,
}

impl Track {
    pub fn new(path: &str) -> Self {
        let p = Path::new(path);
        let file_name = p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Self {
            file_path: path.to_string(),
            file_name,
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            genre: String::new(),
            track_number: 0,
            year: 0,
            duration: Duration::ZERO,
            bitrate: 0,
            file_type: p.extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default(),
            sample_rate: 0,
            bit_depth: 0,
            channels: 0,
            is_cue: false,
            cue_file_path: String::new(),
            cue_track_number: 0,
            start_pos: Duration::ZERO,
            end_pos: Duration::ZERO,
            is_favourite: false,
            rating: 0,
            listen_count: 0,
            lyric_file: String::new(),
            lyric_offset: 0,
            song_id_netease: 0,
            song_id_qq_music: String::new(),
            flags: 0,
            track_gain: 0.0, track_peak: 0.0, album_gain: 0.0, album_peak: 0.0,
        }
    }

    /// Display name based on format
    pub fn display_name(&self, format: &str) -> String {
        match format {
            "title" => {
                if self.title.is_empty() { self.file_name.clone() }
                else { self.title.clone() }
            }
            "artist_title" => {
                if self.artist.is_empty() { self.display_name("title") }
                else { format!("{} - {}", self.artist, self.display_name("title")) }
            }
            "title_artist" => {
                if self.artist.is_empty() { self.display_name("title") }
                else { format!("{} - {}", self.display_name("title"), self.artist) }
            }
            _ => self.file_name.clone(), // file_name
        }
    }

    pub fn duration_str(&self) -> String {
        let secs = self.duration.as_secs();
        format!("{:02}:{:02}", secs / 60, secs % 60)
    }
}

/// The playlist
#[derive(Debug, Clone)]
pub struct Playlist {
    name: String,
    tracks: Vec<Track>,
    current_index: Option<usize>,
    shuffle_list: Vec<usize>,
    shuffle_index: usize,
    random_history: Vec<usize>,
    next_tracks: Vec<usize>, // "play after current" queue
    repeat_mode: RepeatMode,
    sort_mode: SortMode,
    mode: PlaylistMode,
    path: String, // folder path or .playlist file path
}

impl Playlist {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tracks: Vec::new(),
            current_index: None,
            shuffle_list: Vec::new(),
            shuffle_index: 0,
            random_history: Vec::new(),
            next_tracks: Vec::new(),
            repeat_mode: RepeatMode::LoopPlaylist,
            sort_mode: SortMode::Default,
            mode: PlaylistMode::Playlist,
            path: String::new(),
        }
    }

    // === Accessors ===

    pub fn name(&self) -> &str { &self.name }
    pub fn set_name(&mut self, name: String) { self.name = name; }
    pub fn path(&self) -> &str { &self.path }
    pub fn set_path(&mut self, path: String) { self.path = path; }
    pub fn mode(&self) -> PlaylistMode { self.mode }
    pub fn set_mode(&mut self, mode: PlaylistMode) { self.mode = mode; }
    pub fn repeat_mode(&self) -> RepeatMode { self.repeat_mode }
    pub fn set_repeat_mode(&mut self, mode: RepeatMode) { self.repeat_mode = mode; }
    pub fn sort_mode(&self) -> SortMode { self.sort_mode }
    pub fn len(&self) -> usize { self.tracks.len() }
    pub fn is_empty(&self) -> bool { self.tracks.is_empty() }
    pub fn current_index(&self) -> Option<usize> { self.current_index }
    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|i| self.tracks.get(i))
    }
    pub fn tracks(&self) -> &[Track] { &self.tracks }
    pub fn get(&self, index: usize) -> Option<&Track> { self.tracks.get(index) }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Track> { self.tracks.get_mut(index) }

    // === Track management ===

    pub fn add_track(&mut self, track: Track) {
        if self.tracks.len() >= MAX_TRACK_COUNT {
            tracing::warn!("Playlist at max capacity ({})", MAX_TRACK_COUNT);
            return;
        }
        self.tracks.push(track);
    }

    pub fn add_tracks(&mut self, tracks: Vec<Track>) {
        let remaining = MAX_TRACK_COUNT.saturating_sub(self.tracks.len());
        if remaining == 0 {
            tracing::warn!("Playlist at max capacity ({})", MAX_TRACK_COUNT);
            return;
        }
        let total = tracks.len();
        let to_add: Vec<Track> = tracks.into_iter().take(remaining).collect();
        if to_add.len() < total {
            tracing::warn!("Added {} tracks, truncated {} (max {})",
                to_add.len(), total - to_add.len(), MAX_TRACK_COUNT);
        }
        self.tracks.extend(to_add);
    }

    pub fn remove(&mut self, index: usize) -> Option<Track> {
        if index >= self.tracks.len() {
            return None;
        }
        // Adjust current_index
        if let Some(cur) = self.current_index {
            if index < cur {
                self.current_index = Some(cur - 1);
            } else if index == cur {
                if self.tracks.len() == 1 {
                    self.current_index = None;
                } else if index >= self.tracks.len() - 1 {
                    self.current_index = Some(0);
                }
                // else: current_index stays same (next track shifts into place)
            }
        }
        Some(self.tracks.remove(index))
    }

    pub fn remove_multiple(&mut self, mut indices: Vec<usize>) {
        indices.sort_by(|a, b| b.cmp(a)); // Remove from end
        indices.dedup();
        for i in indices {
            self.remove(i);
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.current_index = None;
        self.shuffle_list.clear();
        self.random_history.clear();
        self.next_tracks.clear();
    }

    pub fn set_current(&mut self, index: usize) -> Result<()> {
        if index >= self.tracks.len() {
            return Err(PlayerError::InvalidIndex(index));
        }
        self.current_index = Some(index);
        // Add to random history
        self.random_history.push(index);
        Ok(())
    }

    // === Navigation ===

    /// Get the next track index based on repeat mode
    pub fn next_index(&self) -> Option<usize> {
        let cur = self.current_index?;
        
        // Check "play next" queue first
        // (handled by caller via drain_next_track)

        if self.tracks.is_empty() {
            return None;
        }

        match self.repeat_mode {
            RepeatMode::PlayTrack => None, // Play single track then stop
            RepeatMode::LoopTrack => Some(cur), // Repeat current
            RepeatMode::PlayShuffle => {
                if self.shuffle_list.is_empty() {
                    if self.tracks.len() == 1 { return Some(0); }
                    return None;
                }
                let next = self.shuffle_index + 1;
                if next >= self.shuffle_list.len() {
                    None // End of shuffle list
                } else {
                    Some(self.shuffle_list[next])
                }
            }
            RepeatMode::PlayOrder => {
                let next = cur + 1;
                if next >= self.tracks.len() {
                    None // Stop at end
                } else {
                    Some(next)
                }
            }
            RepeatMode::LoopPlaylist => {
                let next = cur + 1;
                if next >= self.tracks.len() {
                    Some(0) // Wrap around
                } else {
                    Some(next)
                }
            }
            RepeatMode::PlayRandom => {
                // Pick a random track different from current
                if self.tracks.len() <= 1 {
                    return Some(0);
                }
                loop {
                    let idx = rand::random::<usize>() % self.tracks.len();
                    if idx != cur {
                        return Some(idx);
                    }
                }
            }
        }
    }

    /// Get the previous track index
    pub fn prev_index(&self) -> Option<usize> {
        let cur = self.current_index?;
        if self.tracks.is_empty() {
            return None;
        }

        match self.repeat_mode {
            RepeatMode::PlayRandom | RepeatMode::PlayShuffle => {
                // Pop from history
                if self.random_history.len() >= 2 {
                    let idx = self.random_history[self.random_history.len() - 2];
                    Some(idx)
                } else {
                    Some(cur) // Stay on same track
                }
            }
            _ => {
                if cur == 0 {
                    Some(self.tracks.len() - 1) // Wrap to end
                } else {
                    Some(cur - 1)
                }
            }
        }
    }

    /// Get the index to play from a "play after current" request
    pub fn drain_next_track(&mut self) -> Option<usize> {
        if self.next_tracks.is_empty() {
            None
        } else {
            Some(self.next_tracks.remove(0))
        }
    }

    pub fn push_next_track(&mut self, index: usize) {
        if index < self.tracks.len() {
            self.next_tracks.push(index);
        }
    }

    /// Returns indices in the play-queue
    pub fn queued_indices(&self) -> Vec<usize> {
        self.next_tracks.clone()
    }

    /// Add CUE-expanded tracks from a `CueSheet`
    pub fn add_cue_tracks(&mut self, sheet: &crate::cuesheet::CueSheet) {
        for (i, cue_track) in sheet.tracks.iter().enumerate() {
            let mut track = Track::new(&cue_track.file_path);
            track.is_cue = true;
            track.cue_file_path = sheet.cue_path.clone();
            track.cue_track_number = cue_track.track;
            track.start_pos = cue_track.start_pos;
            // Calculate per-track duration: end_pos of this track, or start_pos of next track
            let next_start = sheet.tracks.get(i + 1).map(|t| t.start_pos);
            track.end_pos = if cue_track.end_pos != Duration::ZERO {
                cue_track.end_pos
            } else if let Some(next) = next_start {
                next
            } else {
                cue_track.end_pos
            };
            // Override duration to per-track length
            if track.end_pos > track.start_pos {
                track.duration = track.end_pos.checked_sub(track.start_pos).unwrap();
            }
            track.title = cue_track.title.clone();
            track.artist = if cue_track.artist.is_empty() {
                sheet.album_artist.clone()
            } else {
                cue_track.artist.clone()
            };
            track.album = sheet.album.clone();
            self.tracks.push(track);
        }
    }

    // === Shuffle ===

    pub fn init_shuffle(&mut self, first_song: Option<usize>) {
        let mut indices: Vec<usize> = (0..self.tracks.len()).collect();
        indices.shuffle(&mut thread_rng());
        
        // Move first_song to front if specified
        if let Some(first) = first_song {
            if let Some(pos) = indices.iter().position(|&i| i == first) {
                indices.swap(0, pos);
            }
        }
        
        self.shuffle_list = indices;
        self.shuffle_index = 0;
    }

    // === Sorting ===

    pub fn sort(&mut self, mode: SortMode, desc: bool) {
        match mode {
            SortMode::Default => {}
            SortMode::Title => self.tracks.sort_by(|a, b| a.title.cmp(&b.title)),
            SortMode::Artist => self.tracks.sort_by(|a, b| a.artist.cmp(&b.artist)),
            SortMode::Album => self.tracks.sort_by(|a, b| a.album.cmp(&b.album)),
            SortMode::Track => self.tracks.sort_by(|a, b| a.track_number.cmp(&b.track_number)),
            SortMode::FileName => self.tracks.sort_by(|a, b| a.file_name.cmp(&b.file_name)),
            SortMode::Path => self.tracks.sort_by(|a, b| a.file_path.cmp(&b.file_path)),
            SortMode::Time => self.tracks.sort_by(|a, b| a.duration.cmp(&b.duration)),
            SortMode::Bitrate => self.tracks.sort_by(|a, b| a.bitrate.cmp(&b.bitrate)),
            SortMode::Genre => self.tracks.sort_by(|a, b| a.genre.cmp(&b.genre)),
            SortMode::Year => self.tracks.sort_by(|a, b| a.year.cmp(&b.year)),
            SortMode::ListenCount => self.tracks.sort_by(|a, b| a.listen_count.cmp(&b.listen_count)),
            SortMode::Rating => self.tracks.sort_by(|a, b| a.rating.cmp(&b.rating)),
            SortMode::Random => {
                self.tracks.shuffle(&mut thread_rng());
            }
        }
        if desc && mode != SortMode::Default && mode != SortMode::Random {
            self.tracks.reverse();
        }
        self.sort_mode = mode;
        self.init_shuffle(self.current_index);
    }

    /// Reverse playlist order
    pub fn reverse(&mut self) {
        self.tracks.reverse();
    }

    /// Search tracks by keyword
    pub fn search(&self, keyword: &str) -> Vec<(usize, &Track)> {
        let kw = keyword.to_lowercase();
        self.tracks.iter().enumerate().filter(|(_, t)| {
            t.title.to_lowercase().contains(&kw)
                || t.artist.to_lowercase().contains(&kw)
                || t.album.to_lowercase().contains(&kw)
                || t.file_name.to_lowercase().contains(&kw)
                || t.genre.to_lowercase().contains(&kw)
        }).collect()
    }

    /// Move track from one position to another
    pub fn move_track(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return Err(PlayerError::InvalidIndex(from.max(to)));
        }
        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);
        Ok(())
    }

    /// Toggle favourite for a track
    pub fn toggle_favourite(&mut self, index: usize) -> bool {
        if let Some(track) = self.tracks.get_mut(index) {
            track.is_favourite = !track.is_favourite;
            return track.is_favourite;
        }
        false
    }

    /// Set rating for a track (0-5)
    pub fn set_rating(&mut self, index: usize, rating: u32) -> bool {
        if let Some(track) = self.tracks.get_mut(index) {
            track.rating = rating.min(5);
            return true;
        }
        false
    }

    /// Remove duplicate tracks (by `file_path`)
    pub fn dedup(&mut self) -> usize {
        let before = self.tracks.len();
        let mut seen = std::collections::HashSet::new();
        let mut dup_indices = Vec::new();
        for (i, t) in self.tracks.iter().enumerate() {
            if !seen.insert(t.file_path.clone()) {
                dup_indices.push(i);
            }
        }
        for &i in dup_indices.iter().rev() {
            self.remove(i);
        }
        before - self.tracks.len()
    }

    /// Remove tracks whose file no longer exists on disk
    pub fn clean(&mut self) -> usize {
        let before = self.tracks.len();
        let mut gone = Vec::new();
        for (i, t) in self.tracks.iter().enumerate() {
            if !std::path::Path::new(&t.file_path).exists() {
                gone.push(i);
            }
        }
        for &i in gone.iter().rev() {
            self.remove(i);
        }
        before - self.tracks.len()
    }

    /// Total duration of all tracks
    pub fn total_duration(&self) -> Duration {
        self.tracks.iter().fold(Duration::ZERO, |acc, t| acc + t.duration)
    }

    /// Total duration string
    pub fn total_duration_str(&self) -> String {
        let secs = self.total_duration().as_secs();
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        let s = secs % 60;
        if h > 0 {
            format!("{h:02}:{m:02}:{s:02}")
        } else {
            format!("{m:02}:{s:02}")
        }
    }

    // === M3U Import/Export ===

    /// Export playlist to M3U/M3U8 file
    pub fn export_m3u(&self, path: &str, _utf8: bool) -> Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)
            .map_err(PlayerError::Io)?;

        // M3U header
        writeln!(file, "#EXTM3U").map_err(PlayerError::Io)?;

        for track in &self.tracks {
            let duration_secs = track.duration.as_secs() as i64;
            let title = if track.title.is_empty() {
                &track.file_name
            } else {
                &track.title
            };
            let artist = &track.artist;

            // EXTINF line: duration,artist - title
            if artist.is_empty() {
                writeln!(file, "#EXTINF:{},{}", duration_secs, title)
                    .map_err(PlayerError::Io)?;
            } else {
                writeln!(file, "#EXTINF:{},{} - {}", duration_secs, artist, title)
                    .map_err(PlayerError::Io)?;
            }

            // File path
            writeln!(file, "{}", track.file_path).map_err(PlayerError::Io)?;
        }

        tracing::info!("[Playlist] Exported {} tracks to {}", self.tracks.len(), path);
        Ok(())
    }

    /// Import playlist from M3U/M3U8 file
    pub fn import_m3u(path: &str) -> Result<Vec<Track>> {
        use std::io::{BufRead, BufReader};

        let file = std::fs::File::open(path).map_err(PlayerError::Io)?;
        let reader = BufReader::new(file);

        let mut tracks: Vec<Track> = Vec::new();
        let mut current_title = String::new();
        let mut current_artist = String::new();

        for line in reader.lines() {
            let line = line.map_err(PlayerError::Io)?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                // Parse EXTINF line: #EXTINF:duration,artist - title
                if trimmed.starts_with("#EXTINF:") {
                    let content = &trimmed[8..]; // Skip "#EXTINF:"
                    if let Some(comma_idx) = content.find(',') {
                        let meta = &content[comma_idx + 1..];
                        if let Some(dash_idx) = meta.find(" - ") {
                            current_artist = meta[..dash_idx].trim().to_string();
                            current_title = meta[dash_idx + 3..].trim().to_string();
                        } else {
                            current_title = meta.trim().to_string();
                            current_artist.clear();
                        }
                    }
                }
                continue;
            }

            // This is a file path
            let file_path = trimmed.to_string();
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_path)
                .to_string();

            let title = if current_title.is_empty() {
                std::path::Path::new(&file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&file_name)
                    .to_string()
            } else {
                current_title.clone()
            };

            let artist = current_artist.clone();
            current_title.clear();
            current_artist.clear();

            let mut track = Track::new(&file_path);
            track.title = title;
            track.artist = artist;
            track.file_name = file_name;
            tracks.push(track);
        }

        tracing::info!("[Playlist] Imported {} tracks from {}", tracks.len(), path);
        Ok(tracks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_track(path: &str, title: &str, artist: &str, duration: u64) -> Track {
        let mut t = Track::new(path);
        t.title = title.to_string();
        t.artist = artist.to_string();
        t.duration = Duration::from_secs(duration);
        t
    }

    fn sample_playlist() -> Playlist {
        let mut pl = Playlist::new("test");
        pl.add_track(make_track("/music/a.mp3", "Alpha", "Artist A", 180));
        pl.add_track(make_track("/music/b.flac", "Beta", "Artist B", 240));
        pl.add_track(make_track("/music/c.ogg", "Gamma", "Artist A", 300));
        pl
    }

    // === Basic operations ===

    #[test]
    fn test_playlist_new() {
        let pl = Playlist::new("my_playlist");
        assert_eq!(pl.name(), "my_playlist");
        assert!(pl.is_empty());
        assert_eq!(pl.len(), 0);
        assert_eq!(pl.mode(), PlaylistMode::Playlist);
        assert_eq!(pl.repeat_mode(), RepeatMode::LoopPlaylist);
        assert!(pl.current_index().is_none());
        assert!(pl.current_track().is_none());
    }

    #[test]
    fn test_add_track() {
        let pl = sample_playlist();
        assert_eq!(pl.len(), 3);
        assert!(!pl.is_empty());
        // Verify track 1
        let t1 = pl.get(0).unwrap();
        assert_eq!(t1.file_name, "a.mp3");
        assert_eq!(t1.title, "Alpha");
    }

    #[test]
    fn test_add_tracks() {
        let mut pl = Playlist::new("bulk");
        let tracks = vec![
            make_track("x.flac", "X", "X Artist", 100),
            make_track("y.flac", "Y", "Y Artist", 200),
        ];
        pl.add_tracks(tracks);
        assert_eq!(pl.len(), 2);
        assert_eq!(pl.get(1).unwrap().title, "Y");
    }

    #[test]
    fn test_remove_track() {
        let mut pl = sample_playlist();
        let removed = pl.remove(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().title, "Beta");
        assert_eq!(pl.len(), 2);
        assert_eq!(pl.get(0).unwrap().title, "Alpha");
        assert_eq!(pl.get(1).unwrap().title, "Gamma");
    }

    #[test]
    fn test_remove_out_of_bounds() {
        let mut pl = sample_playlist();
        let removed = pl.remove(10);
        assert!(removed.is_none());
        assert_eq!(pl.len(), 3);
    }

    #[test]
    fn test_remove_updates_current_index() {
        let mut pl = sample_playlist();
        pl.set_current(1).unwrap();
        // Remove track before current — current should shift down
        pl.remove(0);
        assert_eq!(pl.current_index(), Some(0));
        assert_eq!(pl.current_track().unwrap().title, "Beta");
    }

    #[test]
    fn test_remove_current_track_when_last() {
        let mut pl = Playlist::new("single");
        pl.add_track(make_track("one.mp3", "One", "Me", 10));
        pl.set_current(0).unwrap();
        // Remove the last track
        pl.remove(0);
        assert!(pl.current_index().is_none());
        assert!(pl.current_track().is_none());
    }

    #[test]
    fn test_clear_playlist() {
        let mut pl = sample_playlist();
        pl.set_current(0).unwrap();
        pl.clear();
        assert_eq!(pl.len(), 0);
        assert!(pl.current_index().is_none());
    }

    // === Navigation ===

    #[test]
    fn test_set_current_valid() {
        let mut pl = sample_playlist();
        assert!(pl.set_current(1).is_ok());
        assert_eq!(pl.current_index(), Some(1));
        assert_eq!(pl.current_track().unwrap().title, "Beta");
    }

    #[test]
    fn test_set_current_invalid() {
        let mut pl = sample_playlist();
        let result = pl.set_current(10);
        assert!(result.is_err());
    }

    #[test]
    fn test_next_index_loop_playlist() {
        let mut pl = sample_playlist();
        pl.set_repeat_mode(RepeatMode::LoopPlaylist);
        pl.set_current(0).unwrap();
        assert_eq!(pl.next_index(), Some(1));
        pl.set_current(2).unwrap();
        // Wrap to 0
        assert_eq!(pl.next_index(), Some(0));
    }

    #[test]
    fn test_next_index_play_order_stop_at_end() {
        let mut pl = sample_playlist();
        pl.set_repeat_mode(RepeatMode::PlayOrder);
        pl.set_current(2).unwrap();
        assert_eq!(pl.next_index(), None); // Last track, stop
    }

    #[test]
    fn test_next_index_loop_track() {
        let mut pl = sample_playlist();
        pl.set_repeat_mode(RepeatMode::LoopTrack);
        pl.set_current(1).unwrap();
        assert_eq!(pl.next_index(), Some(1)); // Repeat same track
    }

    #[test]
    fn test_next_index_play_track_stop() {
        let mut pl = sample_playlist();
        pl.set_repeat_mode(RepeatMode::PlayTrack);
        pl.set_current(0).unwrap();
        assert_eq!(pl.next_index(), None);
    }

    #[test]
    fn test_next_index_empty_playlist() {
        let pl = Playlist::new("empty");
        assert_eq!(pl.next_index(), None);
    }

    #[test]
    fn test_next_index_no_current() {
        let pl = sample_playlist();
        // No current set
        assert_eq!(pl.next_index(), None);
    }

    #[test]
    fn test_prev_index_normal() {
        let mut pl = sample_playlist();
        pl.set_repeat_mode(RepeatMode::LoopPlaylist);
        pl.set_current(1).unwrap();
        assert_eq!(pl.prev_index(), Some(0));
    }

    #[test]
    fn test_prev_index_wrap_to_end() {
        let mut pl = sample_playlist();
        pl.set_current(0).unwrap();
        assert_eq!(pl.prev_index(), Some(2)); // Wrap to last track
    }

    #[test]
    fn test_prev_index_no_current() {
        let pl = sample_playlist();
        assert_eq!(pl.prev_index(), None);
    }

    // === Play queue ===

    #[test]
    fn test_push_and_drain_next_track() {
        let mut pl = sample_playlist();
        pl.set_current(0).unwrap();
        pl.push_next_track(2);
        assert_eq!(pl.queued_indices(), vec![2]);
        assert_eq!(pl.drain_next_track(), Some(2));
        assert!(pl.queued_indices().is_empty());
        assert_eq!(pl.drain_next_track(), None);
    }

    #[test]
    fn test_push_next_track_out_of_bounds() {
        let mut pl = sample_playlist();
        pl.push_next_track(100); // Should be silently ignored
        assert!(pl.queued_indices().is_empty());
    }

    // === Sorting ===

    #[test]
    fn test_sort_by_title() {
        let mut pl = sample_playlist();
        pl.sort(SortMode::Title, false);
        assert_eq!(pl.get(0).unwrap().title, "Alpha");
        assert_eq!(pl.get(1).unwrap().title, "Beta");
        assert_eq!(pl.get(2).unwrap().title, "Gamma");
    }

    #[test]
    fn test_sort_by_title_desc() {
        let mut pl = sample_playlist();
        pl.sort(SortMode::Title, true);
        assert_eq!(pl.get(0).unwrap().title, "Gamma");
        assert_eq!(pl.get(2).unwrap().title, "Alpha");
    }

    #[test]
    fn test_sort_by_artist() {
        let mut pl = sample_playlist();
        pl.sort(SortMode::Artist, false);
        // Artist A (2 tracks), Artist B (1 track)
        assert_eq!(pl.get(0).unwrap().artist, "Artist A");
        assert_eq!(pl.get(1).unwrap().artist, "Artist A");
        assert_eq!(pl.get(2).unwrap().artist, "Artist B");
    }

    #[test]
    fn test_sort_by_duration() {
        let mut pl = sample_playlist();
        pl.sort(SortMode::Time, false);
        assert_eq!(pl.get(0).unwrap().duration.as_secs(), 180);
        assert_eq!(pl.get(2).unwrap().duration.as_secs(), 300);
    }

    // === Search ===

    #[test]
    fn test_search_by_title() {
        let pl = sample_playlist();
        let results = pl.search("beta");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.title, "Beta");
    }

    #[test]
    fn test_search_by_artist() {
        let pl = sample_playlist();
        let results = pl.search("Artist A");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_by_file_name() {
        let pl = sample_playlist();
        let results = pl.search("c.ogg");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.title, "Gamma");
    }

    #[test]
    fn test_search_no_match() {
        let pl = sample_playlist();
        let results = pl.search("nonexistent");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_case_insensitive() {
        let pl = sample_playlist();
        let results = pl.search("GAMMA");
        assert_eq!(results.len(), 1);
    }

    // === Move ===

    #[test]
    fn test_move_track() {
        let mut pl = sample_playlist();
        // Move Alpha (0) to position 2
        pl.move_track(0, 2).unwrap();
        assert_eq!(pl.get(0).unwrap().title, "Beta");
        assert_eq!(pl.get(2).unwrap().title, "Alpha");
    }

    #[test]
    fn test_move_track_invalid_from() {
        let mut pl = sample_playlist();
        assert!(pl.move_track(10, 0).is_err());
    }

    #[test]
    fn test_move_track_invalid_to() {
        let mut pl = sample_playlist();
        assert!(pl.move_track(0, 10).is_err());
    }

    // === Dedup ===

    #[test]
    fn test_dedup_removes_duplicates() {
        let mut pl = sample_playlist();
        // Add duplicate of first track
        pl.add_track(make_track("/music/a.mp3", "Alpha dup", "Artist X", 999));
        assert_eq!(pl.len(), 4);
        let removed = pl.dedup();
        assert_eq!(removed, 1);
        assert_eq!(pl.len(), 3);
    }

    #[test]
    fn test_dedup_no_duplicates() {
        let mut pl = sample_playlist();
        let removed = pl.dedup();
        assert_eq!(removed, 0);
        assert_eq!(pl.len(), 3);
    }

    // === Total duration ===

    #[test]
    fn test_total_duration() {
        let pl = sample_playlist();
        assert_eq!(pl.total_duration(), Duration::from_secs(180 + 240 + 300));
    }

    #[test]
    fn test_total_duration_empty() {
        let pl = Playlist::new("empty");
        assert_eq!(pl.total_duration(), Duration::ZERO);
    }

    #[test]
    fn test_total_duration_str() {
        let pl = sample_playlist();
        let s = pl.total_duration_str();
        // 180+240+300 = 720 sec = 12:00
        assert_eq!(s, "12:00");
    }

    // === Accessors / Mutators ===

    #[test]
    fn test_set_name_and_path() {
        let mut pl = Playlist::new("old");
        pl.set_name("new_name".to_string());
        pl.set_path("/some/path".to_string());
        assert_eq!(pl.name(), "new_name");
        assert_eq!(pl.path(), "/some/path");
    }

    #[test]
    fn test_set_mode_and_repeat() {
        let mut pl = Playlist::new("test");
        pl.set_mode(PlaylistMode::MediaLib);
        pl.set_repeat_mode(RepeatMode::PlayTrack);
        assert_eq!(pl.mode(), PlaylistMode::MediaLib);
        assert_eq!(pl.repeat_mode(), RepeatMode::PlayTrack);
    }

    #[test]
    fn test_track_display_name() {
        let t = make_track("/m/song.flac", "My Song", "Me", 200);
        assert_eq!(t.display_name("title"), "My Song");
        assert_eq!(t.display_name("artist_title"), "Me - My Song");
        assert_eq!(t.display_name("file_name"), "song.flac");
        assert_eq!(t.display_name("title_artist"), "My Song - Me");
    }

    #[test]
    fn test_track_display_name_empty_title() {
        let mut t = Track::new("/m/song2.flac");
        t.file_name = "song2.flac".to_string();
        assert_eq!(t.display_name("title"), "song2.flac");
    }

    #[test]
    fn test_track_duration_str() {
        let t = make_track("x.mp3", "", "", 65);
        assert_eq!(t.duration_str(), "01:05");
    }

    #[test]
    fn test_track_file_type_from_extension() {
        let t = Track::new("/path/music.flac");
        assert_eq!(t.file_type, "flac");
        let t = Track::new("/path/music.MP3");
        assert_eq!(t.file_type, "mp3");
    }

    #[test]
    fn test_repeat_mode_from_str() {
        assert_eq!(RepeatMode::from_str("order"), RepeatMode::PlayOrder);
        assert_eq!(RepeatMode::from_str("shuffle"), RepeatMode::PlayShuffle);
        assert_eq!(RepeatMode::from_str("random"), RepeatMode::PlayRandom);
        assert_eq!(RepeatMode::from_str("loop"), RepeatMode::LoopPlaylist);
        assert_eq!(RepeatMode::from_str("track"), RepeatMode::LoopTrack);
        assert_eq!(RepeatMode::from_str("play_track"), RepeatMode::PlayTrack);
        assert_eq!(RepeatMode::from_str("unknown"), RepeatMode::LoopPlaylist);
    }

    #[test]
    fn test_repeat_mode_to_str_and_desc() {
        let modes = [
            (RepeatMode::PlayOrder, "order", "顺序播放"),
            (RepeatMode::PlayShuffle, "shuffle", "无序播放"),
            (RepeatMode::PlayRandom, "random", "随机播放"),
            (RepeatMode::LoopPlaylist, "loop", "列表循环"),
            (RepeatMode::LoopTrack, "track", "单曲循环"),
            (RepeatMode::PlayTrack, "play_track", "单曲播放"),
        ];
        for (mode, s, desc) in modes {
            assert_eq!(mode.to_str(), s);
            assert_eq!(mode.description(), desc);
        }
    }

    #[test]
    fn test_sort_mode_from_str() {
        assert_eq!(SortMode::from_str("title"), SortMode::Title);
        assert_eq!(SortMode::from_str("artist"), SortMode::Artist);
        assert_eq!(SortMode::from_str("random"), SortMode::Random);
        assert_eq!(SortMode::from_str("unknown"), SortMode::Default);
    }

    #[test]
    fn test_remove_multiple() {
        let mut pl = sample_playlist();
        pl.remove_multiple(vec![0, 2]);
        assert_eq!(pl.len(), 1);
        assert_eq!(pl.get(0).unwrap().title, "Beta");
    }

    #[test]
    fn test_shuffle_init() {
        let mut pl = sample_playlist();
        pl.init_shuffle(None);
        // Shuffle list should have 3 entries
        assert_eq!(pl.queued_indices().len(), 0); // next_tracks is separate
        // Verify shuffle_list was internalized (no public getter, but check next_index with shuffle mode)
        pl.set_repeat_mode(RepeatMode::PlayShuffle);
        pl.set_current(pl.shuffle_list[0]).unwrap();
        let n1 = pl.next_index();
        assert!(n1.is_some()); // Should have at least one more in shuffle list
    }

    #[test]
    fn test_shuffle_init_with_first() {
        let mut pl = sample_playlist();
        pl.init_shuffle(Some(1)); // Beta should be first
        pl.set_repeat_mode(RepeatMode::PlayShuffle);
        // First shuffle entry should be 1 (Beta)
        assert_eq!(pl.shuffle_list[0], 1);
    }

    #[test]
    fn test_max_track_capacity() {
        let mut pl = Playlist::new("max_test");
        let many: Vec<Track> = (0..MAX_TRACK_COUNT).map(|i| {
            Track::new(&format!("/music/{}.mp3", i))
        }).collect();
        pl.add_tracks(many);
        assert_eq!(pl.len(), MAX_TRACK_COUNT);
        // Adding one more should be silently ignored
        pl.add_track(Track::new("/music/overflow.mp3"));
        assert_eq!(pl.len(), MAX_TRACK_COUNT);
    }

    #[test]
    fn test_next_index_play_random_single_track() {
        let mut pl = Playlist::new("single");
        pl.add_track(make_track("only.mp3", "Only", "Me", 10));
        pl.set_repeat_mode(RepeatMode::PlayRandom);
        pl.set_current(0).unwrap();
        // With only 1 track, PlayRandom always returns Some(0)
        for _ in 0..20 {
            assert_eq!(pl.next_index(), Some(0));
        }
    }

    #[test]
    fn test_next_index_play_random_multi_track() {
        let mut pl = sample_playlist(); // 3 tracks
        pl.set_repeat_mode(RepeatMode::PlayRandom);
        pl.set_current(0).unwrap();
        // With 3 tracks and current=0, next should always be Some(idx) where idx != 0
        for _ in 0..50 {
            let next = pl.next_index();
            assert!(next.is_some(), "PlayRandom should always return Some for 3 tracks");
            let idx = next.unwrap();
            assert!(idx < 3, "Index should be in bounds");
            assert_ne!(idx, 0, "PlayRandom should not return current index when >1 track");
        }
    }

    #[test]
    fn test_next_index_play_shuffle_from_start() {
        let mut pl = sample_playlist(); // 3 tracks: Alpha, Beta, Gamma
        pl.init_shuffle(None);
        pl.set_repeat_mode(RepeatMode::PlayShuffle);
        // After init_shuffle, shuffle_index = 0. next_index returns shuffle_list[1]
        pl.set_current(pl.shuffle_list[0]).unwrap();
        let next = pl.next_index();
        assert_eq!(next, Some(pl.shuffle_list[1]),
            "from shuffle position 0, next should be shuffle_list[1]");
    }

    #[test]
    fn test_next_index_play_shuffle_at_end() {
        let mut pl = sample_playlist();
        pl.init_shuffle(None);
        pl.set_repeat_mode(RepeatMode::PlayShuffle);
        // Manually advance shuffle_index to last position
        let last = pl.shuffle_list.len() - 1;
        pl.shuffle_index = last;
        pl.set_current(pl.shuffle_list[last]).unwrap();
        assert_eq!(pl.next_index(), None, "at end of shuffle list, should return None");
    }

    #[test]
    fn test_next_index_play_shuffle_single_track() {
        let mut pl = Playlist::new("single");
        pl.add_track(make_track("only.mp3", "Only", "Me", 10));
        pl.init_shuffle(None);
        pl.set_repeat_mode(RepeatMode::PlayShuffle);
        pl.set_current(0).unwrap();
        // Single track → no shuffle list → fallback: return Some(0) only if tracks.len() == 1
        assert_eq!(pl.next_index(), None, "PlayShuffle with single track and empty shuffle_list");
    }

    // === M3U Import/Export ===

    /// Export playlist to M3U/M3U8 file
    pub fn export_m3u(&self, path: &str, utf8: bool) -> Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)
            .map_err(|e| PlayerError::Io(e))?;

        // M3U header
        writeln!(file, "#EXTM3U").map_err(|e| PlayerError::Io(e))?;

        for track in &self.tracks {
            let duration_secs = track.duration.as_secs() as i64;
            let title = if track.title.is_empty() {
                &track.file_name
            } else {
                &track.title
            };
            let artist = &track.artist;

            // EXTINF line: duration,artist - title
            if artist.is_empty() {
                writeln!(file, "#EXTINF:{},{}", duration_secs, title)
                    .map_err(|e| PlayerError::Io(e))?;
            } else {
                writeln!(file, "#EXTINF:{},{} - {}", duration_secs, artist, title)
                    .map_err(|e| PlayerError::Io(e))?;
            }

            // File path (use absolute path)
            writeln!(file, "{}", track.file_path).map_err(|e| PlayerError::Io(e))?;
        }

        tracing::info!("[Playlist] Exported {} tracks to {} (utf8={})", self.tracks.len(), path, utf8);
        Ok(())
    }

    /// Import playlist from M3U/M3U8 file
    pub fn import_m3u(path: &str) -> Result<Vec<Track>> {
        use std::io::{BufRead, BufReader};

        let file = std::fs::File::open(path)
            .map_err(|e| PlayerError::Io(e))?;
        let reader = BufReader::new(file);

        let mut tracks: Vec<Track> = Vec::new();
        let mut current_title = String::new();
        let mut current_artist = String::new();

        for line in reader.lines() {
            let line = line.map_err(|e| PlayerError::Io(e))?;
            let trimmed = line.trim();

            if trimmed.is_empty() || trimmed.starts_with('#') {
                // Parse EXTINF line: #EXTINF:duration,artist - title
                if trimmed.starts_with("#EXTINF:") {
                    let content = &trimmed[8..]; // Skip "#EXTINF:"
                    if let Some(comma_idx) = content.find(',') {
                        let _duration_str = &content[..comma_idx]; // Could parse but not used
                        let meta = &content[comma_idx + 1..];
                        if let Some(dash_idx) = meta.find(" - ") {
                            current_artist = meta[..dash_idx].trim().to_string();
                            current_title = meta[dash_idx + 3..].trim().to_string();
                        } else {
                            current_title = meta.trim().to_string();
                            current_artist = String::new();
                        }
                    }
                }
                continue;
            }

            // This is a file path
            let file_path = trimmed.to_string();
            let file_name = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_path)
                .to_string();

            let title = if current_title.is_empty() {
                // Use filename without extension as fallback
                std::path::Path::new(&file_name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&file_name)
                    .to_string()
            } else {
                current_title.clone()
            };

            let artist = current_artist.clone();
            current_title.clear();
            current_artist.clear();

            tracks.push(Track {
                file_path,
                file_name,
                title,
                artist,
                album: String::new(),
                genre: String::new(),
                track_number: 0,
                year: 0,
                duration: std::time::Duration::ZERO,
                bitrate: 0,
                file_type: String::new(),
                is_cue: false,
                cue_file_path: String::new(),
                cue_track_number: 0,
                start_pos: std::time::Duration::ZERO,
                is_favourite: false,
                rating: 0,
                play_count: 0,
                last_played: None,
                tags: std::collections::HashMap::new(),
            });
        }

        tracing::info!("[Playlist] Imported {} tracks from {}", tracks.len(), path);
        Ok(tracks)
    }
}
