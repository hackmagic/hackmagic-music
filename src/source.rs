//! Online music playback sources.
//!
//! Each `MusicSource` can search for tracks and resolve a playable stream URL.
//! Tracks returned by search already include a pre-resolved `stream_url` when
//! available (so the GUI can show "play" immediately); the `resolve_stream()`
//! method is provided for on-demand re-resolution when URLs expire.
//!
//! Current sources (public APIs, no authentication required):
//! - `InternetArchive` — Internet Archive audio items (archive.org)
//! - `FreeMusicArchive` — Free Music Archive (fma.studio)

use crate::error::{PlayerError, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};

// ---------------------------------------------------------------------------
// Core types
// ---------------------------------------------------------------------------

/// Stable identifier for a music source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    InternetArchive,
    FreeMusicArchive,
}

impl SourceId {
    /// Parse from a string key (used in config / CLI).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "internet_archive" | "ia" | "archive" => Some(SourceId::InternetArchive),
            "free_music_archive" | "fma" => Some(SourceId::FreeMusicArchive),
            _ => None,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            SourceId::InternetArchive => "internet_archive",
            SourceId::FreeMusicArchive => "free_music_archive",
        }
    }
}

/// A single track returned by an online source.
#[derive(Debug, Clone)]
pub struct SourceTrack {
    /// Opaque track id within the source (e.g. archive.org identifier).
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    /// Album cover image URL (if any).
    pub cover_url: Option<String>,
    /// Playable stream URL. When `None`, call `MusicSource::resolve_stream()`
    /// before playback.
    pub stream_url: Option<String>,
    /// Duration in seconds (may be `0` when unknown).
    pub duration: u64,
    /// Which source produced this track.
    pub source: SourceId,
}

impl Default for SourceTrack {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            artist: String::new(),
            album: String::new(),
            cover_url: None,
            stream_url: None,
            duration: 0,
            source: SourceId::InternetArchive,
        }
    }
}

/// Trait for online music playback sources.
///
/// `#[async_trait]` expands every `async fn` into a `-> impl Future` shim and
/// stamps `#[must_use]` on it. Since `impl Future` is already `#[must_use]`,
/// the generated attribute is redundant and trips `clippy::double_must_use`
/// on newer toolchains (1.99+). The lint is about the macro's output, not
/// about our code, so it is silenced at the trait.
#[async_trait]
#[allow(clippy::double_must_use)]
pub trait MusicSource: Send + Sync {
    fn id(&self) -> SourceId;
    /// User-facing display name (short).
    fn display_name(&self) -> &'static str;
    /// Human-readable description.
    fn description(&self) -> &'static str;
    /// Public homepage URL.
    fn homepage(&self) -> &'static str;
    /// Whether this source is enabled. May be overridden by config.
    fn enabled(&self) -> bool;

    /// Search for tracks matching `keyword`.
    async fn search(&self, keyword: &str) -> Result<Vec<SourceTrack>>;

    /// Resolve / re-resolve the playable stream URL for a track.
    /// Returns `Err` when the track is no longer playable (e.g. requires login).
    async fn resolve_stream(&self, track: &SourceTrack) -> Result<String>;
}

// ---------------------------------------------------------------------------
// Registry helpers
// ---------------------------------------------------------------------------

/// All registered source instances (one per source type).
pub fn all_sources() -> Vec<Box<dyn MusicSource>> {
    vec![
        Box::new(InternetArchive::default()),
        Box::new(FreeMusicArchive::default()),
    ]
}

/// Filter the full source list to only enabled sources, applying optional
/// overrides from a `source.enabled` config list.
///
/// Each entry may be a canonical id (`free_music_archive`) or an alias
/// (`fma`, `ia`, `archive`) — they are parsed through [`SourceId::from_str`].
/// An empty list means "all sources enabled". Entries that are not recognised
/// are ignored; if none of them resolve, the result is empty (the caller then
/// reports a config problem).
pub fn enabled_sources(enabled_ids: &[String]) -> Vec<Box<dyn MusicSource>> {
    if enabled_ids.is_empty() {
        return all_sources();
    }
    // Previously this compared raw lower-cased strings against `to_str()`, so an
    // alias like "ia" matched nothing and silently disabled every source.
    let wanted: Vec<SourceId> = enabled_ids
        .iter()
        .filter_map(|e| SourceId::from_str(e))
        .collect();
    all_sources()
        .into_iter()
        .filter(|s| wanted.contains(&s.id()))
        .collect()
}

