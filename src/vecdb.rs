//! Local vector storage using an append-only JSON journal.
//!
//! Replaces the previous LanceDB-based implementation. Embeddings are produced
//! by a third-party OpenAI-compatible API (see `crate::ai`); this module
//! only stores them and performs local cosine-similarity search.
//!
//! # On-disk layout
//!
//! * `<db_path>`          — compacted base library (a full `LibraryFile`)
//! * `<db_path>.journal`  — append-only ops log, one `JournalOp` JSON per line
//!
//! The previous design rewrote the *entire* base file on every mutation. With
//! 1536-dim embeddings that is ~30 KB per track, so a 1000-track library meant
//! rewriting ~30 MB on every single song change. Mutations now append one
//! journal line (~30 KB) instead, and the base file is rewritten only by
//! `compact()`, which runs once the journal reaches `COMPACT_AFTER_OPS` entries
//! (or on an explicit `flush()`).
//!
//! Crash safety: the base file is replaced atomically (temp file + rename), and
//! the journal is replayed on top of it at load time. A torn trailing journal
//! line (possible after a crash mid-append) is skipped rather than failing the
//! whole load.
//!
//! Uses `RwLock` for concurrent read access: multiple `search`/`search_lyrics`
//! operations can run in parallel while writes (index/remove/clear) are exclusive.
//! I/O is performed outside the lock scope in `load_or_create` to avoid holding
//! the write lock during file reads.

use crate::error::{PlayerError, Result};
use crate::ai::cosine_similarity;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

/// Compact the journal into the base file once it reaches this many entries.
/// Bounds journal replay time at startup and keeps the journal from growing
/// without limit. Upserts are idempotent, so replaying a longer-than-needed
/// journal is always safe — this is purely a performance bound.
const COMPACT_AFTER_OPS: u64 = 512;

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

/// One append-only journal entry. Upserts carry the full entry (the embedding
/// must survive replay); Remove/Clear carry just enough to apply them.
///
/// Deserialised from one JSON value per line, so adding a variant later must
/// keep this externally tagged (the default) — renaming a variant would break
/// existing journals.
#[derive(Debug, Clone, Serialize, Deserialize)]
enum JournalOp {
    Upsert(TrackEntry),
    Remove { path: String },
    Clear,
}

/// Apply an op to the in-memory library. Upsert is idempotent — last write for
/// a path wins — which is what makes replaying a long journal safe.
fn apply_op(lib: &mut LibraryFile, op: JournalOp) {
    match op {
        JournalOp::Upsert(entry) => {
            let path = entry.path.clone();
            lib.tracks.retain(|t| t.path != path);
            lib.tracks.push(entry);
        }
        JournalOp::Remove { path } => {
            lib.tracks.retain(|t| t.path != path);
        }
        JournalOp::Clear => lib.tracks.clear(),
    }
}

/// Guards against a poisoned RwLock: the lock is only ever held across pure
/// in-memory work (no panicking user code, no I/O), so poisoning should be
/// impossible; recovering keeps the DB usable instead of wedging every later
/// call.
fn lock_read(l: &RwLock<LibraryFile>) -> std::sync::RwLockReadGuard<'_, LibraryFile> {
    l.read().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn lock_write(l: &RwLock<LibraryFile>) -> std::sync::RwLockWriteGuard<'_, LibraryFile> {
    l.write().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct VecDbInner {
    db_path: String,
    /// Append-only op log; replayed on top of db_path at load time.
    journal_path: String,
    /// Entries currently sitting in the journal file. Its own mutex (never held
    /// across an await) serialises appends so compaction cannot interleave.
    journal_ops: std::sync::Mutex<u64>,
    library: RwLock<LibraryFile>,
}

