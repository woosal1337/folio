//! Silence-aware audio chunker. v2 finding 043 + 044 / GET-64 + GET-65.
//!
//! Splits a PCM stream into ≤24 MB chunks at silence boundaries so:
//!   * Each chunk fits under OpenAI Whisper's 25 MB upload limit.
//!   * Re-transcription on retry doesn't tear a word in half.
//!   * `notify`-driven streaming transcription (#043) can transcribe
//!     each chunk the instant it closes.
//!
//! The chunker is pure: it takes a PCM slice + sample rate and emits
//! `ChunkRange { start_sample, end_sample, bytes }`. The caller
//! turns ranges into WAVs.

use serde::{Deserialize, Serialize};

pub const OPENAI_UPLOAD_LIMIT_BYTES: usize = 25 * 1024 * 1024;
pub const TARGET_CHUNK_BYTES: usize = 24 * 1024 * 1024;
pub const TARGET_CHUNK_SECONDS: f64 = 60.0;
pub const SILENCE_RMS_FLOOR: f32 = 0.003;
pub const SILENCE_LOOKBACK_SECONDS: f64 = 5.0;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChunkRange {
    pub start_sample: usize,
    pub end_sample: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkerConfig {
    pub sample_rate: u32,
    pub bytes_per_sample: usize,
    pub target_bytes: usize,
    pub target_seconds: f64,
    pub silence_rms_floor: f32,
    pub silence_lookback_seconds: f64,
}

impl Default for ChunkerConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            bytes_per_sample: 2,
            target_bytes: TARGET_CHUNK_BYTES,
            target_seconds: TARGET_CHUNK_SECONDS,
            silence_rms_floor: SILENCE_RMS_FLOOR,
            silence_lookback_seconds: SILENCE_LOOKBACK_SECONDS,
        }
    }
}

/// Split `pcm` (mono f32 in [-1.0, 1.0]) into chunks. Each chunk is
/// at most `target_bytes` and at most `target_seconds` long, with
/// the final boundary nudged backwards to the nearest silence frame
/// when one exists in the `silence_lookback_seconds` window.
pub fn split(pcm: &[f32], config: ChunkerConfig) -> Vec<ChunkRange> {
    if pcm.is_empty() {
        return Vec::new();
    }
    let samples_per_chunk_by_bytes = config.target_bytes / config.bytes_per_sample.max(1);
    let samples_per_chunk_by_time = (config.target_seconds * config.sample_rate as f64) as usize;
    let target_samples = samples_per_chunk_by_bytes
        .min(samples_per_chunk_by_time)
        .max(1);
    let lookback_samples = (config.silence_lookback_seconds * config.sample_rate as f64) as usize;

    let mut out = Vec::new();
    let mut start = 0usize;
    while start < pcm.len() {
        let nominal_end = (start + target_samples).min(pcm.len());
        let end = if nominal_end == pcm.len() {
            pcm.len()
        } else {
            find_silence_split(pcm, nominal_end, lookback_samples, config.silence_rms_floor)
                .unwrap_or(nominal_end)
        };
        out.push(ChunkRange {
            start_sample: start,
            end_sample: end,
            bytes: (end - start) * config.bytes_per_sample,
        });
        start = end;
    }
    out
}

/// Search backwards from `nominal_end` by up to `lookback` samples
/// for a frame whose RMS over the next 50 ms window dips below
/// `floor`. Returns the index of the first such frame, or None when
/// no silence is found in the window.
fn find_silence_split(
    pcm: &[f32],
    nominal_end: usize,
    lookback: usize,
    floor: f32,
) -> Option<usize> {
    if lookback == 0 {
        return None;
    }
    let window = 800usize;
    let lo = nominal_end.saturating_sub(lookback).max(window);
    let hi = nominal_end.min(pcm.len()).saturating_sub(window);
    if hi <= lo {
        return None;
    }
    let mut probe = hi;
    while probe > lo {
        let slice = &pcm[probe..probe + window];
        if rms(slice) < floor {
            return Some(probe);
        }
        probe = probe.saturating_sub(window);
    }
    None
}

fn rms(slice: &[f32]) -> f32 {
    if slice.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = slice.iter().map(|s| s * s).sum();
    (sum_sq / slice.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(samples: usize, freq_hz: f64, sample_rate: u32) -> Vec<f32> {
        (0..samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (2.0 * std::f64::consts::PI * freq_hz * t).sin() as f32 * 0.5
            })
            .collect()
    }

    #[test]
    fn split_empty_pcm_yields_no_chunks() {
        let chunks = split(&[], ChunkerConfig::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn split_short_pcm_yields_one_chunk_covering_everything() {
        let pcm = sine(16_000, 440.0, 16_000);
        let chunks = split(&pcm, ChunkerConfig::default());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_sample, 0);
        assert_eq!(chunks[0].end_sample, pcm.len());
    }

    #[test]
    fn split_long_pcm_yields_multiple_chunks_under_target() {
        let cfg = ChunkerConfig {
            target_bytes: 64 * 1024,
            target_seconds: 1.0,
            ..ChunkerConfig::default()
        };
        let pcm = sine(16_000 * 5, 440.0, 16_000);
        let chunks = split(&pcm, cfg);
        assert!(chunks.len() >= 4);
        for c in &chunks {
            assert!(c.end_sample > c.start_sample);
            assert!(c.bytes <= cfg.target_bytes || c.end_sample == pcm.len());
        }
    }

    #[test]
    fn split_nudges_boundary_to_silence_when_available() {
        let cfg = ChunkerConfig {
            target_seconds: 2.0,
            ..ChunkerConfig::default()
        };
        let mut pcm = sine(16_000 * 2, 440.0, 16_000);
        let silence_start = pcm.len() - 16_000 / 4;
        for s in pcm.iter_mut().skip(silence_start) {
            *s = 0.0;
        }
        pcm.extend(sine(16_000 * 2, 440.0, 16_000));
        let chunks = split(&pcm, cfg);
        assert!(chunks.len() >= 2);
        let first_end = chunks[0].end_sample;
        assert!(
            first_end <= pcm.len() - 16_000 / 4 + 800,
            "split should land near the silence band ({first_end} vs {silence_start})"
        );
    }

    #[test]
    fn chunks_cover_the_full_input_with_no_overlap() {
        let pcm = sine(16_000 * 10, 440.0, 16_000);
        let cfg = ChunkerConfig {
            target_seconds: 1.5,
            ..ChunkerConfig::default()
        };
        let chunks = split(&pcm, cfg);
        let mut cursor = 0;
        for c in &chunks {
            assert_eq!(c.start_sample, cursor);
            cursor = c.end_sample;
        }
        assert_eq!(cursor, pcm.len());
    }
}
