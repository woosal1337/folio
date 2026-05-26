//! Silero V5 VAD wrapper. Ported from cobanov/autocut's `vad.rs`.
//!
//! Takes 16 kHz mono f32 PCM, returns `SpeechSegment`s in seconds
//! relative to the input buffer. Mirrors silero's reference
//! `get_speech_timestamps` shape:
//!
//!   1. Score each 512-sample chunk (32 ms at 16 kHz) — the Silero V5
//!      mandatory window size.
//!   2. Apply hysteresis: enter "in-speech" when probability ≥
//!      `threshold`, leave when probability < `threshold − 0.15`. The
//!      hysteresis is the critical bit — without it, marginal frames
//!      in the middle of an utterance flicker on/off and produce
//!      false silences. See
//!      `obsidian.md/wiki/voice-activity-detection.md` §7.
//!   3. Group consecutive in-speech chunks into raw regions.
//!   4. Merge regions separated by silence shorter than
//!      `min_silence_ms` so a natural breath in the middle of a
//!      sentence doesn't split it into two.
//!   5. Drop regions shorter than `min_speech_ms` — mic clicks,
//!      throat-clears, plosive bursts that exceed threshold for one
//!      frame but aren't real speech.
//!
//! Padding (`speech_pad_ms`) is intentionally NOT applied here. The
//! caller in `vad_filter` widens the surviving ranges by a fixed
//! `PAD_MS` before writing the speech-only WAV. Keeping pad out of
//! the detector means the user can tune the slider without paying
//! for re-running ONNX inference.
//!
//! Reference: <https://github.com/cobanov/autocut/blob/main/src-tauri/src/vad.rs>
//! Migration plan: `obsidian.md/projects/attune/plan/silero-vad-migration.md`

use voice_activity_detector::VoiceActivityDetector;

use crate::error::{AttuneError, Result};

/// Sample rate Silero V5 expects. The chunk size below is fixed to
/// 512 samples (32 ms at 16 kHz) — Silero V5 will refuse to load with
/// any other combination.
pub const SILERO_SAMPLE_RATE: u32 = 16_000;

/// Mandatory window size for Silero V5 at 16 kHz. Do not change.
const CHUNK_SIZE: usize = 512;

/// Seconds per chunk — used to convert chunk indices back to seconds.
const CHUNK_SECONDS: f64 = CHUNK_SIZE as f64 / SILERO_SAMPLE_RATE as f64;

/// Hysteresis offset below `threshold` for the speech-end transition.
/// Matches silero's reference implementation. 0.15 was tuned by the
/// upstream Silero authors against their evaluation set; deviating
/// from it tends to either bring back the flicker (lower offset) or
/// stretch utterances past their natural end (higher offset).
const HYSTERESIS_OFFSET: f32 = 0.15;

/// Lower bound on the post-hysteresis threshold so a user-set
/// `threshold` close to 0 doesn't produce a negative `neg_threshold`.
const MIN_NEG_THRESHOLD: f32 = 0.05;

#[derive(Debug, Clone, Copy)]
pub struct SileroParams {
    /// Speech-entry probability. 0.0-1.0. Default 0.5. Tuned for
    /// recall (false-negative-averse) over precision — fits meeting
    /// audio where missing 200 ms of a word is more expensive than
    /// keeping 200 ms of breath.
    pub threshold: f32,
    /// Silence gaps shorter than this between two adjacent speech
    /// runs are bridged. Default 100 ms. Set higher to keep more
    /// short breaths inside a single segment; lower to cut more
    /// aggressively.
    pub min_silence_ms: u32,
    /// Speech runs shorter than this are discarded as noise. Default
    /// 150 ms. Lower to keep more very-short interjections ("Yeah."
    /// "Right.") at the cost of letting clicks and plosives through.
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

/// A detected speech segment in seconds, relative to the start of the
/// input buffer passed to `detect`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// Detect speech segments in `samples_16k_mono`.
///
/// Returns segments in source order. Returns `Ok(vec![])` (not an
/// error) when no speech is detected — callers treat zero segments
/// the same way as "no audio".
///
/// Construction cost (loading the embedded ONNX model + initialising
/// the silero session) is amortised inside this call. For batch usage
/// over many recordings, hoist `VoiceActivityDetector::builder()` out
/// to a long-lived struct; for the single-WAV-at-a-time pre-
/// transcription path that the `vad_filter` module drives, the
/// per-call cost is negligible (~ms) compared to the inference itself.
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

    // Step 1+2: per-chunk inference with hysteresis.
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

    // Step 3: contiguous true-runs become (start_chunk, end_chunk) pairs.
    let raw = group_runs(&chunk_is_speech);
    // Step 4: bridge close ranges.
    let merged = merge_close(raw, ms_to_chunks(params.min_silence_ms));
    // Step 5: drop ranges shorter than the speech minimum.
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
        // gap of 1 chunk, min_gap = 2 → merge
        let r = merge_close(vec![(0, 5), (6, 10)], 2);
        assert_eq!(r, vec![(0, 10)]);
    }

    #[test]
    fn merge_close_keeps_long_gap() {
        // gap of 5 chunks, min_gap = 2 → keep separate
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
        // 32 ms = exactly 1 chunk; 33 ms → 2 (ceil).
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
        // 5 s of digital silence at 16 kHz.
        let silence = vec![0.0_f32; SILERO_SAMPLE_RATE as usize * 5];
        let segments = detect(&silence, SileroParams::default()).unwrap();
        assert!(
            segments.is_empty(),
            "5 s of digital silence should yield no speech segments, got {segments:?}"
        );
    }

    #[test]
    fn loud_sine_is_not_speech_so_returns_no_segments() {
        // A 440 Hz sine wave is loud but isn't speech — Silero should
        // reject it. This is the property that distinguishes a real
        // VAD from our previous RMS gate, which would falsely keep
        // music + tones as "speech".
        let tone = loud_sine(SILERO_SAMPLE_RATE as usize * 3, 440, SILERO_SAMPLE_RATE);
        let segments = detect(&tone, SileroParams::default()).unwrap();
        assert!(
            segments.is_empty(),
            "pure sine tone should be rejected by silero, got {segments:?}"
        );
    }

    #[test]
    fn default_params_match_autocut_reference() {
        // We inherited autocut's defaults exactly; this test pins
        // them so a "let me try 0.6 for a sec" tweak can't silently
        // ship without a code review.
        let p = SileroParams::default();
        assert_eq!(p.threshold, 0.5);
        assert_eq!(p.min_silence_ms, 100);
        assert_eq!(p.min_speech_ms, 150);
    }
}
