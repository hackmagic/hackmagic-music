//! CUE sheet parser.
//!
//! Parses .cue files (CDRWIN format) to extract track information
//! with start/end positions within the underlying audio file.
//! Mirrors the original `CCueFile` class.

use std::fs;
use std::path::Path;
use std::time::Duration;

/// A single track parsed from a CUE sheet
#[derive(Debug, Clone)]
pub struct CueTrack {
    /// Track number (1-based)
    pub track: u32,
    /// Track title
    pub title: String,
    /// Track artist (PERFORMER)
    pub artist: String,
    /// Underlying audio file path (resolved absolute)
    pub file_path: String,
    /// Start position in the audio file
    pub start_pos: Duration,
    /// End position (derived from next track's INDEX, or zero if last track)
    pub end_pos: Duration,
}

/// Parsed CUE sheet
#[derive(Debug, Clone)]
pub struct CueSheet {
    /// Path to the .cue file
    pub cue_path: String,
    /// Album artist (global PERFORMER)
    pub album_artist: String,
    /// Album title
    pub album: String,
    /// Genre (REM GENRE)
    pub genre: String,
    /// Year (REM DATE)
    pub year: String,
    /// Comment (REM COMMENT)
    pub comment: String,
    /// Tracks parsed from this cue sheet
    pub tracks: Vec<CueTrack>,
}

/// Parse a .cue file and return a `CueSheet`.
pub fn parse_cue_file(path: &str) -> Result<CueSheet, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read cue file '{path}': {e}"))?;
    parse_cue_content(path, &content)
}

/// Parse CUE content string.
/// `cue_path` is used to resolve relative FILE paths.
pub fn parse_cue_content(cue_path: &str, content: &str) -> Result<CueSheet, String> {
    let cue_dir = Path::new(cue_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut album_artist = String::new();
    let mut album = String::new();
    let mut genre = String::new();
    let mut year = String::new();
    let mut comment = String::new();

    // Current state as we scan
    let mut current_file = String::new();
    let mut current_track_num: u32 = 0;
    let mut current_title = String::new();
    let mut current_artist = String::new();
    let mut current_start = Duration::ZERO;

    // Temporary buffer for building tracks before adding to the list
    // so we can compute end_pos from the next track's start_pos
    let mut pending: Vec<CueTrack> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let upper = trimmed.to_uppercase();

        if upper.starts_with("REM ") {
            let rem_cmd = trimmed[3..].trim();
            if let Some(v) = get_quoted_value(rem_cmd, "GENRE") {
                genre = v;
            } else if let Some(v) = get_quoted_value(rem_cmd, "DATE") {
                year = v;
            } else if let Some(v) = get_quoted_value(rem_cmd, "COMMENT") {
                comment = v;
            }
            // Discard other REM lines (REPLAYGAIN, etc.)
        } else if let Some(v) = get_quoted_value(trimmed, "PERFORMER") {
            if current_track_num > 0 {
                current_artist = v;
            } else {
                album_artist = v;
            }
        } else if let Some(v) = get_quoted_value(trimmed, "TITLE") {
            if current_track_num > 0 {
                current_title = v;
            } else {
                album = v;
            }
        } else if upper.starts_with("FILE ") {
            // Save pending track before switching files
            if current_track_num > 0 {
                pending.push(CueTrack {
                    track: current_track_num,
                    title: std::mem::take(&mut current_title),
                    artist: if current_artist.is_empty() {
                        album_artist.clone()
                    } else {
                        std::mem::take(&mut current_artist)
                    },
                    file_path: current_file.clone(),
                    start_pos: current_start,
                    end_pos: Duration::ZERO,
                });
                current_track_num = 0;
                current_start = Duration::ZERO;
            }

            // Extract the file name from: FILE "filename" WAVE
            let file_rem = trimmed[4..].trim();
            let raw = extract_quoted_or_word(file_rem);
            current_file = resolve_path(&cue_dir, &raw);
        } else if upper.starts_with("TRACK ") {
            // Save previous track
            if current_track_num > 0 {
                pending.push(CueTrack {
                    track: current_track_num,
                    title: std::mem::take(&mut current_title),
                    artist: if current_artist.is_empty() {
                        album_artist.clone()
                    } else {
                        std::mem::take(&mut current_artist)
                    },
                    file_path: current_file.clone(),
                    start_pos: current_start,
                    end_pos: Duration::ZERO,
                });
            }

            // Parse TRACK 01 AUDIO
            let rest = trimmed[5..].trim();
            let num_str = rest.split_whitespace().next().unwrap_or("0");
            current_track_num = num_str.parse().unwrap_or(0);
            current_title.clear();
            current_artist.clear();
            current_start = Duration::ZERO;
        } else if upper.starts_with("INDEX 01") {
            let time_str = trimmed[8..].trim();
            current_start = parse_cue_time(time_str);
        }
        // INDEX 00 (pregap) is ignored — we use INDEX 01
    }

    // Finalize last pending track
    if current_track_num > 0 {
        pending.push(CueTrack {
            track: current_track_num,
            title: std::mem::take(&mut current_title),
            artist: if current_artist.is_empty() {
                album_artist.clone()
            } else {
                std::mem::take(&mut current_artist)
            },
            file_path: current_file,
            start_pos: current_start,
            end_pos: Duration::ZERO,
        });
    }

    // Fill end_pos for each track from the next track's start_pos
    // (the last track's end_pos remains Duration::ZERO = "until file end")
    for i in 0..pending.len() {
        let end = if i + 1 < pending.len() {
            pending[i + 1].start_pos
        } else {
            Duration::ZERO
        };
        pending[i].end_pos = end;
    }

    Ok(CueSheet {
        cue_path: cue_path.to_string(),
        album_artist,
        album,
        genre,
        year,
        comment,
        tracks: pending,
    })
}