/// Index of the source named by `default` (id / alias / display name) within
/// the list returned by `enabled_sources(enabled_ids)` — i.e. the position the
/// GUI's source selector should start at. Returns 0 when the default is not a
/// resolvable or enabled source.
pub fn default_selected_index(default: &str, enabled_ids: &[String]) -> usize {
    let wanted = SourceId::from_str(default).or_else(|| {
        all_sources()
            .into_iter()
            .find(|s| s.display_name().eq_ignore_ascii_case(default))
            .map(|s| s.id())
    });
    let Some(id) = wanted else { return 0 };
    enabled_sources(enabled_ids).into_iter().position(|s| s.id() == id).unwrap_or(0)
}

/// Get the enabled source matching `id`, or the first enabled source.
pub fn get_source(id: &str) -> Option<Box<dyn MusicSource>> {
    let target = SourceId::from_str(id).unwrap_or(SourceId::InternetArchive);
    enabled_sources(&[])
        .into_iter()
        .find(|s| s.id() == target)
}

// ---------------------------------------------------------------------------
// HTTP helpers
// ---------------------------------------------------------------------------

fn build_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("hm/1.0 (hackmagic-music-player)")
        .build()
}

// ---------------------------------------------------------------------------
// Internet Archive
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct IaSearchDoc {
    #[serde(default, rename = "identifier")]
    _identifier: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    creator: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IaSearchResponse {
    response: IaResponseInner,
}

#[derive(Debug, Deserialize)]
struct IaResponseInner {
    docs: Vec<IaSearchDoc>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IaMetadataResponse {
    files: Vec<IaFile>,
    metadata: IaMetadata,
}

#[derive(Debug, Deserialize)]
struct IaFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    format: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IaMetadata {
    #[serde(default)]
    _title: Option<String>,
    #[serde(default)]
    _creator: Option<String>,
    #[serde(default)]
    _length: Option<String>,
}

impl IaFile {
    fn is_mp3(&self) -> bool {
        self.format.eq_ignore_ascii_case("MP3")
            || self.name.to_lowercase().ends_with(".mp3")
    }
    fn is_ogg(&self) -> bool {
        self.format.eq_ignore_ascii_case("Ogg Vorbis")
            || self.format.eq_ignore_ascii_case("OGG VORBIS")
            || self.name.to_lowercase().ends_with(".ogg")
    }
}

/// Resolve the stream URL for an archive.org identifier using a shared client.
async fn resolve_ia_stream(client: &reqwest::Client, identifier: &str) -> Result<String> {
    let url = format!("https://archive.org/metadata/{}", url_path_escape(identifier));
    let resp: IaMetadataResponse = client
        .get(&url)
        .send()
        .await
        .map_err(PlayerError::Network)?
        .json()
        .await
        .map_err(|e| PlayerError::Other(e.to_string()))?;

    // Prefer MP3, fall back to OGG.
    let file = resp
        .files
        .iter()
        .find(|f| f.is_mp3())
        .or_else(|| resp.files.iter().find(|f| f.is_ogg()))
        .ok_or_else(|| PlayerError::Other("No playable audio file found".into()))?;

    Ok(format!(
        "https://archive.org/download/{}/{}?download=1",
        url_path_escape(identifier),
        url_path_escape(&file.name)
    ))
}

pub struct InternetArchive {
    enabled: AtomicBool,
}

impl Default for InternetArchive {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }
}

impl InternetArchive {
    #[allow(dead_code)]
    fn set_enabled(&self, v: bool) {
        self.enabled.store(v, Ordering::Relaxed);
    }
}

#[async_trait]
impl MusicSource for InternetArchive {
    fn id(&self) -> SourceId {
        SourceId::InternetArchive
    }
    fn display_name(&self) -> &'static str {
        "Internet Archive"
    }
    fn description(&self) -> &'static str {
        "Public audio from archive.org (concerts, classical, spoken word, etc.)"
    }
    fn homepage(&self) -> &'static str {
        "https://archive.org/details/opensource"
    }
    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    async fn search(&self, keyword: &str) -> Result<Vec<SourceTrack>> {
        if keyword.trim().is_empty() {
            return Ok(Vec::new());
        }
        let client = build_client().map_err(|e| PlayerError::Other(e.to_string()))?;
        let query = format!("mediatype:audio+AND+{}", url_escape(keyword));
        let url = format!(
            "https://archive.org/advancedsearch.php?q={}&fl%5B%5D=identifier&fl%5B%5D=title&fl%5B%5D=creator&rows=10&output=json",
            query
        );
        let resp: IaSearchResponse = client
            .get(&url)
            .send()
            .await
            .map_err(PlayerError::Network)?
            .json()
            .await
            .map_err(|e| PlayerError::Other(e.to_string()))?;

        // Resolve every hit's stream URL concurrently. Doing this serially cost
        // one metadata round-trip per result (~10 sequential requests), which
        // made a single search feel like it had hung.
        let resolved = futures_util::future::join_all(
            resp.response
                .docs
                .iter()
                .map(|doc| resolve_ia_stream(&client, &doc._identifier)),
        )
        .await;

        let tracks = resp
            .response
            .docs
            .into_iter()
            .zip(resolved)
            .filter(|(doc, _)| !doc._identifier.is_empty())
            .map(|(doc, stream)| SourceTrack {
                id: doc._identifier,
                title: doc.title.unwrap_or_default(),
                artist: doc.creator.unwrap_or_default(),
                album: String::new(),
                cover_url: None,
                // Best-effort: an unresolvable hit is still listed, just not playable.
                stream_url: stream.ok(),
                duration: 0,
                source: SourceId::InternetArchive,
            })
            .collect();

        Ok(tracks)
    }

    async fn resolve_stream(&self, track: &SourceTrack) -> Result<String> {
        let client = build_client().map_err(|e| PlayerError::Other(e.to_string()))?;
        resolve_ia_stream(&client, &track.id).await
    }
}

