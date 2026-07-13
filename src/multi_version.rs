use crate::core::playlist::Track;
use std::collections::HashMap;

/// Manager for merging and switching between multiple versions of the same song.
/// Follows the original C++ `CSongMultiVersion` design.
pub struct SongMultiVersion {
    /// key (title|artists|album[|`cue_track`]) -> all versions of the same song
    duplicate_songs: HashMap<String, Vec<Track>>,
}

impl SongMultiVersion {
    pub fn new() -> Self {
        Self { duplicate_songs: HashMap::new() }
    }

    /// Generate a matching key for a track.
    /// Two tracks with the same key are considered different versions of the same song.
    fn make_key(track: &Track) -> String {
        if track.title.is_empty() || track.artist.is_empty() {
            return if track.is_cue {
                format!("{}|{}", track.file_path, track.cue_track_number)
            } else {
                track.file_path.clone()
            };
        }
        let artists = split_artists(&track.artist);
        let mut key = format!("{}|{}|{}", track.title, artists.join(";"), track.album);
        if track.is_cue {
            key.push('|');
            key.push_str(&track.cue_track_number.to_string());
        }
        key
    }

    /// Merge a playlist: remove duplicate versions, keeping only the best one per group.
    /// Returns the number of tracks removed.
    pub fn merge(&mut self, tracks: &mut Vec<Track>) -> usize {
        self.duplicate_songs.clear();
        // Phase 1: group tracks by key
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, track) in tracks.iter().enumerate() {
            let key = Self::make_key(track);
            groups.entry(key).or_default().push(i);
        }

        // Phase 2: for groups with multiple tracks, pick the best and remove the rest
        let mut to_remove: Vec<usize> = Vec::new();
        for (key, indices) in &groups {
            if indices.len() <= 1 {
                continue;
            }
            // Pick the one with highest bitrate
            let best_idx = indices.iter()
                .max_by_key(|&&i| (tracks[i].bitrate, tracks[i].duration.as_secs()))
                .copied()
                .unwrap_or(indices[0]);
            // Collect all versions
            let mut versions: Vec<Track> = indices.iter()
                .map(|&i| tracks[i].clone())
                .collect();
            // Remove duplicates from versions list
            versions.sort_by(|a, b| b.bitrate.cmp(&a.bitrate));
            versions.dedup_by_key(|t| t.file_path.clone());
            self.duplicate_songs.insert(key.clone(), versions);
            // Mark non-best indices for removal
            for &i in indices {
                if i != best_idx {
                    to_remove.push(i);
                }
            }
        }

        // Remove duplicates (reverse order to preserve indices)
        to_remove.sort_by(|a, b| b.cmp(a));
        to_remove.dedup();
        for &i in &to_remove {
            tracks.remove(i);
        }

        to_remove.len()
    }

    /// Get all versions of a track
    pub fn get_versions(&self, track: &Track) -> Option<&Vec<Track>> {
        let key = Self::make_key(track);
        self.duplicate_songs.get(&key)
    }

    /// Check if a track has multiple versions
    pub fn has_versions(&self, track: &Track) -> bool {
        let key = Self::make_key(track);
        self.duplicate_songs.get(&key).is_some_and(|v| v.len() > 1)
    }

    /// Switch to a different version of a track.
    /// `index` is the index within the versions list.
    pub fn select_version(&mut self, track: &mut Track, index: usize) -> Option<Track> {
        let key = Self::make_key(track);
        let versions = self.duplicate_songs.get_mut(&key)?;
        if index >= versions.len() {
            return None;
        }
        // Save current track back to versions
        let old_version = std::mem::replace(track, versions[index].clone());
        versions[index] = old_version;
        // Move selected to front
        if index > 0 {
            versions.swap(0, index);
        }
        Some(track.clone())
    }

    pub fn is_empty(&self) -> bool {
        self.duplicate_songs.is_empty()
    }

    pub fn clear(&mut self) {
        self.duplicate_songs.clear();
    }
}

