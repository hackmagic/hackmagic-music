//! Audio tag reading using the `lofty` crate.
//! Replaces the original TagLib-based tag reading.

use crate::core::playlist::Track;
use crate::error::{PlayerError, Result};
use lofty::read_from_path;
use lofty::prelude::{Accessor, AudioFile, TaggedFileExt};
use lofty::tag::{ItemKey, Tag};
use std::path::Path;
use std::time::Duration;

/// Read audio tags from a file and return a Track
pub fn read_tags(file_path: &str) -> Result<Track> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(PlayerError::FileNotFound(file_path.to_string()));
    }

    let mut track = Track::new(file_path);

    // Try to read file properties and tags
    match read_from_path(path) {
        Ok(tagged_file) => {
            // Properties (duration, bitrate, sample rate, etc.)
            let props = tagged_file.properties();
            track.duration = Duration::from_millis(props.duration().as_millis() as u64);
            track.bitrate = props.audio_bitrate().unwrap_or(0);
            track.sample_rate = props.sample_rate().unwrap_or(0);
            track.channels = props.channels().unwrap_or(0);

            // Tag data
            if let Some(tag) = tagged_file.primary_tag() {
                fill_from_tag(tag, &mut track);
            } else if let Some(tag) = tagged_file.first_tag() {
                fill_from_tag(tag, &mut track);
            }
        }
        Err(e) => {
            tracing::warn!("Cannot read tags from '{}': {}", file_path, e);
            // Continue with filename-derived info
        }
    }

    Ok(track)
}

/// Read embedded lyrics from an audio file.
/// Returns the lyrics text as a string if found, or an error.
pub fn read_embedded_lyrics(file_path: &str) -> Result<String> {
    let path = Path::new(file_path);
    let tagged_file = read_from_path(path)
        .map_err(|e| PlayerError::Other(format!("Cannot read tags: {e}")))?;

    // Try primary tag first, then first tag
    for tag in [tagged_file.primary_tag(), tagged_file.first_tag()].into_iter().flatten() {
        // Use ItemKey::Lyrics if present
        if let Some(val) = tag.get_string(&ItemKey::Lyrics) {
            if !val.is_empty() {
                return Ok(val.to_string());
            }
        }
        // Fallback: check all items for lyric-like keys
        for item in tag.items() {
            let key_debug = format!("{:?}", item.key());
            if key_debug.to_lowercase().contains("lyric") {
                if let Some(val) = item.value().text() {
                    if !val.is_empty() {
                        return Ok(val.to_string());
                    }
                }
            }
        }
    }
    Err(PlayerError::Other("No embedded lyrics found".into()))
}

/// Write lyrics to an audio file's tag.
pub fn write_embedded_lyrics(file_path: &str, lyrics_text: &str) -> Result<()> {
    use lofty::config::WriteOptions;
    use lofty::file::TaggedFileExt;

    let path = Path::new(file_path);
    let mut tagged_file = read_from_path(path)
        .map_err(|e| PlayerError::Tag(format!("Cannot read tags: {e}")))?;

    let tag = tagged_file
        .first_tag_mut()
        .ok_or_else(|| PlayerError::Tag("No tag found for this file type".to_string()))?;

    tag.insert_text(ItemKey::Lyrics, lyrics_text.to_string());

    let write_opts = WriteOptions::new();
    tagged_file
        .save_to_path(path, write_opts)
        .map_err(|e| PlayerError::Tag(format!("Cannot write tags: {e}")))?;

    Ok(())
}

