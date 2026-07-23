//! Local vector database (LanceDB) for audio embedding similarity search.
//! ponytail: single "tracks" table, 256-dim F32 vectors, L2 distance.

use arrow_array::{Array, ArrayRef, FixedSizeListArray, Float32Array, RecordBatch, StringArray};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use lancedb::query::{ExecutableQuery, IntoQueryVector, QueryBase};
use lancedb::Table;
use std::sync::Arc;

use lancedb::data::scannable::Scannable;
use std::sync::OnceLock;

static VECDB: OnceLock<lancedb::Connection> = OnceLock::new();

/// Vector search result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub distance: f32,
}

const TABLE_NAME: &str = "tracks";
const EMBEDDING_DIM: i32 = 256;

fn track_schema() -> SchemaRef {
    Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, false),
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

/// Open or create a local LanceDB database at the given directory path.
pub async fn open_db(db_path: &str) -> Result<lancedb::Connection, Box<dyn std::error::Error>> {
    let conn = lancedb::connect(db_path).execute().await?;
    Ok(conn)
}

/// Get or lazily initialise the global vector DB connection.
/// Uses the configured vecdb_path from config, or a default path.
pub fn global_db() -> &'static lancedb::Connection {
    VECDB.get().expect("vecdb not initialised; call init_vecdb first")
}

/// Initialise the global vecdb (called once at startup).
pub async fn init_vecdb(db_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let conn = open_db(db_path).await?;
    VECDB.set(conn).map_err(|_| "vecdb already initialised".into())?;
    Ok(unsafe { std::mem::zeroed() }) // unreachable — set returns Err on re-init
    // ponytail: ignore the unreachable line artefact, set() succeeds first time
}

/// Get or create the tracks table.
pub async fn ensure_table(
    conn: &lancedb::Connection,
) -> Result<Arc<Table>, Box<dyn std::error::Error>> {
    let names = conn.table_names().execute().await?;
    let table = if names.iter().any(|n| n == TABLE_NAME) {
        conn.open_table(TABLE_NAME).execute().await?
    } else {
        let empty = make_batch(&[], &[])?;
        conn.create_table(TABLE_NAME, vec![empty])
            .execute()
            .await?
    };
    Ok(Arc::new(table))
}

fn make_batch(
    paths: &[&str],
    embeddings: &[&[f32]],
) -> Result<RecordBatch, Box<dyn std::error::Error>> {
    let path_arr = StringArray::from(paths.to_vec());

    let values: Vec<f32> = embeddings.iter().flat_map(|e| e.iter().copied()).collect();
    let value_arr = Arc::new(Float32Array::from(values)) as ArrayRef;
    let emb_arr = FixedSizeListArray::new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        EMBEDDING_DIM,
        value_arr,
        None,
    );

    let batch =
        RecordBatch::try_new(track_schema(), vec![Arc::new(path_arr), Arc::new(emb_arr)])?;
    Ok(batch)
}

/// Index a track's embedding (upsert by path).
pub async fn index_track(
    conn: &lancedb::Connection,
    path: &str,
    embedding: &[f32],
) -> Result<(), Box<dyn std::error::Error>> {
    let table = ensure_table(conn).await?;
    let _ = table
        .delete(format!("path = '{}'", path.replace('\'', "''")).as_str())
        .await;
    let batch = make_batch(&[path], &[embedding])?;
    table.add(vec![batch]).execute().await?;
    Ok(())
}

/// Remove a track's embedding by path.
pub async fn remove_track(
    conn: &lancedb::Connection,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let table = ensure_table(conn).await?;
    table
        .delete(format!("path = '{}'", path.replace('\'', "''")).as_str())
        .await?;
    Ok(())
}

/// Search for `k` nearest neighbors by embedding similarity (L2).
pub async fn search(
    conn: &lancedb::Connection,
    embedding: &[f32],
    k: usize,
) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
    let table = ensure_table(conn).await?;
    let query = table.query().nearest_to(embedding)?.limit(k);
    let mut stream = query.execute().await?;

    let mut hits = Vec::new();
    use futures_util::TryStreamExt;
    while let Some(batch) = stream.try_next().await? {
        let paths = batch
            .column_by_name("path")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let dists = batch
            .column_by_name("_distance")
            .and_then(|c| c.as_any().downcast_ref::<Float32Array>());
        if let Some(paths) = paths {
            for i in 0..paths.len() {
                hits.push(SearchHit {
                    path: paths.value(i).to_string(),
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

        index_track(&conn, "/a.flac", &embedding(0.1)).await.unwrap();
        index_track(&conn, "/b.flac", &embedding(0.9)).await.unwrap();

        let hits = search(&conn, &embedding(0.1), 5).await.unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].path, "/a.flac");
        assert!(hits[0].distance.abs() < 0.01);
    }

    #[tokio::test]
    async fn test_remove() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_db(dir.path().to_str().unwrap()).await.unwrap();

        index_track(&conn, "/x.flac", &embedding(0.5)).await.unwrap();
        remove_track(&conn, "/x.flac").await.unwrap();

        let hits = search(&conn, &embedding(0.5), 5).await.unwrap();
        assert!(hits.is_empty());
    }
}