// ---------------------------------------------------------------------------
// Free Music Archive (FMA)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct FmaTrackResponse {
    data: Vec<FmaTrack>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FmaTrack {
    #[serde(rename = "id")]
    _id: u64,
    title: Option<String>,
    #[serde(default)]
    artist: Option<FmaArtist>,
    #[serde(default)]
    album: Option<FmaAlbum>,
    #[serde(default)]
    perma_url: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    license_name: Option<String>,
    #[serde(default)]
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct FmaArtist {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FmaAlbum {
    title: Option<String>,
}

pub struct FreeMusicArchive {
    enabled: AtomicBool,
}

impl Default for FreeMusicArchive {
    fn default() -> Self {
        Self {
            enabled: AtomicBool::new(true),
        }
    }
}

impl FreeMusicArchive {
    #[allow(dead_code)]
    fn set_enabled(&self, v: bool) {
        self.enabled.store(v, Ordering::Relaxed);
    }
}

#[async_trait]
impl MusicSource for FreeMusicArchive {
    fn id(&self) -> SourceId {
        SourceId::FreeMusicArchive
    }
    fn display_name(&self) -> &'static str {
        "Free Music Archive"
    }
    fn description(&self) -> &'static str {
        "Royalty-free & CC-licensed music from fma.studio"
    }
    fn homepage(&self) -> &'static str {
        "https://fma.studio"
    }
    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    async fn search(&self, keyword: &str) -> Result<Vec<SourceTrack>> {
        if keyword.trim().is_empty() {
            return Ok(Vec::new());
        }
        let client = build_client().map_err(|e| PlayerError::Other(e.to_string()))?;
        let url = format!(
            "https://fma.studio/api/v1/tracks?q={}&page=1&limit=10",
            url_escape(keyword)
        );
        let resp: FmaTrackResponse = client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(PlayerError::Network)?
            .json()
            .await
            .map_err(|e| PlayerError::Other(e.to_string()))?;

        let tracks: Vec<SourceTrack> = resp
            .data
            .iter()
            .filter_map(|t| {
                let stream_url = t.perma_url.clone()?;
                let title = t.title.clone().unwrap_or_default();
                let artist = t.artist.as_ref().and_then(|a| a.name.clone()).unwrap_or_default();
                let album = t.album.as_ref().and_then(|a| a.title.clone()).unwrap_or_default();
                let duration = t.duration.unwrap_or(0.0) as u64;
                if title.is_empty() && artist.is_empty() {
                    return None;
                }
                Some(SourceTrack {
                    id: t._id.to_string(),
                    title,
                    artist,
                    album,
                    cover_url: None,
                    stream_url: Some(stream_url),
                    duration,
                    source: SourceId::FreeMusicArchive,
                })
            })
            .collect();
        Ok(tracks)
    }

    async fn resolve_stream(&self, track: &SourceTrack) -> Result<String> {
        track
            .stream_url
            .clone()
            .ok_or_else(|| PlayerError::Other("No stream URL for this track".into()))
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Percent-encode a query string for use in a URL.
///
/// Operates on raw UTF-8 bytes: non-ASCII input (e.g. Chinese keywords) is
/// encoded byte-by-byte, so `"中"` becomes `%E4%B8%AD` rather than being
/// truncated to a single bogus byte.
fn url_escape(s: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'*'
            | b'!' | b'\'' | b'(' | b')' => out.push(b as char),
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0x0F) as usize] as char);
            }
        }
    }
    out
}

