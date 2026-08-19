//! Local vector storage using JSON files.
//!
//! Replaces the previous LanceDB-based implementation. Embeddings are produced
//! by a third-party OpenAI-compatible API (see `crate::ai`); this module
//! only stores them and performs local cosine-similarity search.

use crate::error::{PlayerError, Result};
use crate::ai::cosine_similarity;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LibraryFile {
    version: u32,
    tracks: Vec<TrackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrackEntry {
    path: String,
    title: String,
    artist: String,
    album: String,
    lyrics: String,
    embedding: Vec<f32>,
}

/// Vector search result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub lyrics: String,
    pub distance: f32,
}

pub struct VecDbInner {
    db_path: String,
    library: Mutex<LibraryFile>,
}

impl VecDbInner {
    fn load_or_create(&self) -> Result<()> {
        let path = &self.db_path;
        let mut lib = self.library.lock().unwrap();
        if std::path::Path::new(path).exists() {
            let data = std::fs::read_to_string(path).map_err(|e| PlayerError::Io(e))?;
            *lib = serde_json::from_str(&data)
                .map_err(|e| PlayerError::Other(format!("Failed to parse library: {e}")))?;
        } else {
            *lib = LibraryFile { version: 1, tracks: Vec::new() };
        }
        Ok(())
    }

    fn save(&self) -> Result<()> {
        let path = &self.db_path;
        let lib = self.library.lock().unwrap();
        let data = serde_json::to_string_pretty(&*lib)
            .map_err(|e| PlayerError::Other(format!("Failed to serialize library: {e}")))?;
        std::fs::write(path, data).map_err(|e| PlayerError::Io(e))
    }

    fn save_with_guard(&self, lib: &std::sync::MutexGuard<LibraryFile>) -> Result<()> {
        let path = &self.db_path;
        let data = serde_json::to_string_pretty(&**lib)
            .map_err(|e| PlayerError::Other(format!("Failed to serialize library: {e}")))?;
        std::fs::write(path, data).map_err(|e| PlayerError::Io(e))
    }
}

static VECDB: std::sync::OnceLock<VecDbInner> = std::sync::OnceLock::new();

fn get_db() -> Option<&'static VecDbInner> {
    VECDB.get()
}

pub fn default_db_path() -> String {
    crate::config::get_config_dir()
        .join("library.json")
        .to_string_lossy()
        .to_string()
}

pub fn init_vecdb(db_path: &str) -> Result<()> {
    VECDB
        .set(VecDbInner {
            db_path: db_path.to_string(),
            library: Mutex::new(LibraryFile { version: 1, tracks: Vec::new() }),
        })
        .map_err(|_| PlayerError::Other("vecdb already initialised".to_string()))?;
    get_db().unwrap().load_or_create()
}

pub fn try_global_db() -> Option<&'static VecDbInner> {
    get_db()
}

pub async fn index_track(
    conn: &VecDbInner,
    path: &str,
    title: &str,
    artist: &str,
    album: &str,
    lyrics: &str,
    embedding: &[f32],
    _lyric_emb: &[f32],
) -> Result<()> {
    let db = conn;
    let mut lib = db.library.lock().unwrap();
    let exists = lib.tracks.iter().any(|t| t.path == path);
    let entry = TrackEntry {
        path: path.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        lyrics: lyrics.to_string(),
        embedding: embedding.to_vec(),
    };
    if exists {
        lib.tracks.retain(|t| t.path != path);
    }
    lib.tracks.push(entry);
    db.save_with_guard(&lib)
}

pub async fn remove_track(conn: &VecDbInner, path: &str) -> Result<()> {
    let db = conn;
    let mut lib = db.library.lock().unwrap();
    lib.tracks.retain(|t| t.path != path);
    db.save_with_guard(&lib)
}

pub async fn clear(conn: &VecDbInner) -> Result<()> {
    let db = conn;
    let mut lib = db.library.lock().unwrap();
    lib.tracks.clear();
    db.save_with_guard(&lib)
}

pub(crate) fn new_test_vecdb(db_path: &str) -> VecDbInner {
    VecDbInner {
        db_path: db_path.to_string(),
        library: Mutex::new(LibraryFile { version: 1, tracks: Vec::new() }),
    }
}

