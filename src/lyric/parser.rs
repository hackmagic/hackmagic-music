//! LRC/KSC/WebVTT lyrics parser with encoding auto-detection.
//! Translated from the original `CLyrics` C++ implementation.

#![allow(dead_code)]

use crate::charset::decode_to_string;
use crate::error::Result;
use regex::Regex;
use std::collections::HashMap;
use std::fs;

/// A single lyric line
#[derive(Debug, Clone)]
pub struct LyricLine {
    pub time_ms: u64,     // Time in milliseconds
    pub text: String,     // Lyric text
    pub translate: String, // Translation text (from [t:] tags)
    pub ksc_words: Vec<KscWord>, // Karaoke words (KSC format)
}

/// A word in karaoke-style lyrics
#[derive(Debug, Clone)]
pub struct KscWord {
    pub start_ms: u64,
    pub duration_ms: u64,
    pub text: String,
}

/// Translation display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslateMode {
    /// Show translation on a separate line below original
    Separate,
    /// Show translation inline after original (e.g., "Hello / 你好")
    Inline,
    /// Hide translation
    Hidden,
}

/// Parsed lyrics container
#[derive(Debug, Clone)]
pub struct Lyrics {
    pub lines: Vec<LyricLine>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub offset_ms: i64,   // Global offset
    pub translate_mode: TranslateMode,
}