impl VecDbInner {
    fn load_or_create(&self) -> Result<()> {
        let path = &self.db_path;
        // Read file outside the write lock to minimise lock duration.
        let data = if std::path::Path::new(path).exists() {
            let raw = std::fs::read_to_string(path).map_err(PlayerError::Io)?;
            Some(raw)
        } else {
            None
        };

        {
            let mut lib = lock_write(&self.library);
            if let Some(raw) = data {
                *lib = serde_json::from_str(&raw)
                    .map_err(|e| PlayerError::Other(format!("Failed to parse library: {e}")))?;
            } else {
                *lib = LibraryFile { version: 1, tracks: Vec::new() };
            }
        }

        // Replay whatever journal the previous run left behind (including after a
        // crash). A torn final line is skipped rather than failing the load:
        // losing one entry beats refusing to start.
        let journal_raw = match std::fs::read_to_string(&self.journal_path) {
            Ok(raw) => raw,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(PlayerError::Io(e)),
        };
        let mut replayed = 0u64;
        for line in journal_raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let op: JournalOp = match serde_json::from_str(line) {
                Ok(op) => op,
                Err(_) => {
                    tracing::warn!(
                        "[VECDB] Skipping unparsable journal line ({} bytes) — likely a torn append",
                        line.len()
                    );
                    continue;
                }
            };
            apply_op(&mut lock_write(&self.library), op);
            replayed += 1;
        }
        if replayed > 0 {
            *self.journal_ops.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = replayed;
            tracing::info!("[VECDB] Replayed {} journal op(s) into memory", replayed);
        }
        Ok(())
    }

    /// Append one op to the journal, then apply it in memory. The append is the
    /// durability point: if the process dies right after it, replay at next
    /// start recovers the op, which is why the in-memory update comes last.
    ///
    /// Compacts first when the journal has reached COMPACT_AFTER_OPS entries,
    /// folding them into the base file so replay time stays bounded.
    fn journal_op(&self, op: JournalOp) -> Result<()> {
        let mut ops = self.journal_ops.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if *ops >= COMPACT_AFTER_OPS {
            self.compact_locked(&mut ops)?;
        }
        let line = serde_json::to_string(&op)
            .map_err(|e| PlayerError::Other(format!("Failed to serialize journal op: {e}")))?;
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.journal_path)
                .map_err(PlayerError::Io)?;
            writeln!(file, "{line}").map_err(PlayerError::Io)?;
        }
        *ops += 1;

        apply_op(&mut lock_write(&self.library), op);
        Ok(())
    }

    /// Fold the journal into the base file and truncate the journal.
    ///
    /// Callers must hold the journal_ops mutex (i.e. call from journal_op or
    /// flush) so no append can interleave with the truncate.
    ///
    /// The base file is written to "<path>.tmp" and renamed over "<path>", so a
    /// crash mid-compaction leaves the previous base file intact (plus the
    /// still-untruncated journal) rather than a half-written library.
    fn compact_locked(&self, ops: &mut std::sync::MutexGuard<'_, u64>) -> Result<()> {
        let data = {
            let lib = lock_read(&self.library);
            serde_json::to_string_pretty(&*lib)
                .map_err(|e| PlayerError::Other(format!("Failed to serialize library: {e}")))?
        };
        let tmp = tmp_path_for(&self.db_path);
        std::fs::write(&tmp, data).map_err(PlayerError::Io)?;
        std::fs::rename(&tmp, &self.db_path).map_err(PlayerError::Io)?;
        // Truncate only after the rename succeeded: a failed rename keeps the
        // journal intact, so replay still recovers every op.
        std::fs::write(&self.journal_path, "").map_err(PlayerError::Io)?;
        **ops = 0;
        tracing::debug!("[VECDB] Compacted journal into {}", self.db_path);
        Ok(())
    }

    /// Flush the journal into the base file immediately. Called on GUI shutdown
    /// and at the end of rebuild-index. Idempotent — safe on a clean database.
    pub fn flush(&self) -> Result<()> {
        let mut ops = self.journal_ops.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        self.compact_locked(&mut ops)
    }

    /// Entries currently in the journal (not yet folded into the base file).
    pub fn journal_len(&self) -> u64 {
        *self.journal_ops.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Temp file used for the atomic base-file rewrite during compaction.
fn tmp_path_for(db_path: &str) -> String {
    format!("{db_path}.tmp")
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
            journal_path: format!("{db_path}.journal"),
            journal_ops: std::sync::Mutex::new(0),
            library: RwLock::new(LibraryFile { version: 1, tracks: Vec::new() }),
        })
        .map_err(|_| PlayerError::Other("vecdb already initialised".to_string()))?;
    get_db().map(|db| db.load_or_create()).unwrap_or_else(|| {
        Err(PlayerError::Other("vecdb initialisation lost".to_string()))
    })
}

