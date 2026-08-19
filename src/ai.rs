//! AI inference via third-party OpenAI-compatible embedding API.
//!
//! Local ONNX model inference has been removed. All audio and lyric embeddings
//! are produced by calling an OpenAI-compatible `/v1/embeddings` endpoint.
//!
//! Required environment variables:
//!   - `OPENAI_API_KEY`          — API key for the embedding service
//!   - `OPENAI_BASE_URL`        — base URL (default: `https://api.openai.com`)
//!   - `OPENAI_EMBEDDING_MODEL` — model name (default: `text-embedding-3-small`)
//!   - `OPENAI_EMBEDDING_DIM`   — dimension override (default: 1536)
//!
//! Fallback behaviour:
//! - If `OPENAI_API_KEY` is unset, every embedding call returns `None`.
//! - Network failures, HTTP errors, or malformed responses are logged at
//!   `DEBUG` and return `None`.

use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;

// ===== Config from environment =====

#[derive(Debug, Clone)]
struct AiConfig {
    base_url: String,
    api_key: String,
    embedding_model: String,
}

impl AiConfig {
    fn from_env() -> Option<AiConfig> {
        let api_key = env::var("OPENAI_API_KEY").ok()?;
        Some(AiConfig {
            base_url: env::var("OPENAI_BASE_URL")
                .ok()
                .unwrap_or_else(|| "https://api.openai.com".to_string()),
            api_key,
            embedding_model: env::var("OPENAI_EMBEDDING_MODEL")
                .ok()
                .unwrap_or_else(|| "text-embedding-3-small".to_string()),
        })
    }
}

static AI_CONFIG: OnceLock<Option<AiConfig>> = OnceLock::new();

fn ai_config() -> Option<&'static AiConfig> {
    AI_CONFIG
        .get_or_init(|| AiConfig::from_env())
        .as_ref()
}

// ===== Embedding dimension =====

/// Dimension of embeddings produced by the configured third-party model.
pub fn embedding_dim() -> usize {
    env::var("OPENAI_EMBEDDING_DIM")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1536)
}

/// Legacy alias for lyrics embeddings; audio and lyric embeddings now share
/// the same dimension (both are text embeddings).
pub const LYRICS_EMBEDDING_DIM: usize = 1536;

// ===== API types =====

#[derive(Serialize)]
struct EmbeddingRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

// ===== HTTP client (shared) =====

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    CLIENT.get_or_init(|| reqwest::Client::new())
}

// ===== Normalization =====

fn normalize(emb: &mut Vec<f32>) {
    let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in emb.iter_mut() {
            *x /= norm;
        }
    }
}

// ===== Internal: call the API =====

async fn embed_texts_impl(texts: Vec<String>) -> Option<Vec<Vec<f32>>> {
    let config = ai_config()?;
    let client = http_client();

    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());
    let dim = embedding_dim();

    // Send one text at a time (simpler than batched array input;
    // the API accepts a single string). For large batches the caller
    // should call embed_texts_async with reasonable chunk sizes.
    for text in &texts {
        if text.trim().is_empty() {
            all_embeddings.push(vec![0.0f32; dim]);
            continue;
        }

        let body = EmbeddingRequest {
            model: &config.embedding_model,
            input: text,
        };

        let url = format!(
            "{}/v1/embeddings",
            config.base_url.trim_end_matches('/')
        );
        let resp = match client
            .post(&url)
            .bearer_auth(&config.api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("[AI] HTTP request to embedding API failed: {e}");
                return None;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            tracing::debug!(
                "[AI] embedding API returned HTTP {}: {}",
                status,
                body.chars().take(200).collect::<String>()
            );
            return None;
        }

        let result: EmbeddingResponse = match resp.json().await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!("[AI] failed to parse embedding response: {e}");
                return None;
            }
        };

        if let Some(data) = result.data.first() {
            let mut emb = data.embedding.clone();
            emb.resize(dim, 0.0);
            normalize(&mut emb);
            all_embeddings.push(emb);
        } else {
            all_embeddings.push(vec![0.0f32; dim]);
        }
    }

    Some(all_embeddings)
}

// ===== Public API: audio =====

