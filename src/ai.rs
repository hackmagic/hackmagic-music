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

fn fft_planner() -> &'static std::sync::Mutex<FftPlanner<f32>> {
    static P: OnceLock<std::sync::Mutex<FftPlanner<f32>>> = OnceLock::new();
    P.get_or_init(|| std::sync::Mutex::new(FftPlanner::new()))
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
    for m in 0..n_mels {
        let center = mel_to_hz(mel_low + (m + 1) as f32 * mel_step);
        let left = if m == 0 { low_hz } else { mel_to_hz(mel_low + m as f32 * mel_step) };
        let right = if m == n_mels - 1 { high_hz } else { mel_to_hz(mel_low + (m + 2) as f32 * mel_step) };
        let center_bin = center / high_hz * (n_bins - 1) as f32;
        let left_bin = left / high_hz * (n_bins - 1) as f32;
        let right_bin = right / high_hz * (n_bins - 1) as f32;
        for b in 0..n_bins {
            let fb = b as f32;
            if fb >= left_bin && fb <= center_bin {
                filterbank[m][b] = (fb - left_bin) / (center_bin - left_bin + 1e-10);
            } else if fb > center_bin && fb <= right_bin {
                filterbank[m][b] = (right_bin - fb) / (right_bin - center_bin + 1e-10);
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

    let mut planner = fft_planner().lock().unwrap();
    let fft = planner.plan_fft_forward(FFT_SIZE);

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
        let input = tract_ndarray::Array3::from_shape_vec(
            (1, n_mels, n_frames),
            mel_data.clone(),
        ).unwrap();

        let input_tensor: Tensor = input.into();
        if let Ok(result) = model.run(tvec!(input_tensor.into())) {
            if let Some(output) = result.first() {
                let arr = output.to_array_view::<f32>().unwrap();
                let mut emb: Vec<f32> = arr.as_slice().unwrap().to_vec();
                emb.truncate(EMBEDDING_DIM);
                emb.resize(EMBEDDING_DIM, 0.0);
                l2_normalize(&mut emb);
                return emb;
            }
        }
    }

    // Fallback: mel mean-pooling
    let seg_size = (n_frames + 3) / 4;
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
}