pub fn try_global_db() -> Option<&'static VecDbInner> {
    get_db()
}

/// Index one track. Appends a single journal line (~30 KB for a 1536-dim
/// embedding) instead of rewriting the whole library file, which is what made
/// every song change cost a full-library serialisation.
pub async fn index_track(
    conn: &VecDbInner,
    path: &str,
    title: &str,
    artist: &str,
    album: &str,
    lyrics: &str,
    embedding: &[f32],
) -> Result<()> {
    conn.journal_op(JournalOp::Upsert(TrackEntry {
        path: path.to_string(),
        title: title.to_string(),
        artist: artist.to_string(),
        album: album.to_string(),
        lyrics: lyrics.to_string(),
        embedding: embedding.to_vec(),
    }))
}

pub async fn remove_track(conn: &VecDbInner, path: &str) -> Result<()> {
    conn.journal_op(JournalOp::Remove { path: path.to_string() })
}

pub async fn clear(conn: &VecDbInner) -> Result<()> {
    conn.journal_op(JournalOp::Clear)
}

#[cfg(test)]
pub(crate) fn new_test_vecdb(db_path: &str) -> VecDbInner {
    VecDbInner {
        db_path: db_path.to_string(),
        journal_path: format!("{db_path}.journal"),
        journal_ops: std::sync::Mutex::new(0),
        library: RwLock::new(LibraryFile { version: 1, tracks: Vec::new() }),
    }
}

pub async fn search(conn: &VecDbInner, query_emb: &[f32], k: usize) -> Result<Vec<SearchHit>> {
    Ok(top_k(conn, query_emb, k, |_| true))
}

pub async fn search_lyrics(conn: &VecDbInner, query_emb: &[f32], k: usize) -> Result<Vec<SearchHit>> {
    Ok(top_k(conn, query_emb, k, |t| !t.lyrics.trim().is_empty()))
}