/// Percent-encode a single URL path segment (space becomes `%20`, not `+`).
fn url_path_escape(s: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0x0F) as usize] as char);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_id_from_str() {
        assert_eq!(SourceId::from_str("internet_archive"), Some(SourceId::InternetArchive));
        assert_eq!(SourceId::from_str("IA"), Some(SourceId::InternetArchive));
        assert_eq!(SourceId::from_str("archive"), Some(SourceId::InternetArchive));
        assert_eq!(SourceId::from_str("free_music_archive"), Some(SourceId::FreeMusicArchive));
        assert_eq!(SourceId::from_str("FMA"), Some(SourceId::FreeMusicArchive));
        assert_eq!(SourceId::from_str("unknown"), None);
    }

    #[test]
    fn test_source_id_to_str() {
        assert_eq!(SourceId::InternetArchive.to_str(), "internet_archive");
        assert_eq!(SourceId::FreeMusicArchive.to_str(), "free_music_archive");
    }

    #[test]
    fn test_all_sources() {
        let sources = all_sources();
        assert_eq!(sources.len(), 2);
        assert!(sources.iter().any(|s| s.id() == SourceId::InternetArchive));
        assert!(sources.iter().any(|s| s.id() == SourceId::FreeMusicArchive));
    }

    #[test]
    fn test_enabled_sources_filter() {
        let sources = enabled_sources(&["free_music_archive".to_string()]);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id(), SourceId::FreeMusicArchive);

        let sources = enabled_sources(&["internet_archive".to_string(), "free_music_archive".to_string()]);
        assert_eq!(sources.len(), 2);

        let sources = enabled_sources(&["unknown".to_string()]);
        assert!(sources.is_empty());
    }

    #[test]
    fn test_enabled_sources_accepts_aliases() {
        // "ia"/"archive" are documented aliases for internet_archive, "fma" for
        // free_music_archive. Raw-string matching used to drop them entirely.
        for alias in ["ia", "archive", "internet_archive"] {
            let sources = enabled_sources(&[alias.to_string()]);
            assert_eq!(sources.len(), 1, "alias {alias} should select one source");
            assert_eq!(sources[0].id(), SourceId::InternetArchive);
        }
        let sources = enabled_sources(&["fma".to_string()]);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id(), SourceId::FreeMusicArchive);
    }

    #[test]
    fn test_get_source() {
        let s = get_source("internet_archive");
        assert!(s.is_some());
        assert_eq!(s.unwrap().id(), SourceId::InternetArchive);
    }

    #[test]
    fn test_default_selected_index() {
        let none: Vec<String> = Vec::new();
        // Canonical id among all enabled sources (all_sources order: IA, FMA).
        assert_eq!(default_selected_index("internet_archive", &none), 0);
        assert_eq!(default_selected_index("fma", &none), 1);
        assert_eq!(default_selected_index("Internet Archive", &none), 0);
        // Unresolvable / disabled → 0.
        assert_eq!(default_selected_index("bogus", &none), 0);
        let only_fma = vec!["fma".to_string()];
        assert_eq!(default_selected_index("internet_archive", &only_fma), 0);
        assert_eq!(default_selected_index("free_music_archive", &only_fma), 0);
    }

    #[test]
    fn test_url_escape() {
        assert_eq!(url_escape("hello world"), "hello+world");
        assert_eq!(url_escape("rock & roll"), "rock+%26+roll");
        assert_eq!(url_escape("abc123"), "abc123");
    }

    #[test]
    fn test_url_escape_non_ascii_is_utf8_encoded() {
        // Regression: `c as u8` truncated multi-byte chars, so a CJK keyword
        // produced a garbage URL. Each UTF-8 byte must be encoded.
        assert_eq!(url_escape("中"), "%E4%B8%AD");
        assert_eq!(url_escape("周杰伦"), "%E5%91%A8%E6%9D%B0%E4%BC%A6");
        assert_eq!(url_escape("hello 世界"), "hello+%E4%B8%96%E7%95%8C");
        // "曲" is 3 UTF-8 bytes → 3 percent-escapes.
        assert_eq!(url_escape("曲").matches('%').count(), 3);
    }

    #[test]
    fn test_url_path_escape() {
        assert_eq!(url_path_escape("My Song.mp3"), "My%20Song.mp3");
        assert_eq!(url_path_escape("a/b"), "a%2Fb");
        assert_eq!(url_path_escape("normal-name_1.flac"), "normal-name_1.flac");
        assert_eq!(url_path_escape("中.mp3"), "%E4%B8%AD.mp3");
    }

    #[test]
    fn test_source_track_default() {
        let t = SourceTrack::default();
        assert_eq!(t.source, SourceId::InternetArchive);
        assert!(t.stream_url.is_none());
        assert_eq!(t.duration, 0);
    }
}
