//! Playlist file format reader/writer.
//! Supports: .playlist (1028 Music Player native), .m3u, .m3u8

use crate::core::playlist::Track;
use crate::error::Result;
use std::fs;
use std::path::Path;

/// Detect playlist format from file extension
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistFormat {
    M3u,
    M3u8,
    Wpl,
    Ttpl,
    Native, // .playlist (1028 Music Player native format)
}

impl PlaylistFormat {
    pub fn from_path(path: &str) -> Self {
        let lower = path.to_lowercase();
        if lower.ends_with(".m3u8") {
            PlaylistFormat::M3u8
        } else if lower.ends_with(".m3u") {
            PlaylistFormat::M3u
        } else if lower.ends_with(".wpl") {
            PlaylistFormat::Wpl
        } else if lower.ends_with(".ttpl") {
            PlaylistFormat::Ttpl
        } else {
            PlaylistFormat::Native
        }
    }
}

/// Read a playlist file and return file paths
pub fn read_playlist(path: &str) -> Result<Vec<String>> {
    let fmt = PlaylistFormat::from_path(path);
    match fmt {
        PlaylistFormat::M3u | PlaylistFormat::M3u8 => read_m3u(path),
        PlaylistFormat::Wpl => read_wpl(path),
        PlaylistFormat::Ttpl => read_ttpl(path),
        PlaylistFormat::Native => read_native(path),
    }
}

/// Write a playlist file
pub fn write_playlist(path: &str, tracks: &[Track], format: Option<PlaylistFormat>) -> Result<()> {
    let fmt = format.unwrap_or_else(|| PlaylistFormat::from_path(path));
    match fmt {
        PlaylistFormat::M3u | PlaylistFormat::M3u8 => write_m3u(path, tracks),
        PlaylistFormat::Wpl => write_wpl(path, tracks),
        PlaylistFormat::Ttpl => write_ttpl(path, tracks),
        PlaylistFormat::Native => write_native(path, tracks),
    }
}

// === M3U / M3U8 ===

fn read_m3u(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut files = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle relative/absolute paths
        let p = Path::new(line);
        let full_path = if p.is_absolute() {
            p.to_path_buf()
        } else {
            base_dir.join(p)
        };

        if full_path.exists() {
            files.push(full_path.to_string_lossy().to_string());
        }
    }

    Ok(files)
}

fn write_m3u(path: &str, tracks: &[Track]) -> Result<()> {
    let mut content = String::from("#EXTM3U\n");
    for track in tracks {
        let secs = track.duration.as_secs();
        content.push_str(&format!(
            "#EXTINF:{},{}\n{}\n",
            secs,
            track.display_name("artist_title"),
            track.file_path
        ));
    }
    // Ensure parent dir exists
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

// === Native .playlist format (1028 Music Player) ===

fn read_native(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut files = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let p = Path::new(line);
        let full_path = if p.is_absolute() {
            p.to_path_buf()
        } else {
            base_dir.join(p)
        };

        if full_path.exists() {
            files.push(full_path.to_string_lossy().to_string());
        }
    }

    Ok(files)
}