impl Lyrics {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            offset_ms: 0,
            translate_mode: TranslateMode::Separate,
        }
    }

    /// Apply offset to all line times.
    pub fn apply_offset(&mut self) {
        if self.offset_ms == 0 {
            return;
        }
        let offset = self.offset_ms;
        for line in &mut self.lines {
            line.time_ms = if offset >= 0 {
                line.time_ms.saturating_add(offset as u64)
            } else {
                line.time_ms.saturating_sub((-offset) as u64)
            };
            for word in &mut line.ksc_words {
                word.start_ms = if offset >= 0 {
                    word.start_ms.saturating_add(offset as u64)
                } else {
                    word.start_ms.saturating_sub((-offset) as u64)
                };
            }
        }
    }

    /// Get the lyric line at the given time
    pub fn line_at(&self, time_ms: u64) -> Option<&LyricLine> {
        // Find the last line whose time <= given time
        self.lines.iter().rev().find(|line| line.time_ms <= time_ms)
    }

    /// Get the next line index (for highlighting)
    pub fn next_line_index(&self, time_ms: u64) -> Option<usize> {
        self.lines.iter().position(|line| line.time_ms > time_ms)
    }

    /// Current line index
    pub fn current_line_index(&self, time_ms: u64) -> Option<usize> {
        if self.lines.is_empty() {
            return None;
        }
        match self.next_line_index(time_ms) {
            Some(next) if next > 0 => Some(next - 1),
            Some(_) => Some(0),
            None => Some(self.lines.len() - 1), // time past all lines, return last
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Get the effective text for display (text + optional inline translation).
    pub fn display_text(&self, line: &LyricLine) -> String {
        match self.translate_mode {
            TranslateMode::Separate => line.text.clone(),
            TranslateMode::Hidden => line.text.clone(),
            TranslateMode::Inline => {
                if line.translate.is_empty() {
                    line.text.clone()
                } else {
                    format!("{} / {}", line.text, line.translate)
                }
            }
        }
    }

    /// Calculate karaoke progress (0-1000) for a given time position.
    /// Returns progress within the current word or line.
    /// 0 = start of current segment, 1000 = end of current segment.
    pub fn karaoke_progress(&self, time_ms: u64) -> u32 {
        let line_idx = match self.current_line_index(time_ms) {
            Some(i) => i,
            None => return 0,
        };

        let line = &self.lines[line_idx];

        // If this line has KSC word-level timing, use that
        if !line.ksc_words.is_empty() {
            let relative = time_ms.saturating_sub(line.time_ms);
            for word in &line.ksc_words {
                let word_end = word.start_ms + word.duration_ms;
                if relative >= word.start_ms && relative < word_end {
                    if word.duration_ms == 0 {
                        return 1000;
                    }
                    let progress = (relative - word.start_ms) * 1000 / word.duration_ms;
                    return progress.min(1000) as u32;
                }
                if relative < word.start_ms {
                    return 0;
                }
            }
            // Past all words
            return 1000;
        }

        // Line-level progress: time between current line and next line
        let next_time = if line_idx + 1 < self.lines.len() {
            self.lines[line_idx + 1].time_ms
        } else {
            line.time_ms + 5000
        };

        let line_duration = next_time.saturating_sub(line.time_ms);
        if line_duration == 0 {
            return 0;
        }

        let elapsed = time_ms.saturating_sub(line.time_ms);
        let progress = elapsed * 1000 / line_duration;
        progress.min(1000) as u32
    }

    /// Merge lines that have the same timestamp into a single entry.
    /// When two or more consecutive lines share the same time_ms,
    /// their texts are joined with newline characters.
    pub fn merge_same_timestamp(&mut self) {
        if self.lines.len() < 2 {
            return;
        }

        let mut merged: Vec<LyricLine> = Vec::new();
        let mut iter = self.lines.iter().cloned();

        if let Some(mut current) = iter.next() {
            for next in iter {
                if next.time_ms == current.time_ms {
                    // Merge: append next's text as continuation
                    if !current.translate.is_empty() || !next.translate.is_empty() {
                        // Merge translations too
                        if current.translate.is_empty() {
                            current.translate = next.translate;
                        } else if !next.translate.is_empty() {
                            current.translate.push('\n');
                            current.translate.push_str(&next.translate);
                        }
                    }
                    // Append text
                    current.text.push('\n');
                    current.text.push_str(&next.text);
                } else {
                    merged.push(current);
                    current = next;
                }
            }
            merged.push(current);
            self.lines = merged;
        }
    }

    /// Sort lines by time (ascending).
    pub fn sort_by_time(&mut self) {
        self.lines.sort_by(|a, b| a.time_ms.cmp(&b.time_ms));
    }

    /// Extract existing translation from brackets in lyric text.
    /// Example: "Hello(你好)" → text: "Hello", translate: "你好"
    pub fn extract_bracket_translations(&mut self) {
        let bracket_re = Regex::new(r"^(.+?)\s*[（(](.+?)[）)]\s*$").unwrap();
        for line in &mut self.lines {
            if line.translate.is_empty() {
                if let Some(caps) = bracket_re.captures(&line.text) {
                    let main_text = caps.get(1).unwrap().as_str().trim().to_string();
                    let trans_text = caps.get(2).unwrap().as_str().trim().to_string();
                    if !main_text.is_empty() && !trans_text.is_empty() {
                        line.text = main_text;
                        line.translate = trans_text;
                    }
                }
            }
        }
    }
}

/// Parse an LRC file content
#[inline]
pub fn parse_lrc(content: &str) -> Lyrics {
    parse_lrc_with_mode(content, TranslateMode::Separate)
}

/// Parse LRC content with a specified translation display mode.
pub fn parse_lrc_with_mode(content: &str, translate_mode: TranslateMode) -> Lyrics {
    let mut lyrics = Lyrics {
        translate_mode,
        ..Lyrics::new()
    };
    let tag_re = Regex::new(r"^\[([a-zA-Z]+):(.+)\]$").unwrap();
    let time_re = Regex::new(r"\[(\d{2}):(\d{2})\.(\d{2,3})\]").unwrap();
    let multi_time_re = Regex::new(r"(\[\d{2}:\d{2}\.\d{2,3}\])").unwrap();
    let trans_re = Regex::new(r"^\[t:(\d{2}):(\d{2})\.(\d{2,3})\](.*)$").unwrap();
    let mut translations: HashMap<u64, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Check for [t:MM:SS.xxx] translation lines
        if let Some(caps) = trans_re.captures(line) {
            let min: u64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let sec: u64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
            let ms_str = caps.get(3).unwrap().as_str();
            let ms: u64 = if ms_str.len() == 2 {
                ms_str.parse::<u64>().unwrap_or(0) * 10
            } else {
                ms_str.parse().unwrap_or(0)
            };
            let time = min * 60_000 + sec * 1_000 + ms;
            let trans_text = caps.get(4).map(|m| m.as_str().trim().to_string()).unwrap_or_default();
            if !trans_text.is_empty() {
                // Prefer first translation if multiple
                translations.entry(time).or_insert(trans_text);
            }
            continue;
        }

        // Check for metadata tags
        if let Some(caps) = tag_re.captures(line) {
            let key = caps.get(1).unwrap().as_str().to_lowercase();
            let value = caps.get(2).unwrap().as_str().to_string();
            match key.as_str() {
                "ti" | "title" => lyrics.title = value,
                "ar" | "artist" => lyrics.artist = value,
                "al" | "album" => lyrics.album = value,
                "offset" => {
                    if let Ok(off) = value.parse::<i64>() {
                        lyrics.offset_ms = off;
                    }
                }
                _ => {}
            }
            continue;
        }

        // Parse time-stamped lyric lines
        // Handle multiple timestamps: [00:01.00][00:05.00]Lyric text
        let parts: Vec<&str> = multi_time_re.split(line).filter(|s| !s.is_empty()).collect();
        let timestamps: Vec<&str> = time_re.find_iter(line).map(|m| m.as_str()).collect();

        if timestamps.is_empty() {
            continue;
        }

        // Extract text (everything after the last timestamp)
        let text = if let Some(part) = parts.last() {
            part.to_string()
        } else {
            String::new()
        };

        for ts in &timestamps {
            if let Some(caps) = time_re.captures(ts) {
                let min: u64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
                let sec: u64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
                let ms_str = caps.get(3).unwrap().as_str();
                let ms: u64 = if ms_str.len() == 2 {
                    ms_str.parse::<u64>().unwrap_or(0) * 10
                } else {
                    ms_str.parse().unwrap_or(0)
                };
                let time = min * 60_000 + sec * 1_000 + ms;

                lyrics.lines.push(LyricLine {
                    time_ms: time,
                    text: text.clone(),
                    translate: String::new(),
                    ksc_words: Vec::new(),
                });
            }
        }
    }

    // Sort by time
    lyrics.sort_by_time();

    // Merge translations into corresponding lines
    for line in &mut lyrics.lines {
        if let Some(trans) = translations.remove(&line.time_ms) {
            line.translate = trans;
        }
    }

    // Apply offset
    lyrics.apply_offset();

    lyrics
}