pub async fn search(conn: &VecDbInner, query_emb: &[f32], k: usize) -> Result<Vec<SearchHit>> {
    let lib = conn.library.lock().unwrap();
    let mut scored: Vec<(f32, TrackEntry)> = Vec::new();
    for track in lib.tracks.iter() {
        if track.embedding.is_empty() || track.embedding.len() != query_emb.len() {
            continue;
        }
        let sim = cosine_similarity(&track.embedding, query_emb);
        scored.push((sim, track.clone()));
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut hits = Vec::with_capacity(k.min(scored.len()));
    for (_, track) in scored.into_iter().take(k) {
        hits.push(SearchHit {
            path: track.path,
            title: track.title,
            artist: track.artist,
            album: track.album,
            lyrics: track.lyrics,
            distance: (1.0f32 - track.embedding.iter().zip(query_emb.iter()).map(|(x, y)| x * y).sum::<f32>()).max(0.0),
        });
    }
    Ok(hits)
}

pub async fn search_lyrics(conn: &VecDbInner, query_emb: &[f32], k: usize) -> Result<Vec<SearchHit>> {
    let lib = conn.library.lock().unwrap();
    let mut scored: Vec<(f32, TrackEntry)> = Vec::new();
    for track in lib.tracks.iter() {
        if track.lyrics.trim().is_empty() {
            continue;
        }
        if track.embedding.is_empty() || track.embedding.len() != query_emb.len() {
            continue;
        }
        let sim = cosine_similarity(&track.embedding, query_emb);
        scored.push((sim, track.clone()));
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut hits = Vec::with_capacity(k.min(scored.len()));
    for (_, track) in scored.into_iter().take(k) {
        hits.push(SearchHit {
            path: track.path,
            title: track.title,
            artist: track.artist,
            album: track.album,
            lyrics: track.lyrics,
            distance: (1.0f32 - track.embedding.iter().zip(query_emb.iter()).map(|(x, y)| x * y).sum::<f32>()).max(0.0),
        });
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::LYRICS_EMBEDDING_DIM;

    fn make_emb() -> Vec<f32> {
        let mut v = vec![1.0f32; LYRICS_EMBEDDING_DIM];
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in v.iter_mut() {
            *x /= norm;
        }
        v
    }

    #[test]
    fn test_default_db_path() {
        let p = default_db_path();
        assert!(p.contains("library.json"));
    }

    #[test]
    fn test_index_and_search() -> Result<()> {
        let tmp = tempfile::Builder::new()
            .prefix("vecdb-test")
            .suffix(".json")
            .tempfile()
            .expect("failed to create temp file")
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        let db = new_test_vecdb(&tmp);
        let emb1 = make_emb();
        let mut emb2 = make_emb();
        emb2[0] *= -1.0;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "Title1", "Artist1", "Album1", "Lyric A", &emb1, &[]).await?;
                index_track(&db, "/b.mp3", "Title2", "Artist2", "Album2", "Lyric B", &emb2, &[]).await?;
                let hits = search(&db, &emb1, 2).await?;
                assert_eq!(hits.len(), 2);
                assert_eq!(hits[0].path, "/a.mp3");
                assert!(hits[0].distance < hits[1].distance);
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_file(&tmp).ok();
        Ok(())
    }

    #[test]
    fn test_search_lyrics_excludes_empty() -> Result<()> {
        let tmp = tempfile::Builder::new()
            .prefix("vecdb-test")
            .suffix(".json")
            .tempfile()
            .expect("failed to create temp file")
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "Some lyrics", &emb, &[]).await?;
                index_track(&db, "/b.mp3", "T2", "A2", "Al2", "", &emb, &[]).await?;
                let hits = search_lyrics(&db, &emb, 10).await?;
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].path, "/a.mp3");
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_file(&tmp).ok();
        Ok(())
    }

    #[test]
    fn test_remove() -> Result<()> {
        let tmp = tempfile::Builder::new()
            .prefix("vecdb-test")
            .suffix(".json")
            .tempfile()
            .expect("failed to create temp file")
            .into_temp_path()
            .to_string_lossy()
            .to_string();

        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L", &emb, &[]).await?;
                remove_track(&db, "/a.mp3").await?;
                let hits = search(&db, &emb, 10).await?;
                assert!(hits.is_empty());
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_file(&tmp).ok();
        Ok(())
    }
}