fn write_native(path: &str, tracks: &[Track]) -> Result<()> {
    let mut content = String::from("# 1028 Music Player Playlist\n");
    for track in tracks {
        content.push_str(&track.file_path);
        content.push('\n');
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

/// Export playlist to CSV format
#[cfg(test)]
pub fn write_csv(path: &str, tracks: &[Track]) -> Result<()> {
    let mut content = String::from("file_path,title,artist,album,duration\n");
    for track in tracks {
        content.push_str(&format!(
            "{},{},{},{},{}\n",
            track.file_path,
            escape_csv(&track.title),
            escape_csv(&track.artist),
            escape_csv(&track.album),
            track.duration.as_secs()
        ));
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

#[cfg(test)]
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

// === .wpl (Windows Media Player Playlist) ===
// XML-based format:
// <?wpl version="1.0"?>
// <smil><head><title>Name</title></head><body><seq>
//   <media src="path/to/file.mp3"/>
// </seq></body></smil>

fn read_wpl(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut files = Vec::new();

    // Simple XML parsing: find all media src attributes
    for line in content.lines() {
        // Match: <media src="..."/> or <media src='...'/>
        if let Some(start) = line.find("src=") {
            let after_src = &line[start + 4..];
            let quote = after_src.chars().next().unwrap_or('"');
            if let Some(end) = after_src[1..].find(quote) {
                let value = &after_src[1..=end];
                let p = Path::new(value);
                let full_path = if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    base_dir.join(p)
                };
                if full_path.exists() {
                    files.push(full_path.to_string_lossy().to_string());
                }
            }
        }
    }

    Ok(files)
}

// === .ttpl (千千静听 playlist) ===
// INI-like format:
// [PLAYLIST]
// Version=2
// Count=2
//
// [0]
// File=D:\Music\song1.mp3
// Title=Song Title
// Artist=Artist Name

fn write_wpl(path: &str, tracks: &[Track]) -> Result<()> {
    let mut content = String::from("<?wpl version=\"1.0\"?>\n<smil>\n  <head>\n    <title>1028 Music Player</title>\n  </head>\n  <body>\n    <seq>\n");
    for track in tracks {
        let escaped = track.file_path.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
        content.push_str(&format!("      <media src=\"{escaped}\"/>\n"));
    }
    content.push_str("    </seq>\n  </body>\n</smil>\n");
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn write_ttpl(path: &str, tracks: &[Track]) -> Result<()> {
    let mut content = String::from("[PLAYLIST]\nVersion=2\n");
    content.push_str(&format!("Count={}\n\n", tracks.len()));
    for (i, track) in tracks.iter().enumerate() {
        content.push_str(&format!("[{}]\nFile={}\n", i, track.file_path));
        if !track.title.is_empty() {
            content.push_str(&format!("Title={}\n", track.title));
        }
        if !track.artist.is_empty() {
            content.push_str(&format!("Artist={}\n", track.artist));
        }
        if !track.album.is_empty() {
            content.push_str(&format!("Album={}\n", track.album));
        }
        content.push('\n');
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn read_ttpl(path: &str) -> Result<Vec<String>> {
    let content = fs::read_to_string(path)?;
    let base_dir = Path::new(path).parent().unwrap_or(Path::new("."));
    let mut files = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.to_lowercase().starts_with("file=") {
            let value = &line[5..];
            let p = Path::new(value);
            let full_path = if p.is_absolute() {
                p.to_path_buf()
            } else {
                base_dir.join(p)
            };
            if full_path.exists() {
                files.push(full_path.to_string_lossy().to_string());
            }
        }
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_path_m3u() {
        assert_eq!(PlaylistFormat::from_path("songs.m3u"), PlaylistFormat::M3u);
    }

    #[test]
    fn test_from_path_m3u8() {
        assert_eq!(PlaylistFormat::from_path("songs.m3u8"), PlaylistFormat::M3u8);
    }

    #[test]
    fn test_from_path_wpl() {
        assert_eq!(PlaylistFormat::from_path("playlist.wpl"), PlaylistFormat::Wpl);
    }

    #[test]
    fn test_from_path_ttpl() {
        assert_eq!(PlaylistFormat::from_path("playlist.ttpl"), PlaylistFormat::Ttpl);
    }

    #[test]
    fn test_from_path_native() {
        assert_eq!(PlaylistFormat::from_path("playlist.playlist"), PlaylistFormat::Native);
    }

    #[test]
    fn test_from_path_unknown_extension() {
        assert_eq!(PlaylistFormat::from_path("list.txt"), PlaylistFormat::Native);
    }

    #[test]
    fn test_from_path_case_insensitive() {
        assert_eq!(PlaylistFormat::from_path("SONGS.M3U"), PlaylistFormat::M3u);
        assert_eq!(PlaylistFormat::from_path("PlayList.M3U8"), PlaylistFormat::M3u8);
        assert_eq!(PlaylistFormat::from_path("Playlist.WPL"), PlaylistFormat::Wpl);
        assert_eq!(PlaylistFormat::from_path("Playlist.TTPL"), PlaylistFormat::Ttpl);
        assert_eq!(PlaylistFormat::from_path("Playlist.PLAYLIST"), PlaylistFormat::Native);
    }

    #[test]
    fn test_from_path_with_directory() {
        assert_eq!(
            PlaylistFormat::from_path("C:\\Music\\playlists\\summer.m3u"),
            PlaylistFormat::M3u
        );
        assert_eq!(
            PlaylistFormat::from_path("/home/user/music/playlist.m3u8"),
            PlaylistFormat::M3u8
        );
    }

    #[test]
    fn test_from_path_m3u_not_m3u8() {
        // .m3u must not match before .m3u8
        assert_eq!(PlaylistFormat::from_path("file.m3u"), PlaylistFormat::M3u);
        assert_eq!(PlaylistFormat::from_path("file.m3u8"), PlaylistFormat::M3u8);
        assert_eq!(PlaylistFormat::from_path("file.m3u.backup"), PlaylistFormat::Native);
    }

    #[test]
    fn test_from_path_no_extension() {
        assert_eq!(PlaylistFormat::from_path("playlist"), PlaylistFormat::Native);
    }

    #[test]
    fn test_from_path_empty_string() {
        assert_eq!(PlaylistFormat::from_path(""), PlaylistFormat::Native);
    }

}
