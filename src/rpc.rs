use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::core::playlist::Track;
use crate::core::player::Player;

#[derive(Clone)]
pub struct AppState {
    pub player: Arc<Player>,
}

#[derive(Deserialize)]
pub struct CommandRequest {
    pub command: String,
}

#[derive(Serialize)]
pub struct CommandResponse {
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub state: String,
    pub volume: u32,
    pub speed: f32,
    pub pitch: i32,
    pub repeat: String,
    pub track: Option<TrackInfo>,
    pub playlist_index: Option<usize>,
    pub playlist_count: usize,
}

#[derive(Serialize)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub file: String,
    pub duration_secs: u64,
    pub position_secs: u64,
    pub is_favourite: bool,
    pub rating: u32,
    pub is_cue: bool,
    pub cue_track: u32,
}

async fn cmd_handler(
    State(_state): State<AppState>,
    Json(payload): Json<CommandRequest>,
) -> Json<CommandResponse> {
    let result = (|| -> crate::error::Result<()> {
        // Split command string respecting quoted arguments
        let args = split_quoted(&payload.command);
        if args.is_empty() {
            return Err(crate::error::PlayerError::Other("Empty command".into()));
        }
        let cli: crate::cli::Cli = clap::Parser::try_parse_from(
            std::iter::once("hm").chain(args.iter().map(|s| s.as_str())),
        )
        .map_err(|e| crate::error::PlayerError::Other(e.to_string()))?;
        match &cli.command {
            Some(cmd) => crate::commands::dispatch(cmd),
            None => Err(crate::error::PlayerError::Other("No command".into())),
        }
    })();

    match result {
        Ok(()) => Json(CommandResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(CommandResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

/// Split a command string by whitespace, respecting double-quoted substrings.
/// e.g. `play "C:\path\with spaces\file.mp3" --add`
///   → ["play", "C:\path\with spaces\file.mp3", "--add"]
fn split_quoted(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_quote = !in_quote;
            }
            ' ' | '\t' if !in_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
    let player = state.player;
    let pl = player.playlist_mut();
    let pos = player.position();
    let track = pl.current_track().map(|t| {
        // For CUE tracks, report position relative to track start
        let rel_pos = if t.is_cue && t.start_pos != Duration::ZERO {
            if pos >= t.start_pos { pos.checked_sub(t.start_pos).unwrap() } else { pos }
        } else {
            pos
        };
        TrackInfo {
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            file: t.file_path.clone(),
            duration_secs: t.duration.as_secs(),
            position_secs: rel_pos.as_secs(),
            is_favourite: t.is_favourite,
            rating: t.rating,
            is_cue: t.is_cue,
            cue_track: t.cue_track_number,
        }
    });

    Json(StatusResponse {
        state: match player.state() {
            crate::core::engine_trait::EngineState::Playing => "playing",
            crate::core::engine_trait::EngineState::Paused => "paused",
            crate::core::engine_trait::EngineState::Stopped => "stopped",
        }
        .to_string(),
        volume: player.volume(),
        speed: player.speed(),
        pitch: player.pitch(),
        repeat: pl.repeat_mode().to_str().to_string(),
        track,
        playlist_index: pl.current_index(),
        playlist_count: pl.len(),
    })
}

// ----- Spectrum -----

#[derive(Serialize)]
pub struct SpectrumResponse {
    pub data: Vec<f32>,
    pub peaks: Vec<f32>,
    pub columns: usize,
}

async fn spectrum_handler(State(state): State<AppState>) -> Json<SpectrumResponse> {
    let player = state.player;
    let spectrum = player.calculate_spectrum();
    let peaks = player.spectrum_peak_data();
    let (columns, _) = player.spectrum_config();
    Json(SpectrumResponse {
        data: spectrum,
        peaks,
        columns,
    })
}

// ----- Lyrics -----

#[derive(Serialize)]
pub struct LyricLineInfo {
    pub time_ms: u64,
    pub text: String,
    pub translate: String,
    pub is_current: bool,
    pub is_next: bool,
}

#[derive(Serialize)]
pub struct LyricResponse {
    pub lines: Vec<LyricLineInfo>,
    pub current_index: Option<usize>,
    pub next_index: Option<usize>,
    pub position_ms: u64,
    pub has_lyrics: bool,
    pub title: String,
    pub artist: String,
}

async fn lyric_handler(State(state): State<AppState>) -> Json<LyricResponse> {
    let player = state.player;
    let pl = player.playlist_mut();
    let pos_ms = player.position().as_millis() as u64;

    let mut response = LyricResponse {
        lines: Vec::new(),
        current_index: None,
        next_index: None,
        position_ms: pos_ms,
        has_lyrics: false,
        title: String::new(),
        artist: String::new(),
    };

    if let Some(track) = pl.current_track() {
        let lyric_path = if track.lyric_file.is_empty() {
            // Try to find .lrc next to the audio file
            let p = std::path::Path::new(&track.file_path);
            let lrc_path = p.with_extension("lrc");
            if lrc_path.exists() {
                Some(lrc_path.to_string_lossy().to_string())
            } else {
                None
            }
        } else {
            Some(track.lyric_file.clone())
        };

        if let Some(path) = lyric_path {
            if let Ok(lyrics) = crate::lyric::parser::load_lyric_file(&path) {
                response.has_lyrics = true;
                response.title = lyrics.title.clone();
                response.artist = lyrics.artist.clone();

                let adj_pos = if lyrics.offset_ms > 0 {
                    pos_ms.saturating_sub(lyrics.offset_ms as u64)
                } else {
                    pos_ms.saturating_add((-lyrics.offset_ms) as u64)
                };

                response.current_index = lyrics.current_line_index(adj_pos);
                response.next_index = lyrics.next_line_index(adj_pos);

                for (i, line) in lyrics.lines.iter().enumerate() {
                    response.lines.push(LyricLineInfo {
                        time_ms: line.time_ms,
                        text: line.text.clone(),
                        translate: line.translate.clone(),
                        is_current: Some(i) == response.current_index,
                        is_next: Some(i) == response.next_index,
                    });
                }
            }
        }
    }

    Json(response)
}

// ----- Lyric Search/Download -----

#[derive(Deserialize)]
struct LyricSearchRequest { keyword: Option<String> }

#[derive(Serialize)]
struct LyricSearchResponse { success: bool, error: Option<String> }

async fn lyric_search_handler(
    Json(payload): Json<LyricSearchRequest>,
) -> Json<LyricSearchResponse> {
    let keyword = payload.keyword.unwrap_or_default();
    // Use existing CLI command to search and download
    let cmd_str = if keyword.is_empty() {
        "lyric download --service netease"
    } else {
        // Use command to search, download and save
        "lyric download --service netease"
    };
    let cli: crate::cli::Cli = match clap::Parser::try_parse_from(
        std::iter::once("hm").chain(cmd_str.split_whitespace()),
    ) {
        Ok(c) => c,
        Err(e) => return Json(LyricSearchResponse { success: false, error: Some(e.to_string()) }),
    };
    match crate::commands::dispatch(&cli.command.unwrap()) {
        Ok(()) => Json(LyricSearchResponse { success: true, error: None }),
        Err(e) => Json(LyricSearchResponse { success: false, error: Some(e.to_string()) }),
    }
}

// ----- MusicBrainz -----

#[derive(Deserialize)]
struct MusicBrainzRequest { keyword: Option<String>, auto: Option<bool>, artist: Option<String> }

#[derive(Serialize)]
struct MusicBrainzResponse { success: bool, error: Option<String>, message: String, results: Option<Vec<String>> }

async fn musicbrainz_handler(
    State(_state): State<AppState>,
    Json(payload): Json<MusicBrainzRequest>,
) -> Json<MusicBrainzResponse> {
let auto = payload.auto.unwrap_or(true);
let mut cmd_str = String::from("musicbrainz");
if auto {
    cmd_str.push_str(" --auto");
} else if let Some(ref kw) = payload.keyword {
    cmd_str.push(' ');
    cmd_str.push_str(kw);
}
if let Some(ref artist) = payload.artist {
    cmd_str.push_str(" --artist ");
    cmd_str.push_str(artist);
}
    let cli: crate::cli::Cli = match clap::Parser::try_parse_from(
        std::iter::once("hm").chain(cmd_str.split_whitespace()),
    ) {
        Ok(c) => c,
        Err(e) => return Json(MusicBrainzResponse { success: false, error: Some(e.to_string()), message: String::new(), results: None }),
    };
    match crate::commands::dispatch(&cli.command.unwrap()) {
        Ok(()) => Json(MusicBrainzResponse { success: true, error: None, message: "MusicBrainz tag applied".to_string(), results: None }),
        Err(e) => Json(MusicBrainzResponse { success: false, error: Some(e.to_string()), message: String::new(), results: None }),
    }
}

// ----- Cover -----

#[derive(Serialize)]
pub struct CoverResponse {
    pub data: Option<String>,
    pub mime: Option<String>,
    pub has_cover: bool,
}

async fn cover_handler(State(state): State<AppState>) -> Json<CoverResponse> {
    let player = state.player;
    let pl = player.playlist_mut();

    if let Some(track) = pl.current_track() {
        // Check disk cache first
        let cache_dir = crate::config::get_config_dir().join("covers");
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        track.file_path.hash(&mut hasher);
        let cache_key = format!("{:x}", hasher.finish());
        let cache_file = cache_dir.join(&cache_key);
        if cache_file.exists() {
            if let Ok(data) = std::fs::read(&cache_file) {
                let mime = match cache_file.extension().and_then(|e| e.to_str()).unwrap_or("jpg") {
                    "png" => "image/png", "bmp" => "image/bmp", _ => "image/jpeg",
                };
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                return Json(CoverResponse { data: Some(b64), mime: Some(mime.to_string()), has_cover: true });
            }
        }

        // Try embedded cover
        if let Ok(pics) = crate::tag::writer::read_pictures(&track.file_path) {
            if let Some((ext, data)) = pics.into_iter().next() {
                let mime = match ext.as_str() {
                    "png" => "image/png",
                    "bmp" => "image/bmp",
                    _ => "image/jpeg",
                };
                // Write to cache
                let _ = std::fs::create_dir_all(&cache_dir);
                let cache_path = cache_dir.join(format!("{}.{}", cache_key, if ext == "jpeg" { "jpg" } else { &ext }));
                let _ = std::fs::write(&cache_path, &data);
                use base64::Engine;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                return Json(CoverResponse {
                    data: Some(b64),
                    mime: Some(mime.to_string()),
                    has_cover: true,
                });
            }
        }

        // Try external cover file
        let audio_path = std::path::Path::new(&track.file_path);
        if let Some(dir) = audio_path.parent() {
            for ext in &["jpg", "jpeg", "png", "bmp"] {
                for name in &["cover", "folder", "front"] {
                    let cover_path = dir.join(format!("{name}.{ext}"));
                    if cover_path.exists() {
                        if let Ok(data) = std::fs::read(&cover_path) {
                            let mime = match *ext {
                                "png" => "image/png",
                                "bmp" => "image/bmp",
                                _ => "image/jpeg",
                            };
                            use base64::Engine;
                            let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                            return Json(CoverResponse {
                                data: Some(b64),
                                mime: Some(mime.to_string()),
                                has_cover: true,
                            });
                        }
                    }
                }
            }
        }
    }

    Json(CoverResponse {
        data: None,
        mime: None,
        has_cover: false,
    })
}

// ----- Stats -----

#[derive(Serialize)]
struct StatsResponse {
    total_listen_secs: u64,
    total_play_count: u64,
    total_track_count: usize,
    day_secs: u64,
    week_secs: u64,
    month_secs: u64,
    top_tracks: Vec<StatEntryItem>,
    top_artists: Vec<ArtistStatItem>,
}

#[derive(Serialize)]
struct StatEntryItem { path: String, title: String, artist: String, play_count: u64, listen_secs: u64 }

#[derive(Serialize)]
struct ArtistStatItem { artist: String, play_count: u64, listen_secs: u64 }

async fn stats_handler(State(_state): State<AppState>) -> Json<StatsResponse> {
    let total_listen_secs = crate::play_stats::total_listen_secs();
    let total_play_count = crate::play_stats::total_play_count();
    let total_track_count = crate::play_stats::total_track_count();
    let (day_secs, week_secs, month_secs, _all) = crate::play_stats::activity_breakdown();
    let top = crate::play_stats::top_stats(10);
    let top_tracks = top.into_iter().map(|(p, s)| {
        let title = crate::tag::reader::read_tags(&p).ok().map(|t| t.title).unwrap_or_default();
        let artist = crate::tag::reader::read_tags(&p).ok().map(|t| t.artist).unwrap_or_default();
        StatEntryItem { path: p, title, artist, play_count: s.play_count, listen_secs: s.listen_secs }
    }).collect();
    let artists = crate::play_stats::artist_stats(10);
    let top_artists = artists.into_iter().map(|(a, c, s)| ArtistStatItem { artist: a, play_count: c, listen_secs: s }).collect();
    Json(StatsResponse { total_listen_secs, total_play_count, total_track_count, day_secs, week_secs, month_secs, top_tracks, top_artists })
}

// ----- Config -----

#[derive(Serialize)]
pub struct ConfigResponse {
    pub config: Value,
}

async fn config_get_handler(State(_state): State<AppState>) -> Json<ConfigResponse> {
    let cfg = crate::config::Config::load();
    let json = serde_json::to_value(&cfg).unwrap_or_default();
    Json(ConfigResponse { config: json })
}

#[derive(Deserialize)]
pub struct ConfigSetPayload {
    pub key: String,
    pub value: String,
}

async fn config_set_handler(
    State(_state): State<AppState>,
    Json(payload): Json<ConfigSetPayload>,
) -> Json<CommandResponse> {
    match crate::config::Config::set(&payload.key, &payload.value) {
        Ok(()) => Json(CommandResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(CommandResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

// ----- Media Library -----

#[derive(Serialize)]
pub struct MediaArtistsResponse {
    pub artists: Vec<String>,
}

async fn media_artists_handler() -> Json<MediaArtistsResponse> {
    let lib = crate::media::MediaLib::load();
    Json(MediaArtistsResponse {
        artists: lib.artists(),
    })
}

#[derive(Serialize)]
pub struct MediaAlbumsResponse {
    pub albums: Vec<AlbumInfo>,
}

#[derive(Serialize)]
pub struct AlbumInfo {
    pub name: String,
    pub track_count: usize,
}

async fn media_albums_handler(Path(artist): Path<String>) -> Json<MediaAlbumsResponse> {
    let lib = crate::media::MediaLib::load();
    let entries = lib.by_artist(Some(&artist));
    let mut album_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for e in &entries {
        if !e.album.is_empty() {
            *album_map.entry(e.album.clone()).or_default() += 1;
        }
    }
    let albums: Vec<AlbumInfo> = album_map
        .into_iter()
        .map(|(name, track_count)| AlbumInfo { name, track_count })
        .collect();
    Json(MediaAlbumsResponse { albums })
}

#[derive(Serialize)]
pub struct MediaTracksResponse {
    pub tracks: Vec<MediaTrackInfo>,
}

#[derive(Serialize)]
pub struct MediaTrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u64,
    pub file_path: String,
}

async fn media_tracks_handler(
    Path((artist, album)): Path<(String, String)>,
) -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib
        .entries
        .iter()
        .filter(|e| {
            e.artist.to_lowercase() == artist.to_lowercase()
                && e.album.to_lowercase() == album.to_lowercase()
        })
        .map(|e| MediaTrackInfo {
            title: e.title.clone(),
            artist: e.artist.clone(),
            album: e.album.clone(),
            duration_secs: e.duration_secs,
            file_path: e.file_path.clone(),
        })
        .collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_all_handler() -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib
        .entries
        .iter()
        .map(|e| MediaTrackInfo {
            title: e.title.clone(),
            artist: e.artist.clone(),
            album: e.album.clone(),
            duration_secs: e.duration_secs,
            file_path: e.file_path.clone(),
        })
        .collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_genres_handler() -> Json<Vec<String>> {
    let lib = crate::media::MediaLib::load();
    Json(lib.genres())
}

#[derive(Serialize)]
pub struct MediaYearsResponse {
    pub years: Vec<u32>,
}

async fn media_years_handler() -> Json<MediaYearsResponse> {
    let lib = crate::media::MediaLib::load();
    Json(MediaYearsResponse { years: lib.years() })
}

#[derive(Serialize)]
pub struct MediaFileTypesResponse {
    pub types: Vec<String>,
}

async fn media_filetypes_handler() -> Json<MediaFileTypesResponse> {
    let lib = crate::media::MediaLib::load();
    Json(MediaFileTypesResponse { types: lib.file_types() })
}

#[derive(Serialize)]
pub struct MediaBitratesResponse {
    pub bitrates: Vec<u32>,
}

async fn media_bitrates_handler() -> Json<MediaBitratesResponse> {
    let lib = crate::media::MediaLib::load();
    Json(MediaBitratesResponse { bitrates: lib.bitrates() })
}

async fn media_genre_tracks_handler(Path(genre): Path<String>) -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.by_genre(&genre)
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_year_tracks_handler(Path(year): Path<u32>) -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.by_year(year)
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_type_tracks_handler(Path(ext): Path<String>) -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.by_file_type(&ext)
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_bitrate_tracks_handler(Path(bitrate): Path<u32>) -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.by_bitrate(bitrate)
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_recent_handler() -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.recent(200)
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

async fn media_favourites_handler() -> Json<MediaTracksResponse> {
    let lib = crate::media::MediaLib::load();
    let tracks: Vec<MediaTrackInfo> = lib.favourites()
        .iter().map(|e| MediaTrackInfo {
            title: e.title.clone(), artist: e.artist.clone(), album: e.album.clone(),
            duration_secs: e.duration_secs, file_path: e.file_path.clone(),
        }).collect();
    Json(MediaTracksResponse { tracks })
}

// ----- Playlist as JSON -----

#[derive(Serialize)]
pub struct PlaylistResponse {
    pub name: String,
    pub current_index: Option<usize>,
    pub tracks: Vec<PlaylistTrackInfo>,
}

#[derive(Serialize, Clone)]
pub struct PlaylistTrackInfo {
    pub index: usize,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u64,
    pub file_path: String,
    pub is_favourite: bool,
    pub rating: u32,
    pub is_cue: bool,
    pub cue_track: u32,
}

async fn playlist_handler(State(state): State<AppState>) -> Json<PlaylistResponse> {
    let player = state.player;
    let pl = player.playlist_mut();
    let tracks: Vec<PlaylistTrackInfo> = pl
        .tracks()
        .iter()
        .enumerate()
        .map(|(i, t)| PlaylistTrackInfo {
            index: i,
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            duration_secs: t.duration.as_secs(),
            file_path: t.file_path.clone(),
            is_favourite: t.is_favourite,
            rating: t.rating,
            is_cue: t.is_cue,
            cue_track: t.cue_track_number,
        })
        .collect();
    Json(PlaylistResponse {
        name: pl.name().to_string(),
        current_index: pl.current_index(),
        tracks,
    })
}

// ----- Playlist Queue -----

#[derive(Serialize)]
struct QueuedTrackInfo { index: usize, track: PlaylistTrackInfo }

async fn playlist_queue_handler(State(state): State<AppState>) -> Json<Vec<QueuedTrackInfo>> {
    let pl = state.player.playlist();
    let all_tracks: Vec<PlaylistTrackInfo> = pl
        .tracks()
        .iter()
        .enumerate()
        .map(|(i, t)| PlaylistTrackInfo {
            index: i,
            title: t.title.clone(),
            artist: t.artist.clone(),
            album: t.album.clone(),
            duration_secs: t.duration.as_secs(),
            file_path: t.file_path.clone(),
            is_favourite: t.is_favourite,
            rating: t.rating,
            is_cue: t.is_cue,
            cue_track: t.cue_track_number,
        })
        .collect();
    let indices = pl.queued_indices();
    Json(indices.into_iter().filter_map(|i| {
        all_tracks.get(i).cloned().map(|t| QueuedTrackInfo { index: i, track: t })
    }).collect())
}

// ----- Playlist Export/Import -----

#[derive(Serialize)]
struct ExportResponse { m3u: String, count: usize }

async fn playlist_export_handler(State(state): State<AppState>) -> Json<ExportResponse> {
    let pl = state.player.playlist();
    let m3u = pl.tracks().iter().map(|t| t.file_path.clone()).collect::<Vec<_>>().join("\n");
    Json(ExportResponse { count: pl.tracks().len(), m3u })
}

#[derive(Deserialize)]
struct ImportRequest { m3u: String }

async fn playlist_import_handler(
    State(state): State<AppState>,
    Json(payload): Json<ImportRequest>,
) -> Json<CommandResponse> {
    let mut pl = state.player.playlist_mut();
    for line in payload.m3u.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let track = crate::tag::reader::read_tags(line).unwrap_or_else(|_| Track::new(line));
        pl.add_track(track);
    }
    Json(CommandResponse { success: true, error: None })
}

// ----- Playlist Reorder -----

#[derive(Deserialize)]
struct PlaylistReorderRequest {
    from: usize,
    to: usize,
}

async fn playlist_reorder_handler(
    State(state): State<AppState>,
    Json(payload): Json<PlaylistReorderRequest>,
) -> Json<CommandResponse> {
    let player = state.player;
    let mut pl = player.playlist_mut();
    match pl.move_track(payload.from, payload.to) {
        Ok(()) => Json(CommandResponse {
            success: true,
            error: None,
        }),
        Err(e) => Json(CommandResponse {
            success: false,
            error: Some(e.to_string()),
        }),
    }
}

// ----- WebSocket Spectrum -----

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_millis(80));
    loop {
        interval.tick().await;
        let spectrum = state.player.calculate_spectrum();
        let peaks = state.player.spectrum_peak_data();
        let payload = serde_json::json!({
            "spectrum": spectrum,
            "peaks": peaks
        });
        if socket
            .send(Message::Text(payload.to_string().into()))
            .await
            .is_err()
        {
            break;
        }
    }
}

// ----- Audio Device List -----

#[derive(Serialize)]
struct AudioDeviceInfo { id: i32, name: String }

async fn audio_devices_handler(State(state): State<AppState>) -> Json<Vec<AudioDeviceInfo>> {
    let devices = state.player.list_audio_devices();
    Json(devices.into_iter().map(|(id, name)| AudioDeviceInfo { id, name }).collect())
}

// ----- Media Search -----

#[derive(Deserialize)]
struct MediaSearchRequest { keyword: String }

#[derive(Serialize)]
struct MediaSearchResult {
    title: String,
    artist: String,
    album: String,
    file_path: String,
    duration_secs: u64,
}

async fn media_search_handler(
    Json(payload): Json<MediaSearchRequest>,
) -> Json<Vec<MediaSearchResult>> {
    if payload.keyword.trim().is_empty() {
        return Json(Vec::new());
    }
    let lib = crate::media::MediaLib::load();
    let results: Vec<MediaSearchResult> = lib.search(&payload.keyword).into_iter().map(|e| MediaSearchResult {
        title: e.title.clone(),
        artist: e.artist.clone(),
        album: e.album.clone(),
        file_path: e.file_path.clone(),
        duration_secs: e.duration_secs,
    }).collect();
    Json(results)
}

// ----- Folder Browse -----

#[derive(Deserialize)]
struct TagReadRequest { file: String }

#[derive(Serialize)]
struct TagReadResponse {
    title: String, artist: String, album: String,
    genre: String, year: u32, track: u32,
    bitrate: u32, duration_secs: u64,
    sample_rate: u32, bit_depth: u32, channels: u8,
}

async fn tag_read_handler(Json(payload): Json<TagReadRequest>) -> Json<Option<TagReadResponse>> {
    let path = std::path::Path::new(&payload.file);
    if !path.exists() { return Json(None); }
    match crate::tag::reader::read_tags(&payload.file) {
        Ok(track) => Json(Some(TagReadResponse {
            title: track.title, artist: track.artist, album: track.album,
            genre: track.genre, year: track.year, track: track.track_number,
            bitrate: track.bitrate, duration_secs: track.duration.as_secs(),
            sample_rate: track.sample_rate, bit_depth: track.bit_depth, channels: track.channels,
        })),
        Err(_) => Json(None),
    }
}

#[derive(Deserialize)]
struct BrowseRequest { path: String }

#[derive(Serialize, Debug, PartialEq)]
pub(crate) struct BrowseEntry {
    name: String,
    path: String,
    is_dir: bool,
    is_audio: bool,
    size: u64,
}

const AUDIO_EXTS: &[&str] = &["mp3","flac","wav","ogg","opus","m4a","aac","wma","ape","dsf","aiff","alac"];

/// Browse a directory and return sorted entries (directories first, then audio files).
/// Returns empty vec if the path does not exist or is not a directory.
pub fn browse_directory(p: &std::path::Path) -> Vec<BrowseEntry> {
    if !p.exists() || !p.is_dir() {
        return Vec::new();
    }
    let mut entries = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(p) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let is_dir = path.is_dir();
            let ext = path.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            let is_audio = !is_dir && AUDIO_EXTS.contains(&ext.as_str());
            if !is_dir && !is_audio { continue; }
            if name.starts_with('.') { continue; }
            let size = if is_dir { 0 } else { std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) };
            entries.push(BrowseEntry { name, path: path.to_string_lossy().to_string(), is_dir, is_audio, size });
        }
    }
    entries.sort_by(|a, b| {
        if a.is_dir != b.is_dir { return if a.is_dir { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater }; }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    entries
}

async fn browse_handler(Json(payload): Json<BrowseRequest>) -> Json<Vec<BrowseEntry>> {
    Json(browse_directory(std::path::Path::new(&payload.path)))
}

// ----- EQ -----

#[derive(Serialize)]
pub struct EqResponse {
    pub enabled: bool,
    pub bands: Vec<i32>,
    pub preset: String,
}

// ----- Playlist List -----

#[derive(Serialize)]
struct PlaylistListItem {
    name: String,
    track_count: usize,
    is_active: bool,
}

async fn playlist_list_handler(State(state): State<AppState>) -> Json<Vec<PlaylistListItem>> {
    let player = state.player;
    let active = player.active_playlist_name();
    let list = player.list_playlists();
    Json(list.into_iter().map(|(name, count)| {
        let is_active = name == active;
        PlaylistListItem { name, track_count: count, is_active }
    }).collect())
}

// ----- EQ -----

async fn eq_handler(State(state): State<AppState>) -> Json<EqResponse> {
    let player = state.player;
    Json(EqResponse {
        enabled: player.eq_is_enabled(),
        bands: player.eq_get().to_vec(),
        preset: String::new(),
    })
}

// ----- Reverb -----

#[derive(Serialize)]
pub struct ReverbResponse {
    pub enabled: bool,
    pub mix: u32,
    pub time: u32,
}

async fn reverb_handler(State(state): State<AppState>) -> Json<ReverbResponse> {
    let player = state.player;
    let (mix, time) = player.reverb_get();
    Json(ReverbResponse {
        enabled: player.reverb_is_enabled(),
        mix,
        time,
    })
}

// ----- Health -----

#[derive(Serialize)]
pub struct HealthResponse {
    pub ok: bool,
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

// ----- Router -----

pub fn start_rpc_server(player: Arc<Player>, port: u16) {
    let state = AppState { player };

    // Serve gui/ directory if it exists next to the executable
    let gui_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("gui")))
        .filter(|d| d.exists());

    let app = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/command", axum::routing::post(cmd_handler))
        .route("/api/status", get(status_handler))
        .route("/api/nowplaying", get(status_handler))
        .route("/api/spectrum", get(spectrum_handler))
        .route("/api/lyric", get(lyric_handler))
        .route("/api/lyric/search", axum::routing::post(lyric_search_handler))
        .route("/api/musicbrainz", axum::routing::post(musicbrainz_handler))
        .route("/api/cover", get(cover_handler))
        .route("/api/config", get(config_get_handler).post(config_set_handler))
        .route("/api/media/artists", get(media_artists_handler))
        .route("/api/media/albums/{artist}", get(media_albums_handler))
        .route(
            "/api/media/tracks/{artist}/{album}",
            get(media_tracks_handler),
        )
        .route("/api/media/all", get(media_all_handler))
        .route("/api/media/genres", get(media_genres_handler))
        .route("/api/media/years", get(media_years_handler))
        .route("/api/media/filetypes", get(media_filetypes_handler))
        .route("/api/media/bitrates", get(media_bitrates_handler))
        .route("/api/media/genre/{genre}", get(media_genre_tracks_handler))
        .route("/api/media/year/{year}", get(media_year_tracks_handler))
        .route("/api/media/type/{ext}", get(media_type_tracks_handler))
        .route("/api/media/bitrate/{bitrate}", get(media_bitrate_tracks_handler))
        .route("/api/media/recent", get(media_recent_handler))
        .route("/api/media/favourites", get(media_favourites_handler))
        .route("/api/eq", get(eq_handler))
        .route("/api/reverb", get(reverb_handler))
        .route("/api/playlist", get(playlist_handler))
        .route("/api/playlist/list", get(playlist_list_handler))
        .route("/api/playlist/reorder", axum::routing::post(playlist_reorder_handler))
        .route("/api/playlist/queue", axum::routing::get(playlist_queue_handler))
        .route("/api/playlist/export", axum::routing::get(playlist_export_handler))
        .route("/api/playlist/import", axum::routing::post(playlist_import_handler))
        .route("/api/media/search", axum::routing::post(media_search_handler))
        .route("/api/media/browse", axum::routing::post(browse_handler))
        .route("/api/tag/read", axum::routing::post(tag_read_handler))
        .route("/api/audio/devices", get(audio_devices_handler))
        .route("/api/stats", get(stats_handler))
        .route("/ws", get(ws_handler))
        .with_state(state);

    // CORS for Tauri windows (allow Tauri dev/prod origins + same origin)
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);
    let app = app.layer(cors);

    // Serve gui/ as static files at root
    let gui_path = gui_dir.unwrap_or_else(|| PathBuf::from("gui"));
    let app = app.fallback_service(
        ServeDir::new(&gui_path).append_index_html_on_directories(true),
    );

    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            let addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port);
            tracing::info!("RPC server listening on http://{}", addr);
            if let Err(e) = axum::serve(
                tokio::net::TcpListener::bind(&addr).await.unwrap(),
                app,
            )
            .await
            {
                tracing::error!("RPC server error: {}", e);
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Test the daemon IPC health endpoint handler.
    /// This is the simplest RPC handler — it requires no state and always returns `ok: true`.
    #[tokio::test]
    async fn test_health_handler() {
        let response = health_handler().await;
        assert!(response.0.ok, "health handler should return ok=true");
    }

    #[test]
    fn test_browse_directory_nonexistent_path() {
        let result = browse_directory(Path::new(r"C:\no_such_dir_xy42"));
        assert!(result.is_empty());
    }

    #[test]
    fn test_browse_directory_empty_dir() {
        let dir = std::env::temp_dir().join("test_browse_empty_xy42");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let result = browse_directory(&dir);
        assert!(result.is_empty());
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_browse_directory_finds_audio_files() {
        let dir = std::env::temp_dir().join("test_browse_audio_xy42");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Create an audio file
        fs::write(dir.join("song.mp3"), b"dummy").unwrap();
        // Create a non-audio file (should be excluded)
        fs::write(dir.join("notes.txt"), b"hello").unwrap();
        // Create a subdirectory (should be included)
        fs::create_dir(dir.join("sub")).unwrap();
        // Create an audio file in subdirectory (should be excluded — not a direct child)
        fs::write(dir.join("sub").join("track.flac"), b"dummy").unwrap();

        let mut result = browse_directory(&dir);
        result.sort_by(|a, b| a.name.cmp(&b.name)); // stable sort for comparison

        // Expect: sub dir (is_dir=true), song.mp3 (is_audio=true)
        // notes.txt should be excluded
        assert_eq!(result.len(), 2, "expected 2 entries: sub/ and song.mp3");

        let sub_entry = result.iter().find(|e| e.name == "sub").unwrap();
        assert!(sub_entry.is_dir);
        assert!(!sub_entry.is_audio);

        let audio_entry = result.iter().find(|e| e.name == "song.mp3").unwrap();
        assert!(!audio_entry.is_dir);
        assert!(audio_entry.is_audio);
        assert!(audio_entry.size > 0);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_browse_directory_hidden_files_skipped() {
        let dir = std::env::temp_dir().join("test_browse_hidden_xy42");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(".hidden.mp3"), b"dummy").unwrap();
        fs::write(dir.join("visible.mp3"), b"dummy").unwrap();

        let result = browse_directory(&dir);
        // Only visible.mp3 should be present
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "visible.mp3");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_browse_directory_sort_order() {
        let dir = std::env::temp_dir().join("test_browse_sort_xy42");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::create_dir(dir.join("alpha")).unwrap();
        fs::write(dir.join("beta.mp3"), b"dummy").unwrap();
        fs::create_dir(dir.join("gamma")).unwrap();
        fs::write(dir.join("delta.flac"), b"dummy").unwrap();

        let result = browse_directory(&dir);
        // Order: directories first (alpha, gamma), then audio files (beta, delta)
        assert_eq!(result.len(), 4);
        assert!(result[0].is_dir);
        assert!(result[1].is_dir);
        assert!(!result[2].is_dir);
        assert!(!result[3].is_dir);
        // Within same type, alphabetical by lowercase name
        assert_eq!(result[0].name, "alpha");
        assert_eq!(result[1].name, "gamma");
        assert_eq!(result[2].name, "beta.mp3");
        assert_eq!(result[3].name, "delta.flac");

        let _ = fs::remove_dir_all(&dir);
    }
}