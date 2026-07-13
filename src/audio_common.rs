use std::path::Path;

/// Audio file type enum, matching original `AudioType`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioType {
    Mp3,
    WmaAsf,
    Ogg,
    Mp4,
    Aac,
    Ape,
    Aiff,
    Flac,
    Cue,
    Midi,
    Wav,
    Mpc,
    Dsd,
    Opus,
    Wv,
    Spx,
    Tta,
    Other,
}

/// Get audio type by file extension
pub fn get_audio_type_by_extension(ext: &str) -> AudioType {
    let ext = ext.trim_start_matches('.').to_lowercase();
    match ext.as_str() {
        "mp3" => AudioType::Mp3,
        "wma" | "asf" => AudioType::WmaAsf,
        "ogg" | "oga" => AudioType::Ogg,
        "mp4" | "m4a" | "m4b" | "m4r" | "aac" => {
            if ext == "aac" {
                AudioType::Aac
            } else {
                AudioType::Mp4
            }
        }
        "ape" => AudioType::Ape,
        "aiff" | "aif" => AudioType::Aiff,
        "flac" => AudioType::Flac,
        "cue" => AudioType::Cue,
        "mid" | "midi" | "rmi" => AudioType::Midi,
        "wav" | "wave" => AudioType::Wav,
        "mpc" | "mp+" | "mpp" => AudioType::Mpc,
        "dsf" | "dff" | "dsd" => AudioType::Dsd,
        "opus" => AudioType::Opus,
        "wv" => AudioType::Wv,
        "spx" => AudioType::Spx,
        "tta" => AudioType::Tta,
        _ => AudioType::Other,
    }
}

/// Check if a file is an audio file based on extension
pub fn file_is_audio(path: &str) -> bool {
    let path = Path::new(path);
    path.extension()
        .is_some_and(|ext| !matches!(get_audio_type_by_extension(&ext.to_string_lossy()), AudioType::Other))
}

/// All supported audio extensions
pub fn supported_extensions() -> Vec<&'static str> {
    vec![
        "mp3", "wma", "asf", "ogg", "oga", "mp4", "m4a", "m4b", "aac", "ape", "aiff", "aif",
        "flac", "mid", "midi", "rmi", "wav", "mpc", "mp+", "dsf", "dff", "opus", "wv", "spx",
        "tta",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_audio_type_by_extension_mp3() {
        assert_eq!(get_audio_type_by_extension("mp3"), AudioType::Mp3);
        assert_eq!(get_audio_type_by_extension(".mp3"), AudioType::Mp3);
    }

    #[test]
    fn test_get_audio_type_by_extension_flac() {
        assert_eq!(get_audio_type_by_extension("flac"), AudioType::Flac);
        assert_eq!(get_audio_type_by_extension("FLAC"), AudioType::Flac);
    }

    #[test]
    fn test_get_audio_type_by_extension_wav() {
        assert_eq!(get_audio_type_by_extension("wav"), AudioType::Wav);
        assert_eq!(get_audio_type_by_extension("wave"), AudioType::Wav);
    }

    #[test]
    fn test_get_audio_type_by_extension_ogg() {
        assert_eq!(get_audio_type_by_extension("ogg"), AudioType::Ogg);
        assert_eq!(get_audio_type_by_extension("oga"), AudioType::Ogg);
    }

    #[test]
    fn test_get_audio_type_by_extension_mp4_family() {
        assert_eq!(get_audio_type_by_extension("mp4"), AudioType::Mp4);
        assert_eq!(get_audio_type_by_extension("m4a"), AudioType::Mp4);
        assert_eq!(get_audio_type_by_extension("aac"), AudioType::Aac);
    }

    #[test]
    fn test_get_audio_type_by_extension_midi() {
        assert_eq!(get_audio_type_by_extension("mid"), AudioType::Midi);
        assert_eq!(get_audio_type_by_extension("midi"), AudioType::Midi);
        assert_eq!(get_audio_type_by_extension("rmi"), AudioType::Midi);
    }

    #[test]
    fn test_get_audio_type_by_extension_case_insensitive() {
        assert_eq!(get_audio_type_by_extension("MP3"), AudioType::Mp3);
        assert_eq!(get_audio_type_by_extension(".OGG"), AudioType::Ogg);
        assert_eq!(get_audio_type_by_extension("Flac"), AudioType::Flac);
    }

    #[test]
    fn test_get_audio_type_by_extension_other() {
        assert_eq!(get_audio_type_by_extension("txt"), AudioType::Other);
        assert_eq!(get_audio_type_by_extension("png"), AudioType::Other);
        assert_eq!(get_audio_type_by_extension(""), AudioType::Other);
    }

    #[test]
    fn test_supported_extensions_contains_common_formats() {
        let exts = supported_extensions();
        assert!(exts.contains(&"mp3"), "should contain mp3");
        assert!(exts.contains(&"flac"), "should contain flac");
        assert!(exts.contains(&"wav"), "should contain wav");
        assert!(exts.contains(&"ogg"), "should contain ogg");
        assert!(exts.contains(&"opus"), "should contain opus");
        assert!(exts.contains(&"aac"), "should contain aac");
        assert!(exts.contains(&"ape"), "should contain ape");
    }

    #[test]
    fn test_supported_extensions_no_duplicates() {
        let exts = supported_extensions();
        let mut sorted = exts.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(exts.len(), sorted.len(), "extensions should not contain duplicates");
    }

    #[test]
    fn test_supported_extensions_all_recognized() {
        let exts = supported_extensions();
        for ext in &exts {
            let at = get_audio_type_by_extension(ext);
            assert_ne!(at, AudioType::Other, "extension '{}' should map to a known audio type", ext);
        }
    }

    #[test]
    fn test_supported_extensions_at_least_15() {
        let exts = supported_extensions();
        assert!(
            exts.len() >= 15,
            "expected at least 15 supported extensions, got {}",
            exts.len()
        );
    }

    #[test]
    fn test_file_is_audio_known_extensions() {
        assert!(file_is_audio("song.mp3"));
        assert!(file_is_audio("track.flac"));
        assert!(file_is_audio("audio.wav"));
        assert!(file_is_audio("music.ogg"));
        assert!(file_is_audio("file.m4a"));
    }

    #[test]
    fn test_file_is_audio_non_audio() {
        assert!(!file_is_audio("readme.txt"));
        assert!(!file_is_audio("image.png"));
        assert!(!file_is_audio("script.js"));
        assert!(!file_is_audio("no_extension"));
        assert!(!file_is_audio(""));
    }

    #[test]
    fn test_file_is_audio_case_insensitive() {
        assert!(file_is_audio("song.MP3"));
        assert!(file_is_audio("track.FLAC"));
        assert!(file_is_audio("audio.WAV"));
    }
}
