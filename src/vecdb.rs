//! Local vector database (LanceDB) for audio embedding similarity search.
//! ponytail: single "tracks" table, 256-dim F32 vectors, L2 distance.

use crate::error::{PlayerError, Result};
use arrow_array::{Array, ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use lancedb::query::{ExecutableQuery, QueryBase};
use lancedb::Table;
use std::sync::Arc;

use std::sync::OnceLock;

static VECDB: OnceLock<lancedb::Connection> = OnceLock::new();

/// Vector search result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub distance: f32,
}

const TABLE_NAME: &str = "tracks";
const EMBEDDING_DIM: i32 = 512;

fn track_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, true),
        Field::new("artist", DataType::Utf8, true),
        Field::new("album", DataType::Utf8, true),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                EMBEDDING_DIM,
            ),
            false,
        ),
    ]))
}

fn map_err(e: impl std::fmt::Display) -> PlayerError {
    PlayerError::VecDb(e.to_string())
}

/// Open or create a local LanceDB database at the given directory path.
pub async fn open_db(db_path: &str) -> Result<lancedb::Connection> {
    lancedb::connect(db_path).execute().await.map_err(map_err)
}

/// Get or lazily initialise the global vector DB connection.
pub fn global_db() -> &'static lancedb::Connection {
    VECDB.get().expect("vecdb not initialised; call init_vecdb first")
}

/// Initialise the global vecdb (called once at startup).
pub async fn init_vecdb(db_path: &str) -> Result<()> {
    let conn = open_db(db_path).await?;
    VECDB
        .set(conn)
        .map_err(|_| PlayerError::VecDb("vecdb already initialised".into()))?;
    Ok(())
}

/// Get or create the tracks table.
pub async fn ensure_table(conn: &lancedb::Connection) -> Result<Arc<Table>> {
    let names = conn.table_names().execute().await.map_err(map_err)?;
    let table = if names.iter().any(|n| n == TABLE_NAME) {
        conn.open_table(TABLE_NAME).execute().await.map_err(map_err)?
    } else {
        let empty = make_batch(&[], &[], &[], &[], &[])?;
        conn.create_table(TABLE_NAME, vec![empty])
            .execute()
            .await
            .map_err(map_err)?
    };
    Ok(Arc::new(table))
}

fn make_batch(
    paths: &[&str],
    titles: &[&str],
    artists: &[&str],
    albums: &[&str],
    embeddings: &[&[f32]],
) -> Result<RecordBatch> {
    let path_arr = StringArray::from(paths.to_vec());
    let title_arr = StringArray::from(titles.to_vec());
    let artist_arr = StringArray::from(artists.to_vec());
    let album_arr = StringArray::from(albums.to_vec());

    let values: Vec<f32> = embeddings.iter().flat_map(|e| e.iter().copied()).collect();
    let value_arr = Arc::new(Float32Array::from(values)) as ArrayRef;
    let emb_arr = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        EMBEDDING_DIM,
        value_arr,
        None,
    );

    let batch = RecordBatch::try_new(
        track_schema(),
        vec![
            Arc::new(path_arr),
            Arc::new(title_arr),
            Arc::new(artist_arr),
            Arc::new(album_arr),
            Arc::new(emb_arr),
        ],
    )
    .map_err(map_err)?;
    Ok(batch)
}

/// Index a track's embedding (upsert by path).
pub async fn index_track(
    conn: &lancedb::Connection,
    path: &str,
    title: &str,
    artist: &str,
    album: &str,
    embedding: &[f32],
) -> Result<()> {
    let table = ensure_table(conn).await?;
    let _ = table
        .delete(format!("path = '{}'", path.replace('\'', "''")).as_str())
        .await;
    let batch = make_batch(&[path], &[title], &[artist], &[album], &[embedding])?;
    table.add(vec![batch]).execute().await.map_err(map_err)?;
    Ok(())
}

/// Remove a track's embedding by path.
pub async fn remove_track(conn: &lancedb::Connection, path: &str) -> Result<()> {
    let table = ensure_table(conn).await?;
    table
        .delete(format!("path = '{}'", path.replace('\'', "''")).as_str())
        .await
        .map_err(map_err)?;
    Ok(())
}

/// Search for `k` nearest neighbors by embedding similarity (L2).
pub async fn search(
    conn: &lancedb::Connection,
    embedding: &[f32],
    k: usize,
) -> Result<Vec<SearchHit>> {
    let table = ensure_table(conn).await?;
    let query = table
        .query()
        .nearest_to(embedding)
        .map_err(map_err)?
        .limit(k);
    let mut stream = query.execute().await.map_err(map_err)?;

    let mut hits = Vec::new();
    use futures_util::TryStreamExt;
    while let Some(batch) = stream.try_next().await.map_err(map_err)? {
        let paths = batch
            .column_by_name("path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let titles = batch
            .column_by_name("title")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let artists = batch
            .column_by_name("artist")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let albums = batch
            .column_by_name("album")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let dists = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        if let Some(paths) = paths {
            for i in 0..paths.len() {
                hits.push(SearchHit {
                    path: paths.value(i).to_string(),
                    title: titles.map(|t| t.value(i).to_string()).unwrap_or_default(),
                    artist: artists
                        .map(|a| a.value(i).to_string())
                        .unwrap_or_default(),
                    album: albums.map(|a| a.value(i).to_string()).unwrap_or_default(),
                    distance: dists.map(|d| d.value(i)).unwrap_or(0.0),
                });
            }
        }
    }
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedding(v: f32) -> Vec<f32> {
        vec![v; EMBEDDING_DIM as usize]
    }

    #[tokio::test]
    async fn test_index_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).await.unwrap();

        index_track(&conn, "/a.flac", "Song A", "Artist X", "Album 1", &embedding(0.1))
            .await
            .unwrap();
        index_track(&conn, "/b.flac", "Song B", "Artist Y", "Album 2", &embedding(0.9))
            .await
            .unwrap();

        let hits = search(&conn, &embedding(0.1), 5).await.unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].path, "/a.flac");
        assert_eq!(hits[0].title, "Song A");
        assert_eq!(hits[0].artist, "Artist X");
        assert_eq!(hits[0].album, "Album 1");
        assert!(hits[0].distance.abs() < 0.01);
    }

    #[tokio::test]
    async fn test_remove() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).await.unwrap();

        index_track(&conn, "/x.flac", "Test", "Artist", "Album", &embedding(0.5))
            .await
            .unwrap();
        remove_track(&conn, "/x.flac").await.unwrap();

        let hits = search(&conn, &embedding(0.5), 5).await.unwrap();
        assert!(hits.is_empty());
    }
}
