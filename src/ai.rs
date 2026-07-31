//! AI inference engine using DCLAP ONNX model for audio feature extraction.
//! ponytail: loads model once via OnceLock, CPU-only inference via tract-onnx.
//! Input: audio PCM → log-mel spectrogram [1, 128, n_frames]
//! Output: 512-dim L2-normalized embedding vector

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::OnceLock;

use rodio::Source;
use rustfft::{FftPlanner, num_complex::Complex};
use tract_onnx::prelude::*;

const N_MELS: usize = 128;
const FFT_SIZE: usize = 2048;
const HOP_SIZE: usize = 480;
const SAMPLE_RATE: u32 = 48000;
const EMBEDDING_DIM: usize = 512;

/// Dimension of lyric text embeddings.
pub const LYRICS_EMBEDDING_DIM: usize = 256;

/// Resolve the model directory: looks for `models/` next to the executable,
/// or in the current working directory.
fn model_dir() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let candidates = [
        exe_dir.clone().map(|d| d.join("models")),
        std::env::current_dir().ok().map(|d| d.join("models")),
    ];
    for d in candidates.into_iter().flatten() {
        if d.join("model_epoch_36.onnx").exists() {
            return d;
        }
    }
    exe_dir.unwrap_or_else(|| PathBuf::from(".")).join("models")
}

/// Global ONNX model, loaded once.
fn model() -> Option<&'static tract_onnx::prelude::TypedRunnableModel<tract_onnx::prelude::TypedModel>> {
    static M: OnceLock<Option<tract_onnx::prelude::TypedRunnableModel<tract_onnx::prelude::TypedModel>>> = OnceLock::new();
    M.get_or_init(|| {
        let dir = model_dir();
        let onnx_path = dir.join("model_epoch_36.onnx");
        if !onnx_path.exists() {
            tracing::warn!("[AI] DCLAP model not found at {}", onnx_path.display());
            return None;
        }
        tracing::info!("[AI] Loading DCLAP model from {}", onnx_path.display());
        let model = tract_onnx::onnx()
            .model_for_path(&onnx_path)
            .map_err(|e| { tracing::error!("[AI] Failed to load model: {}", e); e })
            .ok()?;
        let model = model.into_optimized()
            .map_err(|e| { tracing::error!("[AI] Failed to optimize model: {}", e); e })
            .ok()?
            .into_runnable()
            .map_err(|e| { tracing::error!("[AI] Failed to make model runnable: {}", e); e })
            .ok()?;
        tracing::info!("[AI] DCLAP model loaded successfully");
        Some(model)
    }).as_ref()
}

thread_local! {
    /// Per-thread FFT planner. `plan_fft_forward` returns an owned `Arc<dyn Fft>`,
    /// so the planner itself only needs to be alive while planning; keeping it
    /// thread-local avoids serialising concurrent embedding computations on a
    /// global `Mutex` (previously held for the entire log-mel loop).
    static FFT_PLANNER: std::cell::RefCell<FftPlanner<f32>> =
        std::cell::RefCell::new(FftPlanner::new());
}

/// Plan an FFT of `FFT_SIZE` on the current thread.
fn plan_fft() -> std::sync::Arc<dyn rustfft::Fft<f32>> {
    FFT_PLANNER.with(|p| p.borrow_mut().plan_fft_forward(FFT_SIZE))
}

fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos()))
        .collect()
}

