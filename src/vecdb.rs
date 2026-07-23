//! Local vector database (LanceDB) for audio embedding similarity search.
//! ponytail: single table "tracks", 256-dim F32 vectors, L2 distance.
//! Upgrade: add metadata columns, IVF-PQ index when scale demands it.

use lancedb::{connect, Connection, Table};
use lancedb::query::Executable;
use std::sync::Arc;

/// Vector search result.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub path: String,
    pub distance: f32,
}

/// Local LanceDB wrapper for track embedding search.
pub struct VecDb {
    conn: Connection,
    /// Cache the table handle after first access.
    table: Option<Arc<Table>>,
}

impl VecDb {
    /// Open or create a vector database at the given directory path.
    pub async fn open(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = connect(db_path).execute().await?;
        Ok(Self { conn, table: None })
    }

    /// Ensure the tracks table exists with the right schema.
    async fn ensure_table(&mut self) -> Result<Arc<Table>, Box<dyn std::error::Error>> {
        if let Some(ref t) = self.table {
            return Ok(t.clone());
        }
        let names = self.conn.table_names().execute().await?;
        let table = if names.iter().any(|n| n == "tracks") {
            self.conn.open_table("tracks").execute().await?
        } else {
            // Create with empty schema — first insert defines the columns
            self.conn
                .create_empty_table("tracks")
                .execute()
                .await?
        };
        let table = Arc::new(table);
        self.table = Some(table.clone());
        Ok(table)
    }

    /// Index a track's embedding. Creates or replaces the row by `path`.
    pub async fn index_track(
        &mut self,
        path: &str,
        embedding: &[f32],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let table = self.ensure_table().await?;

        // ponytail: naive delete-then-insert for upsert.
        // Upgrade: use native merge when lancedb supports it.
        let _ = table.delete(format!("path = '{}'", path.replace('\'', "''"))).await;

        // Build a RecordBatch manually via arrow
        let path_arr = arrow_array::StringArray::from(vec![path]);
        let emb_arr = arrow_array::Float32Array::from(
            embedding.iter().copied().collect::<Vec<f32>>(),
        );
        let emb_len = embedding.len() as i64;
        let emb_fixed = arrow_array::FixedSizeListArray::new(
            arrow_array::types::Float32Type::as_ref(),
            arrow_array::DataType::Float32,
            Some(emb_len),
            vec![Arc::new(emb_arr)],
            None,
        );

        let batch = arrow_array::RecordBatch::try_new(
            arrow_schema::Schema::new(vec![
                arrow_schema::Field::new("path", arrow_schema::DataType::Utf8, false),
                arrow_schema::Field::new(
                    "embedding",
                    arrow_schema::DataType::FixedSizeList(
                        Arc::new(arrow_schema::Field::new("item", arrow_schema::DataType::Float32, true)),
                        embedding.len() as i32,
                    ),
                    false,
                ),
            ]).into(),
            vec![Arc::new(path_arr), Arc::new(emb_fixed)],
        )?;

        table.add(vec![batch]).execute().await?;
        Ok(())
    }

    /// Remove a track's embedding by path.
    pub async fn remove_track(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let table = self.ensure_table().await?;
        table
            .delete(format!("path = '{}'", path.replace('\'', "''")))
            .await?;
        Ok(())
    }

    /// Search for `k` nearest neighbors by embedding similarity (L2).
    pub async fn search(
        &self,
        embedding: &[f32],
        k: usize,
    ) -> Result<Vec<SearchHit>, Box<dyn std::error::Error>> {
        let Some(ref table) = self.table else {
            return Ok(vec![]);
        };
        let results = table
            .search(&arrow_array::Float32Array::from(embedding.to_vec()))
            .limit(k as u32)
            .execute()
            .await?;

        let mut hits = Vec::new();
        while let Some(batch) = results.next().await {
            let batch = batch?;
            if let Some(paths) = batch.column_by_name("path") {
                let paths = paths.as_any().downcast_ref::<arrow_array::StringArray>().unwrap();
                // LanceDB returns distance in _distance column
                let dists = batch
                    .column_by_name("_distance")
                    .and_then(|c| c.as_any().downcast_ref::<arrow_array::Float32Array>());
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_db() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = VecDb::open(dir.path().to_str().unwrap()).await.unwrap();
        let path = "/test/song.flac";
        let emb = vec![0.1f32; 256];
        db.index_track(path, &emb).await.unwrap();
        let hits = db.search(&emb, 5).await.unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].path, path);
        // Same vector should have distance ~0
        assert!(hits[0].distance.abs() < 0.01);
    }

    #[tokio::test]
    async fn test_remove_track() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = VecDb::open(dir.path().to_str().unwrap()).await.unwrap();
        let path = "/test/song.flac";
        db.index_track(path, &vec![0.2f32; 256]).await.unwrap();
        db.remove_track(path).await.unwrap();
        let hits = db.search(&vec![0.2f32; 256], 5).await.unwrap();
        assert!(hits.is_empty());
    }
}