/// Top-k over the in-memory library. Scoring keeps only (similarity, index)
/// pairs behind the read lock, so the 1536-dim embeddings are never cloned;
/// only the k winning entries get their metadata copied afterwards.
fn top_k(
    conn: &VecDbInner,
    query_emb: &[f32],
    k: usize,
    keep: impl Fn(&TrackEntry) -> bool,
) -> Vec<SearchHit> {
    let lib = lock_read(&conn.library);
    let mut scored: Vec<(f32, usize)> = Vec::with_capacity(lib.tracks.len());
    for (i, track) in lib.tracks.iter().enumerate() {
        if !keep(track) {
            continue;
        }
        if track.embedding.is_empty() || track.embedding.len() != query_emb.len() {
            continue;
        }
        let sim = cosine_similarity(&track.embedding, query_emb);
        scored.push((sim, i));
    }
    drop(lib);
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let lib = lock_read(&conn.library);
    let mut hits = Vec::with_capacity(k.min(scored.len()));
    for (sim, i) in scored.into_iter().take(k) {
        if let Some(track) = lib.tracks.get(i) {
            hits.push(SearchHit {
                path: track.path.clone(),
                title: track.title.clone(),
                artist: track.artist.clone(),
                album: track.album.clone(),
                lyrics: track.lyrics.clone(),
                distance: (1.0 - sim).max(0.0),
            });
        }
    }
    hits
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

    /// Unique temp db path (parent dir is created by the caller).
    fn tmp_db(tag: &str) -> String {
        let dir = std::env::temp_dir().join(format!("vecdb-test-{tag}"));
        std::fs::create_dir_all(&dir).ok();
        dir.join("library.json").to_string_lossy().to_string()
    }

    #[test]
    fn test_index_and_search() -> Result<()> {
        let tmp = tmp_db("basic");
        let db = new_test_vecdb(&tmp);
        let emb1 = make_emb();
        let mut emb2 = make_emb();
        emb2[0] *= -1.0;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "Title1", "Artist1", "Album1", "Lyric A", &emb1).await?;
                index_track(&db, "/b.mp3", "Title2", "Artist2", "Album2", "Lyric B", &emb2).await?;
                let hits = search(&db, &emb1, 2).await?;
                assert_eq!(hits.len(), 2);
                assert_eq!(hits[0].path, "/a.mp3");
                assert!(hits[0].distance < hits[1].distance);
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-basic")).ok();
        Ok(())
    }

    /// The regression this module exists for: a mutation must append one journal
    /// line, never rewrite the whole base file. Indexing a second track must
    /// leave the base file byte-identical.
    #[test]
    fn test_index_appends_journal_not_full_rewrite() -> Result<()> {
        let tmp = tmp_db("append");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L1", &emb).await?;
                // Flush so the base file exists and reflects the first track.
                db.flush()?;
                let base_before = std::fs::read(&tmp).map_err(PlayerError::Io)?;

                index_track(&db, "/b.mp3", "T2", "A2", "Al2", "L2", &emb).await?;
                let base_after = std::fs::read(&tmp).map_err(PlayerError::Io)?;
                assert_eq!(
                    base_before, base_after,
                    "base library must not be rewritten on index (that is the perf bug)"
                );
                assert_eq!(db.journal_len(), 1, "second track must be journaled, not folded");

                let journal = std::fs::read_to_string(format!("{tmp}.journal"))
                    .map_err(PlayerError::Io)?;
                assert!(journal.contains("\"path\":\"/b.mp3\""));
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-append")).ok();
        Ok(())
    }

    /// Durability contract: base + journal must reproduce the in-memory state
    /// after a "crash" (process death without flush).
    #[test]
    fn test_journal_replay_recovers_state() -> Result<()> {
        let tmp = tmp_db("replay");
        let emb = make_emb();

        {
            let db = new_test_vecdb(&tmp);
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L1", &emb).await?;
                    db.flush()?;
                    index_track(&db, "/b.mp3", "T2", "A2", "Al2", "L2", &emb).await?;
                    index_track(&db, "/c.mp3", "T3", "A3", "Al3", "L3", &emb).await?;
                    remove_track(&db, "/b.mp3").await?;
                    Ok::<(), PlayerError>(())
                })?;
        }

        // Reload from disk only — the in-memory state above is gone.
        let db = new_test_vecdb(&tmp);
        db.load_or_create()?;
        let paths = lock_read(&db.library)
            .tracks
            .iter()
            .map(|t| t.path.clone())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["/a.mp3".to_string(), "/c.mp3".to_string()]);

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-replay")).ok();
        Ok(())
    }

    /// A torn trailing journal line (crash mid-append) must not break the load.
    #[test]
    fn test_torn_journal_line_is_skipped() -> Result<()> {
        let tmp = tmp_db("torn");
        let emb = make_emb();

        {
            let db = new_test_vecdb(&tmp);
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async {
                    index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L1", &emb).await?;
                    Ok::<(), PlayerError>(())
                })?;
        }
        // Append a truncated line by hand, simulating a crash mid-write.
        {
            use std::io::Write;
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(format!("{tmp}.journal"))
                .map_err(PlayerError::Io)?;
            f.write_all(b"{\"Upsert\":{\"path\":\"/b.mp3\",\"title\":\"T2\"").map_err(PlayerError::Io)?;
        }

        let db = new_test_vecdb(&tmp);
        db.load_or_create()?;
        let paths = lock_read(&db.library)
            .tracks
            .iter()
            .map(|t| t.path.clone())
            .collect::<Vec<_>>();
        assert_eq!(paths, vec!["/a.mp3".to_string()], "torn line must be skipped");

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-torn")).ok();
        Ok(())
    }

    /// Compaction must fold the journal into the base file atomically and leave
    /// no temp file behind.
    #[test]
    fn test_flush_compacts_and_removes_tmp() -> Result<()> {
        let tmp = tmp_db("flush");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L1", &emb).await?;
                assert_eq!(db.journal_len(), 1);
                db.flush()?;
                assert_eq!(db.journal_len(), 0);
                let journal = std::fs::read_to_string(format!("{tmp}.journal"))
                    .map_err(PlayerError::Io)?;
                assert!(journal.is_empty(), "journal must be truncated after flush");
                assert!(
                    !std::path::Path::new(&format!("{tmp}.tmp")).exists(),
                    "atomic temp file must be renamed away"
                );
                let base = std::fs::read_to_string(&tmp).map_err(PlayerError::Io)?;
                assert!(base.contains("/a.mp3"), "flushed base must contain the track");
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-flush")).ok();
        Ok(())
    }

    /// Compaction must trigger automatically once the journal is long enough,
    /// so replay time stays bounded without any caller cooperation.
    #[test]
    fn test_auto_compaction_threshold() -> Result<()> {
        let tmp = tmp_db("autocompact");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                // Fill just below the threshold, then cross it.
                for i in 0..COMPACT_AFTER_OPS {
                    let path = format!("/t{i}.mp3");
                    index_track(&db, &path, "T", "A", "Al", "L", &emb).await?;
                }
                assert_eq!(db.journal_len(), COMPACT_AFTER_OPS);
                index_track(&db, "/over.mp3", "T", "A", "Al", "L", &emb).await?;
                assert_eq!(db.journal_len(), 1, "journal must compact on crossing threshold");
                assert!(std::path::Path::new(&tmp).exists(), "base file written by compaction");
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-autocompact")).ok();
        Ok(())
    }

    /// Upserting the same path must replace, not duplicate (replay is idempotent).
    #[test]
    fn test_upsert_replaces_same_path() -> Result<()> {
        let tmp = tmp_db("upsert");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "Old", "A", "Al", "L", &emb).await?;
                index_track(&db, "/a.mp3", "New", "A", "Al", "L", &emb).await?;
                let hits = search(&db, &emb, 10).await?;
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].title, "New");
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-upsert")).ok();
        Ok(())
    }

    #[test]
    fn test_search_lyrics_excludes_empty() -> Result<()> {
        let tmp = tmp_db("lyrics");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "Some lyrics", &emb).await?;
                index_track(&db, "/b.mp3", "T2", "A2", "Al2", "", &emb).await?;
                let hits = search_lyrics(&db, &emb, 10).await?;
                assert_eq!(hits.len(), 1);
                assert_eq!(hits[0].path, "/a.mp3");
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-lyrics")).ok();
        Ok(())
    }

    #[test]
    fn test_remove() -> Result<()> {
        let tmp = tmp_db("remove");
        let db = new_test_vecdb(&tmp);
        let emb = make_emb();

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async {
                index_track(&db, "/a.mp3", "T1", "A1", "Al1", "L", &emb).await?;
                remove_track(&db, "/a.mp3").await?;
                let hits = search(&db, &emb, 10).await?;
                assert!(hits.is_empty());
                Ok::<(), PlayerError>(())
            })?;

        std::fs::remove_dir_all(std::env::temp_dir().join("vecdb-test-remove")).ok();
        Ok(())
    }
}