/// Parse a KSC (karaoke) lyric file
#[inline]
pub fn parse_ksc(content: &str) -> Lyrics {
    let mut lyrics = Lyrics::new();
    let line_re = Regex::new(r"^\[(\d+),(\d+)\]").unwrap();
    let tag_re = Regex::new(r"^#?(\w+):(.+)$").unwrap();
    let word_re = Regex::new(r"([^\(]+)\((\d+),(\d+)\)").unwrap();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Tag metadata
        if let Some(caps) = tag_re.captures(line) {
            let key = caps.get(1).unwrap().as_str().to_lowercase();
            let value = caps.get(2).unwrap().as_str().to_string();
            match key.as_str() {
                "title" | "ti" => lyrics.title = value,
                "artist" | "ar" => lyrics.artist = value,
                "album" | "al" => lyrics.album = value,
                "offset" => {
                    if let Ok(off) = value.parse() {
                        lyrics.offset_ms = off;
                    }
                }
                _ => {}
            }
            continue;
        }

        // KSC line: [start_ms,duration]word1(word1_dur,word1_start)word2(word2_dur,word2_start)...
        if let Some(caps) = line_re.captures(line) {
            let start_ms: u64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let _duration_ms: u64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
            let rest = &line[caps.get(0).unwrap().end()..];

            let mut text = String::new();
            let mut words = Vec::new();

            // Parse word segments: text(duration,start)
            for word_cap in word_re.captures_iter(rest) {
                let word_text = word_cap.get(1).unwrap().as_str().to_string();
                let word_dur: u64 = word_cap.get(2).unwrap().as_str().parse().unwrap_or(0);
                let word_start: u64 = word_cap.get(3).unwrap().as_str().parse().unwrap_or(0);
                text += &word_text;
                words.push(KscWord {
                    start_ms: word_start,
                    duration_ms: word_dur,
                    text: word_text,
                });
            }

            if text.is_empty() {
                text = rest.to_string();
            }

            lyrics.lines.push(LyricLine {
                time_ms: start_ms,
                text,
                translate: String::new(),
                ksc_words: words,
            });
        }
    }

    lyrics.sort_by_time();
    lyrics.apply_offset();
    lyrics
}

/// Parse an LRC string directly
pub fn parse_lrc_str(content: &str) -> Result<Lyrics> {
    Ok(parse_lrc(content))
}

