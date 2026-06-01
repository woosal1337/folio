//! Cross-track acoustic echo cancellation (GET-202).
//!
//! When a user is on speakers (not headphones), the mic re-records the
//! far-end audio from the system track, polluting the "me" channel and
//! degrading both diarization and transcription. This module removes
//! that echo using the system WAV as a reference signal.
//!
//! ## Algorithm
//!
//! **Normalized Least-Mean-Squares (NLMS) adaptive filtering.** The
//! system track is the reference; the mic track is the desired signal
//! mixed with the reference echo. NLMS iteratively identifies and
//! subtracts the echo component:
//!
//! ```text
//! y[n]    = Σ w[k] · ref[n-k]          (filter output ≈ echo estimate)
//! error[n] = mic[n] - y[n]              (residual ≈ clean mic)
//! w[k]    += μ · error · ref[n-k]       (update taps, NLMS-normalized)
//!             / (ε + ‖ref‖²)
//! ```
//!
//! Default filter length: 512 taps at 48 kHz ≈ 10.7 ms. This covers
//! the typical echo delay from the sound card output to the mic, which
//! on macOS varies between 2 and ~30 ms depending on the driver.
//!
//! ## Usage
//!
//! Called as an offline pass after `stop_recording`, analogous to the
//! existing RNNoise pass. Reads `mic.wav` + `system.wav`, writes
//! `mic.aec.wav`. The originals are left untouched; callers can swap the
//! enhanced file in place if the quality improvement is confirmed.

use std::path::Path;
use std::time::Instant;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use tracing::{debug, info};

use crate::error::{AttuneError, Result};

/// Number of NLMS filter taps. 512 taps × (1 / 48 000 Hz) ≈ 10.7 ms.
const FILTER_TAPS: usize = 512;
/// NLMS step size. Smaller = slower convergence, more stable.
const MU: f32 = 0.05;
/// NLMS regularization term — prevents division by zero on silence.
const EPSILON: f32 = 1e-6;
/// Cross-correlation window (samples) for delay estimation.
const XCORR_WINDOW: usize = 2_400; // 50 ms at 48 kHz

pub struct AecStats {
    pub input_mic_samples: u64,
    pub echo_delay_samples: usize,
    pub processing_secs: f64,
    pub audio_secs: f64,
}

impl AecStats {
    pub fn rtf(&self) -> f64 {
        if self.audio_secs == 0.0 {
            0.0
        } else {
            self.processing_secs / self.audio_secs
        }
    }
}

