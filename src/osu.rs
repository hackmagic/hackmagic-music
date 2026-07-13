use crate::error::{PlayerError, Result};
use std::path::Path;

/// Parsed .osu beatmap metadata
#[derive(Debug, Default)]
pub struct OsuBeatmap {
    pub path: String,
    pub title: String,
    pub title_unicode: String,
    pub artist: String,
    pub artist_unicode: String,
    pub creator: String,
    pub version: String,
    pub audio_file: String,
    pub audio_lead_in: u64,
    pub mode: u32,
    pub bpm_min: f64,
    pub bpm_max: f64,
    pub total_length_ms: u64,
    pub hit_objects: usize,
}

impl OsuBeatmap {
    pub fn display_name(&self) -> String {
        if self.title.is_empty() {
            std::path::Path::new(&self.path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
        } else {
            format!("{} - {} [{}]", self.artist, self.title, self.version)
        }
    }
}

/// Parse a .osu beatmap file
pub fn parse_osu_file(path: &str) -> Result<OsuBeatmap> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(PlayerError::FileNotFound(path.to_string()));
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| PlayerError::Other(format!("Cannot read .osu file: {e}")))?;

    let mut beatmap = OsuBeatmap {
        path: path.to_string(),
        ..Default::default()
    };

    let mut in_hit_objects = false;
    let mut timing_points: Vec<(f64, f64)> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_hit_objects = section == "HitObjects";
            continue;
        }

        if in_hit_objects {
            // Count hit objects (each comma-separated list with at least 5+ elements)
            if line.split(',').count() >= 5 {
                beatmap.hit_objects += 1;
            }
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Title" => beatmap.title = value.to_string(),
                "TitleUnicode" => beatmap.title_unicode = value.to_string(),
                "Artist" => beatmap.artist = value.to_string(),
                "ArtistUnicode" => beatmap.artist_unicode = value.to_string(),
                "Creator" => beatmap.creator = value.to_string(),
                "Version" => beatmap.version = value.to_string(),
                "AudioFilename" => beatmap.audio_file = value.trim_matches('"').to_string(),
                "AudioLeadIn" => beatmap.audio_lead_in = value.parse().unwrap_or(0),
                "Mode" => beatmap.mode = value.parse().unwrap_or(0),
                _ => {}
            }
        }

        // Parse timing points
        if line.contains(',') && !line.starts_with('[') {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                if let Ok(offset) = parts[0].trim().parse::<f64>() {
                    if let Ok(ms_per_beat) = parts[1].trim().parse::<f64>() {
                        if ms_per_beat > 0.0 {
                            timing_points.push((offset, ms_per_beat));
                        }
                    }
                }
            }
        }
    }

    // Calculate BPM range from timing points
    if !timing_points.is_empty() {
        let bpms: Vec<f64> = timing_points
            .iter()
            .map(|&(_, ms)| 60000.0 / ms)
            .filter(|bpm| bpm.is_finite() && *bpm > 0.0)
            .collect();
        if !bpms.is_empty() {
            beatmap.bpm_min = bpms.iter().copied().fold(f64::MAX, f64::min);
            beatmap.bpm_max = bpms.iter().copied().fold(f64::MIN, f64::max);
        }
    }

    Ok(beatmap)
}

/// Search for .osu files in a directory
pub fn search_beatmaps(dir: &str, keyword: Option<&str>) -> Result<Vec<OsuBeatmap>> {
    let p = Path::new(dir);
    if !p.is_dir() {
        return Err(PlayerError::Other(format!("Not a directory: {dir}")));
    }

    let mut results = Vec::new();
    search_dir_recursive(p, &mut results, keyword)?;
    Ok(results)
}

fn search_dir_recursive(
    dir: &Path,
    results: &mut Vec<OsuBeatmap>,
    keyword: Option<&str>,
) -> Result<()> {
    for entry in std::fs::read_dir(dir).map_err(|e| PlayerError::Other(e.to_string()))? {
        let entry = entry.map_err(|e| PlayerError::Other(e.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            search_dir_recursive(&path, results, keyword)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("osu") {
            continue;
        }
        let path_str = path.to_str().unwrap_or_default().to_string();
        if let Ok(beatmap) = parse_osu_file(&path_str) {
            if let Some(kw) = keyword {
                let kw = kw.to_lowercase();
                if beatmap.title.to_lowercase().contains(&kw)
                    || beatmap.artist.to_lowercase().contains(&kw)
                    || beatmap.creator.to_lowercase().contains(&kw)
                    || beatmap.version.to_lowercase().contains(&kw)
                {
                    results.push(beatmap);
                }
            } else {
                results.push(beatmap);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_name_with_title() {
        let bm = OsuBeatmap {
            path: "D:/osu/Songs/12345/beatmap.osu".to_string(),
            title: "Honesty".to_string(),
            artist: "DISH/".to_string(),
            version: "Insane".to_string(),
            ..Default::default()
        };
        assert_eq!(bm.display_name(), "DISH/ - Honesty [Insane]");
    }

    #[test]
    fn test_display_name_empty_title_uses_filename() {
        let bm = OsuBeatmap {
            path: "D:/osu/Songs/12345/beatmap.osu".to_string(),
            title: "".to_string(),
            ..Default::default()
        };
        assert_eq!(bm.display_name(), "beatmap");
    }

    #[test]
    fn test_display_name_empty_title_and_empty_path() {
        let bm = OsuBeatmap {
            path: "".to_string(),
            title: "".to_string(),
            ..Default::default()
        };
        // An empty path has no file_stem → falls back to ""
        assert_eq!(bm.display_name(), "");
    }

    #[test]
    fn test_display_name_empty_title_no_extension() {
        let bm = OsuBeatmap {
            path: "no_ext".to_string(),
            title: "".to_string(),
            ..Default::default()
        };
        assert_eq!(bm.display_name(), "no_ext");
    }

    #[test]
    fn test_display_name_special_chars_in_title() {
        let bm = OsuBeatmap {
            path: "/tmp/beatmap.osu".to_string(),
            title: "CØmplèx Tïtle!".to_string(),
            artist: "Ártïst".to_string(),
            version: "[Hard]".to_string(),
            ..Default::default()
        };
        assert_eq!(bm.display_name(), "Ártïst - CØmplèx Tïtle! [[Hard]]");
    }

    #[test]
    fn test_display_name_empty_artist() {
        let bm = OsuBeatmap {
            path: "/tmp/test.osu".to_string(),
            title: "Test".to_string(),
            artist: "".to_string(),
            version: "Easy".to_string(),
            ..Default::default()
        };
        assert_eq!(bm.display_name(), " - Test [Easy]");
    }
}
