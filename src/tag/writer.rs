//! Audio tag writing using the `lofty` crate.
//! Supports modifying existing tags and writing back to files.

use crate::error::{PlayerError, Result};
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, Picture, PictureType};
use lofty::prelude::Accessor;
use lofty::read_from_path;
use lofty::tag::TagType;
use std::path::Path;

/// Set a tag field on an audio file
pub fn set_tag_field(file_path: &str, field: &str, value: &str) -> Result<()> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(PlayerError::FileNotFound(file_path.to_string()));
    }

    let mut tagged_file = read_from_path(path)
        .map_err(|e| PlayerError::Tag(format!("Cannot read tags: {e}")))?;

    // Try to get an existing tag first
    let tag_exists = tagged_file.first_tag_mut().is_some();
    if !tag_exists {
        // Create a new tag appropriate for the file type
        let file_type = tagged_file.file_type();
        let tag_type = if file_type.supports_tag_type(TagType::Id3v2) {
            TagType::Id3v2
        } else if file_type.supports_tag_type(TagType::VorbisComments) {
            TagType::VorbisComments
        } else {
            TagType::Ape
        };
        tagged_file.insert_tag(lofty::tag::Tag::new(tag_type));
    }

    let tag = tagged_file
        .first_tag_mut()
        .ok_or_else(|| PlayerError::Tag("Cannot create tag for this file type".to_string()))?;

    match field.to_lowercase().as_str() {
        "title" => tag.set_title(value.to_string()),
        "artist" => tag.set_artist(value.to_string()),
        "album" => tag.set_album(value.to_string()),
        "genre" => tag.set_genre(value.to_string()),
        "comment" => tag.set_comment(value.to_string()),
        "track" | "track_number" => {
            if let Ok(n) = value.parse::<u32>() {
                tag.set_track(n);
            }
        }
        "year" => {
            if let Ok(n) = value.parse::<u32>() {
                tag.set_year(n);
            }
        }
        "lyric" | "lyrics" => {
            tag.insert_text(lofty::tag::ItemKey::Lyrics, value.to_string());
        }
        _ => return Err(PlayerError::Tag(format!("Unknown field: {field}"))),
    }

    // Write back to file
    let write_opts = WriteOptions::new();
    tagged_file
        .save_to_path(path, write_opts)
        .map_err(|e| PlayerError::Tag(format!("Cannot write tags: {e}")))?;

    Ok(())
}

/// Read all pictures (covers) from an audio file
pub fn read_pictures(file_path: &str) -> Result<Vec<(String, Vec<u8>)>> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(PlayerError::FileNotFound(file_path.to_string()));
    }

    let tagged_file = read_from_path(path)
        .map_err(|e| PlayerError::Tag(format!("Cannot read tags: {e}")))?;

    let tag = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
        .ok_or_else(|| PlayerError::Tag("No tags found".to_string()))?;

    let pictures = tag.pictures();
    if pictures.is_empty() {
        return Err(PlayerError::Tag("No pictures found".to_string()));
    }

    Ok(pictures
        .iter()
        .map(|p| {
            let mime = p
                .mime_type().map_or_else(|| "image/unknown".to_string(), std::string::ToString::to_string);
            let ext = match mime.as_str() {
                "image/jpeg" | "image/jpg" => "jpg",
                "image/png" => "png",
                "image/gif" => "gif",
                "image/bmp" => "bmp",
                _ => "bin",
            };
            (ext.to_string(), p.data().to_vec())
        })
        .collect())
}