/// Apply cross-track AEC to `mic_path` using `system_path` as the
/// reference signal. Writes the cleaned mic to `output_path`.
///
/// # Errors
///
/// Returns `Err` when either WAV cannot be read or the output cannot
/// be written.
pub fn apply_aec(mic_path: &Path, system_path: &Path, output_path: &Path) -> Result<AecStats> {
    let t0 = Instant::now();

    let (mic_samples, mic_rate) = read_wav_mono(mic_path)?;
    let (ref_samples, ref_rate) = read_wav_mono(system_path)?;

    if mic_samples.is_empty() || ref_samples.is_empty() {
        return Err(AttuneError::Storage("AEC: empty input track".into()));
    }

    // Align rates: both should be 48 kHz from the capture pipeline; warn
    // if they differ but continue (the filter degrades gracefully).
    if mic_rate != ref_rate {
        tracing::warn!(
            mic_rate,
            ref_rate,
            "cross-track AEC: sample-rate mismatch — results may be degraded"
        );
    }

    // Estimate echo delay via cross-correlation over the opening window.
    let echo_delay = estimate_delay(&mic_samples, &ref_samples, XCORR_WINDOW);
    debug!(echo_delay_samples = echo_delay, "AEC echo delay estimated");

    // Shift reference by the estimated delay.
    let ref_delayed: Vec<f32> = std::iter::repeat_n(0.0_f32, echo_delay)
        .chain(ref_samples.iter().copied())
        .take(mic_samples.len())
        .collect();

    // NLMS adaptive filter.
    let cleaned = nlms_filter(&mic_samples, &ref_delayed, FILTER_TAPS, MU, EPSILON);
    let audio_secs = mic_samples.len() as f64 / mic_rate as f64;

    // Write output.
    let spec = WavSpec {
        channels: 1,
        sample_rate: mic_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(output_path, spec)
        .map_err(|e| AttuneError::Storage(format!("AEC write: {e}")))?;
    for s in &cleaned {
        writer
            .write_sample(float_to_i16(*s))
            .map_err(|e| AttuneError::Storage(format!("AEC write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| AttuneError::Storage(format!("AEC finalize: {e}")))?;

    let processing_secs = t0.elapsed().as_secs_f64();
    info!(
        echo_delay_samples = echo_delay,
        audio_secs,
        processing_secs,
        rtf = processing_secs / audio_secs.max(1e-9),
        "cross-track AEC complete"
    );

    Ok(AecStats {
        input_mic_samples: mic_samples.len() as u64,
        echo_delay_samples: echo_delay,
        processing_secs,
        audio_secs,
    })
}

/// Read a WAV file to a mono f32 sample vector. Multi-channel files
/// are downmixed to mono by averaging channels.
fn read_wav_mono(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = WavReader::open(path)
        .map_err(|e| AttuneError::Storage(format!("AEC read {}: {e}", path.display())))?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let rate = spec.sample_rate;

    let samples_raw: Vec<f32> = match spec.sample_format {
        SampleFormat::Int => reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / 32768.0))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| AttuneError::Storage(format!("AEC read samples: {e}")))?,
        SampleFormat::Float => reader
            .samples::<f32>()
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| AttuneError::Storage(format!("AEC read samples: {e}")))?,
    };

    let mono: Vec<f32> = if channels == 1 {
        samples_raw
    } else {
        samples_raw
            .chunks(channels)
            .map(|ch| ch.iter().sum::<f32>() / channels as f32)
            .collect()
    };
    Ok((mono, rate))
}

/// Estimate the echo delay (in samples): how many samples behind `mic`
/// does `reference` trail? In the AEC context this is the time it takes
/// for the system-audio output to physically travel to the microphone.
///
/// Computes corr(lag) = Σ mic[i] · reference[i - lag] for lag ≥ 0,
/// then returns the lag at which the correlation is maximised.
fn estimate_delay(mic: &[f32], reference: &[f32], window: usize) -> usize {
    let len = window.min(mic.len()).min(reference.len());
    let max_lag = (len / 4).max(1);

    let mut best_lag = 0usize;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in 0..max_lag {
        // Σ mic[i + lag] · reference[i]  — positive lag means reference
        // leads mic (system output reaches mic after `lag` samples).
        let mut corr = 0.0_f32;
        for i in 0..len.saturating_sub(lag) {
            corr += mic[i + lag] * reference[i];
        }
        if corr > best_corr {
            best_corr = corr;
            best_lag = lag;
        }
    }
    best_lag
}

/// NLMS adaptive filter with a circular delay-line for the reference.
/// Returns the error signal (clean mic estimate).
fn nlms_filter(mic: &[f32], reference: &[f32], taps: usize, mu: f32, epsilon: f32) -> Vec<f32> {
    let n = mic.len().min(reference.len());
    let mut weights = vec![0.0_f32; taps];
    // Delay line: ring buffer holding the last `taps` reference samples.
    let mut delay_line = vec![0.0_f32; taps];
    let mut delay_pos = 0usize;
    let mut output = vec![0.0_f32; n];

    for i in 0..n {
        // Insert current reference sample into delay line.
        delay_line[delay_pos] = reference[i];

        // Compute filter output: y = w · x (x = delay line in causal order).
        let mut y = 0.0_f32;
        let mut ref_power = 0.0_f32;
        for (k, &w) in weights.iter().enumerate().take(taps) {
            let idx = (delay_pos + taps - k) % taps;
            y += w * delay_line[idx];
            ref_power += delay_line[idx] * delay_line[idx];
        }

        let error = mic[i] - y;
        output[i] = error;

        // NLMS weight update.
        let norm = mu / (epsilon + ref_power);
        for (k, w) in weights.iter_mut().enumerate().take(taps) {
            let idx = (delay_pos + taps - k) % taps;
            *w += norm * error * delay_line[idx];
        }

        delay_pos = (delay_pos + 1) % taps;
    }

    output
}

fn float_to_i16(s: f32) -> i16 {
    (s * 32767.0).clamp(-32768.0, 32767.0) as i16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_delay_finds_zero_lag_on_identical_signals() {
        let sig: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
        let lag = estimate_delay(&sig, &sig, 500);
        assert_eq!(lag, 0);
    }

    #[test]
    fn estimate_delay_finds_positive_lag() {
        // Use a non-periodic pulse train for a clear correlation peak.
        // In AEC: `reference` (system) arrives first; `mic` picks up
        // the echo `lag_true` samples later.
        let n = 1000;
        let mut reference = vec![0.0_f32; n];
        reference[50] = 1.0;
        reference[200] = 0.8;
        reference[400] = 0.6;
        let lag_true = 10;
        // mic = reference delayed by lag_true (echo arrives later).
        let mic: Vec<f32> = std::iter::repeat_n(0.0_f32, lag_true)
            .chain(reference[..n - lag_true].iter().copied())
            .collect();
        // estimate_delay(mic, reference, ..) should return lag_true.
        let lag = estimate_delay(&mic, &reference, 500);
        assert!(
            lag >= lag_true - 2 && lag <= lag_true + 2,
            "expected lag ~{lag_true}, got {lag}"
        );
    }

    #[test]
    fn nlms_filter_suppresses_echo_on_clean_reference() {
        // mic = speech + echo; reference = echo source.
        // Speech and echo use DIFFERENT frequencies so the linear filter
        // can distinguish them (same-frequency mixing is inseparable).
        let n = 4000;
        let speech_freq = 0.07_f32; // ~500 Hz at 48 kHz
        let echo_freq = 0.13_f32; // ~900 Hz at 48 kHz — far-end voice
        let echo_scale = 0.4_f32;

        let speech: Vec<f32> = (0..n)
            .map(|i| (i as f32 * speech_freq).sin() * 0.5)
            .collect();
        let reference: Vec<f32> = (0..n).map(|i| (i as f32 * echo_freq).sin()).collect();
        let mic: Vec<f32> = speech
            .iter()
            .zip(reference.iter())
            .map(|(s, r)| s + r * echo_scale)
            .collect();

        // Use shorter filter + smaller μ for stable convergence in test.
        let cleaned = nlms_filter(&mic, &reference, 16, 0.01, EPSILON);

        // Evaluate over the second half after convergence.
        let half = n / 2;
        let mic_err: f32 = mic[half..]
            .iter()
            .zip(speech[half..].iter())
            .map(|(m, s)| (m - s).powi(2))
            .sum::<f32>();
        let clean_err: f32 = cleaned[half..]
            .iter()
            .zip(speech[half..].iter())
            .map(|(c, s)| (c - s).powi(2))
            .sum::<f32>();
        assert!(
            clean_err < mic_err,
            "AEC should reduce echo after convergence: {clean_err:.3} < {mic_err:.3}"
        );
    }
}