/// Embed an audio file by reading its associated lyrics and embedding them
/// as text via the third-party API. Returns `None` if no lyrics are found
/// or the API is unavailable.
///
/// This replaces the previous DCLAP-based audio embedding. Audio similarity
/// is now performed via lyric text similarity.
pub fn embed_file(path: &str) -> Option<Vec<f32>> {
    crate::runtime().block_on(async { embed_file_async(path.to_string()).await })
}

/// Async version of `embed_file`. Finds lyrics from a sibling `.lrc` file
/// next to the audio path and embeds them via the third-party API.
pub async fn embed_file_async(path: String) -> Option<Vec<f32>> {
    let text = find_lyric_text_from_path(&path)?;
    embed_text_async(text).await
}

/// Try to find plain lyric text for an audio file by looking for a sibling
/// `.lrc` file next to the audio path. Returns `None` if not found.
fn find_lyric_text_from_path(path: &str) -> Option<String> {
    let p = std::path::Path::new(path);
    let (parent, stem) = match (p.parent(), p.file_stem()) {
        (Some(p), Some(s)) => (p, s),
        _ => return None,
    };
    let lrc = parent.join(format!("{}.lrc", stem.to_string_lossy()));
    let lyrics = crate::lyric::load_lyric_file(lrc.to_str().unwrap_or_default()).ok()?;
    let lines: Vec<String> = lyrics
        .lines
        .iter()
        .map(|l| l.text.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("
"))
    }
}

// ===== Public API: text (lyrics) =====

/// Embed lyric text as a vector via the third-party API.
///
/// Returns `None` if the API is unavailable or if the text is empty
/// (empty text returns a zero vector).
pub fn embed_lyrics_text(text: &str) -> Option<Vec<f32>> {
    if text.trim().is_empty() {
        return Some(vec![0.0f32; LYRICS_EMBEDDING_DIM]);
    }
    crate::runtime().block_on(async { embed_lyrics_async(text.to_string()).await })
}

/// Async version of `embed_lyrics_text`.
pub async fn embed_lyrics_async(text: String) -> Option<Vec<f32>> {
    if text.trim().is_empty() {
        return Some(vec![0.0f32; LYRICS_EMBEDDING_DIM]);
    }
    embed_text_async(text).await
}

/// Embed arbitrary text. Returns `None` on any API failure.
pub async fn embed_text_async(text: String) -> Option<Vec<f32>> {
    if text.trim().is_empty() {
        return Some(vec![0.0f32; LYRICS_EMBEDDING_DIM]);
    }
    let results = embed_texts_impl(vec![text]).await?;
    results.into_iter().next()
}

/// Embed multiple texts in a single batched API call.
pub async fn embed_texts_async(texts: Vec<String>) -> Option<Vec<Vec<f32>>> {
    if texts.is_empty() || texts.iter().all(|t| t.trim().is_empty()) {
        return Some(vec![vec![0.0f32; LYRICS_EMBEDDING_DIM]; texts.len()]);
    }
    embed_texts_impl(texts).await
}

// ===== Utilities =====

/// Cosine similarity between two embeddings (assumes both are L2-normalized).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_dim_default() {
        assert_eq!(embedding_dim(), 1536);
    }

    #[test]
    fn test_embedding_dim_from_env() {
        std::env::set_var("OPENAI_EMBEDDING_DIM", "768");
        assert_eq!(embedding_dim(), 768);
        std::env::remove_var("OPENAI_EMBEDDING_DIM");
    }

    #[test]
    fn test_embed_lyrics_empty_text() {
        let emb = embed_lyrics_text("");
        assert!(emb.is_some());
        let emb = emb.unwrap();
        assert_eq!(emb.len(), LYRICS_EMBEDDING_DIM);
        assert!(emb.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_embed_lyrics_whitespace_text() {
        let emb = embed_lyrics_text("   
  ");
        assert!(emb.is_some());
        assert!(emb.unwrap().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_normalize() {
        let mut emb = vec![3.0f32, 4.0f32];
        normalize(&mut emb);
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let mut emb = vec![0.0f32; 10];
        normalize(&mut emb);
        assert!(emb.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![0.6f32, 0.8f32];
        let b = vec![0.6f32, 0.8f32];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0f32, 0.0f32];
        let b = vec![0.0f32, 1.0f32];
        assert!((cosine_similarity(&a, &b) - 0.0).abs() < 1e-6);
    }
}
