//! AI inference engine using candle for audio feature extraction.
//! ponytail: single file, CPU-only. Embedding = mel spectrogram time slices.
//! Upgrade path: load CLAP/music2vec via candle for learned embeddings.

use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::OnceLock;

fn fft_planner() -> &'static std::sync::Mutex<FftPlanner<f32>> {
    static P: OnceLock<std::sync::Mutex<FftPlanner<f32>>> = OnceLock::new();
    P.get_or_init(|| std::sync::Mutex::new(FftPlanner::new()))
}

/// Build a Hann window of length `n`.
fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (n - 1) as f32).cos()))
        .collect()
}

/// Build mel filterbank: `n_mels` triangular filters for `n_fft/2+1` bins.
/// Returns shape (n_mels, n_bins).
fn mel_filterbank(n_fft: usize, n_mels: usize, sample_rate: u32) -> Vec<Vec<f32>> {
    let n_bins = n_fft / 2 + 1;
    let low_hz = 0.0f32;
    let high_hz = sample_rate as f32 / 2.0;

    // Convert to mel scale
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

/// Extract a 256-dim audio embedding from raw PCM samples.
/// Strategy: 64 mel bands × 4 time segments (mean-pooled) = 256.
pub fn extract_embedding(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let win_size = 1024;
    let hop = 512;
    let n_mels = 64;

    if samples.len() < win_size {
        return vec![0.0f32; 256];
    }

    let window = hann(win_size);
    let n_frames = (samples.len() - win_size) / hop + 1;
    let n_bins = win_size / 2 + 1;

    // Build FFT
    let mut planner = fft_planner().lock().unwrap();
    let fft = planner.plan_fft_forward(win_size);

    // Compute power spectrum per frame
    let mut power_spectra: Vec<Vec<f32>> = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let start = f * hop;
        let mut buf: Vec<Complex<f32>> = (0..win_size)
            .map(|i| Complex::new(samples[start + i] * window[i], 0.0))
            .collect();
        fft.process(&mut buf);

        let mut power = Vec::with_capacity(n_bins);
        for b in 0..n_bins {
            let mag = buf[b].norm_sqr();
            power.push(mag / win_size as f32);
        }
        power_spectra.push(power);
    }

    // Build mel filterbank once
    let filterbank = mel_filterbank(win_size, n_mels, sample_rate);

    // Apply filterbank → log-mel per frame
    let mut mel_frames: Vec<Vec<f32>> = Vec::with_capacity(n_frames);
    for f in 0..n_frames {
        let mut mel = vec![0.0f32; n_mels];
        for m in 0..n_mels {
            let mut val = 0.0f32;
            for b in 0..n_bins {
                val += power_spectra[f][b] * filterbank[m][b];
            }
            mel[m] = (val + 1e-10).ln();
        }
        mel_frames.push(mel);
    }

    if mel_frames.is_empty() {
        return vec![0.0f32; 256];
    }

    // Split time into 4 segments, average mel bands per segment
    let seg_size = (n_frames + 3) / 4; // ceiling division
    let mut emb = Vec::with_capacity(256);
    for seg in 0..4 {
        let start = seg * seg_size;
        let end = (start + seg_size).min(n_frames);
        if start >= n_frames {
            emb.extend_from_slice(&[0.0f32; 64]);
            continue;
        }
        let count = (end - start) as f32;
        for m in 0..n_mels {
            let mut sum = 0.0f32;
            for f in start..end {
                sum += mel_frames[f][m];
            }
            emb.push(sum / count);
        }
    }

    debug_assert_eq!(emb.len(), 256);
    emb
}

// ponytail: test with real FFT path.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_samples() {
        let emb = extract_embedding(&[], 44100);
        assert_eq!(emb.len(), 256);
        assert!(emb.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_short_samples() {
        let emb = extract_embedding(&[0.0f32; 100], 44100);
        assert_eq!(emb.len(), 256);
    }

    #[test]
    fn test_sine_wave_embedding() {
        let sr = 44100u32;
        let freq = 440.0;
        let dur = 1.0; // 1 second
        let n = (sr as f32 * dur) as usize;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();
        let emb = extract_embedding(&samples, sr);
        assert_eq!(emb.len(), 256);
        // Should have non-zero energy (sine wave is not silence)
        assert!(emb.iter().any(|&x| x.abs() > 0.01));
    }

    #[test]
    fn test_similar_inputs_similar_embeddings() {
        let sr = 44100u32;
        let n = sr as usize;
        let a: Vec<f32> = (0..n).map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin()).collect();
        let b: Vec<f32> = (0..n).map(|i| (2.0 * std::f32::consts::PI * 442.0 * i as f32 / sr as f32).sin()).collect();
        let noise: Vec<f32> = (0..n).map(|_| rand::random::<f32>() * 2.0 - 1.0).collect();

        let emb_a = extract_embedding(&a, sr);
        let emb_b = extract_embedding(&b, sr);
        let emb_n = extract_embedding(&noise, sr);

        // Cosine similarity between A and B should be higher than A and noise
        let dot_ab: f32 = emb_a.iter().zip(&emb_b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = emb_a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = emb_b.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_n: f32 = emb_n.iter().map(|x| x * x).sum::<f32>().sqrt();
        let sim_ab = dot_ab / (norm_a * norm_b + 1e-10);
        let dot_an: f32 = emb_a.iter().zip(&emb_n).map(|(x, y)| x * y).sum();
        let sim_an = dot_an / (norm_a * norm_n + 1e-10);

        assert!(sim_ab > sim_an, "similar tones should be closer than tone vs noise");
    }

    #[test]
    fn test_mel_filterbank_shape() {
        let fb = mel_filterbank(1024, 64, 44100);
        assert_eq!(fb.len(), 64);
        assert_eq!(fb[0].len(), 513);
    }
}
