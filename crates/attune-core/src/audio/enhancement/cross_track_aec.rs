use std::path::Path;
use std::time::Instant;

use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use tracing::{debug, info};

use crate::error::{AttuneError, Result};

const FILTER_TAPS: usize = 512;

const MU: f32 = 0.05;

const EPSILON: f32 = 1e-6;

const XCORR_WINDOW: usize = 2_400;

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

pub fn apply_aec(mic_path: &Path, system_path: &Path, output_path: &Path) -> Result<AecStats> {
    let t0 = Instant::now();

    let (mic_samples, mic_rate) = read_wav_mono(mic_path)?;
    let (ref_samples, ref_rate) = read_wav_mono(system_path)?;

    if mic_samples.is_empty() || ref_samples.is_empty() {
        return Err(AttuneError::Storage("AEC: empty input track".into()));
    }

    if mic_rate != ref_rate {
        tracing::warn!(
            mic_rate,
            ref_rate,
            "cross-track AEC: sample-rate mismatch — results may be degraded"
        );
    }

    let echo_delay = estimate_delay(&mic_samples, &ref_samples, XCORR_WINDOW);
    debug!(echo_delay_samples = echo_delay, "AEC echo delay estimated");

    let ref_delayed: Vec<f32> = std::iter::repeat_n(0.0_f32, echo_delay)
        .chain(ref_samples.iter().copied())
        .take(mic_samples.len())
        .collect();

    let cleaned = nlms_filter(&mic_samples, &ref_delayed, FILTER_TAPS, MU, EPSILON);
    let audio_secs = mic_samples.len() as f64 / mic_rate as f64;

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

fn estimate_delay(mic: &[f32], reference: &[f32], window: usize) -> usize {
    let len = window.min(mic.len()).min(reference.len());
    let max_lag = (len / 4).max(1);

    let mut best_lag = 0usize;
    let mut best_corr = f32::NEG_INFINITY;

    for lag in 0..max_lag {
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

fn nlms_filter(mic: &[f32], reference: &[f32], taps: usize, mu: f32, epsilon: f32) -> Vec<f32> {
    let n = mic.len().min(reference.len());
    let mut weights = vec![0.0_f32; taps];

    let mut delay_line = vec![0.0_f32; taps];
    let mut delay_pos = 0usize;
    let mut output = vec![0.0_f32; n];

    for i in 0..n {
        delay_line[delay_pos] = reference[i];

        let mut y = 0.0_f32;
        let mut ref_power = 0.0_f32;
        for (k, &w) in weights.iter().enumerate().take(taps) {
            let idx = (delay_pos + taps - k) % taps;
            y += w * delay_line[idx];
            ref_power += delay_line[idx] * delay_line[idx];
        }

        let error = mic[i] - y;
        output[i] = error;

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
        let n = 1000;
        let mut reference = vec![0.0_f32; n];
        reference[50] = 1.0;
        reference[200] = 0.8;
        reference[400] = 0.6;
        let lag_true = 10;

        let mic: Vec<f32> = std::iter::repeat_n(0.0_f32, lag_true)
            .chain(reference[..n - lag_true].iter().copied())
            .collect();

        let lag = estimate_delay(&mic, &reference, 500);
        assert!(
            lag >= lag_true - 2 && lag <= lag_true + 2,
            "expected lag ~{lag_true}, got {lag}"
        );
    }

    #[test]
    fn nlms_filter_suppresses_echo_on_clean_reference() {
        let n = 4000;
        let speech_freq = 0.07_f32;
        let echo_freq = 0.13_f32;
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

        let cleaned = nlms_filter(&mic, &reference, 16, 0.01, EPSILON);

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