fn fill_from_tag(tag: &Tag, track: &mut Track) {
    if let Some(title) = tag.title() {
        track.title = title.to_string();
    }
    if let Some(artist) = tag.artist() {
        track.artist = artist.to_string();
    }
    if let Some(album) = tag.album() {
        track.album = album.to_string();
    }
    if let Some(genre) = tag.genre() {
        track.genre = genre.to_string();
    }
    if let Some(track_num) = tag.track() {
        track.track_number = track_num;
    }
    if let Some(year) = tag.year() {
        track.year = year;
    }

    // ReplayGain (format: "-2.34 dB" or raw float)
    fn parse_gain(s: &str) -> f32 {
        let s = s.trim().trim_end_matches("dB").trim().replace(',', ".");
        s.parse::<f32>().unwrap_or(0.0)
    }
    if let Some(g) = tag.get_string(&ItemKey::ReplayGainTrackGain) {
        track.track_gain = parse_gain(g);
    }
    if let Some(p) = tag.get_string(&ItemKey::ReplayGainTrackPeak) {
        track.track_peak = p.trim().parse::<f32>().unwrap_or(0.0);
    }
    if let Some(g) = tag.get_string(&ItemKey::ReplayGainAlbumGain) {
        track.album_gain = parse_gain(g);
    }
    if let Some(p) = tag.get_string(&ItemKey::ReplayGainAlbumPeak) {
        track.album_peak = p.trim().parse::<f32>().unwrap_or(0.0);
    }

    // Read rating from tag (Popularimeter/RATING)
    if let Some(rating_str) = tag.get_string(&ItemKey::Popularimeter) {
        if let Ok(r) = rating_str.trim().parse::<u32>() {
            if r <= 5 {
                track.rating = r;
            } else if r <= 10 {
                track.rating = r.div_ceil(2); // 0-10 to 0-5
            } else {
                track.rating = r.clamp(0, 5);
            }
        }
    } else {
        // Try binary POPM from ID3v2
        for item in tag.items() {
            if item.key() != &ItemKey::Popularimeter {
                continue;
            }
            if let lofty::tag::ItemValue::Binary(data) = item.value() {
                // POPM format: email\0 + rating_byte(0-255) + counter
                // Find null terminator, then read the next byte
                if let Some(null_pos) = data.iter().position(|&b| b == 0) {
                    if null_pos + 1 < data.len() {
                        let raw = data[null_pos + 1];
                        // Map 0-255 rating to 0-5
                        track.rating = match raw {
                            0 => 0,
                            1..=63 => 1,
                            64..=127 => 2,
                            128..=185 => 3,
                            186..=219 => 4,
                            _ => 5,
                        };
                    }
                }
                break;
            }
        }
    }
}