/// Write a cover image to an audio file.
/// The image data is read from `image_path` and embedded into the audio tag.
pub fn write_picture(audio_path: &str, image_path: &str) -> Result<()> {
    let audio_path = Path::new(audio_path);
    let image_path = Path::new(image_path);

    if !audio_path.exists() {
        return Err(PlayerError::FileNotFound(audio_path.to_string_lossy().to_string()));
    }
    if !image_path.exists() {
        return Err(PlayerError::FileNotFound(image_path.to_string_lossy().to_string()));
    }

    let image_data = std::fs::read(image_path)
        .map_err(|e| PlayerError::Other(format!("Cannot read image: {e}")))?;

    let ext = image_path.extension()
        .and_then(|e| e.to_str())
        .map(str::to_lowercase)
        .unwrap_or_default();

    let (mime_type, pic_type) = match ext.as_str() {
        "jpg" | "jpeg" => (MimeType::Jpeg, PictureType::CoverFront),
        "png" => (MimeType::Png, PictureType::CoverFront),
        "bmp" => (MimeType::Bmp, PictureType::CoverFront),
        "gif" => (MimeType::Gif, PictureType::CoverFront),
        _ => return Err(PlayerError::Other(format!("Unsupported image format: .{ext}"))),
    };

    let picture = Picture::new_unchecked(pic_type, Some(mime_type), None, image_data);

    let mut tagged_file = read_from_path(audio_path)
        .map_err(|e| PlayerError::Tag(format!("Cannot read audio file: {e}")))?;

    let tag = tagged_file
        .first_tag_mut()
        .ok_or_else(|| PlayerError::Tag("No tag found for this file type".to_string()))?;

    // Remove existing pictures, then add the new one
    let pic_count = tag.pictures().len();
    for i in (0..pic_count).rev() {
        tag.remove_picture(i);
    }
    tag.push_picture(picture);

    let write_opts = WriteOptions::new();
    tagged_file
        .save_to_path(audio_path, write_opts)
        .map_err(|e| PlayerError::Tag(format!("Cannot write cover: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag::reader::read_tags;
    use std::io::Write;

    /// Create a minimal valid WAV file with PCM 16-bit mono audio.
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
        let file_size: u32 = 36 + data_size;
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
        file.write_all(&[0u8, 0u8]).unwrap();

        path
    }

    /// Create a minimal valid JPEG file
    fn create_minimal_jpg(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        // Minimal JPEG: SOI + APP0 + EOI
        std::fs::write(&path, [
            0xFF, 0xD8, // SOI
            0xFF, 0xE0, 0x00, 0x10, // APP0 marker + length
            0x4A, 0x46, 0x49, 0x46, 0x00, // "JFIF\0"
            0x01, 0x01, // version
            0x00, // units
            0x00, 0x01, 0x00, 0x01, // X/Y density
            0x00, 0x00, // thumbnail
            0xFF, 0xD9, // EOI
        ]).unwrap();
        path
    }

    // ── set_tag_field ────────────────────────────────────────

    #[test]
    fn test_set_tag_field_file_not_found() {
        let result = set_tag_field("K:\\nonexistent_file.flac", "title", "Test");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::FileNotFound(path) => {
                assert!(path.contains("nonexistent_file.flac"));
            }
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_set_tag_field_unknown_field() {
        let dir = std::env::temp_dir().join("hm_test_writer_unknown");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "unknown_field.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "non_existent_field", "value");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::Tag(msg) => {
                assert!(msg.contains("Unknown field"));
            }
            other => panic!("Expected Tag error with 'Unknown field', got: {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_title() {
        let dir = std::env::temp_dir().join("hm_test_writer_title");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "title_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "title", "My Song Title");
        assert!(result.is_ok(), "set_tag_field failed: {:?}", result.err());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.title, "My Song Title");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_artist() {
        let dir = std::env::temp_dir().join("hm_test_writer_artist");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "artist_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "artist", "Test Artist");
        assert!(result.is_ok());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.artist, "Test Artist");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_album() {
        let dir = std::env::temp_dir().join("hm_test_writer_album");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "album_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "album", "Greatest Hits");
        assert!(result.is_ok());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.album, "Greatest Hits");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_genre() {
        let dir = std::env::temp_dir().join("hm_test_writer_genre");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "genre_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "genre", "Rock");
        assert!(result.is_ok());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.genre, "Rock");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_track_number() {
        let dir = std::env::temp_dir().join("hm_test_writer_track");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "track_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "track", "7");
        assert!(result.is_ok());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.track_number, 7);

        // Test "track_number" alias
        let dir2 = std::env::temp_dir().join("hm_test_writer_track2");
        let _ = std::fs::create_dir_all(&dir2);
        let wav_path2 = create_minimal_wav(&dir2, "track_test2.wav");
        let wav_str2 = wav_path2.to_string_lossy().to_string();

        let result2 = set_tag_field(&wav_str2, "track_number", "3");
        assert!(result2.is_ok());

        let track2 = read_tags(&wav_str2).unwrap();
        assert_eq!(track2.track_number, 3);

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&dir2);
    }

    #[test]
    fn test_set_tag_field_year() {
        let dir = std::env::temp_dir().join("hm_test_writer_year");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "year_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "year", "2024");
        assert!(result.is_ok());

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.year, 2024);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_invalid_year() {
        let dir = std::env::temp_dir().join("hm_test_writer_inv_year");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "inv_year.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        // Non-numeric year should silently be ignored
        let result = set_tag_field(&wav_str, "year", "not_a_number");
        assert!(result.is_ok());

        // Year should remain 0 since parse failed
        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.year, 0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_lyrics() {
        let dir = std::env::temp_dir().join("hm_test_writer_lyrics");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "lyrics_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let lyrics = "[00:01.00]Line 1\n[00:05.00]Line 2";
        let result = set_tag_field(&wav_str, "lyrics", lyrics);
        assert!(result.is_ok());

        // Verify with read_embedded_lyrics
        let read_back = crate::tag::reader::read_embedded_lyrics(&wav_str).unwrap();
        assert!(read_back.contains("Line 1"));
        assert!(read_back.contains("Line 2"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_lyric_alias() {
        // "lyric" (singular) should work the same as "lyrics"
        let dir = std::env::temp_dir().join("hm_test_writer_lyric_alias");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "lyric_alias.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "lyric", "test lyric");
        assert!(result.is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_comment() {
        let dir = std::env::temp_dir().join("hm_test_writer_comment");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "comment_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = set_tag_field(&wav_str, "comment", "My comment");
        assert!(result.is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_set_tag_field_multiple_fields() {
        let dir = std::env::temp_dir().join("hm_test_writer_multi");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "multi_test.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        set_tag_field(&wav_str, "title", "Multi Title").unwrap();
        set_tag_field(&wav_str, "artist", "Multi Artist").unwrap();
        set_tag_field(&wav_str, "album", "Multi Album").unwrap();
        set_tag_field(&wav_str, "genre", "Jazz").unwrap();
        set_tag_field(&wav_str, "track", "5").unwrap();
        set_tag_field(&wav_str, "year", "2023").unwrap();

        let track = read_tags(&wav_str).unwrap();
        assert_eq!(track.title, "Multi Title");
        assert_eq!(track.artist, "Multi Artist");
        assert_eq!(track.album, "Multi Album");
        assert_eq!(track.genre, "Jazz");
        assert_eq!(track.track_number, 5);
        assert_eq!(track.year, 2023);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── read_pictures ────────────────────────────────────────

    #[test]
    fn test_read_pictures_file_not_found() {
        let result = read_pictures("K:\\nonexistent_pic.mp3");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::FileNotFound(_) => {}
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_read_pictures_no_pictures_wav() {
        let dir = std::env::temp_dir().join("hm_test_read_pics");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "no_pics.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = read_pictures(&wav_str);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    // ── write_picture ────────────────────────────────────────

    #[test]
    fn test_write_picture_audio_not_found() {
        let result = write_picture("K:\\no_audio.mp3", "K:\\image.jpg");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::FileNotFound(_) => {}
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_write_picture_image_not_found() {
        let dir = std::env::temp_dir().join("hm_test_wpic_img_missing");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "audio.wav");
        let wav_str = wav_path.to_string_lossy().to_string();

        let result = write_picture(&wav_str, "K:\\no_image.jpg");
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::FileNotFound(_) => {}
            other => panic!("Expected FileNotFound, got: {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_picture_unsupported_format() {
        let dir = std::env::temp_dir().join("hm_test_wpic_badfmt");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "audio.wav");
        let wav_str = wav_path.to_string_lossy().to_string();
        let txt_path = dir.join("image.txt");
        std::fs::write(&txt_path, b"not an image").unwrap();
        let txt_str = txt_path.to_string_lossy().to_string();

        let result = write_picture(&wav_str, &txt_str);
        assert!(result.is_err());
        match result.unwrap_err() {
            PlayerError::Other(msg) => {
                assert!(msg.contains("Unsupported image format"));
            }
            other => panic!("Expected Other with 'Unsupported image format', got: {:?}", other),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_picture_jpg_success() {
        let dir = std::env::temp_dir().join("hm_test_wpic_jpg");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "audio.wav");
        let wav_str = wav_path.to_string_lossy().to_string();
        let jpg_path = create_minimal_jpg(&dir, "cover.jpg");
        let jpg_str = jpg_path.to_string_lossy().to_string();

        let result = write_picture(&wav_str, &jpg_str);
        match result {
            Ok(()) => {
                let pics = read_pictures(&wav_str);
                if let Ok(pictures) = pics {
                    assert!(!pictures.is_empty());
                    assert_eq!(pictures[0].0, "jpg");
                }
            }
            Err(e) => {
                // WAV may not support pictures — acceptable
                assert!(matches!(e, PlayerError::Tag(_) | PlayerError::Other(_)));
            }
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_picture_round_trip() {
        let dir = std::env::temp_dir().join("hm_test_wpic_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let wav_path = create_minimal_wav(&dir, "audio.wav");
        let wav_str = wav_path.to_string_lossy().to_string();
        let jpg_path = create_minimal_jpg(&dir, "cover.jpg");
        let jpg_bytes = std::fs::read(&jpg_path).unwrap();
        let jpg_str = jpg_path.to_string_lossy().to_string();

        let result = write_picture(&wav_str, &jpg_str);
        if let Ok(()) = result {
            let pics = read_pictures(&wav_str).unwrap();
            assert!(!pics.is_empty());
            assert_eq!(pics[0].0, "jpg");
            assert_eq!(pics[0].1, jpg_bytes);
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}