/// Parse a `WebVTT` (.vtt) subtitle file for lyrics
///
/// Format:
/// ```text
/// WEBVTT
///
/// 00:01.500 --> 00:04.000
/// First lyric line
///
/// 00:05.000 --> 00:09.500
/// Second lyric line
/// ```
pub fn parse_vtt(content: &str) -> Lyrics {
    let mut lyrics = Lyrics::new();
    let cue_re = Regex::new(
        r"(\d{2}):(\d{2})\.(\d{3})\s*-->\s*(\d{2}):(\d{2})\.(\d{3})"
    ).unwrap();

    let mut current_time: Option<u64> = None;
    let mut current_text = String::new();
    let mut in_cue = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip header and notes
        if line == "WEBVTT" || line.starts_with("NOTE") {
            in_cue = false;
            continue;
        }

        // Check for cue timing
        if let Some(caps) = cue_re.captures(line) {
            // Save previous cue if exists
            if in_cue && !current_text.is_empty() {
                if let Some(time) = current_time {
                    lyrics.lines.push(LyricLine {
                        time_ms: time,
                        text: current_text.trim().to_string(),
                        translate: String::new(),
                        ksc_words: Vec::new(),
                    });
                }
            }

            let m: u64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);  // minutes
            let s: u64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);  // seconds
            let ms: u64 = caps.get(3).unwrap().as_str().parse().unwrap_or(0); // milliseconds
            current_time = Some(m * 60_000 + s * 1_000 + ms);
            current_text.clear();
            in_cue = true;
            continue;
        }

        if in_cue {
            if line.is_empty() {
                // End of cue
                if !current_text.is_empty() {
                    if let Some(time) = current_time {
                        lyrics.lines.push(LyricLine {
                            time_ms: time,
                            text: current_text.trim().to_string(),
                            translate: String::new(),
                            ksc_words: Vec::new(),
                        });
                    }
                }
                current_text.clear();
                in_cue = false;
            } else {
                if !current_text.is_empty() {
                    current_text.push(' ');
                }
                current_text.push_str(line);
            }
        }
    }

    // Flush last cue
    if in_cue && !current_text.is_empty() {
        if let Some(time) = current_time {
            lyrics.lines.push(LyricLine {
                time_ms: time,
                text: current_text.trim().to_string(),
                translate: String::new(),
                ksc_words: Vec::new(),
            });
        }
    }

    lyrics.sort_by_time();
    lyrics.apply_offset();
    lyrics
}

/// Parse Netease encrypted LRC format.
/// Netease encrypts lyrics by XOR-ing each byte with a repeating key.
/// The key is: "neteasecloudmusic"
/// Format: [id:$tag]$`encrypted_content`
fn netease_decrypt(content: &str) -> Result<String> {
    let key = b"neteasecloudmusic";
    let key_len = key.len();

    // Strip [id:] prefix if present
    let body = if let Some(stripped) = content.strip_prefix("[id:") {
        if let Some(end) = stripped.find(']') {
            &stripped[end + 1..]
        } else {
            content
        }
    } else {
        content
    };

    // Decode URL-encoded bytes if needed, then XOR
    let raw_bytes = match urlencoding_or_raw(body) {
        Ok(bytes) => bytes,
        Err(()) => body.as_bytes().to_vec(),
    };

    let decrypted: Vec<u8> = raw_bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ key[i % key_len])
        .collect();

    let mut result = String::from_utf8_lossy(&decrypted).to_string();

    // Strip any leading/trailing garbage
    if let Some(lrc_start) = result.find("[ti:") {
        result = result[lrc_start..].to_string();
    } else if let Some(lrc_start) = result.find("[00:") {
        result = result[lrc_start..].to_string();
    }

    Ok(result)
}

fn urlencoding_or_raw(input: &str) -> std::result::Result<Vec<u8>, ()> {
    // Try to interpret as percent-encoded bytes
    let mut bytes = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '%' && i + 2 < chars.len() {
            let hex = &input[i + 1..i + 3];
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                bytes.push(byte);
                i += 3;
                continue;
            }
        }
        bytes.push(chars[i] as u8);
        i += 1;
    }
    Ok(bytes)
}

/// Detect Netease encrypted LRC by looking for `[id:` prefix and absence of `[ti:`.
fn is_netease_encrypted(content: &[u8]) -> bool {
    // Check for [id: in first 32 bytes
    let header = String::from_utf8_lossy(&content[..content.len().min(32)]);
    header.contains("[id:") && !header.contains("[ti:") && !header.contains("[00:")
}