/// Split artist string by common separators
fn split_artists(artist: &str) -> Vec<&str> {
    artist.split(&['/', ';', '&', '、', ',', '\\'][..])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn make_track(title: &str, artist: &str, album: &str, bitrate: u32, path: &str) -> Track {
        Track {
            file_path: path.to_string(),
            file_name: path.rsplit_once('\\').map(|(_, f)| f).unwrap_or(path).to_string(),
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            bitrate,
            duration: Duration::from_secs(200),
            ..Track::new(path)
        }
    }

    #[test]
    fn test_same_song_different_formats() {
        let mut tracks = vec![
            make_track("Song A", "Artist X", "Album 1", 320, "/music/song.mp3"),
            make_track("Song B", "Artist Y", "Album 2", 256, "/music/song2.mp3"),
            make_track("Song A", "Artist X", "Album 1", 1411, "/music/song.flac"),
        ];
        let mut mw = SongMultiVersion::new();
        let removed = mw.merge(&mut tracks);
        assert_eq!(removed, 1);
        assert_eq!(tracks.len(), 2);
        // The FLAC version (higher bitrate) should be kept (may shift indices)
        let has_flac = tracks.iter().any(|t| t.file_path == "/music/song.flac");
        let has_mp3 = tracks.iter().any(|t| t.file_path == "/music/song.mp3");
        assert!(has_flac, "FLAC version should be kept");
        assert!(!has_mp3, "MP3 version should be removed");
    }

    #[test]
    fn test_no_duplicates() {
        let tracks = vec![
            make_track("A", "X", "1", 320, "/a.mp3"),
            make_track("B", "Y", "2", 256, "/b.mp3"),
            make_track("C", "Z", "3", 128, "/c.mp3"),
        ];
        let mut mw = SongMultiVersion::new();
        let mut t = tracks.clone();
        let removed = mw.merge(&mut t);
        assert_eq!(removed, 0);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn test_split_artists() {
        let artists = split_artists("A / B; C & D");
        assert_eq!(artists, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_empty_title_no_merge() {
        let mut tracks = vec![
            make_track("", "", "", 320, "/a.mp3"),
            make_track("", "", "", 256, "/b.mp3"),
        ];
        let mut mw = SongMultiVersion::new();
        let removed = mw.merge(&mut tracks);
        assert_eq!(removed, 0);
        assert_eq!(tracks.len(), 2);
    }

    #[test]
    fn test_select_version_first_index() {
        // Setup: merge duplicates so versions are tracked
        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/song.mp3"),
            make_track("Song", "Artist", "Album", 1411, "/song.flac"),
        ];
        let mut mw = SongMultiVersion::new();
        mw.merge(&mut tracks);

        // Select the first version (index 0 = the kept track)
        let mut track = tracks[0].clone();
        let result = mw.select_version(&mut track, 0);
        assert!(result.is_some());
        // After selecting index 0, track should match the selected version
        assert_eq!(result.unwrap().file_path, track.file_path);
        // The version list should still have 2 entries
        assert!(mw.has_versions(&track));
    }

    #[test]
    fn test_select_version_switch_to_second() {
        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/song.mp3"),
            make_track("Song", "Artist", "Album", 1411, "/song.flac"),
        ];
        let mut mw = SongMultiVersion::new();
        mw.merge(&mut tracks);

        // Start with the kept track (FLAC, index 0 after merge)
        let mut track = tracks[0].clone();
        // Initially it's the FLAC version
        assert_eq!(track.file_path, "/song.flac");

        // Switch to the second version (MP3, index 1)
        let result = mw.select_version(&mut track, 1);
        assert!(result.is_some());
        // Track should now be the MP3 version
        assert_eq!(track.file_path, "/song.mp3");
        // The returned track should match
        assert_eq!(result.unwrap().file_path, "/song.mp3");
    }

    #[test]
    fn test_select_version_out_of_bounds_returns_none() {
        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/song.mp3"),
            make_track("Song", "Artist", "Album", 1411, "/song.flac"),
        ];
        let mut mw = SongMultiVersion::new();
        mw.merge(&mut tracks);

        let mut track = tracks[0].clone();
        // Index 5 is out of bounds (only 2 versions)
        let result = mw.select_version(&mut track, 5);
        assert!(result.is_none());
        // Track should remain unchanged
        assert_eq!(track.file_path, "/song.flac");
    }

    #[test]
    fn test_select_version_no_versions_returns_none() {
        // A track that was never merged has no versions
        let mut mw = SongMultiVersion::new();
        let mut track = make_track("Alone", "Solo", "Single", 320, "/alone.mp3");
        let result = mw.select_version(&mut track, 0);
        assert!(result.is_none());
    }

    #[test]
    fn test_has_versions_after_merge() {
        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/a.mp3"),
            make_track("Song", "Artist", "Album", 256, "/b.mp3"),
        ];
        let mut mw = SongMultiVersion::new();
        mw.merge(&mut tracks);

        // The kept track should have multiple versions
        assert!(mw.has_versions(&tracks[0]));
        // A non-existent track should not
        let other = make_track("Other", "Artist", "Album", 128, "/other.mp3");
        assert!(!mw.has_versions(&other));
    }

    #[test]
    fn test_get_versions_returns_expected() {
        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/a.mp3"),
            make_track("Song", "Artist", "Album", 256, "/b.mp3"),
        ];
        let mut mw = SongMultiVersion::new();
        mw.merge(&mut tracks);

        let versions = mw.get_versions(&tracks[0]);
        assert!(versions.is_some());
        assert_eq!(versions.unwrap().len(), 2);

        // Non-existent track returns None
        let other = make_track("Other", "Artist", "Album", 128, "/other.mp3");
        assert!(mw.get_versions(&other).is_none());
    }

    #[test]
    fn test_is_empty_and_clear() {
        let mut mw = SongMultiVersion::new();
        assert!(mw.is_empty());

        let mut tracks = vec![
            make_track("Song", "Artist", "Album", 320, "/a.mp3"),
            make_track("Song", "Artist", "Album", 256, "/b.mp3"),
        ];
        mw.merge(&mut tracks);
        assert!(!mw.is_empty());

        mw.clear();
        assert!(mw.is_empty());
    }
}
