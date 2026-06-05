use voice_activity_detector::VoiceActivityDetector;

use crate::error::{AttuneError, Result};

pub const SILERO_SAMPLE_RATE: u32 = 16_000;

const CHUNK_SIZE: usize = 512;

const CHUNK_SECONDS: f64 = CHUNK_SIZE as f64 / SILERO_SAMPLE_RATE as f64;

const HYSTERESIS_OFFSET: f32 = 0.15;

const MIN_NEG_THRESHOLD: f32 = 0.05;

#[derive(Debug, Clone, Copy)]
pub struct SileroParams {
    pub threshold: f32,

    pub min_silence_ms: u32,

    pub min_speech_ms: u32,
}

impl Default for SileroParams {
    fn default() -> Self {
        Self {
            threshold: 0.5,
            min_silence_ms: 100,
            min_speech_ms: 150,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

pub fn detect(samples_16k_mono: &[f32], params: SileroParams) -> Result<Vec<SpeechSegment>> {
    if samples_16k_mono.is_empty() {
        return Ok(Vec::new());
    }

    let mut vad = VoiceActivityDetector::builder()
        .sample_rate(SILERO_SAMPLE_RATE as i64)
        .chunk_size(CHUNK_SIZE)
        .build()
        .map_err(|e| {
            AttuneError::Transcription(format!("silero: failed to initialise detector: {e}"))
        })?;

    let neg_threshold = (params.threshold - HYSTERESIS_OFFSET).max(MIN_NEG_THRESHOLD);
    let mut in_speech = false;
    let mut chunk_is_speech: Vec<bool> =
        Vec::with_capacity(samples_16k_mono.len() / CHUNK_SIZE + 1);
    for chunk in samples_16k_mono.chunks(CHUNK_SIZE) {
        let prob = vad.predict(chunk.iter().copied());
        if !in_speech && prob >= params.threshold {
            in_speech = true;
        } else if in_speech && prob < neg_threshold {
            in_speech = false;
        }
        chunk_is_speech.push(in_speech);
    }

    let raw = group_runs(&chunk_is_speech);

    let merged = merge_close(raw, ms_to_chunks(params.min_silence_ms));

    let filtered = drop_short(merged, ms_to_chunks(params.min_speech_ms));

    Ok(filtered
        .into_iter()
        .map(|(s, e)| SpeechSegment {
            start_seconds: s as f64 * CHUNK_SECONDS,
            end_seconds: e as f64 * CHUNK_SECONDS,
        })
        .collect())
}

fn ms_to_chunks(ms: u32) -> usize {
    ((ms as f64 / 1000.0) / CHUNK_SECONDS).ceil().max(0.0) as usize
}

fn group_runs(flags: &[bool]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, &is_speech) in flags.iter().enumerate() {
        match (start, is_speech) {
            (None, true) => start = Some(i),
            (Some(s), false) => {
                out.push((s, i));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push((s, flags.len()));
    }
    out
}

fn merge_close(regions: Vec<(usize, usize)>, min_gap: usize) -> Vec<(usize, usize)> {
    let mut merged: Vec<(usize, usize)> = Vec::with_capacity(regions.len());
    for (s, e) in regions {
        if let Some(last) = merged.last_mut() {
            if s.saturating_sub(last.1) < min_gap {
                last.1 = e.max(last.1);
                continue;
            }
        }
        merged.push((s, e));
    }
    merged
}

fn drop_short(regions: Vec<(usize, usize)>, min_len: usize) -> Vec<(usize, usize)> {
    regions
        .into_iter()
        .filter(|(s, e)| e.saturating_sub(*s) >= min_len)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    fn loud_sine(samples: usize, freq_hz: u32, sample_rate: u32) -> Vec<f32> {
        (0..samples)
            .map(|i| (2.0 * PI * freq_hz as f32 * i as f32 / sample_rate as f32).sin() * 0.5)
            .collect()
    }

    #[test]
    fn group_runs_basic() {
        let flags = vec![false, true, true, false, false, true, false];
        assert_eq!(group_runs(&flags), vec![(1, 3), (5, 6)]);
    }

    #[test]
    fn group_runs_trailing_speech() {
        let flags = vec![false, true, true];
        assert_eq!(group_runs(&flags), vec![(1, 3)]);
    }

    #[test]
    fn merge_close_combines_short_gap() {
        let r = merge_close(vec![(0, 5), (6, 10)], 2);
        assert_eq!(r, vec![(0, 10)]);
    }

    #[test]
    fn merge_close_keeps_long_gap() {
        let r = merge_close(vec![(0, 5), (10, 15)], 2);
        assert_eq!(r, vec![(0, 5), (10, 15)]);
    }

    #[test]
    fn drop_short_filters_below_min() {
        let r = drop_short(vec![(0, 3), (10, 20)], 5);
        assert_eq!(r, vec![(10, 20)]);
    }

    #[test]
    fn ms_to_chunks_rounds_up() {
        assert_eq!(ms_to_chunks(32), 1);
        assert_eq!(ms_to_chunks(33), 2);
        assert_eq!(ms_to_chunks(0), 0);
    }

    #[test]
    fn empty_input_returns_no_segments() {
        let segments = detect(&[], SileroParams::default()).unwrap();
        assert!(segments.is_empty());
    }

    #[test]
    fn pure_silence_returns_no_segments() {
        let silence = vec![0.0_f32; SILERO_SAMPLE_RATE as usize * 5];
        let segments = detect(&silence, SileroParams::default()).unwrap();
        assert!(
            segments.is_empty(),
            "5 s of digital silence should yield no speech segments, got {segments:?}"
        );
    }

    #[test]
    fn loud_sine_is_not_speech_so_returns_no_segments() {
        let tone = loud_sine(SILERO_SAMPLE_RATE as usize * 3, 440, SILERO_SAMPLE_RATE);
        let segments = detect(&tone, SileroParams::default()).unwrap();
        assert!(
            segments.is_empty(),
            "pure sine tone should be rejected by silero, got {segments:?}"
        );
    }

    #[test]
    fn default_params_match_autocut_reference() {
        let p = SileroParams::default();
        assert_eq!(p.threshold, 0.5);
        assert_eq!(p.min_silence_ms, 100);
        assert_eq!(p.min_speech_ms, 150);
    }
}