/// Load and parse a lyric file (auto-detect format and encoding).
///
/// Supports UTF-8, UTF-8-BOM, UTF-16LE, UTF-16BE, GBK, Shift-JIS encodings.
pub fn load_lyric_file(file_path: &str) -> Result<Lyrics> {
    let raw_bytes = fs::read(file_path)?;

    // Decode with encoding auto-detection
    let content = decode_to_string(&raw_bytes);

    let lower = file_path.to_lowercase();
    if lower.ends_with(".ksc") || lower.ends_with(".kscproj") {
        Ok(parse_ksc(&content))
    } else if lower.ends_with(".vtt") {
        Ok(parse_vtt(&content))
    } else if lower.ends_with(".lrc") && is_netease_encrypted(&raw_bytes) {
        // Try decrypting as Netease encrypted LRC
        match netease_decrypt(&content) {
            Ok(decrypted) => Ok(parse_lrc(&decrypted)),
            Err(_) => Ok(parse_lrc(&content)),
        }
    } else {
        Ok(parse_lrc(&content))
    }
}

/// Convert lyrics to Traditional Chinese.
pub fn lyrics_to_traditional(lyrics: &Lyrics) -> Lyrics {
    use crate::charset::to_traditional_chinese;
    let mut result = lyrics.clone();
    result.title = to_traditional_chinese(&result.title);
    result.artist = to_traditional_chinese(&result.artist);
    result.album = to_traditional_chinese(&result.album);
    for line in &mut result.lines {
        line.text = to_traditional_chinese(&line.text);
        line.translate = to_traditional_chinese(&line.translate);
        for word in &mut line.ksc_words {
            word.text = to_traditional_chinese(&word.text);
        }
    }
    result
}

