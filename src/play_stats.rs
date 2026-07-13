//! Playback statistics tracker.
//!
//! Records per-song cumulative listening time, play count, and last played time.
//! Persisted to `<config_dir>/stats.json`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

/// Per-song statistics entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct StatEntry {
    /// Total seconds the song has been listened to
    pub listen_secs: u64,
    /// Number of times playback has started
    pub play_count: u64,
    /// Unix timestamp of last play start
    pub last_played: u64,
    /// User rating (1-5, 0 = unrated)
    #[serde(default)]
    pub rating: u32,
    /// Whether the track is marked as favourite
    #[serde(default)]
    pub favourite: bool,
}


/// All statistics data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct StatsData {
    /// Map from file path / cue track key to statistics
    pub songs: HashMap<String, StatEntry>,
}


/// Global statistics state
static STATS: std::sync::OnceLock<Mutex<StatsData>> = std::sync::OnceLock::new();

fn stats_path() -> PathBuf {
    crate::config::get_config_dir().join("stats.json")
}

fn get_stats() -> &'static Mutex<StatsData> {
    STATS.get_or_init(|| {
        let data = load_stats_inner();
        Mutex::new(data)
    })
}

fn load_stats_inner() -> StatsData {
    let path = stats_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(_) => StatsData::default(),
        }
    } else {
        StatsData::default()
    }
}

/// Load statistics from disk (thread-safe)
#[cfg(test)]
pub fn load_stats() -> StatsData {
    get_stats().lock().unwrap().clone()
}

/// Save statistics to disk (thread-safe)
pub fn save_stats() {
    let data = get_stats().lock().unwrap();
    let path = stats_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&*data) {
        let _ = std::fs::write(&path, json);
    }
}

/// Record that `secs` seconds of a song have been played.
/// `current_path` is the file path (or cue key) of the playing track.
pub fn record_playback(current_path: Option<&str>, secs: u64) {
    let Some(path) = current_path else { return };
    let mut data = get_stats().lock().unwrap();
    let entry = data.songs.entry(path.to_string()).or_default();
    entry.listen_secs += secs;
}

/// Mark that a track has started playing (increments play count, records timestamp).
pub fn track_started(current_path: Option<&str>) {
    let Some(path) = current_path else { return };
    let mut data = get_stats().lock().unwrap();
    let entry = data.songs.entry(path.to_string()).or_default();
    entry.play_count += 1;
    entry.last_played = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
}

/// Get sorted stats list (by `listen_secs` descending)
pub fn top_stats(limit: usize) -> Vec<(String, StatEntry)> {
    let data = get_stats().lock().unwrap();
    let mut items: Vec<(String, StatEntry)> = data.songs.clone().into_iter().collect();
    items.sort_by(|a, b| b.1.listen_secs.cmp(&a.1.listen_secs));
    items.truncate(limit);
    items
}

/// Clear all statistics
pub fn clear_stats() {
    let mut data = get_stats().lock().unwrap();
    data.songs.clear();
    save_stats();
}

/// Set rating for a track (1-5, 0 to clear)
pub fn set_rating(path: &str, rating: u32) {
    let mut data = get_stats().lock().unwrap();
    let entry = data.songs.entry(path.to_string()).or_default();
    entry.rating = rating.min(5);
    save_stats();
}

/// Get rating for a track
pub fn get_rating(path: &str) -> u32 {
    let data = get_stats().lock().unwrap();
    data.songs.get(path).map_or(0, |e| e.rating)
}

/// Set favourite status for a track
pub fn set_favourite(path: &str, fav: bool) {
    let mut data = get_stats().lock().unwrap();
    let entry = data.songs.entry(path.to_string()).or_default();
    entry.favourite = fav;
    save_stats();
}

/// Get favourite status for a track
pub fn get_favourite(path: &str) -> bool {
    let data = get_stats().lock().unwrap();
    data.songs.get(path).is_some_and(|e| e.favourite)
}

/// Get rating and favourite for backfilling a Track
pub fn get_track_meta(path: &str) -> (u32, bool) {
    let data = get_stats().lock().unwrap();
    match data.songs.get(path) {
        Some(e) => (e.rating, e.favourite),
        None => (0, false),
    }
}

/// Backfill a single Track with persisted metadata (rating, favourite, `listen_count`)
pub fn backfill_track(track: &mut crate::core::playlist::Track) {
    let data = get_stats().lock().unwrap();
    if let Some(entry) = data.songs.get(&track.file_path) {
        if track.rating == 0 && entry.rating > 0 {
            track.rating = entry.rating;
        }
        if !track.is_favourite && entry.favourite {
            track.is_favourite = true;
        }
        if track.listen_count == 0 && entry.play_count > 0 {
            track.listen_count = entry.play_count as u32;
        }
    }
}