/// Parse CUE time format MM:SS:FF (frames, 75 fps) -> Duration
fn parse_cue_time(time_str: &str) -> Duration {
    let parts: Vec<&str> = time_str.trim().split(':').collect();
    if parts.len() == 3 {
        let m: u64 = parts[0].parse().unwrap_or(0);
        let s: u64 = parts[1].parse().unwrap_or(0);
        let f: u64 = parts[2].parse().unwrap_or(0);
        Duration::from_millis(m * 60_000 + s * 1_000 + f * 1000 / 75)
    } else {
        Duration::ZERO
    }
}

/// Extract a quoted string value from a line.
/// Looks for: COMMAND "value"
fn get_quoted_value(line: &str, cmd: &str) -> Option<String> {
    get_quoted_value_raw(line, cmd).map(|s| normalize_string(&s))
}

/// Extract the raw quoted value (no normalization)
fn get_quoted_value_raw(line: &str, cmd: &str) -> Option<String> {
    let upper = line.to_uppercase();
    // Find the command (case-insensitive)
    let pos = upper.find(&cmd.to_uppercase())?;
    let after = &line[pos + cmd.len()..];
    // Find first quote
    let q1 = after.find('"')?;
    let after_q1 = &after[q1 + 1..];
    let q2 = after_q1.find('"')?;
    Some(after_q1[..q2].to_string())
}

/// Extract the first quoted string, or first word (for unquoted values like FILE)
fn extract_quoted_or_word(s: &str) -> String {
    let s = s.trim();
    if let Some(q1) = s.find('"') {
        if let Some(q2) = s[q1 + 1..].find('"') {
            return s[q1 + 1..q1 + 1 + q2].to_string();
        }
    }
    s.split_whitespace().next().unwrap_or("").to_string()
}

/// Resolve a potentially relative path against the cue file directory
fn resolve_path(cue_dir: &str, file_name: &str) -> String {
    if file_name.is_empty() {
        return String::new();
    }
    let p = Path::new(file_name);
    if p.is_absolute() {
        p.to_string_lossy().to_string()
    } else {
        let base = Path::new(cue_dir);
        let full = base.join(file_name);
        full.to_string_lossy().to_string()
    }
}