/// Convert lyrics to Simplified Chinese.
pub fn lyrics_to_simplified(lyrics: &Lyrics) -> Lyrics {
    use crate::charset::to_simplified_chinese;
    let mut result = lyrics.clone();
    result.title = to_simplified_chinese(&result.title);
    result.artist = to_simplified_chinese(&result.artist);
    result.album = to_simplified_chinese(&result.album);
    for line in &mut result.lines {
        line.text = to_simplified_chinese(&line.text);
        line.translate = to_simplified_chinese(&line.translate);
        for word in &mut line.ksc_words {
            word.text = to_simplified_chinese(&word.text);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lrc_basic() {
        let content = "[00:01.00]Line one\n[00:02.50]Line two\n[00:04.00]Line three";
        let result = parse_lrc(content);
        assert_eq!(result.lines.len(), 3);
        assert_eq!(result.lines[0].text, "Line one");
        assert_eq!(result.lines[0].time_ms, 1000);
        assert_eq!(result.lines[1].text, "Line two");
        assert_eq!(result.lines[1].time_ms, 2500);
        assert_eq!(result.lines[2].text, "Line three");
        assert_eq!(result.lines[2].time_ms, 4000);
    }

    #[test]
    fn test_parse_lrc_metadata() {
        let content = "[ti:Test Song]\n[ar:Test Artist]\n[al:Test Album]\n[offset:-500]\n[00:01.00]Hello";
        let result = parse_lrc(content);
        assert_eq!(result.title, "Test Song");
        assert_eq!(result.artist, "Test Artist");
        assert_eq!(result.album, "Test Album");
        // offset -500 applied: 1000 - 500 = 500
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].text, "Hello");
        assert_eq!(result.lines[0].time_ms, 500);
    }

    #[test]
    fn test_parse_lrc_multiple_timestamps() {
        let content = "[00:01.00][00:05.00][00:10.00]Repeated line";
        let result = parse_lrc(content);
        assert_eq!(result.lines.len(), 3);
        assert_eq!(result.lines[0].time_ms, 1000);
        assert_eq!(result.lines[1].time_ms, 5000);
        assert_eq!(result.lines[2].time_ms, 10000);
        assert_eq!(result.lines[0].text, "Repeated line");
        assert_eq!(result.lines[1].text, "Repeated line");
        assert_eq!(result.lines[2].text, "Repeated line");
    }

    #[test]
    fn test_parse_lrc_two_digit_ms() {
        let content = "[00:01.50]Two digit ms\n[00:02.99]Ninety nine";
        let result = parse_lrc(content);
        assert_eq!(result.lines[0].time_ms, 1500);
        assert_eq!(result.lines[1].time_ms, 2990);
    }

    #[test]
    fn test_parse_lrc_three_digit_ms() {
        let content = "[00:01.500]Three digit ms\n[00:02.999]Nine nine nine";
        let result = parse_lrc(content);
        assert_eq!(result.lines[0].time_ms, 1500);
        assert_eq!(result.lines[1].time_ms, 2999);
    }

    #[test]
    fn test_parse_lrc_translation_lines() {
        let content = "[00:01.00]Hello\n[t:00:01.00]你好\n[00:02.00]World\n[t:00:02.00]世界";
        let result = parse_lrc(content);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0].text, "Hello");
        assert_eq!(result.lines[0].translate, "你好");
        assert_eq!(result.lines[1].text, "World");
        assert_eq!(result.lines[1].translate, "世界");
    }

    #[test]
    fn test_parse_lrc_empty_content() {
        let result = parse_lrc("");
        assert_eq!(result.lines.len(), 0);
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_lrc_skip_non_timestamp_lines() {
        let content = "This is a comment\n[00:01.00]Hello\nAnother comment\n[00:02.00]World";
        let result = parse_lrc(content);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0].text, "Hello");
        assert_eq!(result.lines[1].text, "World");
    }

    #[test]
    fn test_parse_lrc_sort_by_time() {
        let content = "[00:03.00]Third\n[00:01.00]First\n[00:02.00]Second";
        let result = parse_lrc(content);
        assert_eq!(result.lines.len(), 3);
        assert_eq!(result.lines[0].text, "First");
        assert_eq!(result.lines[0].time_ms, 1000);
        assert_eq!(result.lines[1].text, "Second");
        assert_eq!(result.lines[1].time_ms, 2000);
        assert_eq!(result.lines[2].text, "Third");
        assert_eq!(result.lines[2].time_ms, 3000);
    }

    #[test]
    fn test_parse_lrc_line_at() {
        let content = "[00:01.00]First\n[00:05.00]Second\n[00:10.00]Third";
        let result = parse_lrc(content);
        assert_eq!(result.line_at(0).map(|l| l.text.as_str()), None);
        assert_eq!(result.line_at(1000).map(|l| l.text.as_str()), Some("First"));
        assert_eq!(result.line_at(3000).map(|l| l.text.as_str()), Some("First"));
        assert_eq!(result.line_at(5000).map(|l| l.text.as_str()), Some("Second"));
        assert_eq!(result.line_at(20000).map(|l| l.text.as_str()), Some("Third"));
    }

    #[test]
    fn test_parse_lrc_next_line_index() {
        let content = "[00:01.00]First\n[00:05.00]Second\n[00:10.00]Third";
        let result = parse_lrc(content);
        assert_eq!(result.next_line_index(0), Some(0));
        assert_eq!(result.next_line_index(1000), Some(1));
        assert_eq!(result.next_line_index(3000), Some(1));
        assert_eq!(result.next_line_index(5000), Some(2));
        assert_eq!(result.next_line_index(10000), None);
    }

    #[test]
    fn test_parse_lrc_current_line_index() {
        let content = "[00:01.00]First\n[00:05.00]Second\n[00:10.00]Third";
        let result = parse_lrc(content);
        assert_eq!(result.current_line_index(0), Some(0));
        assert_eq!(result.current_line_index(1000), Some(0));
        assert_eq!(result.current_line_index(3000), Some(0));
        assert_eq!(result.current_line_index(5000), Some(1));
        assert_eq!(result.current_line_index(20000), Some(2));
    }

    #[test]
    fn test_parse_ksc_basic() {
        let content = "[0,5000]Hello(1000,0)World(2000,1000)";
        let result = parse_ksc(content);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].time_ms, 0);
        assert_eq!(result.lines[0].text, "HelloWorld");
        assert_eq!(result.lines[0].ksc_words.len(), 2);
        assert_eq!(result.lines[0].ksc_words[0].text, "Hello");
        assert_eq!(result.lines[0].ksc_words[0].duration_ms, 1000);
        assert_eq!(result.lines[0].ksc_words[0].start_ms, 0);
        assert_eq!(result.lines[0].ksc_words[1].text, "World");
        assert_eq!(result.lines[0].ksc_words[1].duration_ms, 2000);
        assert_eq!(result.lines[0].ksc_words[1].start_ms, 1000);
    }

    #[test]
    fn test_parse_ksc_metadata() {
        let content = "title:Hello KSC\nartist:Test Artist\nalbum:Karaoke Album\noffset:-300\n[0,5000]Line one";
        let result = parse_ksc(content);
        assert_eq!(result.title, "Hello KSC");
        assert_eq!(result.artist, "Test Artist");
        assert_eq!(result.album, "Karaoke Album");
        // offset applied: 0 - 300 saturating = 0
        assert_eq!(result.lines.len(), 1);
    }

    #[test]
    fn test_parse_ksc_metadata_with_hash() {
        let content = "#title:Hash Title\n#artist:Hash Artist\n[0,2000]Content line";
        let result = parse_ksc(content);
        assert_eq!(result.title, "Hash Title");
        assert_eq!(result.artist, "Hash Artist");
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].text, "Content line");
    }

    #[test]
    fn test_parse_ksc_empty_content() {
        let result = parse_ksc("");
        assert_eq!(result.lines.len(), 0);
        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_ksc_sort_by_time() {
        let content = "[3000,2000]Third line\n[1000,2000]First line\n[2000,2000]Second line";
        let result = parse_ksc(content);
        assert_eq!(result.lines.len(), 3);
        assert_eq!(result.lines[0].time_ms, 1000);
        assert_eq!(result.lines[0].text, "First line");
        assert_eq!(result.lines[1].time_ms, 2000);
        assert_eq!(result.lines[1].text, "Second line");
        assert_eq!(result.lines[2].time_ms, 3000);
        assert_eq!(result.lines[2].text, "Third line");
    }

    #[test]
    fn test_parse_ksc_text_without_words() {
        let content = "[500,3000]Plain text line without word timing";
        let result = parse_ksc(content);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].time_ms, 500);
        assert_eq!(result.lines[0].text, "Plain text line without word timing");
        assert!(result.lines[0].ksc_words.is_empty());
    }

    #[test]
    fn test_parse_ksc_multiple_lines() {
        let content = "\
[0,2000]Word1(500,0)Word2(500,500)
[3000,3000]Next(1000,0)line(1000,1000)here(1000,2000)";
        let result = parse_ksc(content);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0].text, "Word1Word2");
        assert_eq!(result.lines[0].ksc_words.len(), 2);
        assert_eq!(result.lines[1].text, "Nextlinehere");
        assert_eq!(result.lines[1].ksc_words.len(), 3);
    }

    #[test]
    fn test_parse_vtt_basic() {
        let content = "WEBVTT\n\n00:01.500 --> 00:04.000\nFirst lyric\n\n00:05.000 --> 00:09.500\nSecond lyric line";
        let result = parse_vtt(content);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0].time_ms, 1500);
        assert_eq!(result.lines[0].text, "First lyric");
        assert_eq!(result.lines[1].time_ms, 5000);
        assert_eq!(result.lines[1].text, "Second lyric line");
    }

    #[test]
    fn test_parse_vtt_empty() {
        let result = parse_vtt("");
        assert_eq!(result.lines.len(), 0);
    }

    #[test]
    fn test_parse_vtt_header_only() {
        let result = parse_vtt("WEBVTT");
        assert_eq!(result.lines.len(), 0);
    }

    #[test]
    fn test_parse_vtt_multiline_cue() {
        let content = "WEBVTT\n\n00:01.000 --> 00:03.000\nLine one\ncontinuation";
        let result = parse_vtt(content);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0].text, "Line one continuation");
    }

    #[test]
    fn test_karaoke_progress_no_lines() {
        let lyrics = Lyrics::new();
        assert_eq!(lyrics.karaoke_progress(0), 0);
        assert_eq!(lyrics.karaoke_progress(1000), 0);
    }

    #[test]
    fn test_karaoke_progress_line_level() {
        let content = "[00:00.00]First line\n[00:05.00]Second line";
        let lyrics = parse_lrc(content);
        assert_eq!(lyrics.karaoke_progress(0), 0);
        assert_eq!(lyrics.karaoke_progress(2500), 500);
        assert_eq!(lyrics.karaoke_progress(5000), 0);
        assert_eq!(lyrics.karaoke_progress(20000), 1000);
    }

    #[test]
    fn test_karaoke_progress_word_level() {
        let content = "[0,1000]音(500,0)楽(500,500)";
        let lyrics = parse_ksc(content);
        assert_eq!(lyrics.karaoke_progress(0), 0);
        assert_eq!(lyrics.karaoke_progress(250), 500);
        assert_eq!(lyrics.karaoke_progress(499), 998);
        assert_eq!(lyrics.karaoke_progress(500), 0);
        assert_eq!(lyrics.karaoke_progress(750), 500);
        assert_eq!(lyrics.karaoke_progress(1200), 1000);
    }

    #[test]
    fn test_netease_decrypt_simple() {
        let plaintext = "[00:01.00]Hello";
        let key = b"neteasecloudmusic";
        let encrypted: Vec<u8> = plaintext.bytes()
            .enumerate()
            .map(|(i, b)| b ^ key[i % key.len()])
            .collect();
        let content = format!("[id:0]{}", String::from_utf8_lossy(&encrypted));
        let result = netease_decrypt(&content).unwrap();
        assert!(result.contains("Hello") || result.contains("[00:"));
    }

    #[test]
    fn test_netease_decrypt_bad_format() {
        let content = "[ti:Normal]\n[00:01.00]Normal LRC";
        let result = netease_decrypt(content).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_parse_lrc_str_wrapper() {
        let content = "[00:01.00]Line one\n[00:02.00]Line two";
        let result = parse_lrc_str(content).unwrap();
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0].text, "Line one");
        assert_eq!(result.lines[1].text, "Line two");
    }

    #[test]
    fn test_lyrics_new_creates_empty() {
        let lyrics = Lyrics::new();
        assert!(lyrics.is_empty());
        assert_eq!(lyrics.len(), 0);
        assert!(lyrics.title.is_empty());
        assert!(lyrics.artist.is_empty());
        assert!(lyrics.album.is_empty());
        assert_eq!(lyrics.offset_ms, 0);
    }

    #[test]
    fn test_line_at_before_first() {
        let content = "[00:05.00]Start here";
        let lyrics = parse_lrc(content);
        assert!(lyrics.line_at(0).is_none());
        assert!(lyrics.line_at(4999).is_none());
    }

    #[test]
    fn test_line_at_exact_and_past() {
        let content = "[00:05.00]Only line";
        let lyrics = parse_lrc(content);
        assert_eq!(lyrics.line_at(5000).map(|l| l.text.as_str()), Some("Only line"));
        assert_eq!(lyrics.line_at(9999).map(|l| l.text.as_str()), Some("Only line"));
    }

    #[test]
    fn test_karaoke_progress_zero_duration_word() {
        let mut lyrics = Lyrics::new();
        lyrics.lines.push(LyricLine {
            time_ms: 0,
            text: "Test".to_string(),
            translate: String::new(),
            ksc_words: vec![KscWord {
                start_ms: 0,
                duration_ms: 0,
                text: "Test".to_string(),
            }],
        });
        assert_eq!(lyrics.karaoke_progress(0), 1000);
    }

    #[test]
    fn test_karaoke_progress_between_words() {
        let content = "[0,2000]A(500,0)B(500,1000)";
        let lyrics = parse_ksc(content);
        assert_eq!(lyrics.karaoke_progress(600), 0);
    }

    #[test]
    fn test_parse_lrc_with_offset_positive() {
        let content = "[offset:1000]\n[00:01.00]Hello";
        let result = parse_lrc(content);
        // offset 1000ms applied: 1000 + 1000 = 2000
        assert_eq!(result.lines[0].time_ms, 2000);
    }

    #[test]
    fn test_merge_same_timestamp() {
        let content = "[00:01.00]Line A1\n[00:01.00]Line A2\n[00:02.00]Line B\n[00:02.00]Line B2\n[00:03.00]Line C";
        let mut lyrics = parse_lrc(content);
        lyrics.merge_same_timestamp();
        assert_eq!(lyrics.lines.len(), 3);
        assert_eq!(lyrics.lines[0].text, "Line A1\nLine A2");
        assert_eq!(lyrics.lines[1].text, "Line B\nLine B2");
        assert_eq!(lyrics.lines[2].text, "Line C");
    }

    #[test]
    fn test_display_text_separate_mode() {
        let content = "[00:01.00]Hello\n[t:00:01.00]你好";
        let lyrics = parse_lrc(content);
        let line = &lyrics.lines[0];
        assert_eq!(lyrics.display_text(line), "Hello");
    }

    #[test]
    fn test_display_text_inline_mode() {
        let content = "[00:01.00]Hello\n[t:00:01.00]你好";
        let mut lyrics = parse_lrc(content);
        lyrics.translate_mode = TranslateMode::Inline;
        let line = &lyrics.lines[0];
        assert_eq!(lyrics.display_text(line), "Hello / 你好");
    }

    #[test]
    fn test_extract_bracket_translations() {
        let content = "[00:01.00]Hello(你好)\n[00:02.00]World";
        let mut lyrics = parse_lrc(content);
        lyrics.extract_bracket_translations();
        assert_eq!(lyrics.lines[0].text, "Hello");
        assert_eq!(lyrics.lines[0].translate, "你好");
        // Second line has no brackets, translation stays empty
        assert!(lyrics.lines[1].translate.is_empty());
    }

    #[test]
    fn test_apply_offset_does_not_go_negative() {
        let content = "[offset:-5000]\n[00:01.00]Hello";
        let result = parse_lrc(content);
        // 1000 - 5000 saturates to 0
        assert_eq!(result.lines[0].time_ms, 0);
    }
}
