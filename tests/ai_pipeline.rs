//! Integration test for the full AI pipeline:
//! decode audio file → extract embedding → index into LanceDB → nearest-neighbour search.
//!
//! Runs hermetically: the WAV files are generated in a temp dir, and the vector
//! DB is created in another temp dir. Works with or without the DCLAP ONNX
//! model present (extract_embedding falls back to mel mean-pooling).

use std::io::Write;
use std::path::Path;

const SAMPLE_RATE: u32 = 44100;
const EMBEDDING_DIM: usize = 512;

/// Write a mono 16-bit PCM WAV file containing a sine wave.
fn write_sine_wav(path: &Path, freq: f32, seconds: f32) {
    let n = (SAMPLE_RATE as f32 * seconds) as usize;
    let mut samples = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        let v = (2.0 * std::f32::consts::PI * freq * t).sin();
        samples.push((v * i16::MAX as f32) as i16);
    }

    let data_len = (samples.len() * 2) as u32;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    file.write_all(b"WAVE").unwrap();
    file.write_all(b"fmt ").unwrap();
    file.write_all(&16u32.to_le_bytes()).unwrap();
    file.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
    file.write_all(&1u16.to_le_bytes()).unwrap(); // mono
    file.write_all(&SAMPLE_RATE.to_le_bytes()).unwrap();
    file.write_all(&(SAMPLE_RATE * 2).to_le_bytes()).unwrap(); // byte rate
    file.write_all(&2u16.to_le_bytes()).unwrap(); // block align
    file.write_all(&16u16.to_le_bytes()).unwrap(); // bits per sample
    file.write_all(b"data").unwrap();
    file.write_all(&data_len.to_le_bytes()).unwrap();
    for s in samples {
        file.write_all(&s.to_le_bytes()).unwrap();
    }
}

#[tokio::test]
async fn decode_embed_index_search_roundtrip() {
    let audio_dir = tempfile::tempdir().unwrap();
    let a_path = audio_dir.path().join("a.wav");
    let b_path = audio_dir.path().join("b.wav");
    write_sine_wav(&a_path, 440.0, 3.0);
    write_sine_wav(&b_path, 880.0, 3.0);

    // 1. Decode both files to PCM.
    let (samples_a, sr_a) = hm::ai::decode_file(a_path.to_str().unwrap())
        .expect("440Hz wav should decode");
    let (samples_b, sr_b) = hm::ai::decode_file(b_path.to_str().unwrap())
        .expect("880Hz wav should decode");
    assert_eq!(sr_a, SAMPLE_RATE);
    assert_eq!(sr_b, SAMPLE_RATE);

    // 2. Extract embeddings.
    let emb_a = hm::ai::extract_embedding(&samples_a, sr_a);
    let emb_b = hm::ai::extract_embedding(&samples_b, sr_b);
    assert_eq!(emb_a.len(), EMBEDDING_DIM);
    assert_eq!(emb_b.len(), EMBEDDING_DIM);
    assert!(emb_a.iter().any(|&x| x.abs() > 1e-3), "embedding must be non-zero");
    assert!(emb_b.iter().any(|&x| x.abs() > 1e-3), "embedding must be non-zero");

    // 3. Index both tracks with metadata.
    let db_dir = tempfile::tempdir().unwrap();
    let conn = hm::vecdb::open_db(db_dir.path().to_str().unwrap())
        .await
        .unwrap();
    hm::vecdb::index_track(
        &conn,
        a_path.to_str().unwrap(),
        "Tone A",
        "Test Artist",
        "Test Album",
        "sine wave tone a",
        &emb_a,
        &hm::ai::embed_lyrics_text("sine wave tone a"),
    )
    .await
    .unwrap();
    hm::vecdb::index_track(
        &conn,
        b_path.to_str().unwrap(),
        "Tone B",
        "Test Artist",
        "Test Album",
        "sine wave tone b",
        &emb_b,
        &hm::ai::embed_lyrics_text("sine wave tone b"),
    )
    .await
    .unwrap();

    // 4. Search by A's embedding: A must be the nearest hit with its metadata.
    let hits = hm::vecdb::search(&conn, &emb_a, 2).await.unwrap();
    assert_eq!(hits.len(), 2, "both tracks should be returned");
    assert_eq!(hits[0].path, a_path.to_str().unwrap());
    assert_eq!(hits[0].title, "Tone A");
    assert_eq!(hits[0].artist, "Test Artist");
    assert_eq!(hits[0].album, "Test Album");
    assert_eq!(hits[0].lyrics, "sine wave tone a");
    assert!(
        hits[0].distance < hits[1].distance,
        "identical embedding must be closer than the other tone"
    );

    // Lyric-text search returns A as the closest match.
    let lyr_hits = hm::vecdb::search_lyrics(&conn, &hm::ai::embed_lyrics_text("sine wave tone a"), 2)
        .await
        .unwrap();
    assert_eq!(lyr_hits.len(), 2);
    assert_eq!(lyr_hits[0].path, a_path.to_str().unwrap());

    // 5. Remove A: only B remains.
    hm::vecdb::remove_track(&conn, a_path.to_str().unwrap())
        .await
        .unwrap();
    let hits = hm::vecdb::search(&conn, &emb_a, 2).await.unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, b_path.to_str().unwrap());
}
