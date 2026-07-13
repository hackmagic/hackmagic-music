use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayerError {
    #[error("BASS error: {0}")]
    BassError(String),

    #[error("BASS not loaded: {0}")]
    BassNotLoaded(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Audio format not supported: {0}")]
    UnsupportedFormat(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Cannot open file: {0}")]
    CannotOpen(String),

    #[error("Playback error: {0}")]
    Playback(String),

    #[error("No track loaded")]
    NoTrack,

    #[error("Invalid index: {0}")]
    InvalidIndex(usize),

    #[error("Playlist is empty")]
    EmptyPlaylist,

    #[error("Config error: {0}")]
    Config(String),

    #[error("Tag error: {0}")]
    Tag(String),

    #[error("Lyric error: {0}")]
    Lyric(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PlayerError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_bass_error_display() {
        let err = PlayerError::BassError("device not found".into());
        assert_eq!(err.to_string(), "BASS error: device not found");
    }

    #[test]
    fn test_bass_not_loaded_display() {
        let err = PlayerError::BassNotLoaded("BASS.dll not found".into());
        assert_eq!(err.to_string(), "BASS not loaded: BASS.dll not found");
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err: PlayerError = io_err.into();
        assert!(err.to_string().contains("IO error:"));
        assert!(err.to_string().contains("file missing"));
    }

    #[test]
    fn test_unsupported_format_display() {
        let err = PlayerError::UnsupportedFormat(".xyz".into());
        assert_eq!(err.to_string(), "Audio format not supported: .xyz");
    }

    #[test]
    fn test_file_not_found_display() {
        let err = PlayerError::FileNotFound("/music/song.flac".into());
        assert_eq!(err.to_string(), "File not found: /music/song.flac");
    }

    #[test]
    fn test_cannot_open_display() {
        let err = PlayerError::CannotOpen("/music/song.mp3".into());
        assert_eq!(err.to_string(), "Cannot open file: /music/song.mp3");
    }

    #[test]
    fn test_playback_error_display() {
        let err = PlayerError::Playback("stream corrupted".into());
        assert_eq!(err.to_string(), "Playback error: stream corrupted");
    }

    #[test]
    fn test_no_track_display() {
        let err = PlayerError::NoTrack;
        assert_eq!(err.to_string(), "No track loaded");
    }

    #[test]
    fn test_invalid_index_display() {
        let err = PlayerError::InvalidIndex(42);
        assert_eq!(err.to_string(), "Invalid index: 42");
    }

    #[test]
    fn test_empty_playlist_display() {
        let err = PlayerError::EmptyPlaylist;
        assert_eq!(err.to_string(), "Playlist is empty");
    }

    #[test]
    fn test_config_error_display() {
        let err = PlayerError::Config("missing field 'theme'".into());
        assert_eq!(err.to_string(), "Config error: missing field 'theme'");
    }

    #[test]
    fn test_tag_error_display() {
        let err = PlayerError::Tag("invalid tag encoding".into());
        assert_eq!(err.to_string(), "Tag error: invalid tag encoding");
    }

    #[test]
    fn test_lyric_error_display() {
        let err = PlayerError::Lyric("lrc parse failed".into());
        assert_eq!(err.to_string(), "Lyric error: lrc parse failed");
    }

    #[test]
    fn test_other_error_display() {
        let err = PlayerError::Other("something went wrong".into());
        assert_eq!(err.to_string(), "something went wrong");
    }

    #[test]
    fn test_other_error_empty_string() {
        let err = PlayerError::Other(String::new());
        assert_eq!(err.to_string(), "");
    }

    #[test]
    fn test_debug_output() {
        let err = PlayerError::InvalidIndex(7);
        let debug = format!("{:?}", err);
        assert!(debug.contains("InvalidIndex"));
        assert!(debug.contains("7"));
    }

    #[test]
    fn test_error_trait() {
        let err = PlayerError::EmptyPlaylist;
        assert!(Error::source(&err).is_none());
    }

    #[test]
    fn test_error_trait_for_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = PlayerError::Io(io_err);
        // The Io variant wraps std::io::Error, so source should return Some
        assert!(Error::source(&err).is_some());
        let source = Error::source(&err).unwrap();
        assert_eq!(source.to_string(), "access denied");
    }

    #[test]
    fn test_result_type_alias() {
        let ok_result: Result<i32> = Ok(42);
        assert!(ok_result.is_ok());
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: Result<i32> = Err(PlayerError::EmptyPlaylist);
        assert!(err_result.is_err());
        assert_eq!(err_result.unwrap_err().to_string(), "Playlist is empty");
    }

    #[test]
    fn test_display_contains_variant_name_for_invalid_index() {
        let err = PlayerError::InvalidIndex(usize::MAX);
        assert!(err.to_string().contains("Invalid index:"));
        assert!(err.to_string().contains(&usize::MAX.to_string()));
    }

    #[test]
    fn test_all_variants_are_different() {
        let variants: Vec<String> = vec![
            PlayerError::BassError("a".into()).to_string(),
            PlayerError::BassNotLoaded("a".into()).to_string(),
            PlayerError::UnsupportedFormat("a".into()).to_string(),
            PlayerError::FileNotFound("a".into()).to_string(),
            PlayerError::CannotOpen("a".into()).to_string(),
            PlayerError::Playback("a".into()).to_string(),
            PlayerError::Config("a".into()).to_string(),
            PlayerError::Tag("a".into()).to_string(),
            PlayerError::Lyric("a".into()).to_string(),
            PlayerError::Other("a".into()).to_string(),
        ];
        // Each variant should produce a distinct display string
        let mut sorted = variants.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), variants.len(), "all display strings should be unique");
    }
}