/// Backfill all tracks in a slice with persisted metadata
pub fn backfill_tracks(tracks: &mut [crate::core::playlist::Track]) {
    let data = get_stats().lock().unwrap();
    for track in tracks.iter_mut() {
        if let Some(entry) = data.songs.get(&track.file_path) {
            if track.rating == 0 && entry.rating > 0 {
                track.rating = entry.rating;
            }
            if !track.is_favourite && entry.favourite {
                track.is_favourite = true;
            }
            if track.listen_count == 0 && entry.play_count > 0 {
                track.listen_count = entry.play_count as u32;
            }
        }
    }
}

/// Get recently played tracks (by `last_played` descending)
pub fn recent_played(limit: usize) -> Vec<(String, StatEntry)> {
    let data = get_stats().lock().unwrap();
    let mut items: Vec<(String, StatEntry)> = data.songs.clone().into_iter()
        .filter(|(_, e)| e.last_played > 0)
        .collect();
    items.sort_by(|a, b| b.1.last_played.cmp(&a.1.last_played));
    items.truncate(limit);
    items
}

/// Total listening time across all songs (seconds)
pub fn total_listen_secs() -> u64 {
    let data = get_stats().lock().unwrap();
    data.songs.values().map(|e| e.listen_secs).sum()
}

/// Total play count across all songs
pub fn total_play_count() -> u64 {
    let data = get_stats().lock().unwrap();
    data.songs.values().map(|e| e.play_count).sum()
}

/// Number of unique tracks with stats
pub fn total_track_count() -> usize {
    let data = get_stats().lock().unwrap();
    data.songs.len()
}

/// Per-artist aggregated stats (cross-references with media lib)
pub fn artist_stats(limit: usize) -> Vec<(String, u64, u64)> {
    use std::path::Path;
    let media = crate::media::MediaLib::load();
    let data = get_stats().lock().unwrap();
    let mut artist_map: HashMap<String, (u64, u64)> = HashMap::new();

    for (path, entry) in &data.songs {
        // Try to find artist from media lib first
        let artist = media.entries.iter()
            .find(|e| e.file_path == *path)
            .map(|e| e.artist.clone())
            .filter(|a| !a.is_empty())
            .unwrap_or_else(|| {
                Path::new(path)
                    .parent()
                    .and_then(|p| p.file_name()).map_or_else(|| "Unknown".to_string(), |s| s.to_string_lossy().to_string())
            });
        let agg = artist_map.entry(artist).or_insert((0, 0));
        agg.0 += entry.listen_secs;
        agg.1 += entry.play_count;
    }

    let mut items: Vec<(String, u64, u64)> = artist_map
        .into_iter()
        .map(|(k, (s, c))| (k, s, c))
        .collect();
    items.sort_by(|a, b| b.1.cmp(&a.1));
    items.truncate(limit);
    items
}

/// Listening activity in recent periods (last 24h, 7d, 30d, all time)
pub fn activity_breakdown() -> (u64, u64, u64, u64) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let day_ago = now - 86400;
    let week_ago = now - 604_800;
    let month_ago = now - 2_592_000;

    let data = get_stats().lock().unwrap();
    let mut day_secs = 0u64;
    let mut week_secs = 0u64;
    let mut month_secs = 0u64;

    for entry in data.songs.values() {
        if entry.last_played > month_ago {
            month_secs += entry.listen_secs;
            if entry.last_played > week_ago {
                week_secs += entry.listen_secs;
                if entry.last_played > day_ago {
                    day_secs += entry.listen_secs;
                }
            }
        }
    }
    let all = data.songs.values().map(|e| e.listen_secs).sum();

    (day_secs, week_secs, month_secs, all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_top() {
        // Should not crash
        record_playback(Some("/music/song1.flac"), 30);
        record_playback(Some("/music/song1.flac"), 45);
        record_playback(Some("/music/song2.flac"), 120);
        let top = top_stats(10);
        assert!(!top.is_empty());
        // song2 should be first (120 > 75)
        assert_eq!(top[0].0, "/music/song2.flac");
        assert_eq!(top[0].1.listen_secs, 120);
        assert_eq!(top[1].1.listen_secs, 75);
    }

    #[test]
    fn test_track_started() {
        track_started(Some("/music/song3.flac"));
        track_started(Some("/music/song3.flac"));
        let top = top_stats(10);
        if let Some(entry) = top.iter().find(|(k, _)| k == "/music/song3.flac") {
            assert_eq!(entry.1.play_count, 2);
            assert!(entry.1.last_played > 0);
        }
    }
}