fn mel_filterbank(n_fft: usize, n_mels: usize, sample_rate: u32) -> Vec<Vec<f32>> {
    let n_bins = n_fft / 2 + 1;
    let low_hz = 0.0f32;
    let high_hz = sample_rate as f32 / 2.0;
    let hz_to_mel = |h: f32| 2595.0 * (1.0 + h / 700.0).log10();
    let mel_to_hz = |m: f32| 700.0 * (10.0f32.powf(m / 2595.0) - 1.0);
    let mel_low = hz_to_mel(low_hz);
    let mel_high = hz_to_mel(high_hz);
    let mel_step = (mel_high - mel_low) / (n_mels + 1) as f32;
    let mut filterbank = vec![vec![0.0f32; n_bins]; n_mels];
    for (m, row) in filterbank.iter_mut().enumerate() {
        let center = mel_to_hz(mel_low + (m + 1) as f32 * mel_step);
        let left = if m == 0 { low_hz } else { mel_to_hz(mel_low + m as f32 * mel_step) };
        let right = if m == n_mels - 1 { high_hz } else { mel_to_hz(mel_low + (m + 2) as f32 * mel_step) };
        let center_bin = center / high_hz * (n_bins - 1) as f32;
        let left_bin = left / high_hz * (n_bins - 1) as f32;
        let right_bin = right / high_hz * (n_bins - 1) as f32;
        for (b, fb_cell) in row.iter_mut().enumerate() {
            let fb = b as f32;
            if fb >= left_bin && fb <= center_bin {
                *fb_cell = (fb - left_bin) / (center_bin - left_bin + 1e-10);
            } else if fb > center_bin && fb <= right_bin {
                *fb_cell = (right_bin - fb) / (right_bin - center_bin + 1e-10);
            }
        }
    }
    filterbank
}

/// Compute log-mel spectrogram: returns (data, [n_mels, n_frames])
fn compute_log_mel(samples: &[f32], sample_rate: u32) -> (Vec<f32>, usize, usize) {
    let window = hann(FFT_SIZE);
    let n_frames = (samples.len().saturating_sub(FFT_SIZE)) / HOP_SIZE + 1;
    let n_bins = FFT_SIZE / 2 + 1;
    let filterbank = mel_filterbank(FFT_SIZE, N_MELS, sample_rate);

    let fft = plan_fft();

    let mut mel_data = vec![0.0f32; N_MELS * n_frames];

    for f in 0..n_frames {
        let start = f * HOP_SIZE;
        let end = (start + FFT_SIZE).min(samples.len());
        if end - start < FFT_SIZE { break; }

        let mut buf: Vec<Complex<f32>> = (0..FFT_SIZE)
            .map(|i| Complex::new(samples[start + i] * window[i], 0.0))
            .collect();
        fft.process(&mut buf);

        for m in 0..N_MELS {
            let mut val = 0.0f32;
            for b in 0..n_bins {
                let mag = buf[b].norm_sqr() / FFT_SIZE as f32;
                val += mag * filterbank[m][b];
            }
            mel_data[m * n_frames + f] = (val + 1e-10).ln();
        }
    }

    (mel_data, N_MELS, n_frames)
}

/// Normalize embedding to unit L2 norm.
fn l2_normalize(emb: &mut [f32]) {
    let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 1e-8 {
        for x in emb.iter_mut() {
            *x /= norm;
        }
    }
}

/// Extract a 512-dim audio embedding from raw PCM samples using DCLAP ONNX model.
/// Falls back to mel mean-pooling if model is not available.
pub fn extract_embedding(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    if samples.len() < FFT_SIZE {
        return vec![0.0f32; EMBEDDING_DIM];
    }

    // Resample to 48kHz if needed (DCLAP expects 48kHz)
    let samples = if sample_rate != SAMPLE_RATE {
        let ratio = SAMPLE_RATE as f64 / sample_rate as f64;
        let new_len = (samples.len() as f64 * ratio) as usize;
        let mut resampled = Vec::with_capacity(new_len);
        for i in 0..new_len {
            let src_idx = i as f64 / ratio;
            let idx = src_idx as usize;
            let frac = src_idx - idx as f64;
            let a = samples[idx.min(samples.len() - 1)];
            let b = samples[(idx + 1).min(samples.len() - 1)];
            resampled.push(a + (b - a) * frac as f32);
        }
        resampled
    } else {
        samples.to_vec()
    };

    let (mel_data, n_mels, n_frames) = compute_log_mel(&samples, SAMPLE_RATE);

    if let Some(model) = model() {
        if let Ok(input) = tract_ndarray::Array3::from_shape_vec(
            (1, n_mels, n_frames),
            mel_data.clone(),
        ) {
            let input_tensor: Tensor = input.into();
            if let Ok(result) = model.run(tvec!(input_tensor.into())) {
                if let Some(output) = result.first() {
                    // Fall back to mel pooling if the model output is not a
                    // contiguous f32 array instead of panicking.
                    if let Ok(arr) = output.to_array_view::<f32>() {
                        if let Some(slice) = arr.as_slice() {
                            let mut emb: Vec<f32> = slice.to_vec();
                            emb.truncate(EMBEDDING_DIM);
                            emb.resize(EMBEDDING_DIM, 0.0);
                            l2_normalize(&mut emb);
                            return emb;
                        }
                    }
                }
            }
        }
    }

    // Fallback: mel mean-pooling
    let seg_size = n_frames.div_ceil(4);
    let mut emb = Vec::with_capacity(EMBEDDING_DIM);
    for seg in 0..4 {
        let start = seg * seg_size;
        let end = (start + seg_size).min(n_frames);
        if start >= n_frames {
            emb.extend_from_slice(&[0.0f32; 128]);
            continue;
        }
        let count = (end - start) as f32;
        for m in 0..n_mels {
            let mut sum = 0.0f32;
            for f in start..end {
                sum += mel_data[m * n_frames + f];
            }
            emb.push(sum / count);
        }
    }
    emb.resize(EMBEDDING_DIM, 0.0);
    l2_normalize(&mut emb);
    emb
}