fn normalize_string(s: &str) -> String {
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cue_time() {
        assert_eq!(parse_cue_time("00:00:00"), Duration::ZERO);
        assert_eq!(parse_cue_time("01:30:00"), Duration::from_secs(90));
        assert_eq!(parse_cue_time("00:01:75"), Duration::from_millis(1000 + 1000)); // 1s + 1s (75 frames)
        assert_eq!(parse_cue_time(""), Duration::ZERO);
        assert_eq!(parse_cue_time("not_a_time"), Duration::ZERO);
    }

    #[test]
    fn test_parse_cue_content_empty() {
        let result = parse_cue_content("/music/test.cue", "");
        assert!(result.is_ok());
        let sheet = result.unwrap();
        assert_eq!(sheet.album, "");
        assert_eq!(sheet.genre, "");
        assert_eq!(sheet.year, "");
        assert_eq!(sheet.comment, "");
        assert_eq!(sheet.tracks.len(), 0);
        assert_eq!(sheet.cue_path, "/music/test.cue");
    }

    #[test]
    fn test_parse_cue_content_multiple_files() {
        let cue = r#"TITLE "Multi-File Album"
PERFORMER "Test Artist"
FILE "disc1.flac" WAVE
  TRACK 01 AUDIO
    TITLE "Song One"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Song Two"
    INDEX 01 03:00:00
FILE "disc2.flac" WAVE
  TRACK 03 AUDIO
    TITLE "Song Three"
    INDEX 01 00:00:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        assert_eq!(sheet.album, "Multi-File Album");
        assert_eq!(sheet.tracks.len(), 3);
        // On Windows, Path::join uses backslash; on Unix, forward slash
        assert!(sheet.tracks[0].file_path.ends_with("disc1.flac"));
        assert!(sheet.tracks[1].file_path.ends_with("disc1.flac"));
        assert!(sheet.tracks[2].file_path.ends_with("disc2.flac"));
        // Track 2's end_pos should be track 3's start_pos (which is 0 on new file)
        assert_eq!(sheet.tracks[1].end_pos, Duration::from_secs(0));
        assert_eq!(sheet.tracks[2].start_pos, Duration::ZERO);
    }

    #[test]
    fn test_parse_cue_content_pregap_ignored() {
        let cue = r#"PERFORMER "Pregap Artist"
TITLE "Pregap Album"
FILE "track.flac" WAVE
  TRACK 01 AUDIO
    TITLE "With Pregap"
    INDEX 00 00:00:00
    INDEX 01 00:02:30
  TRACK 02 AUDIO
    TITLE "No Pregap"
    INDEX 01 05:00:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        // INDEX 00 (pregap) should be ignored; start should be from INDEX 01
        // CUE time format: mm:ss:ff where ff = frames (75 per second)
        // 00:02:30 = 0min 2sec 30frames = 2000ms + 30*1000/75ms = 2400ms = 2.4s
        assert_eq!(sheet.tracks[0].start_pos, Duration::from_millis(2000 + 30 * 1000 / 75));
        // 05:00:00 = 5min 0sec 0frames = 300s
        assert_eq!(sheet.tracks[1].start_pos, Duration::from_secs(300));
    }

    #[test]
    fn test_parse_cue_content_comment_field() {
        let cue = r#"REM COMMENT "This is a comment"
TITLE "Comment Album"
FILE "track.flac" WAVE
  TRACK 01 AUDIO
    TITLE "A Track"
    INDEX 01 00:00:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        assert_eq!(sheet.comment, "This is a comment");
    }

    #[test]
    fn test_parse_cue_content_track_without_title_falls_back() {
        // When a track has no TITLE, its title should be empty
        let cue = r#"PERFORMER "AlbumArtist"
TITLE "Fallback Album"
FILE "track.flac" WAVE
  TRACK 01 AUDIO
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Has Title"
    INDEX 01 03:00:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        assert_eq!(sheet.tracks[0].title, "");
        // Artist should fall back to album_artist
        assert_eq!(sheet.tracks[0].artist, "AlbumArtist");
        assert_eq!(sheet.tracks[1].title, "Has Title");
    }

    #[test]
    fn test_parse_cue_content_multiple_rem_fields() {
        let cue = r#"REM GENRE "Jazz"
REM DATE "1995"
TITLE "Rem Test"
FILE "track.flac" WAVE
  TRACK 01 AUDIO
    TITLE "Track One"
    INDEX 01 00:00:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        assert_eq!(sheet.genre, "Jazz");
        assert_eq!(sheet.year, "1995");
    }

    #[test]
    fn test_simple_cue() {
        let cue = r#"REM GENRE "Classical"
REM DATE "2020"
TITLE "Best Of"
PERFORMER "Various Artists"
FILE "track.flac" WAVE
  TRACK 01 AUDIO
    TITLE "First Movement"
    PERFORMER "Artist A"
    INDEX 01 00:00:00
  TRACK 02 AUDIO
    TITLE "Second Movement"
    INDEX 01 05:30:00
"#;
        let sheet = parse_cue_content("/music/test.cue", cue).unwrap();
        assert_eq!(sheet.album, "Best Of");
        assert_eq!(sheet.genre, "Classical");
        assert_eq!(sheet.tracks.len(), 2);
        assert_eq!(sheet.tracks[0].track, 1);
        assert_eq!(sheet.tracks[0].title, "First Movement");
        assert_eq!(sheet.tracks[0].artist, "Artist A");
        assert_eq!(sheet.tracks[0].start_pos, Duration::ZERO);
        assert_eq!(sheet.tracks[0].end_pos, Duration::from_secs(330)); // 5:30
        assert_eq!(sheet.tracks[1].track, 2);
        assert_eq!(sheet.tracks[1].title, "Second Movement");
        assert_eq!(sheet.tracks[1].start_pos, Duration::from_secs(330));
    }
}