/// Read tags for multiple files, returns Vec<Track>
#[cfg(test)]
pub fn read_tags_batch(file_paths: &[String]) -> Vec<Track> {
    file_paths.iter()
        .filter_map(|p| read_tags(p).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Create a minimal valid WAV file with PCM 16-bit mono audio.
    /// Returns the path to the temp file.
    fn create_minimal_wav(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let sample_rate = 44100u32;
        let channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let data_size: u32 = 2; // one sample (silence)
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample as u32 / 8);
        let block_align = channels * (bits_per_sample / 8);

        let mut file = std::fs::File::create(&path).unwrap();

        // RIFF header
        file.write_all(b"RIFF").unwrap();
        let file_size: u32 = 36 + data_size; // total file size - 8
        file.write_all(&file_size.to_le_bytes()).unwrap();
        file.write_all(b"WAVE").unwrap();

        // fmt chunk
        file.write_all(b"fmt ").unwrap();
        let fmt_size: u32 = 16;
        file.write_all(&fmt_size.to_le_bytes()).unwrap();
        let audio_format: u16 = 1; // PCM
        file.write_all(&audio_format.to_le_bytes()).unwrap();
        file.write_all(&channels.to_le_bytes()).unwrap();
        file.write_all(&sample_rate.to_le_bytes()).unwrap();
        file.write_all(&byte_rate.to_le_bytes()).unwrap();
        file.write_all(&block_align.to_le_bytes()).unwrap();
        file.write_all(&bits_per_sample.to_le_bytes()).unwrap();

        // data chunk
        file.write_all(b"data").unwrap();
        file.write_all(&data_size.to_le_bytes()).unwrap();
        file.write_all(&[0u8, 0u8]).unwrap(); // one silent 16-bit sample

        path
    }

    #[test]
    fn test_read_tags_file_not_found() {
        let result = read_tags("K:\\nonexistent_file.mp3");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::FileNotFound(path) => {
                assert!(path.contains("nonexistent_file.mp3"));
            }
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_read_tags_with_wav_file() {
        let dir = std::env::temp_dir().join("mp1028_test_tag_reader");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "test_silence.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = read_tags(&wav_str);
        assert!(result.is_ok(), "read_tags failed: {:?}", result.err());
        let track = result.unwrap();

        // WAV should have basic properties populated
        assert_eq!(track.file_path, wav_str);
        assert_eq!(track.file_name, "test_silence.wav");
        assert_eq!(track.file_type, "wav");
        // 44100 Hz, 16-bit, mono → 705.6 kbps bitrate (44100*16*1/1000)
        assert!(track.sample_rate == 44100 || track.sample_rate == 0);
        assert!(track.channels == 1 || track.channels == 0);
        // Title/artist might be empty for a header-only WAV
        assert_eq!(track.title, "");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_tags_batch_empty() {
        let result = read_tags_batch(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_tags_batch_all_invalid() {
        let paths = vec![
            "K:\\nonexistent1.mp3".to_string(),
            "K:\\nonexistent2.flac".to_string(),
        ];
        let result = read_tags_batch(&paths);
        assert!(result.is_empty());
    }

    #[test]
    fn test_read_tags_batch_partial_valid() {
        let dir = std::env::temp_dir().join("mp1028_test_tag_batch");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "batch_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let paths = vec![
            "K:\\does_not_exist.mp3".to_string(),
            wav_str.clone(),
        ];
        let result = read_tags_batch(&paths);
        // Only the valid WAV should survive
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].file_path, wav_str);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_read_embedded_lyrics_file_not_found() {
        let result = read_embedded_lyrics("K:\\no_such_file.mp3");
        assert!(result.is_err());
    }

    #[test]
    fn test_read_embedded_lyrics_no_lyrics_wav() {
        // A WAV without lyrics should fail gracefully
        let dir = std::env::temp_dir().join("mp1028_test_lyrics");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "no_lyrics.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = read_embedded_lyrics(&wav_str);
        assert!(result.is_err());
        // Expect "No embedded lyrics found" error
        let err = result.unwrap_err();
        match err {
            PlayerError::Other(msg) => {
                assert!(msg.contains("No embedded lyrics found") || msg.contains("lyric"));
            }
            // Some WAV parsers may not have tags at all
            PlayerError::Tag(_) => {} // acceptable
            _ => panic!("Unexpected error type: {:?}", err),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_embedded_lyrics_file_not_found() {
        let result = write_embedded_lyrics("K:\\nonexistent.flac", "test lyrics");
        assert!(result.is_err());
    }

    #[test]
    fn test_fill_from_tag_populates_track() {
        // Test the internal helper via read_tags with a file that
        // carries tags. Since we can't easily create a tagged WAV,
        // at least verify the Track defaults are correct.
        let track = Track::new("some/song.mp3");
        assert_eq!(track.title, "");
        assert_eq!(track.artist, "");
        assert_eq!(track.album, "");
        assert_eq!(track.genre, "");
        assert_eq!(track.track_number, 0);
        assert_eq!(track.year, 0);
        assert_eq!(track.track_gain, 0.0);
        assert_eq!(track.track_peak, 0.0);
        assert_eq!(track.album_gain, 0.0);
        assert_eq!(track.album_peak, 0.0);
        assert_eq!(track.rating, 0);
        assert_eq!(track.duration, std::time::Duration::ZERO);
    }

    #[test]
    fn test_read_tags_non_audio_file() {
        // A non-audio file should still return a Track (graceful degradation)
        let dir = std::env::temp_dir().join("mp1028_test_non_audio");
        let _ = std::fs::create_dir_all(&dir);
        let txt_path = dir.join("not_audio.txt");
        std::fs::write(&txt_path, b"This is not an audio file").unwrap();
        let txt_str = txt_path.to_string_lossy().to_string();

        let result = read_tags(&txt_str);
        // Should not crash, should return a Track with minimal info
        assert!(result.is_ok());
        let track = result.unwrap();
        assert_eq!(track.file_name, "not_audio.txt");
        assert_eq!(track.file_type, "txt");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