/// Decode an audio file to f32 PCM samples using rodio.
pub fn decode_file(path: &str) -> Option<(Vec<f32>, u32)> {
    let file = std::fs::File::open(path).ok()?;
    let source = rodio::Decoder::new(BufReader::new(file)).ok()?;
    let sample_rate = source.sample_rate();
    let samples: Vec<f32> = source.convert_samples().collect();
    if samples.is_empty() { None } else { Some((samples, sample_rate)) }
}

/// Decode a file and extract its 512-dim embedding in one call.
pub fn embed_file(path: &str) -> Option<Vec<f32>> {
    let (samples, sr) = decode_file(path)?;
    Some(extract_embedding(&samples, sr))
}

/// Async version of `embed_file` — runs heavy DSP + inference on a blocking thread
/// so it doesn't block the tokio worker pool.
pub async fn embed_file_async(path: String) -> Option<Vec<f32>> {
    let task = move || embed_file(&path);
    tokio::task::spawn_blocking(task).await.unwrap_or(None)
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x4E00..=0x9FFF   // CJK Unified Ideographs
        | 0x3400..=0x4DBF // Extension A
        | 0x3040..=0x30FF // Hiragana + Katakana
        | 0xAC00..=0xD7AF // Hangul
    )
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn djb2(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 5381;
    for &b in bytes {
        hash = hash.wrapping_mul(33).wrapping_add(b as u64);
    }
    hash
}

/// Embed lyric text as a deterministic 256-dim bag-of-tokens vector.
///
/// Interim implementation until a real text-embedding ONNX model is integrated:
/// ASCII words + CJK bigrams are hashed into a signed L2-normalized vector, so
/// lexical overlap (shared words/phrases) yields similarity. Empty text returns
/// a zero vector.
pub fn embed_lyrics_text(text: &str) -> Vec<f32> {
    let mut emb = vec![0.0f32; LYRICS_EMBEDDING_DIM];
    if text.trim().is_empty() {
        return emb;
    }

    let mut tokens: Vec<String> = Vec::new();
    let mut word = String::new();
    let mut cjk_run = String::new();
    let flush_cjk = |tokens: &mut Vec<String>, run: &mut String| {
        if run.is_empty() {
            return;
        }
        let chars: Vec<char> = run.chars().collect();
        if chars.len() >= 2 {
            for w in chars.windows(2) {
                tokens.push(w.iter().collect());
            }
        } else {
            tokens.push(chars[0].to_string());
        }
        run.clear();
    };

    for ch in text.to_lowercase().chars() {
        if is_cjk(ch) {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            cjk_run.push(ch);
        } else if ch.is_ascii_alphanumeric() {
            if !cjk_run.is_empty() {
                flush_cjk(&mut tokens, &mut cjk_run);
            }
            word.push(ch);
        } else {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            if !cjk_run.is_empty() {
                flush_cjk(&mut tokens, &mut cjk_run);
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    if !cjk_run.is_empty() {
        flush_cjk(&mut tokens, &mut cjk_run);
    }

    for token in tokens {
        let bytes = token.as_bytes();
        let h1 = fnv1a(bytes) as usize % LYRICS_EMBEDDING_DIM;
        let h2 = djb2(bytes) as usize % LYRICS_EMBEDDING_DIM;
        let sign = if (fnv1a(bytes) >> 63) & 1 == 0 { 1.0 } else { -1.0 };
        emb[h1] += sign;
        emb[h2] += sign;
    }
    l2_normalize(&mut emb);
    emb
}

/// Async version of `embed_lyrics_text` — runs on a blocking thread.
pub async fn embed_lyrics_async(text: String) -> Vec<f32> {
    tokio::task::spawn_blocking(move || embed_lyrics_text(&text))
        .await
        .unwrap_or_else(|_| vec![0.0f32; LYRICS_EMBEDDING_DIM])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_samples() {
        let emb = extract_embedding(&[], 44100);
        assert_eq!(emb.len(), EMBEDDING_DIM);
        assert!(emb.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_sine_wave_embedding() {
        let sr = 48000u32;
        let freq = 440.0;
        let dur = 2.0;
        let n = (sr as f32 * dur) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();
        let emb = extract_embedding(&samples, sr);
        assert_eq!(emb.len(), EMBEDDING_DIM);
        assert!(emb.iter().any(|&x| x.abs() > 0.001));
        // L2 norm should be ~1.0
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "embedding should be L2-normalized, got norm={}", norm);
    }

    #[test]
    fn test_similar_inputs_similar_embeddings() {
        let sr = 48000u32;
        let n = sr as usize * 3;
        let a: Vec<f32> = (0..n).map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin()).collect();
        let b: Vec<f32> = (0..n).map(|i| (2.0 * std::f32::consts::PI * 442.0 * i as f32 / sr as f32).sin()).collect();
        let noise: Vec<f32> = (0..n).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect();

        let emb_a = extract_embedding(&a, sr);
        let emb_b = extract_embedding(&b, sr);
        let emb_n = extract_embedding(&noise, sr);

        let dot_ab: f32 = emb_a.iter().zip(&emb_b).map(|(x, y)| x * y).sum();
        let dot_an: f32 = emb_a.iter().zip(&emb_n).map(|(x, y)| x * y).sum();

        assert!(dot_ab > dot_an, "similar tones (cos={:.3}) should be closer than tone vs noise (cos={:.3})", dot_ab, dot_an);
    }

    #[test]
    fn test_lyrics_embedding_dim_and_norm() {
        let emb = embed_lyrics_text("Hello world, this is a song lyric line.");
        assert_eq!(emb.len(), LYRICS_EMBEDDING_DIM);
        let norm: f32 = emb.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "lyrics embedding should be normalized, got {}", norm);
    }

    #[test]
    fn test_lyrics_embedding_empty_text_is_zero() {
        let emb = embed_lyrics_text("");
        assert!(emb.iter().all(|&x| x == 0.0));
        assert!(embed_lyrics_text("   \n  ").iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_lyrics_embedding_lexical_similarity() {
        let a = embed_lyrics_text("I love you, baby, forever and always");
        let b = embed_lyrics_text("I love you, darling, forever and ever");
        let c = embed_lyrics_text("股市大涨，今天全线飘红，成交量创新高");

        let dot_ab: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let dot_ac: f32 = a.iter().zip(&c).map(|(x, y)| x * y).sum();
        assert!(
            dot_ab > dot_ac,
            "overlapping lyrics (cos={:.3}) should be closer than unrelated (cos={:.3})",
            dot_ab, dot_ac
        );
    }

    #[test]
    fn test_lyrics_embedding_cjk_bigrams() {
        let a = embed_lyrics_text("月亮代表我的心");
        let b = embed_lyrics_text("月亮知道我的心");
        let c = embed_lyrics_text("完全不同的另一首歌");
        let dot_ab: f32 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
        let dot_ac: f32 = a.iter().zip(&c).map(|(x, y)| x * y).sum();
        assert!(dot_ab > dot_ac, "shared CJK bigrams should be closer (ab={:.3}, ac={:.3})", dot_ab, dot_ac);
    }
}
